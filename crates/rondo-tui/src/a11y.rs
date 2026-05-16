//! Accessibility helpers: NO_COLOR + reduced-motion detection.

pub fn no_color() -> bool {
    std::env::var("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

pub fn reduced_motion(cli_flag: bool) -> bool {
    if cli_flag {
        return true;
    }
    if std::env::var("RONDO_REDUCED_MOTION")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false)
    {
        return true;
    }
    if std::env::var("RONDO_FX")
        .map(|v| v == "0" || v.to_lowercase() == "false")
        .unwrap_or(false)
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_env<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        let r = f();
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        r
    }
    #[test]
    fn no_color_set_non_empty() {
        with_env("NO_COLOR", Some("1"), || assert!(no_color()));
    }
    #[test]
    fn no_color_empty_is_false() {
        with_env("NO_COLOR", Some(""), || assert!(!no_color()));
    }
    #[test]
    fn no_color_unset_is_false() {
        with_env("NO_COLOR", None, || assert!(!no_color()));
    }
    #[test]
    fn reduced_motion_cli_flag_wins() {
        assert!(reduced_motion(true));
    }
}
