use gpui::prelude::*;
use gpui::{div, px, rgb, Entity};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};

use crate::models::{OrderSide, OrderType};

pub struct OrderPanel {
    pub side: OrderSide,
    pub order_type: OrderType,
    pub symbol: String,
    price_input: Entity<InputState>,
    size_input: Entity<InputState>,
}

impl OrderPanel {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let price_input = cx.new(|cx| InputState::new(window, cx).placeholder("Price"));
        let size_input = cx.new(|cx| InputState::new(window, cx).placeholder("Size"));
        Self {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            symbol: "ETH-USD".to_string(),
            price_input,
            size_input,
        }
    }
}

impl Render for OrderPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let show_price = self.order_type != OrderType::Market;

        let mut root = div()
            .w_full()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .bg(rgb(0x16213e))
            // Order type tabs
            .child(
                div()
                    .flex()
                    .gap(px(4.))
                    .child(
                        Button::new("type-limit")
                            .label("Limit")
                            .compact()
                            .map(|b| {
                                if self.order_type == OrderType::Limit {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.order_type = OrderType::Limit;
                            })),
                    )
                    .child(
                        Button::new("type-market")
                            .label("Market")
                            .compact()
                            .map(|b| {
                                if self.order_type == OrderType::Market {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.order_type = OrderType::Market;
                            })),
                    )
                    .child(
                        Button::new("type-tpsl")
                            .label("TP/SL")
                            .compact()
                            .map(|b| {
                                if matches!(
                                    self.order_type,
                                    OrderType::TakeProfit | OrderType::StopLoss
                                ) {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.order_type = OrderType::TakeProfit;
                            })),
                    ),
            )
            // Buy / Sell toggle
            .child(
                div()
                    .flex()
                    .gap(px(4.))
                    .child(
                        Button::new("side-buy")
                            .label("Buy / Long")
                            .compact()
                            .w_full()
                            .map(|b| {
                                if self.side == OrderSide::Buy {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.side = OrderSide::Buy;
                            })),
                    )
                    .child(
                        Button::new("side-sell")
                            .label("Sell / Short")
                            .compact()
                            .w_full()
                            .map(|b| {
                                if self.side == OrderSide::Sell {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.side = OrderSide::Sell;
                            })),
                    ),
            );

        // Price input (only for Limit / TP-SL, not Market)
        if show_price {
            root = root.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0xaaaaaa))
                            .child("Price"),
                    )
                    .child(Input::new(&self.price_input)),
            );
        }

        root
            // Size input
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0xaaaaaa))
                            .child("Size"),
                    )
                    .child(Input::new(&self.size_input)),
            )
            // Percentage buttons
            .child(
                div()
                    .flex()
                    .gap(px(4.))
                    .child(Button::new("pct-25").label("25%").compact().ghost())
                    .child(Button::new("pct-50").label("50%").compact().ghost())
                    .child(Button::new("pct-75").label("75%").compact().ghost())
                    .child(Button::new("pct-100").label("100%").compact().ghost()),
            )
            // Submit button
            .child(
                Button::new("submit-order")
                    .label(match self.side {
                        OrderSide::Buy => "Buy / Long",
                        OrderSide::Sell => "Sell / Short",
                    })
                    .primary()
                    .w_full(),
            )
    }
}
