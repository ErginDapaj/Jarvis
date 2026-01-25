use std::time::Duration;

/// Spam detection thresholds (defaults, can be overridden via env vars)
pub const DEFAULT_SPAM_PROMPT_THRESHOLD: u32 = 5;   // Join/leave count to prompt owner
pub const DEFAULT_SPAM_TIMEOUT_THRESHOLD: u32 = 10; // Join/leave count for auto-timeout
pub const DEFAULT_SPAM_WINDOW_SECONDS: u64 = 60;    // Rolling window for spam detection (1 minute)

/// VC naming deadline
pub const VC_NAMING_DEADLINE_SECONDS: u64 = 60;

/// Progressive timeout durations (levels 0-7)
pub const TIMEOUT_DURATIONS: &[Duration] = &[
    Duration::from_secs(15 * 60),          // Level 0: 15 minutes
    Duration::from_secs(60 * 60),          // Level 1: 1 hour
    Duration::from_secs(6 * 60 * 60),      // Level 2: 6 hours
    Duration::from_secs(24 * 60 * 60),     // Level 3: 24 hours
    Duration::from_secs(3 * 24 * 60 * 60), // Level 4: 3 days
    Duration::from_secs(7 * 24 * 60 * 60), // Level 5: 1 week
    Duration::from_secs(14 * 24 * 60 * 60), // Level 6: 2 weeks
    Duration::from_secs(14 * 24 * 60 * 60), // Level 7: 2 weeks (max)
];

/// Days of good behavior before timeout level resets
pub const TIMEOUT_RESET_DAYS: i64 = 30;

/// JTC flow timeout (how long user has to complete modal/tag selection)
pub const JTC_FLOW_TIMEOUT_SECONDS: u64 = 120;

/// Rate limit for rename and retag commands (30 minutes)
pub const RENAME_RETAG_RATE_LIMIT_SECONDS: u64 = 30 * 60;

/// Get timeout duration for a given level
pub fn get_timeout_duration(level: u32) -> Duration {
    let level = level.min(TIMEOUT_DURATIONS.len() as u32 - 1) as usize;
    TIMEOUT_DURATIONS[level]
}

/// Format duration for display
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs < 60 {
        format!("{} seconds", total_secs)
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        format!("{} minute{}", mins, if mins == 1 { "" } else { "s" })
    } else if total_secs < 86400 {
        let hours = total_secs / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else {
        let days = total_secs / 86400;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    }
}
