//! Grouping helpers for the task list.
//!
//! Sorting lives in [`crate::app::ui_state::SortOrder`] and runs first
//! against the visible-task slice. Grouping is a second, optional pass
//! that buckets the already-sorted indices under a stable, ordered set
//! of group headers without changing the in-group order.

use chrono::NaiveDate;
use rondo_core::domain::task::{Priority, Status, Task};

/// User-visible grouping mode. `None` (i.e. `Option<GroupBy>::None`)
/// means the task list renders as a flat slice — the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    Priority,
    Status,
    Due,
}

impl GroupBy {
    /// Parse the palette argument `priority|status|due|none`.
    /// Returns `Ok(None)` for `"none"` so callers can clear the grouping.
    /// `Err` carries back the offending input for the caller to surface.
    pub fn parse(input: &str) -> Result<Option<GroupBy>, String> {
        match input.trim().to_ascii_lowercase().as_str() {
            "priority" | "prio" | "p" => Ok(Some(GroupBy::Priority)),
            "status" | "s" => Ok(Some(GroupBy::Status)),
            "due" | "d" | "date" => Ok(Some(GroupBy::Due)),
            "none" | "off" | "clear" | "" => Ok(None),
            other => Err(other.to_string()),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GroupBy::Priority => "priority",
            GroupBy::Status => "status",
            GroupBy::Due => "due",
        }
    }
}

/// Compute the group key/label for a task under the active grouping.
///
/// The label is what shows in the header row; the key is what determines
/// which bucket a task lands in — two tasks share a bucket iff they have
/// the same key.
pub fn group_label(task: &Task, today: NaiveDate, by: GroupBy) -> String {
    match by {
        GroupBy::Priority => match task.priority {
            Priority::Urgent => "URGENT".to_string(),
            Priority::High => "HIGH".to_string(),
            Priority::Med => "MED".to_string(),
            Priority::Low => "LOW".to_string(),
        },
        GroupBy::Status => match task.status {
            Status::InProgress => "IN PROGRESS".to_string(),
            Status::Pending => "PENDING".to_string(),
            Status::Done => "DONE".to_string(),
        },
        GroupBy::Due => due_bucket(task.due_date, today).to_string(),
    }
}

/// Stable sort key so groups appear in a consistent, meaningful order
/// regardless of the underlying SortOrder.
fn group_rank(task: &Task, today: NaiveDate, by: GroupBy) -> i64 {
    match by {
        // Highest priority first.
        GroupBy::Priority => -(task.priority as i64),
        // In-progress on top, then pending, then done.
        GroupBy::Status => match task.status {
            Status::InProgress => 0,
            Status::Pending => 1,
            Status::Done => 2,
        },
        GroupBy::Due => due_rank(task.due_date, today),
    }
}

fn due_bucket(due: Option<NaiveDate>, today: NaiveDate) -> &'static str {
    match due {
        None => "NO DUE DATE",
        Some(d) if d < today => "OVERDUE",
        Some(d) if d == today => "TODAY",
        Some(d) if (d - today).num_days() <= 7 => "THIS WEEK",
        Some(_) => "LATER",
    }
}

fn due_rank(due: Option<NaiveDate>, today: NaiveDate) -> i64 {
    match due {
        Some(d) if d < today => 0,
        Some(d) if d == today => 1,
        Some(d) if (d - today).num_days() <= 7 => 2,
        Some(_) => 3,
        None => 4,
    }
}

/// One bucket: header label + the indices (into `tasks`) belonging to it,
/// preserving the in-group order from the input slice.
pub struct GroupedBucket {
    pub label: String,
    pub indices: Vec<usize>,
}

/// Bucket `sorted` (already sorted by the caller's `SortOrder`) into the
/// groups dictated by `by`. The bucket order is determined by
/// [`group_rank`]; tie-broken by first-appearance to keep results stable.
pub fn group_sorted_indices(
    tasks: &[Task],
    sorted: &[usize],
    today: NaiveDate,
    by: GroupBy,
) -> Vec<GroupedBucket> {
    use std::collections::BTreeMap;
    // BTreeMap keyed by (rank, first-seen position) preserves deterministic
    // ordering across runs while keeping rank-1 groups above rank-2 etc.
    let mut buckets: BTreeMap<(i64, usize), GroupedBucket> = BTreeMap::new();
    let mut first_seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (pos, &idx) in sorted.iter().enumerate() {
        let task = &tasks[idx];
        let label = group_label(task, today, by);
        let rank = group_rank(task, today, by);
        let first = *first_seen.entry(label.clone()).or_insert(pos);
        buckets
            .entry((rank, first))
            .or_insert_with(|| GroupedBucket {
                label: label.clone(),
                indices: Vec::new(),
            })
            .indices
            .push(idx);
    }
    buckets.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_palette_args() {
        assert_eq!(GroupBy::parse("priority"), Ok(Some(GroupBy::Priority)));
        assert_eq!(GroupBy::parse("Status"), Ok(Some(GroupBy::Status)));
        assert_eq!(GroupBy::parse("due"), Ok(Some(GroupBy::Due)));
        assert_eq!(GroupBy::parse("none"), Ok(None));
        assert_eq!(GroupBy::parse(""), Ok(None));
        assert_eq!(GroupBy::parse("xyz"), Err("xyz".to_string()));
    }

    #[test]
    fn due_bucketing_known_dates() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        assert_eq!(
            due_bucket(Some(NaiveDate::from_ymd_opt(2024, 6, 10).unwrap()), today),
            "OVERDUE"
        );
        assert_eq!(due_bucket(Some(today), today), "TODAY");
        assert_eq!(
            due_bucket(Some(NaiveDate::from_ymd_opt(2024, 6, 20).unwrap()), today),
            "THIS WEEK"
        );
        assert_eq!(
            due_bucket(Some(NaiveDate::from_ymd_opt(2024, 7, 30).unwrap()), today),
            "LATER"
        );
        assert_eq!(due_bucket(None, today), "NO DUE DATE");
    }
}
