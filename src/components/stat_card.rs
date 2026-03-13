use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::*;

/// A small card displaying a label and a colored value.
pub fn stat_card(label: &str, value: &str, value_color: gpui::Rgba) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(10.))
        .rounded(px(6.))
        .bg(bg_header())
        .child(
            div()
                .text_size(px(11.))
                .text_color(text_dimmest())
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(14.))
                .text_color(value_color)
                .child(value.to_string()),
        )
}
