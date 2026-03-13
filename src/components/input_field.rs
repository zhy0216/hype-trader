use gpui::prelude::*;
use gpui::{div, px, Entity, Div};
use gpui_component::input::{Input, InputState};

use super::theme::*;

/// A labeled input field (label on top, input below).
pub fn input_field(label: &str, input: &Entity<InputState>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(13.))
                .text_color(text_secondary())
                .child(label.to_string()),
        )
        .child(Input::new(input))
}
