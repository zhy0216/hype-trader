mod models;
mod services;
mod state;

use gpui::prelude::*;
use gpui::{Application, WindowOptions, WindowBounds, Bounds};

struct HypeTrader;

impl Render for HypeTrader {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        gpui::div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child("Hype Trader - Loading...")
    }
}

fn main() {
    Application::new().run(|cx| {
        gpui_component::init(cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    gpui::size(gpui::px(1400.), gpui::px(900.)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let inner_view = cx.new(|_cx| HypeTrader);
                cx.new(|cx| gpui_component::Root::new(inner_view, window, cx))
            },
        )
        .unwrap();
    });
}
