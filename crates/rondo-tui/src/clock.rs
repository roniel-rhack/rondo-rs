//! Clock surface for render-time date/time queries.
//!
//! Two complementary APIs:
//!
//! - Free functions [`today()`] and [`now()`] honour the `RONDO_TEST_TODAY`
//!   env var (`YYYY-MM-DD`) so CI can pin a date without code changes.
//!   Right choice for call sites that don't have `AppState` access (e.g.
//!   `Filter::applies_to`, components called by builtin plugins).
//!
//! - Trait [`Clock`] with [`SystemClock`] / [`FixedClock`] impls, held in
//!   `AppState::clock`. Used where the app already carries state and the
//!   test wants deterministic injection without an env var.
//!
//! Snapshot tests use [`FixedClock`]; CI integration tests use
//! `RONDO_TEST_TODAY=2026-05-17`. Both converge on the same `today` value
//! when set.

use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};

const TEST_TODAY_ENV: &str = "RONDO_TEST_TODAY";

/// Local-time `today` with env-var override for deterministic snapshots.
pub fn today() -> NaiveDate {
    if let Ok(raw) = std::env::var(TEST_TODAY_ENV) {
        if let Ok(d) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
            return d;
        }
    }
    Local::now().date_naive()
}

/// Local-time `now` with env-var override; falls back to local noon on the
/// pinned date when the env var is present.
pub fn now() -> DateTime<Local> {
    if let Ok(raw) = std::env::var(TEST_TODAY_ENV) {
        if let Ok(d) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
            let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            if let Some(dt) = Local.from_local_datetime(&d.and_time(noon)).single() {
                return dt;
            }
        }
    }
    Local::now()
}

/// Source of "now" for code paths that carry `AppState`. Implementations
/// must be `Send + Sync` because `AppState::clock` is shared via `Arc`.
pub trait Clock: Send + Sync {
    /// Current instant as a UTC `DateTime`.
    fn now(&self) -> DateTime<Utc>;

    /// Today's date in the user's local timezone.
    fn today(&self) -> NaiveDate;
}

/// Real wall-clock implementation. Both methods honour `RONDO_TEST_TODAY`
/// so CI integration tests can pin the date without rewriting call sites.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        now().with_timezone(&Utc)
    }

    fn today(&self) -> NaiveDate {
        today()
    }
}

/// Deterministic clock for snapshot tests. `now()` returns the stored
/// instant on every call; `today()` is derived from it in UTC.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    pub now: DateTime<Utc>,
}

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }

    fn today(&self) -> NaiveDate {
        self.now.date_naive()
    }
}
