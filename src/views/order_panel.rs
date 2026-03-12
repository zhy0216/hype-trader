use gpui::prelude::*;
use gpui::{div, px, rgb, Entity};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};

use gpui_component::Disableable as _;

use crate::models::{Network, OrderSide, OrderType};
use crate::services::exchange_service::ExchangeService;
use crate::services::wallet_service;
use crate::views::toast::{Toast, ToastKind};

pub struct OrderPanel {
    pub side: OrderSide,
    pub order_type: OrderType,
    pub symbol: String,
    pub wallet_connected: bool,
    price_input: Entity<InputState>,
    size_input: Entity<InputState>,
    private_key: Option<String>,
    network: Network,
    toast: Entity<Toast>,
}

impl OrderPanel {
    pub fn new(
        wallet_connected: bool,
        private_key: Option<String>,
        network: Network,
        toast: Entity<Toast>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let price_input = cx.new(|cx| InputState::new(window, cx).placeholder("Price"));
        let size_input = cx.new(|cx| InputState::new(window, cx).placeholder("Size"));
        Self {
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            symbol: "ETH-USD".to_string(),
            wallet_connected,
            price_input,
            size_input,
            private_key,
            network,
            toast,
        }
    }

    /// Strip the "-USD" suffix to get the coin name the SDK expects (e.g. "ETH").
    fn sdk_coin(&self) -> String {
        self.symbol
            .strip_suffix("-USD")
            .unwrap_or(&self.symbol)
            .to_string()
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
                    .w_full()
                    .disabled(!self.wallet_connected)
                    .on_click(cx.listener(|this, _, window, cx| {
                        let Some(ref key) = this.private_key else {
                            tracing::error!("No private key available for order submission");
                            return;
                        };

                        // Read input values
                        let size_str = this.size_input.read(cx).value().to_string();
                        let size: f64 = match size_str.trim().parse() {
                            Ok(v) if v > 0.0 => v,
                            _ => {
                                tracing::error!("Invalid size input: '{}'", size_str);
                                return;
                            }
                        };

                        let order_type = this.order_type;
                        let is_buy = this.side == OrderSide::Buy;
                        let coin = this.sdk_coin();
                        let network = this.network;

                        // For limit / trigger orders, parse price
                        let price: Option<f64> = if order_type != OrderType::Market {
                            let price_str = this.price_input.read(cx).value().to_string();
                            match price_str.trim().parse() {
                                Ok(v) if v > 0.0 => Some(v),
                                _ => {
                                    tracing::error!("Invalid price input: '{}'", price_str);
                                    return;
                                }
                            }
                        } else {
                            None
                        };

                        let wallet = match wallet_service::wallet_from_key(key) {
                            Ok(w) => w,
                            Err(e) => {
                                tracing::error!("Failed to create wallet: {}", e);
                                return;
                            }
                        };

                        // Clone input handles for clearing after success
                        let price_input = this.price_input.clone();
                        let size_input = this.size_input.clone();
                        let toast = this.toast.clone();

                        cx.spawn_in(window, async move |_this, cx| {
                            let mut service = ExchangeService::new(network);
                            if let Err(e) = service.connect(wallet).await {
                                tracing::error!("Failed to connect ExchangeService: {}", e);
                                return;
                            }

                            let result = match order_type {
                                OrderType::Limit => {
                                    service
                                        .place_limit_order(&coin, is_buy, price.unwrap(), size, false)
                                        .await
                                }
                                OrderType::Market => {
                                    service.place_market_order(&coin, is_buy, size).await
                                }
                                OrderType::TakeProfit => {
                                    service
                                        .place_trigger_order(&coin, is_buy, price.unwrap(), size, true)
                                        .await
                                }
                                OrderType::StopLoss => {
                                    service
                                        .place_trigger_order(&coin, is_buy, price.unwrap(), size, false)
                                        .await
                                }
                            };

                            match result {
                                Ok(resp) => {
                                    tracing::info!("Order placed successfully: {}", resp);
                                    // Clear form inputs on success
                                    let _ = cx.update(|window, cx| {
                                        price_input.update(cx, |state, cx| {
                                            state.set_value("", window, cx);
                                        });
                                        size_input.update(cx, |state, cx| {
                                            state.set_value("", window, cx);
                                        });
                                        toast.update(cx, |t, cx| {
                                            t.show("Order placed successfully", ToastKind::Success, cx);
                                        });
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("Order placement failed: {}", e);
                                    let _ = cx.update(|_window, cx| {
                                        toast.update(cx, |t, cx| {
                                            t.show(format!("Failed to place order: {}", e), ToastKind::Error, cx);
                                        });
                                    });
                                }
                            }
                        })
                        .detach();
                    })),
            )
    }
}
