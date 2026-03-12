mod models;
mod services;
mod state;
mod views;

use gpui::prelude::*;
use gpui::{div, Application, Bounds, Entity, Subscription, WindowBounds, WindowOptions};
use views::main_view::MainView;
use views::welcome_view::{WelcomeEvent, WelcomeView};

enum AppScreen {
    Welcome(Entity<WelcomeView>),
    Trading(Entity<MainView>),
}

struct HypeTrader {
    screen: AppScreen,
    _subscription: Option<Subscription>,
}

impl HypeTrader {
    fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let welcome_view = cx.new(|cx| WelcomeView::new(window, cx));
        let subscription =
            cx.subscribe_in(&welcome_view, window, Self::on_welcome_event);
        Self {
            screen: AppScreen::Welcome(welcome_view),
            _subscription: Some(subscription),
        }
    }

    fn on_welcome_event(
        &mut self,
        _welcome: &Entity<WelcomeView>,
        event: &WelcomeEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let (key, net) = match event {
            WelcomeEvent::ConnectWallet { private_key, network } => {
                (Some(private_key.clone()), *network)
            }
            WelcomeEvent::BrowseReadOnly { network } => (None, *network),
        };
        let main_view = cx.new(|cx| MainView::new(key, net, window, cx));
        self.screen = AppScreen::Trading(main_view);
        self._subscription = None;
    }
}

impl Render for HypeTrader {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        match &self.screen {
            AppScreen::Welcome(view) => div().size_full().child(view.clone()),
            AppScreen::Trading(view) => div().size_full().child(view.clone()),
        }
    }
}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let _guard = rt.enter();

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
