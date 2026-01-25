use serenity::all::{Colour, CreateEmbed};

// ============================================================================
// Color Palette - Cohesive, professional design
// ============================================================================

/// Primary brand color - Deep blue
pub const PRIMARY_COLOR: Colour = Colour::from_rgb(59, 130, 246);

/// Secondary/accent color - Indigo
pub const SECONDARY_COLOR: Colour = Colour::from_rgb(99, 102, 241);

/// Success color - Emerald green
pub const SUCCESS_COLOR: Colour = Colour::from_rgb(16, 185, 129);

/// Error color - Rose red
pub const ERROR_COLOR: Colour = Colour::from_rgb(244, 63, 94);

/// Warning color - Amber
pub const WARNING_COLOR: Colour = Colour::from_rgb(245, 158, 11);

/// Info/neutral color - Slate
pub const INFO_COLOR: Colour = Colour::from_rgb(100, 116, 139);

// ============================================================================
// Text Formatting - Unicode dividers and separators
// ============================================================================

/// Section divider (thin line)
pub const DIVIDER: &str = "───────────────────────";

/// Short divider
pub const DIVIDER_SHORT: &str = "─────────";

/// Bullet point character
pub const BULLET: &str = "•";

// ============================================================================
// Embed Builders
// ============================================================================

/// Create a standard/primary embed
pub fn standard_embed() -> CreateEmbed {
    CreateEmbed::new().color(PRIMARY_COLOR)
}

/// Create a success embed
pub fn success_embed() -> CreateEmbed {
    CreateEmbed::new().color(SUCCESS_COLOR)
}

/// Create an error embed
pub fn error_embed() -> CreateEmbed {
    CreateEmbed::new().color(ERROR_COLOR)
}

/// Create a warning embed
pub fn warning_embed() -> CreateEmbed {
    CreateEmbed::new().color(WARNING_COLOR)
}

/// Create an info/neutral embed
pub fn info_embed() -> CreateEmbed {
    CreateEmbed::new().color(INFO_COLOR)
}

/// Create a secondary/accent embed
pub fn secondary_embed() -> CreateEmbed {
    CreateEmbed::new().color(SECONDARY_COLOR)
}

// ============================================================================
// Text Helpers
// ============================================================================

/// Format a list of items with bullet points
pub fn bullet_list(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("{} {}", BULLET, item))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a section with a header and content
pub fn section(header: &str, content: &str) -> String {
    format!("**{}**\n{}", header, content)
}
