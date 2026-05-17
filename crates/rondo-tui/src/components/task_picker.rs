//! Fuzzy task picker used by the dep_overlay (E2).
//!
//! Filters every task in the store by the user's typed needle using the
//! shared `nucleo`-backed `SearchEngine`, then ranks by score. The
//! caller decides which candidates to exclude (e.g. the currently
//! selected task itself + its existing blockers).

use crate::search::SearchEngine;
use rondo_core::domain::task::Task;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: i64,
    pub title: String,
}

/// Filter `tasks` by `needle`. Excludes ids in `exclude`. When `needle`
/// is empty, returns every non-excluded task sorted by `id ASC` so the
/// list is stable and predictable.
pub fn rank(tasks: &[Task], needle: &str, exclude: &[i64]) -> Vec<Candidate> {
    let needle = needle.trim();
    if needle.is_empty() {
        let mut out: Vec<Candidate> = tasks
            .iter()
            .filter(|t| !exclude.contains(&t.id))
            .map(|t| Candidate {
                id: t.id,
                title: t.title.clone(),
            })
            .collect();
        out.sort_by_key(|c| c.id);
        return out;
    }
    let mut engine = SearchEngine::new();
    let mut scored: Vec<(u16, Candidate)> = Vec::new();
    for t in tasks {
        if exclude.contains(&t.id) {
            continue;
        }
        // Match against `#<id> title` so digits in the needle also
        // catch id prefixes.
        let hay = format!("#{} {}", t.id, t.title);
        if let Some(score) = engine.score_only(needle, &hay) {
            scored.push((
                score,
                Candidate {
                    id: t.id,
                    title: t.title.clone(),
                },
            ));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_core::domain::task::{Priority, RecurFreq, Status};
    use std::collections::HashMap;

    fn t(id: i64, title: &str) -> Task {
        Task {
            id,
            title: title.into(),
            description: None,
            status: Status::Pending,
            priority: Priority::Low,
            due_date: None,
            created_at: chrono::Utc::now(),
            recur_freq: RecurFreq::None,
            recur_interval: 0,
            metadata: HashMap::new(),
            tags: vec![],
            subtasks: vec![],
            time_logs: vec![],
            notes: vec![],
            blocked_by_ids: vec![],
            blocks_ids: vec![],
        }
    }

    #[test]
    fn empty_needle_returns_all_sorted_by_id() {
        let tasks = vec![t(3, "c"), t(1, "a"), t(2, "b")];
        let out = rank(&tasks, "", &[]);
        assert_eq!(out.iter().map(|c| c.id).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn excluded_ids_dropped() {
        let tasks = vec![t(1, "a"), t(2, "b"), t(3, "c")];
        let out = rank(&tasks, "", &[2]);
        assert_eq!(out.iter().map(|c| c.id).collect::<Vec<_>>(), vec![1, 3]);
    }

    #[test]
    fn fuzzy_filter_orders_by_score() {
        let tasks = vec![
            t(1, "Review API spec"),
            t(2, "Plan release"),
            t(3, "Refactor auth"),
        ];
        let out = rank(&tasks, "api", &[]);
        assert!(!out.is_empty());
        assert_eq!(out[0].id, 1);
    }

    #[test]
    fn digit_needle_matches_id_prefix() {
        let tasks = vec![t(11, "alpha"), t(2, "bravo"), t(33, "charlie")];
        let out = rank(&tasks, "11", &[]);
        assert_eq!(out[0].id, 11);
    }
}
