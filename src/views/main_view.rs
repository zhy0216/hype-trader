use gpui::prelude::*;
use gpui::div;

pub struct MainView;

impl MainView {
    pub fn new() -> Self {
        Self
    }
}

impl Render for MainView {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::rgb(0x1a1a2e))
            .child("Main trading view - coming soon")
    }
}
