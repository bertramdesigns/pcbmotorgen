//! Peak / valley computation for the infinity braid (ported from pcbBraid).
//!
//! `calc_peaks` / `calc_valleys` locate the braid crossing points within a
//! period; `top_peaks` / `bottom_valleys` pair them into segments.

use crate::model::Point;

use super::diamonds::{crossing, wrap_idx};

pub(crate) fn calc_peaks(dlist: &[Vec<Point>]) -> Vec<Point> {
    let num_diamonds = dlist.len() as i64;
    if num_diamonds == 0 {
        return vec![];
    }
    let peak_tot = (num_diamonds - 1) * 2;
    let mut peaks: Vec<Point> = Vec::new();
    for pt in 0..peak_tot {
        if pt == peak_tot - 1 {
            peaks.push(dlist[num_diamonds as usize - 1][0]);
        } else if pt < num_diamonds - 1 {
            let p1 = dlist[pt as usize][3];
            let p2 = dlist[pt as usize][0];
            let p3 = dlist[0][0];
            let p4 = dlist[0][1];
            if let Some(i) = crossing(p1, p2, p3, p4) {
                peaks.push(i);
            }
        } else {
            let diamond_idx = pt - peak_tot; // negative
            let dia = &dlist[wrap_idx(dlist.len(), diamond_idx)];
            let p1 = dia[0];
            let p2 = dia[1];
            let p3 = dlist[num_diamonds as usize - 2][3];
            let p4 = dlist[num_diamonds as usize - 2][0];
            if let Some(i) = crossing(p1, p2, p3, p4) {
                peaks.push(i);
            }
        }
    }
    peaks
}

pub(crate) fn calc_valleys(dlist: &[Vec<Point>]) -> Vec<Point> {
    let num_diamonds = dlist.len() as i64;
    if num_diamonds == 0 {
        return vec![];
    }
    let valley_tot = num_diamonds * 2 - 1;
    let mut valleys: Vec<Point> = Vec::new();
    for pt in 0..valley_tot {
        if pt < num_diamonds {
            let p1 = dlist[0][1];
            let p2 = dlist[0][2];
            let p3 = dlist[pt as usize][2];
            let p4 = dlist[pt as usize][3];
            if let Some(i) = crossing(p1, p2, p3, p4) {
                valleys.push(i);
            }
        } else {
            let diamond_idx = pt - valley_tot; // negative
            let dia = &dlist[wrap_idx(dlist.len(), diamond_idx)];
            let p1 = dlist[num_diamonds as usize - 1][2];
            let p2 = dlist[num_diamonds as usize - 1][3];
            let p3 = dia[1];
            let p4 = dia[2];
            if let Some(i) = crossing(p1, p2, p3, p4) {
                valleys.push(i);
            }
        }
    }
    valleys
}

pub(crate) fn top_peaks(peaks: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = peaks.len();
    let reversed: Vec<Point> = peaks.iter().rev().copied().collect();
    for i in 0..n / 2 {
        if (n - 2 - i) != i {
            segs.push((peaks[i], reversed[i + 1]));
        }
    }
    segs
}

pub(crate) fn bottom_valleys(valleys: &[Point]) -> Vec<(Point, Point)> {
    let mut segs = Vec::new();
    let n = valleys.len();
    let reversed: Vec<Point> = valleys.iter().rev().copied().collect();
    for i in 0..n / 2 {
        if (n - 1 - i) != i {
            segs.push((valleys[i], reversed[i]));
        }
    }
    segs
}