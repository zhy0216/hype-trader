mod models;
mod services;
mod state;
mod views;

use gpui::prelude::*;
use gpui::{div, Application, Bounds, Entity, WindowBounds, WindowOptions};
use views::welcome_view::WelcomeView;

struct HypeTrader {
    welcome_view: Entity<WelcomeView>,
}

impl HypeTrader {
    fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let welcome_view = cx.new(|cx| WelcomeView::new(window, cx));
        Self { welcome_view }
    }
}

impl Render for HypeTrader {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div().size_full().child(self.welcome_view.clone())
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
                let inner_view = cx.new(|cx| HypeTrader::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(inner_view, window, cx))
            },
        )
        .unwrap();
    });
}
