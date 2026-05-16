use chrono::NaiveDate;
use rondo_core::domain::task::{NewTask, Priority, RecurFreq, Status, Task};
use rondo_core::recurrence::{next_occurrence, spawn_recurrent_instances};
use rondo_core::store::sqlite::SqliteStore;

fn task(due: Option<NaiveDate>, freq: RecurFreq, interval: i64) -> Task {
    Task {
        id: 1,
        title: "t".into(),
        description: None,
        status: Status::Done,
        priority: Priority::Low,
        due_date: due,
        created_at: chrono::Utc::now(),
        recur_freq: freq,
        recur_interval: interval,
        metadata: Default::default(),
        tags: vec![],
        subtasks: vec![],
        time_logs: vec![],
        notes: vec![],
        blocked_by_ids: vec![],
        blocks_ids: vec![],
    }
}

fn d(y: i32, m: u32, dd: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, dd).unwrap()
}

fn fixture_db() -> (tempfile::NamedTempFile, SqliteStore) {
    let f = tempfile::NamedTempFile::new().unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(f.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed).unwrap())
        .unwrap();
    drop(conn);
    let store = SqliteStore::open_readwrite(f.path()).unwrap();
    (f, store)
}

#[test]
fn no_freq_returns_none() {
    let t = task(Some(d(2026, 1, 1)), RecurFreq::None, 0);
    assert!(next_occurrence(&t, d(2026, 5, 1)).is_none());
}

#[test]
fn no_due_returns_none() {
    let t = task(None, RecurFreq::Daily, 1);
    assert!(next_occurrence(&t, d(2026, 5, 1)).is_none());
}

#[test]
fn daily_simple() {
    let t = task(Some(d(2026, 5, 1)), RecurFreq::Daily, 1);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 2)));
    assert_eq!(next_occurrence(&t, d(2026, 5, 5)), Some(d(2026, 5, 6)));
}

#[test]
fn daily_interval_3() {
    let t = task(Some(d(2026, 5, 1)), RecurFreq::Daily, 3);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 4)));
    assert_eq!(next_occurrence(&t, d(2026, 5, 5)), Some(d(2026, 5, 7)));
    // 2026-05-01 + 33*3 days = 2026-08-08. now=2026-08-08 → next step = 2026-08-11.
    assert_eq!(next_occurrence(&t, d(2026, 8, 8)), Some(d(2026, 8, 11)));
}

#[test]
fn daily_before_due_returns_first_step() {
    let t = task(Some(d(2026, 5, 10)), RecurFreq::Daily, 1);
    // now is before due — loop body never executes; `next` starts at due,
    // which is > now, so we return `due` itself.
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 10)));
}

#[test]
fn weekly_simple() {
    let t = task(Some(d(2026, 5, 1)), RecurFreq::Weekly, 1);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 8)));
}

#[test]
fn weekly_interval_2() {
    let t = task(Some(d(2026, 5, 1)), RecurFreq::Weekly, 2);
    assert_eq!(next_occurrence(&t, d(2026, 5, 8)), Some(d(2026, 5, 15)));
}

#[test]
fn weekly_far_future() {
    let t = task(Some(d(2026, 1, 1)), RecurFreq::Weekly, 1);
    // 2026-01-01 + 18 weeks = 2026-05-07; advance until > 2026-05-01
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 7)));
}

#[test]
fn monthly_simple() {
    let t = task(Some(d(2026, 1, 15)), RecurFreq::Monthly, 1);
    assert_eq!(next_occurrence(&t, d(2026, 1, 15)), Some(d(2026, 2, 15)));
}

#[test]
fn monthly_clamps_to_end_of_month_jan_31_to_feb() {
    let t = task(Some(d(2026, 1, 31)), RecurFreq::Monthly, 1);
    assert_eq!(next_occurrence(&t, d(2026, 1, 31)), Some(d(2026, 2, 28)));
}

#[test]
fn monthly_clamps_in_leap_year() {
    let t = task(Some(d(2024, 1, 31)), RecurFreq::Monthly, 1);
    assert_eq!(next_occurrence(&t, d(2024, 1, 31)), Some(d(2024, 2, 29)));
}

#[test]
fn monthly_interval_3() {
    let t = task(Some(d(2026, 1, 15)), RecurFreq::Monthly, 3);
    assert_eq!(next_occurrence(&t, d(2026, 1, 15)), Some(d(2026, 4, 15)));
}

#[test]
fn monthly_year_wrap() {
    let t = task(Some(d(2026, 11, 15)), RecurFreq::Monthly, 1);
    assert_eq!(next_occurrence(&t, d(2026, 12, 31)), Some(d(2027, 1, 15)));
}

#[test]
fn yearly_simple() {
    let t = task(Some(d(2025, 6, 1)), RecurFreq::Yearly, 1);
    assert_eq!(next_occurrence(&t, d(2025, 6, 1)), Some(d(2026, 6, 1)));
}

#[test]
fn yearly_leap_day() {
    let t = task(Some(d(2024, 2, 29)), RecurFreq::Yearly, 1);
    assert_eq!(next_occurrence(&t, d(2024, 2, 29)), Some(d(2025, 2, 28)));
}

#[test]
fn yearly_interval_2() {
    let t = task(Some(d(2024, 6, 1)), RecurFreq::Yearly, 2);
    assert_eq!(next_occurrence(&t, d(2024, 6, 1)), Some(d(2026, 6, 1)));
}

#[test]
fn dst_does_not_affect_naive_date() {
    let t = task(Some(d(2026, 3, 8)), RecurFreq::Daily, 1);
    assert_eq!(next_occurrence(&t, d(2026, 3, 8)), Some(d(2026, 3, 9)));
}

#[test]
fn interval_zero_treated_as_one() {
    let t = task(Some(d(2026, 5, 1)), RecurFreq::Daily, 0);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 2)));
}

#[test]
fn interval_negative_treated_as_one() {
    let t = task(Some(d(2026, 5, 1)), RecurFreq::Daily, -5);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 2)));
}

#[test]
fn far_future_now_advances_many_periods() {
    let t = task(Some(d(2020, 1, 1)), RecurFreq::Daily, 1);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 2)));
}

#[test]
fn far_future_monthly() {
    let t = task(Some(d(2020, 1, 15)), RecurFreq::Monthly, 1);
    assert_eq!(next_occurrence(&t, d(2026, 5, 1)), Some(d(2026, 5, 15)));
}

#[test]
fn spawn_creates_new_pending_task_for_done_recurrent() {
    let (_f, store) = fixture_db();
    let (_id, _) = store
        .create_task(NewTask {
            title: "weekly review".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Weekly,
            recur_interval: 1,
            tags: vec!["work".into()],
        })
        .unwrap();
    let new_ids = spawn_recurrent_instances(&store, d(2026, 5, 8)).unwrap();
    assert!(!new_ids.is_empty());
    let new_id = new_ids[0];
    let new_task = store.task_by_id(new_id).unwrap();
    assert_eq!(new_task.title, "weekly review");
    assert_eq!(new_task.status, Status::Pending);
    // due=May 1 weekly, now=May 8: loop walks May 1 → May 8 (still <= now) → May 15 (>now).
    assert_eq!(new_task.due_date, Some(d(2026, 5, 15)));
    assert_eq!(new_task.tags, vec!["work"]);
}

#[test]
fn spawn_is_idempotent() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "daily standup".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Daily,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    let first = spawn_recurrent_instances(&store, d(2026, 5, 2)).unwrap();
    assert_eq!(first.len(), 1);
    let second = spawn_recurrent_instances(&store, d(2026, 5, 2)).unwrap();
    assert!(
        second.is_empty(),
        "second spawn at same `now` should be a no-op"
    );
}

#[test]
fn spawn_skips_non_done_tasks() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "pending recurrent".into(),
            description: None,
            status: Status::Pending,
            priority: Priority::Low,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Daily,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    // Pre-existing tasks in the seed should also not be spawned (non-recurrent).
    let pending_only_spawn: Vec<i64> = spawn_recurrent_instances(&store, d(2026, 5, 10))
        .unwrap()
        .into_iter()
        .collect();
    assert!(
        pending_only_spawn.is_empty(),
        "no Done recurrent tasks → no spawn"
    );
}

#[test]
fn spawn_skips_non_recurrent_tasks() {
    let (_f, store) = fixture_db();
    let spawned = spawn_recurrent_instances(&store, d(2026, 5, 10)).unwrap();
    assert!(spawned.is_empty());
}

#[test]
fn spawn_skips_when_now_before_next_occurrence() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "future task".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: Some(d(2026, 6, 1)),
            recur_freq: RecurFreq::Weekly,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    // now is before next occurrence (2026-06-08) — but spawn still creates
    // because next_occurrence(due=2026-06-01, now=2026-05-01) returns 2026-06-01
    // which is != due... wait, it IS == due. Let's check: loop condition is
    // `next <= now`. due=Jun 1, now=May 1. next=Jun 1, not <= May 1, so loop
    // skips and returns Jun 1 (== due), and our `next == due` guard kicks in.
    let spawned = spawn_recurrent_instances(&store, d(2026, 5, 1)).unwrap();
    assert!(spawned.is_empty(), "next == due should be a no-op");
}

#[test]
fn spawn_preserves_priority_and_description() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "high prio rec".into(),
            description: Some("body".into()),
            status: Status::Done,
            priority: Priority::Urgent,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Daily,
            recur_interval: 1,
            tags: vec!["a".into(), "b".into()],
        })
        .unwrap();
    let ids = spawn_recurrent_instances(&store, d(2026, 5, 5)).unwrap();
    assert_eq!(ids.len(), 1);
    let new_t = store.task_by_id(ids[0]).unwrap();
    assert_eq!(new_t.priority, Priority::Urgent);
    assert_eq!(new_t.description.as_deref(), Some("body"));
    assert_eq!(new_t.recur_freq, RecurFreq::Daily);
    assert_eq!(new_t.recur_interval, 1);
    assert_eq!(new_t.tags, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn spawn_skips_done_task_without_due_date() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "no due".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: None,
            recur_freq: RecurFreq::Daily,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    let spawned = spawn_recurrent_instances(&store, d(2026, 5, 10)).unwrap();
    assert!(spawned.is_empty());
}

#[test]
fn spawn_handles_multiple_done_recurrent_tasks() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "rec a".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Daily,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    store
        .create_task(NewTask {
            title: "rec b".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Weekly,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    let spawned = spawn_recurrent_instances(&store, d(2026, 5, 10)).unwrap();
    assert_eq!(spawned.len(), 2);
}

#[test]
fn spawn_idempotent_after_multiple_runs() {
    let (_f, store) = fixture_db();
    store
        .create_task(NewTask {
            title: "rec".into(),
            description: None,
            status: Status::Done,
            priority: Priority::Low,
            due_date: Some(d(2026, 5, 1)),
            recur_freq: RecurFreq::Daily,
            recur_interval: 1,
            tags: vec![],
        })
        .unwrap();
    let first = spawn_recurrent_instances(&store, d(2026, 5, 2)).unwrap();
    let second = spawn_recurrent_instances(&store, d(2026, 5, 2)).unwrap();
    let third = spawn_recurrent_instances(&store, d(2026, 5, 2)).unwrap();
    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
    assert!(third.is_empty());
}
