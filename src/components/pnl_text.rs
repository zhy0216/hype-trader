use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::pnl_color;

/// Displays a value with green (positive) or red (negative) coloring.
/// format options: "signed" (+1.23), "percent" (+1.23%), default (1.23)
pub fn pnl_text(value: f64, format: &str, font_size: f32) -> Div {
    let text = match format {
        "signed" => format!("{:+.2}", value),
        "percent" => format!("{:+.2}%", value),
        _ => format!("{:.2}", value),
    };
    div()
        .text_size(px(font_size))
        .text_color(pnl_color(value))
        .child(text)
}
