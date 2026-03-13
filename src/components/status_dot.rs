use gpui::prelude::*;
use gpui::{div, px, Div, Rgba};

/// A small colored circle indicator.
pub fn status_dot(color: Rgba) -> Div {
    div()
        .w(px(8.))
        .h(px(8.))
        .rounded(px(4.))
        .bg(color)
}
