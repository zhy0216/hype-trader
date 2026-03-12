use gpui::prelude::*;
use gpui::div;

pub struct TopBar;

impl TopBar {
    pub fn new() -> Self {
        Self
    }
}

impl Render for TopBar {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .h(gpui::px(48.))
            .w_full()
            .bg(gpui::rgb(0x16213e))
            .flex()
            .items_center()
            .px(gpui::px(16.))
            .child(
                div().text_color(gpui::rgb(0xe94560)).child("Hype Trader"),
            )
    }
}
