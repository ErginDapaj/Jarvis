use serenity::all::UserId;

/// Format a user mention
pub fn mention_user(user_id: UserId) -> String {
    format!("<@{}>", user_id)
}

/// Format a channel mention
pub fn mention_channel(channel_id: u64) -> String {
    format!("<#{}>", channel_id)
}

/// Format a role mention
pub fn mention_role(role_id: u64) -> String {
    format!("<@&{}>", role_id)
}

/// Format a number with commas
pub fn format_number(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }

    if n < 0 {
        result.push('-');
    }

    result.chars().rev().collect()
}

/// Truncate a string to a maximum length, adding ellipsis if needed
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Format tags for display
pub fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        "None".to_string()
    } else {
        tags.iter()
            .map(|t| format!("`{}`", t))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
