//! Property-based fuzz coverage for `store::sqlite::parse_dt`.
//!
//! Properties:
//!   1. `parse_dt` never panics on any UTF-8 string.
//!   2. Round-trips of canonical formats (`%Y-%m-%d %H:%M:%S` and RFC-3339)
//!      yield exactly the originating instant (to second resolution).

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use proptest::prelude::*;
use rondo_core::store::sqlite::parse_dt;

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn never_panics_on_arbitrary_input(s in ".*") {
        let _ = parse_dt(&s);
    }

    #[test]
    fn never_panics_on_ascii_garbage(s in "[ -~]{0,64}") {
        let _ = parse_dt(&s);
    }

    #[test]
    fn never_panics_on_almost_iso(s in r"\d{1,5}-\d{1,3}-\d{1,3}( \d{1,3}:\d{1,3}:\d{1,3})?") {
        let _ = parse_dt(&s);
    }

    #[test]
    fn roundtrip_sqlite_format(
        y in 1970i32..2100,
        m in 1u32..=12,
        d in 1u32..=28,
        hh in 0u32..24,
        mm in 0u32..60,
        ss in 0u32..60,
    ) {
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let dt = date.and_hms_opt(hh, mm, ss).unwrap();
        let serialized = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        let parsed = parse_dt(&serialized);
        let expected = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
        prop_assert_eq!(parsed, expected);
    }

    #[test]
    fn roundtrip_rfc3339(
        y in 1970i32..2100,
        m in 1u32..=12,
        d in 1u32..=28,
        hh in 0u32..24,
        mm in 0u32..60,
        ss in 0u32..60,
    ) {
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let naive = date.and_hms_opt(hh, mm, ss).unwrap();
        let dt = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        let serialized = dt.to_rfc3339();
        let parsed = parse_dt(&serialized);
        prop_assert_eq!(parsed, dt);
    }
}

#[test]
fn known_invalid_inputs_dont_panic() {
    // Spot-check fixtures alongside the proptest cases so regressions are
    // identifiable even when proptest seeds change.
    let cases = [
        "",
        "not-a-date",
        "2026-13-40 25:61:99",
        "\u{0}\u{1}\u{2}",
        "9999-12-31 23:59:59",
        "1970-01-01T00:00:00Z",
    ];
    for s in cases {
        let _: DateTime<Utc> = parse_dt(s);
    }
    // Also ensure NaiveDateTime helper used by codebase doesn't panic on
    // arbitrary parse failures (parse_dt's fallback path).
    assert!(NaiveDateTime::parse_from_str("garbage", "%Y-%m-%d %H:%M:%S").is_err());
}
