use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone};

const TEST_TODAY_ENV: &str = "RONDO_TEST_TODAY";

pub fn today() -> NaiveDate {
    if let Ok(raw) = std::env::var(TEST_TODAY_ENV) {
        if let Ok(d) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
            return d;
        }
    }
    Local::now().date_naive()
}

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
