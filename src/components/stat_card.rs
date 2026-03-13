use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::*;

/// A small card displaying a label and a colored value.
pub fn stat_card(label: &str, value: &str, value_color: gpui::Rgba) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(14.))
        .rounded(px(8.))
        .bg(bg_elevated())
        .border_1()
        .border_color(border_primary())
        .child(
            div()
                .text_size(px(11.))
                .text_color(text_dim())
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(value_color)
                .child(value.to_string()),
        )
}
