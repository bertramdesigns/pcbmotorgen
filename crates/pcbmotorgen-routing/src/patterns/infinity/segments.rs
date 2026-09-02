//! Per-layer segment builders for the infinity braid (ported from pcbBraid).

use crate::model::Point;

use super::diamonds::PeriodEndpoints;
use super::peaks_valleys::{bottom_valleys, top_peaks};

pub(crate) fn compute_top_layer_segments(obj: &[PeriodEndpoints]) -> Vec<(Point, Point)> {
    let n_periods = obj.len();
    let left = obj[0].left.clone();
    let right = obj[n_periods - 1].right.clone();
    let mut segs = Vec::new();
    for n in 0..n_periods {
        let peaks = &obj[n].peaks;
        let valleys = &obj[n].valleys;
        let next_valleys = if n < n_periods - 1 { &obj[n + 1].valleys } else { &EMPTY[..0] };
        segs.extend(top_peaks(peaks));
        segs.extend(bottom_valleys(valleys));
        if n == 0 {
            segs.extend(left_to_valley(&left, valleys));
            segs.extend(peak_to_next_valley(peaks, next_valleys));
        } else if n == n_periods - 1 {
            segs.extend(peak_to_right(peaks, &right));
        } else {
            segs.extend(peak_to_next_valley(peaks, next_valleys));
        }
    }
    segs
}

pub(crate) fn compute_bottom_layer_segments(obj: &[PeriodEndpoints]) -> Vec<(Point, Point)> {
    let n_periods = obj.len();
    let left = obj[0].left.clone();
    let right = obj[n_periods - 1].right.clone();
    let mut segs = Vec::new();
    for n in 0..n_periods {
        let peaks = &obj[n].peaks;
        let valleys = &obj[n].valleys;
        let next_peaks = if n < n_periods - 1 { &obj[n + 1].peaks } else { &EMPTY[..0] };
        segs.extend(top_peaks(peaks));
        segs.extend(bottom_valleys(valleys));
        if n == 0 {
            segs.extend(left_to_peak(&left, peaks));
            segs.extend(valley_to_next_peak(valleys, next_peaks));
        } else if n == n_periods - 1 {
            segs.extend(valley_to_right(valleys, &right));
        } else {
            segs.extend(valley_to_next_peak(valleys, next_peaks));
        }
    }
    segs
}

static EMPTY: [Point; 0] = [];

pub(crate) fn peak_to_next_valley(peaks: &[Point], next_valleys: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = peaks.len();
    for i in 0..(n / 2 + 1) {
        let start_idx = i + n / 2 - 1;
        segs.push((peaks[start_idx], next_valleys[i]));
    }
    segs
}

pub(crate) fn valley_to_next_peak(valleys: &[Point], next_peaks: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = valleys.len();
    for i in 0..(n / 2) {
        let start_idx = i + n / 2;
        segs.push((valleys[start_idx], next_peaks[i]));
    }
    // Python used `valleys[-1], next_peaks[-1]` — the LAST element of each,
    // regardless of differing lengths.
    segs.push((valleys[n - 1], next_peaks[next_peaks.len() - 1]));
    segs
}

pub(crate) fn left_to_peak(left: &[Point], peaks: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = left.len();
    for i in 0..(n - 1) {
        segs.push((left[i], peaks[i]));
    }
    // Python: `left[-1], peaks[-1]` — the LAST left point connects to the LAST
    // peak (peaks is longer than left, e.g. len(peaks) == 2*(n-1)), not the
    // nth peak which would be "next in line".
    segs.push((left[n - 1], peaks[peaks.len() - 1]));
    segs
}

pub(crate) fn left_to_valley(left: &[Point], valleys: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = left.len();
    for i in 0..n {
        segs.push((left[i], valleys[i]));
    }
    segs
}

pub(crate) fn peak_to_right(peaks: &[Point], right: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = right.len();
    let peaks_rev: Vec<Point> = peaks.iter().rev().copied().collect();
    let right_rev: Vec<Point> = right.iter().rev().copied().collect();
    for i in 0..n {
        segs.push((peaks_rev[i], right_rev[i]));
    }
    segs
}

pub(crate) fn valley_to_right(valleys: &[Point], right: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = right.len();
    let valleys_rev: Vec<Point> = valleys.iter().rev().copied().collect();
    let right_rev: Vec<Point> = right.iter().rev().copied().collect();
    for i in 0..n {
        segs.push((valleys_rev[i], right_rev[i]));
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RoutingContext;
    use crate::model::Point;
    use crate::patterns::infinity::diamonds::{compute_diamonds, compute_endpoints};
    use std::collections::HashMap;

    fn ctx() -> RoutingContext {
        let mut params = HashMap::new();
        params.insert("num_strands".to_string(), 5.0);
        params.insert("n_periods".to_string(), 4.0);
        RoutingContext {
            active_area_length_mm: 600.0,
            board_width_mm: 20.0,
            num_layers: 2,
            phases: 3,
            min_trace_mm: 0.1,
            min_space_mm: 0.1,
            expects_continuous: false,
            params,
            ..RoutingContext::default()
        }
    }

    #[test]
    fn left_to_peak_connects_last_left_to_last_peak() {
        // Regression: the last left point (P3 of the last first-period diamond)
        // must connect to the LAST peak, not the "next in line" peak at index
        // left.len()-1. peaks is longer than left (len(peaks) = 2*(n-1)).
        let left = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.25),
            Point::new(0.0, 0.5),
            Point::new(0.0, 0.75),
        ];
        let peaks = vec![
            Point::new(1.0, 0.0),
            Point::new(1.0, 0.1),
            Point::new(1.0, 0.2),
            Point::new(1.0, 0.3),
            Point::new(1.0, 0.4),
            Point::new(1.0, 0.5),
            Point::new(1.0, 0.6),
        ];
        let segs = left_to_peak(&left, &peaks);
        assert_eq!(segs.len(), left.len());
        for i in 0..left.len() - 1 {
            assert_eq!(segs[i], (left[i], peaks[i]));
        }
        // The last segment must end at the final peak, not peaks[n-1].
        let last = segs.last().unwrap();
        assert_eq!(last.0, left[3]);
        assert_eq!(last.1, *peaks.last().unwrap());
        assert_eq!(last.1, Point::new(1.0, 0.6));
    }

    #[test]
    fn first_period_last_left_connects_to_last_peak() {
        // Structural check against the Python reference (`compute_traces.py` +
        // `segments/calc_segments.py::left_to_peak`): for the first period, the
        // last bottom-layer segment leaving the last left point must land on the
        // very LAST peak of the period, not peaks[n-1].
        let ctx = ctx();
        let phases = ctx.phases.max(1) as i64;
        let num_strands = (ctx.param("num_strands", 5.0) as i64).max(2);
        let n_periods = (ctx.param("n_periods", 4.0) as i64).max(1);
        let d_tot = ctx.active_area_length_mm;
        let a = ctx.board_width_mm / 2.0;

        let o = d_tot / ((n_periods + 1) * num_strands * phases - 1) as f64 * -1.0;
        let offset_step = o * num_strands as f64 * -1.0;
        let d_phase = d_tot - (offset_step * phases as f64) - o;

        // Phase 0, first period — where the left points live.
        let diamonds = compute_diamonds(0.0, d_phase, n_periods, num_strands, o, a);
        let (obj, _) = compute_endpoints(&diamonds);

        let left = &obj[0].left;
        let peaks = &obj[0].peaks;

        let segs = left_to_peak(left, peaks);
        assert_eq!(segs.len(), left.len());
        let last = segs.last().unwrap();
        assert_eq!(last.0, *left.last().unwrap());
        assert_eq!(last.1, *peaks.last().unwrap());
        assert_ne!(last.1, peaks[left.len() - 1]);
        // The preceding segments still pair left[i] → peaks[i].
        for (i, s) in segs[..segs.len() - 1].iter().enumerate() {
            assert_eq!(s.0, left[i]);
            assert_eq!(s.1, peaks[i]);
        }
    }
}
