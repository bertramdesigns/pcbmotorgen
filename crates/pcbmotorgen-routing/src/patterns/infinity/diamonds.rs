//! Diamond / period geometry engine for the infinity braid (ported from pcbBraid).

use crate::model::Point;

use super::peaks_valleys::{calc_peaks, calc_valleys};

/// `diamondlist[N][k][P]` — period N, diamond k, vertex P.
pub(crate) type DiamondList = Vec<Vec<Vec<Point>>>;

pub(crate) fn compute_diamonds(
    start_offset: f64,
    d: f64,
    n_periods: i64,
    num_strands: i64,
    o: f64,
    a: f64,
) -> DiamondList {
    let omega = (std::f64::consts::PI * n_periods as f64) / d;
    let period_length = d / n_periods as f64;
    let half_width = period_length / 2.0;

    let mut out = Vec::with_capacity(n_periods as usize);
    for n in 0..n_periods {
        let period_offset = n as f64 * period_length;
        let mut period = Vec::with_capacity(num_strands as usize);
        for k in 0..num_strands {
            let x_c = (std::f64::consts::PI / 2.0) / omega - k as f64 * o + period_offset + start_offset;
            // Offset diamond y by +a to map its native range [-a, +a] onto the
            // board width [0, 2a] = [0, board_width] (a = board_width / 2).
            period.push(vec![
                Point::new(x_c, 2.0 * a),           // P0 top    (+a → +2a)
                Point::new(x_c + half_width, a),    // P1 right  (0 → +a)
                Point::new(x_c, 0.0),               // P2 bottom (-a → 0)
                Point::new(x_c - half_width, a),    // P3 left   (0 → +a)
            ]);
        }
        out.push(period);
    }
    out
}

/// Per-period endpoint object: left/right/peaks/valleys + flattened list.
pub(crate) fn compute_endpoints(diamondlist: &DiamondList) -> (Vec<PeriodEndpoints>, Vec<Point>) {
    let n_periods = diamondlist.len();
    let mut obj = vec![PeriodEndpoints::default(); n_periods];
    let mut flat = Vec::new();

    for n in 0..n_periods {
        let dlist = &diamondlist[n];
        let peaks = calc_peaks(dlist);
        let valleys = calc_valleys(dlist);

        if n == 0 {
            let left: Vec<Point> = dlist.iter().map(|pt| pt[3]).collect();
            obj[n].left = left.clone();
            flat.extend(left);
        }
        if n == n_periods - 1 {
            let right: Vec<Point> = dlist.iter().map(|pt| pt[1]).collect();
            obj[n].right = right.clone();
            flat.extend(right);
        }
        obj[n].peaks = peaks.clone();
        flat.extend(peaks);
        obj[n].valleys = valleys.clone();
        flat.extend(valleys);
    }

    (obj, flat)
}

/// Return one pole-pitch region per diamond period.
///
/// Point 1 is the left-hand vertex and point 3 is the right-hand vertex of a
/// diamond. At the boundary between two pole periods, the rightmost point-3
/// of the preceding period and the leftmost point-1 of the following period
/// define the boundary. The boundary is their midpoint, which is shared as
/// the end of one region and the start of the next. In particular, do not use
/// point 0 (the top vertex) as a pole boundary.
pub(crate) fn compute_pole_region_xs(diamondlist: &DiamondList) -> Vec<(f64, f64)> {
    if diamondlist.is_empty() {
        return Vec::new();
    }

    let mut internal_boundaries = Vec::with_capacity(diamondlist.len().saturating_sub(1));
    for periods in diamondlist.windows(2) {
        let previous_period_point_3 = periods[0].last().map(|diamond| diamond[3].x);
        let next_period_point_1 = periods[1].first().map(|diamond| diamond[1].x);
        if let (Some(right), Some(left)) = (previous_period_point_3, next_period_point_1) {
            internal_boundaries.push((right + left) / 2.0);
        }
    }

    let first = &diamondlist[0];
    let last = &diamondlist[diamondlist.len() - 1];
    let mut boundaries = Vec::with_capacity(diamondlist.len() + 1);
    if internal_boundaries.is_empty() {
        // A single period has no neighboring median. Use the lateral width of
        // that period so the sole region still has the same centered treatment
        // as a normal pole-pitch region.
        let left = first.first().map(|diamond| diamond[1].x).unwrap_or(0.0);
        let right = last.last().map(|diamond| diamond[3].x).unwrap_or(left);
        let width = (right - left).abs();
        let center = (left + right) / 2.0;
        boundaries.extend([center - width / 2.0, center + width / 2.0]);
    } else {
        // The first/last regions are visual pole regions, not clipped board
        // remnants. Extrapolate the same width as the neighboring median
        // interval so every region is equal width.
        let fallback_width = || {
            diamondlist
                .windows(2)
                .find_map(|periods| {
                    Some((periods[1].first()?[0].x - periods[0].first()?[0].x).abs())
                })
                .filter(|width| *width > 0.0)
                .unwrap_or(1.0)
        };
        let first_width = if internal_boundaries.len() >= 2 {
            internal_boundaries[1] - internal_boundaries[0]
        } else {
            fallback_width()
        };
        let last_width = if internal_boundaries.len() >= 2 {
            internal_boundaries[internal_boundaries.len() - 1]
                - internal_boundaries[internal_boundaries.len() - 2]
        } else {
            fallback_width()
        };
        boundaries.push(internal_boundaries[0] - first_width);
        boundaries.extend(internal_boundaries.iter().copied());
        boundaries.push(internal_boundaries[internal_boundaries.len() - 1] + last_width);
    }
    boundaries
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PeriodEndpoints {
    pub(crate) left: Vec<Point>,
    pub(crate) right: Vec<Point>,
    pub(crate) peaks: Vec<Point>,
    pub(crate) valleys: Vec<Point>,
}

/// Python-style negative index wrap into a slice.
pub(crate) fn wrap_idx(len: usize, idx: i64) -> usize {
    let l = len as i64;
    let i = if idx < 0 { l + idx } else { idx };
    i as usize
}

/// Intersection of segments (p1→p2) and (p3→p4), or None if parallel / outside.
pub(crate) fn crossing(p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
    let (x1, y1) = (p1.x, p1.y);
    let (x2, y2) = (p2.x, p2.y);
    let (x3, y3) = (p3.x, p3.y);
    let (x4, y4) = (p4.x, p4.y);

    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if denom.abs() < 1e-15 {
        return None;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(Point::new(x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
    } else {
        None
    }
}
