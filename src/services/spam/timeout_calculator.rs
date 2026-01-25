use std::time::Duration;

use crate::constants::timeouts::TIMEOUT_DURATIONS;

/// Get the timeout duration for a given level
pub fn get_timeout_duration(level: u32) -> Duration {
    let level = level.min(TIMEOUT_DURATIONS.len() as u32 - 1) as usize;
    TIMEOUT_DURATIONS[level]
}

/// Get a human-readable description of the timeout duration
pub fn format_timeout_level(level: u32) -> String {
    let duration = get_timeout_duration(level);
    let total_secs = duration.as_secs();

    if total_secs < 3600 {
        let mins = total_secs / 60;
        format!("{} minute{}", mins, if mins == 1 { "" } else { "s" })
    } else if total_secs < 86400 {
        let hours = total_secs / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else if total_secs < 604800 {
        let days = total_secs / 86400;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    } else {
        let weeks = total_secs / 604800;
        format!("{} week{}", weeks, if weeks == 1 { "" } else { "s" })
    }
}

/// Get the maximum timeout level
pub fn max_level() -> u32 {
    (TIMEOUT_DURATIONS.len() - 1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_levels() {
        assert_eq!(get_timeout_duration(0).as_secs(), 15 * 60);
        assert_eq!(get_timeout_duration(1).as_secs(), 60 * 60);
        assert_eq!(get_timeout_duration(7).as_secs(), 14 * 24 * 60 * 60);
        // Level beyond max should return max
        assert_eq!(get_timeout_duration(100).as_secs(), 14 * 24 * 60 * 60);
    }

    #[test]
    fn test_format_timeout() {
        assert_eq!(format_timeout_level(0), "15 minutes");
        assert_eq!(format_timeout_level(1), "1 hour");
        assert_eq!(format_timeout_level(3), "1 day");
        assert_eq!(format_timeout_level(5), "1 week");
        assert_eq!(format_timeout_level(6), "2 weeks");
    }
}
