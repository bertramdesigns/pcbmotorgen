//! Pure per-item failure tallying for `CreateItemsResponse`.
//!
//! Extracted from the write path in [`super::write`] so the grouping /
//! sorting logic is a self-contained, unit-testable function.

use super::{ITEM_STATUS_OK, MAX_FAILURES_TO_REPORT};
use crate::proto::common::commands::ItemCreationResult;

/// Tally per-item outcomes from a `CreateItemsResponse`.
///
/// KiCad returns one `ItemCreationResult` per submitted item; we treat
/// `status.code == ISC_OK` as success and capture the first
/// [`MAX_FAILURES_TO_REPORT`] error messages for the caller. Individual
/// rejections are NOT turned into errors — they are surfaced via the counts
/// so the UI can show them.
///
/// The returned `failure_summary` is a code-grouped list: each entry is
/// `(code, count)` where `code` is the `ItemStatus.code` value KiCad
/// returned (e.g. 7 for `ISC_INVALID_DATA`, 2 for `ISC_INVALID_TYPE`) and
/// `count` is the number of items rejected with that code. The summary is
/// computed from **all** per-item outcomes, not just the surfaced ones — so
/// even if [`MAX_FAILURES_TO_REPORT`] truncates the individual `failures`
/// list, the summary is always complete. This is the property the user
/// needs to debug the 99-of-588 case: with the previous
/// `MAX_FAILURES_TO_REPORT=10` cap, the user only saw 10 of the 99
/// individual messages and had no way to tell whether the other 89 were
/// the same error or a different one.
///
/// Returns `(items_created, failures, failure_summary)`. The summary is
/// sorted by count descending (most-frequent failure first); ties broken by
/// `code` ascending, which makes the rendered output stable across runs
/// (good for snapshot tests and easier to diff when debugging).
pub(crate) fn tally_item_results(
    created_items: &[ItemCreationResult],
) -> (u32, Vec<String>, Vec<(i32, u32)>) {
    let mut items_created: u32 = 0;
    let mut failures: Vec<String> = Vec::new();
    // `BTreeMap` so the final ordering is deterministic: ties on
    // `count` are broken by `code` ascending, which makes the
    // rendered output stable across runs (good for snapshot tests
    // and easier to diff when debugging).
    let mut failure_codes: std::collections::BTreeMap<i32, u32> =
        std::collections::BTreeMap::new();
    for (i, result) in created_items.iter().enumerate() {
        let status = result.status.as_ref();
        let code = status.map(|s| s.code).unwrap_or(0);
        if code == ITEM_STATUS_OK {
            items_created += 1;
        } else {
            // Count this rejection for the summary regardless of
            // whether we surface the individual message below.
            *failure_codes.entry(code).or_insert(0) += 1;
            if failures.len() < MAX_FAILURES_TO_REPORT {
                let msg = status
                    .map(|s| s.error_message.clone())
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| format!("<no error message>"));
                failures.push(format!("item {i}: code={code}: {msg}"));
            }
        }
    }

    // Materialise the summary as a Vec sorted by count descending
    // (most-frequent failure first), ties broken by code ascending.
    // This is the most useful ordering for the UI: the dominant
    // error appears at the top of the warning banner.
    let mut failure_summary: Vec<(i32, u32)> = failure_codes
        .into_iter()
        .map(|(code, count)| (code, count))
        .collect();
    failure_summary.sort_by(|(code_a, count_a), (code_b, count_b)| {
        // Primary: count descending (b.count.cmp(&a.count))
        // Secondary: code ascending (a.code.cmp(&b.code))
        count_b.cmp(count_a).then(code_a.cmp(code_b))
    });

    (items_created, failures, failure_summary)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::common::commands::ItemStatus;

    fn item(code: i32, error_message: &str) -> ItemCreationResult {
        ItemCreationResult {
            status: Some(ItemStatus {
                code,
                error_message: error_message.to_string(),
            }),
            item: None,
        }
    }

    #[test]
    fn test_tally_counts_successes_and_failures() {
        let results = vec![
            item(1, ""),              // ISC_OK
            item(7, "no overlap"),    // ISC_INVALID_DATA
            item(2, "bad type"),      // ISC_INVALID_TYPE
            item(1, ""),              // ISC_OK
        ];
        let (created, failures, summary) = tally_item_results(&results);
        assert_eq!(created, 2, "2 ISC_OK items");
        assert_eq!(failures.len(), 2, "2 rejections surfaced");
        assert!(failures[0].contains("code=7"));
        assert!(failures[0].contains("no overlap"));
        assert!(failures[1].contains("code=2"));
        assert_eq!(summary, vec![(2, 1), (7, 1)], "sorted by count desc, code asc");
    }

    #[test]
    fn test_tally_groups_by_code_and_sorts_count_descending() {
        let results: Vec<ItemCreationResult> = vec![
            item(2, "a"),
            item(7, "b"),
            item(7, "c"),
            item(7, "d"),
            item(7, "e"),
            item(1, ""),
        ];
        let (created, failures, summary) = tally_item_results(&results);
        assert_eq!(created, 1);
        assert_eq!(failures.len(), 5);
        assert_eq!(summary, vec![(7, 4), (2, 1)], "most frequent failure first");
    }

    #[test]
    fn test_tally_caps_surfaced_failures_but_summary_stays_complete() {
        // MAX_FAILURES_TO_REPORT (1000) bounds `failures`; the summary is
        // still computed from every item.
        let mut results: Vec<ItemCreationResult> = (0..1500)
            .map(|_| item(7, "rejected"))
            .collect();
        results.push(item(1, ""));
        let (created, failures, summary) = tally_item_results(&results);
        assert_eq!(created, 1);
        assert_eq!(failures.len(), MAX_FAILURES_TO_REPORT);
        assert_eq!(summary, vec![(7, 1500)]);
    }

    #[test]
    fn test_tally_missing_status_is_a_failure() {
        // A result with no `status` counts as a non-OK item (code 0) and
        // surfaces a placeholder message.
        let results = vec![ItemCreationResult {
            status: None,
            item: None,
        }];
        let (created, failures, summary) = tally_item_results(&results);
        assert_eq!(created, 0);
        assert_eq!(failures, vec!["item 0: code=0: <no error message>".to_string()]);
        assert_eq!(summary, vec![(0, 1)]);
    }
}