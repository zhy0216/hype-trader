use gpui::prelude::*;
use gpui::div;

use crate::models::BottomTab;

pub struct MainView {
    pub active_tab: BottomTab,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            active_tab: BottomTab::Positions,
        }
    }
}

impl Render for MainView {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let tab_color = |tab: BottomTab| -> gpui::Hsla {
            if self.active_tab == tab {
                gpui::rgb(0xe94560).into()
            } else {
                gpui::rgb(0xaaaaaa).into()
            }
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(gpui::rgb(0x1a1a2e))
            // Main content area
            .child(
                div()
                    .flex_1()
                    .flex()
                    // Left: Symbol list
                    .child(
                        div()
                            .w(gpui::px(200.))
                            .border_r_1()
                            .border_color(gpui::rgb(0x0f3460))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .p(gpui::px(12.))
                                    .text_color(gpui::rgb(0xaaaaaa))
                                    .child("Symbol List"),
                            ),
                    )
                    // Center: Chart + Order panel
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                // Chart area
                                div()
                                    .flex_1()
                                    .border_b_1()
                                    .border_color(gpui::rgb(0x0f3460))
                                    .p(gpui::px(12.))
                                    .text_color(gpui::rgb(0xaaaaaa))
                                    .child("Candle Chart Area"),
                            )
                            .child(
                                // Order panel
                                div()
                                    .h(gpui::px(200.))
                                    .border_b_1()
                                    .border_color(gpui::rgb(0x0f3460))
                                    .p(gpui::px(12.))
                                    .text_color(gpui::rgb(0xaaaaaa))
                                    .child("Order Panel"),
                            ),
                    )
                    // Right: Order book
                    .child(
                        div()
                            .w(gpui::px(280.))
                            .border_l_1()
                            .border_color(gpui::rgb(0x0f3460))
                            .p(gpui::px(12.))
                            .text_color(gpui::rgb(0xaaaaaa))
                            .child("Order Book"),
                    ),
            )
            // Bottom panel
            .child(
                div()
                    .h(gpui::px(250.))
                    .border_t_1()
                    .border_color(gpui::rgb(0x0f3460))
                    .flex()
                    .flex_col()
                    // Tab bar
                    .child(
                        div()
                            .flex()
                            .gap(gpui::px(4.))
                            .p(gpui::px(8.))
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(tab_color(BottomTab::Positions))
                                    .child("Positions"),
                            )
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(tab_color(BottomTab::OpenOrders))
                                    .child(" | Orders"),
                            )
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(tab_color(BottomTab::TradeHistory))
                                    .child(" | History"),
                            )
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(tab_color(BottomTab::Funds))
                                    .child(" | Funds"),
                            ),
                    )
                    // Table content
                    .child(
                        div()
                            .flex_1()
                            .p(gpui::px(12.))
                            .text_color(gpui::rgb(0x888888))
                            .child("Table data here..."),
                    ),
            )
    }
}
