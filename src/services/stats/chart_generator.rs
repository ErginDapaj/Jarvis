use std::io::Cursor;

use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};

use crate::bot::error::Error;
use crate::services::stats::aggregator::UserStats;

/// Chart dimensions - Higher resolution for crisp text
const CHART_WIDTH: u32 = 1200;
const CHART_HEIGHT: u32 = 600;

// Modern color palette - Discord dark theme inspired
const BG_COLOR: RGBColor = RGBColor(49, 51, 56);         // Discord dark bg
const CARD_COLOR: RGBColor = RGBColor(43, 45, 49);       // Slightly darker for contrast
const TEXT_COLOR: RGBColor = RGBColor(255, 255, 255);    // White text
const TEXT_MUTED: RGBColor = RGBColor(148, 155, 164);    // Muted text
const ACCENT_BLUE: RGBColor = RGBColor(88, 101, 242);    // Discord blurple
const ACCENT_GREEN: RGBColor = RGBColor(87, 242, 135);   // Discord green
const ACCENT_RED: RGBColor = RGBColor(237, 66, 69);      // Discord red
const ACCENT_PURPLE: RGBColor = RGBColor(155, 89, 182);  // Purple
const ACCENT_AMBER: RGBColor = RGBColor(250, 166, 26);   // Discord yellow/amber

/// Stat bar data
struct StatBar {
    label: &'static str,
    value: i32,
    color: RGBColor,
}

/// Generate a modern PNG chart showing user statistics
pub fn generate_user_stats_chart(stats: &UserStats, username: &str) -> Result<Vec<u8>, Error> {
    let mut buffer = vec![0u8; (CHART_WIDTH * CHART_HEIGHT * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (CHART_WIDTH, CHART_HEIGHT))
            .into_drawing_area();

        // Dark background
        root.fill(&BG_COLOR).map_err(|e| Error::custom(e.to_string()))?;

        let bars = [
            StatBar { label: "Mutes Received", value: stats.mutes_received as i32, color: ACCENT_BLUE },
            StatBar { label: "Mutes Given", value: stats.mutes_given as i32, color: ACCENT_GREEN },
            StatBar { label: "Bans Received", value: stats.bans_received as i32, color: ACCENT_RED },
            StatBar { label: "Bans Given", value: stats.bans_given as i32, color: ACCENT_PURPLE },
            StatBar { label: "Spam Infractions", value: stats.spam_infractions as i32, color: ACCENT_AMBER },
        ];

        let max_value = bars.iter().map(|b| b.value).max().unwrap_or(1).max(1);

        // Layout constants
        let margin = 60;
        let title_height = 80;
        let bar_area_top = title_height + 40;
        let bar_area_bottom = CHART_HEIGHT as i32 - margin - 40;
        let bar_area_left = margin + 150; // Space for labels
        let bar_area_right = CHART_WIDTH as i32 - margin;
        let bar_height = 50;
        let bar_spacing = 20;

        // Draw title
        let title = format!("Statistics for {}", username);
        root.draw(&Text::new(
            title,
            (CHART_WIDTH as i32 / 2, 45),
            ("sans-serif", 36).into_font().color(&TEXT_COLOR).pos(Pos::new(HPos::Center, VPos::Center)),
        )).map_err(|e| Error::custom(e.to_string()))?;

        // Draw horizontal bars
        let bar_area_width = bar_area_right - bar_area_left;

        for (i, bar) in bars.iter().enumerate() {
            let y_center = bar_area_top + (i as i32) * (bar_height + bar_spacing) + bar_height / 2;
            let y_top = y_center - bar_height / 2;
            let y_bottom = y_center + bar_height / 2;

            // Draw label on the left
            root.draw(&Text::new(
                bar.label,
                (bar_area_left - 15, y_center),
                ("sans-serif", 22).into_font().color(&TEXT_MUTED).pos(Pos::new(HPos::Right, VPos::Center)),
            )).map_err(|e| Error::custom(e.to_string()))?;

            // Draw background bar (track)
            root.draw(&Rectangle::new(
                [(bar_area_left, y_top), (bar_area_right, y_bottom)],
                CARD_COLOR.filled(),
            )).map_err(|e| Error::custom(e.to_string()))?;

            // Draw value bar
            let bar_width = if max_value > 0 {
                ((bar.value as f64 / max_value as f64) * bar_area_width as f64) as i32
            } else {
                0
            };

            if bar_width > 0 {
                // Draw with rounded appearance using main bar
                root.draw(&Rectangle::new(
                    [(bar_area_left, y_top + 2), (bar_area_left + bar_width, y_bottom - 2)],
                    bar.color.filled(),
                )).map_err(|e| Error::custom(e.to_string()))?;
            }

            // Draw value text
            let value_text = bar.value.to_string();
            let text_x = if bar_width > 60 {
                bar_area_left + bar_width - 10
            } else {
                bar_area_left + bar_width + 15
            };
            let text_color = if bar_width > 60 { TEXT_COLOR } else { TEXT_MUTED };
            let text_align = if bar_width > 60 { HPos::Right } else { HPos::Left };

            root.draw(&Text::new(
                value_text,
                (text_x, y_center),
                ("sans-serif", 24).into_font().color(&text_color).pos(Pos::new(text_align, VPos::Center)),
            )).map_err(|e| Error::custom(e.to_string()))?;
        }

        // Draw timeout level indicator at the bottom
        let timeout_y = bar_area_bottom + 30;
        let timeout_text = format!("Current Timeout Level: {}", stats.current_timeout_level);
        root.draw(&Text::new(
            timeout_text,
            (CHART_WIDTH as i32 / 2, timeout_y),
            ("sans-serif", 20).into_font().color(&TEXT_MUTED).pos(Pos::new(HPos::Center, VPos::Center)),
        )).map_err(|e| Error::custom(e.to_string()))?;

        root.present().map_err(|e| Error::custom(e.to_string()))?;
    }

    // Encode as PNG
    let img = image::RgbImage::from_raw(CHART_WIDTH, CHART_HEIGHT, buffer)
        .ok_or_else(|| Error::custom("Failed to create image buffer"))?;

    let mut png_buffer = Cursor::new(Vec::new());
    img.write_to(&mut png_buffer, image::ImageFormat::Png)
        .map_err(|e| Error::custom(e.to_string()))?;

    Ok(png_buffer.into_inner())
}

/// Generate a modern bar chart for guild statistics
pub fn generate_guild_stats_chart(
    total_mutes: i64,
    total_bans: i64,
    active_channels: i64,
    guild_name: &str,
) -> Result<Vec<u8>, Error> {
    let mut buffer = vec![0u8; (CHART_WIDTH * CHART_HEIGHT * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (CHART_WIDTH, CHART_HEIGHT))
            .into_drawing_area();

        // Dark background
        root.fill(&BG_COLOR).map_err(|e| Error::custom(e.to_string()))?;

        let bars = [
            StatBar { label: "Total Mutes", value: total_mutes as i32, color: ACCENT_BLUE },
            StatBar { label: "Total Bans", value: total_bans as i32, color: ACCENT_RED },
            StatBar { label: "Active Channels", value: active_channels as i32, color: ACCENT_GREEN },
        ];

        let max_value = bars.iter().map(|b| b.value).max().unwrap_or(1).max(1);

        // Layout constants
        let margin = 60;
        let title_height = 80;
        let bar_area_top = title_height + 60;
        let bar_area_left = margin + 180;
        let bar_area_right = CHART_WIDTH as i32 - margin;
        let bar_height = 60;
        let bar_spacing = 30;

        // Draw title
        let title = format!("Server Statistics: {}", guild_name);
        root.draw(&Text::new(
            title,
            (CHART_WIDTH as i32 / 2, 50),
            ("sans-serif", 36).into_font().color(&TEXT_COLOR).pos(Pos::new(HPos::Center, VPos::Center)),
        )).map_err(|e| Error::custom(e.to_string()))?;

        // Draw horizontal bars
        let bar_area_width = bar_area_right - bar_area_left;

        for (i, bar) in bars.iter().enumerate() {
            let y_center = bar_area_top + (i as i32) * (bar_height + bar_spacing) + bar_height / 2;
            let y_top = y_center - bar_height / 2;
            let y_bottom = y_center + bar_height / 2;

            // Draw label on the left
            root.draw(&Text::new(
                bar.label,
                (bar_area_left - 15, y_center),
                ("sans-serif", 24).into_font().color(&TEXT_MUTED).pos(Pos::new(HPos::Right, VPos::Center)),
            )).map_err(|e| Error::custom(e.to_string()))?;

            // Draw background bar (track)
            root.draw(&Rectangle::new(
                [(bar_area_left, y_top), (bar_area_right, y_bottom)],
                CARD_COLOR.filled(),
            )).map_err(|e| Error::custom(e.to_string()))?;

            // Draw value bar
            let bar_width = if max_value > 0 {
                ((bar.value as f64 / max_value as f64) * bar_area_width as f64) as i32
            } else {
                0
            };

            if bar_width > 0 {
                root.draw(&Rectangle::new(
                    [(bar_area_left, y_top + 2), (bar_area_left + bar_width, y_bottom - 2)],
                    bar.color.filled(),
                )).map_err(|e| Error::custom(e.to_string()))?;
            }

            // Draw value text
            let value_text = bar.value.to_string();
            let text_x = if bar_width > 60 {
                bar_area_left + bar_width - 10
            } else {
                bar_area_left + bar_width + 15
            };
            let text_color = if bar_width > 60 { TEXT_COLOR } else { TEXT_MUTED };
            let text_align = if bar_width > 60 { HPos::Right } else { HPos::Left };

            root.draw(&Text::new(
                value_text,
                (text_x, y_center),
                ("sans-serif", 28).into_font().color(&text_color).pos(Pos::new(text_align, VPos::Center)),
            )).map_err(|e| Error::custom(e.to_string()))?;
        }

        root.present().map_err(|e| Error::custom(e.to_string()))?;
    }

    // Encode as PNG
    let img = image::RgbImage::from_raw(CHART_WIDTH, CHART_HEIGHT, buffer)
        .ok_or_else(|| Error::custom("Failed to create image buffer"))?;

    let mut png_buffer = Cursor::new(Vec::new());
    img.write_to(&mut png_buffer, image::ImageFormat::Png)
        .map_err(|e| Error::custom(e.to_string()))?;

    Ok(png_buffer.into_inner())
}
