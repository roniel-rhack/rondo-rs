//! Abstract clock for render-time date/time queries.
//!
//! Production paths use [`SystemClock`] (wall-clock `Utc::now()` /
//! `Local::now().date_naive()`). Tests inject [`FixedClock`] so snapshot
//! output is deterministic and date strings can be verified rather than
//! redacted.
//!
//! `AppState::clock` is a `Arc<dyn Clock + Send + Sync>` so it can be
//! shared across handlers and rendered components without lifetime
//! gymnastics.

use chrono::{DateTime, NaiveDate, Utc};

/// Source of "now" for the TUI. Implementations must be `Send + Sync`.
pub trait Clock: Send + Sync {
    /// Current instant as a UTC `DateTime`.
    fn now(&self) -> DateTime<Utc>;

    /// Today's date in the user's local timezone.
    fn today(&self) -> NaiveDate;
}

/// Real wall-clock implementation. Calls `Utc::now()` and
/// `Local::now().date_naive()` on each query.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn today(&self) -> NaiveDate {
        chrono::Local::now().date_naive()
    }
}

/// Deterministic clock for tests. `now()` returns the stored instant on
/// every call; `today()` is derived from that instant in UTC.
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
