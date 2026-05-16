//! Recurrence engine for tasks.
//!
//! Computes the next occurrence date for a recurrent task and spawns new
//! pending instances when a done recurrent task's next due date has elapsed.
//! Idempotent: re-running with the same `now` yields zero new spawns.

use crate::domain::task::{NewTask, RecurFreq, Status, Task};
use chrono::{Months, NaiveDate};

/// Returns the next occurrence date for `task` strictly AFTER the given `now`,
/// or None if the task is not recurrent (RecurFreq::None) or has no due_date.
///
/// `recur_interval` of 0 is treated as 1.
pub fn next_occurrence(task: &Task, now: NaiveDate) -> Option<NaiveDate> {
    let due = task.due_date?;
    if matches!(task.recur_freq, RecurFreq::None) {
        return None;
    }
    let interval = task.recur_interval.max(1) as u32;
    let mut next = due;
    while next <= now {
        next = advance(next, task.recur_freq, interval)?;
    }
    Some(next)
}

fn advance(date: NaiveDate, freq: RecurFreq, interval: u32) -> Option<NaiveDate> {
    match freq {
        RecurFreq::None => None,
        RecurFreq::Daily => date.checked_add_signed(chrono::Duration::days(interval as i64)),
        RecurFreq::Weekly => date.checked_add_signed(chrono::Duration::weeks(interval as i64)),
        RecurFreq::Monthly => date.checked_add_months(Months::new(interval)),
        RecurFreq::Yearly => date.checked_add_months(Months::new(interval * 12)),
    }
}

/// Walks all DONE recurrent tasks; for each, if `next_occurrence` is in the
/// past (i.e. it's due to spawn now), inserts a new task cloned from the
/// source with the new due_date and status=Pending. Idempotent: re-running
/// with no progress yields zero spawns.
///
/// Returns IDs of newly created tasks.
pub fn spawn_recurrent_instances(
    store: &crate::store::sqlite::SqliteStore,
    now: NaiveDate,
) -> crate::error::Result<Vec<i64>> {
    let mut spawned = Vec::new();
    let tasks = store.list_tasks()?;
    for t in &tasks {
        if !matches!(t.status, Status::Done) {
            continue;
        }
        if matches!(t.recur_freq, RecurFreq::None) {
            continue;
        }
        let Some(due) = t.due_date else { continue };
        let next = match next_occurrence(t, now) {
            Some(d) => d,
            None => continue,
        };
        // Skip if a sibling already exists with this due_date + same recur lineage
        if tasks.iter().any(|other| {
            other.id != t.id
                && other.title == t.title
                && other.due_date == Some(next)
                && other.recur_freq == t.recur_freq
        }) {
            continue;
        }
        // Sanity: don't spawn if `next` is the same as the source's due
        if next == due {
            continue;
        }
        let new_task = NewTask {
            title: t.title.clone(),
            description: t.description.clone(),
            status: Status::Pending,
            priority: t.priority,
            due_date: Some(next),
            recur_freq: t.recur_freq,
            recur_interval: t.recur_interval,
            tags: t.tags.clone(),
        };
        let (id, _) = store.create_task(new_task)?;
        spawned.push(id);
    }
    Ok(spawned)
}
