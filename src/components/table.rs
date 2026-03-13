use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::*;

/// Renders a table header row with column labels and widths.
pub fn table_header(headers: &[(&str, f32)]) -> Div {
    let items: Vec<_> = headers
        .iter()
        .map(|(header, w)| {
            div()
                .w(px(*w))
                .text_size(px(11.))
                .text_color(text_dim())
                .child(header.to_string())
        })
        .collect();

    div()
        .w_full()
        .px(px(12.))
        .py(px(8.))
        .flex()
        .items_center()
        .bg(bg_header())
        .border_b_1()
        .border_color(border_primary())
        .children(items)
}

/// A single table cell with fixed width.
pub fn table_cell(width: f32, text: impl Into<String>, color: gpui::Rgba) -> Div {
    div()
        .w(px(width))
        .text_size(px(12.))
        .text_color(color)
        .child(text.into())
}

/// A standard table data row with zebra-striping.
pub fn table_row(index: usize) -> Div {
    div()
        .w_full()
        .px(px(12.))
        .py(px(6.))
        .flex()
        .items_center()
        .bg(row_bg(index))
        .hover(|s| s.bg(bg_hover()))
}

/// Empty state placeholder for tables with no data.
pub fn empty_state(message: &str) -> Div {
    div()
        .w_full()
        .py(px(32.))
        .flex()
        .justify_center()
        .child(
            div()
                .text_size(px(13.))
                .text_color(text_disabled())
                .child(message.to_string()),
        )
}

/// Formats a unix-ms timestamp as relative time (e.g. "5m ago").
pub fn format_timestamp(ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let diff_secs = (now.saturating_sub(ts)) / 1000;
    if diff_secs < 60 {
        format!("{}s ago", diff_secs)
    } else if diff_secs < 3600 {
        format!("{}m ago", diff_secs / 60)
    } else if diff_secs < 86400 {
        format!("{}h ago", diff_secs / 3600)
    } else {
        format!("{}d ago", diff_secs / 86400)
    }
}
