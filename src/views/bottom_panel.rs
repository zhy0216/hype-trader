use gpui::prelude::*;
use gpui::{div, px, Entity, SharedString};
use gpui_component::button::Button;
use gpui_component::tab::{Tab, TabBar};

use crate::components::theme::*;
use crate::components::table::{table_header, table_cell, table_row, empty_state, format_timestamp};
use crate::components::stat_card::stat_card;
use crate::models::*;
use crate::services::exchange_service::ExchangeService;
use crate::services::info_service::InfoService;
use crate::services::wallet_service;
use crate::views::toast::{Toast, ToastKind};

pub struct BottomPanel {
    pub active_tab: BottomTab,
    pub positions: Vec<Position>,
    pub open_orders: Vec<OpenOrder>,
    pub trade_history: Vec<TradeHistory>,
    pub balances: Vec<Balance>,
    pub pnl: PnlSummary,
    pub private_key: Option<String>,
    pub network: Network,
    toast: Entity<Toast>,
}

impl BottomPanel {
    pub fn new(
        private_key: Option<String>,
        network: Network,
        toast: Entity<Toast>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Self {
        Self {
            active_tab: BottomTab::Positions,
            positions: Vec::new(),
            open_orders: Vec::new(),
            trade_history: Vec::new(),
            balances: Vec::new(),
            pnl: PnlSummary::default(),
            private_key,
            network,
            toast,
        }
    }

    fn tab_index(&self) -> usize {
        match self.active_tab {
            BottomTab::Positions => 0,
            BottomTab::OpenOrders => 1,
            BottomTab::TradeHistory => 2,
            BottomTab::Funds => 3,
        }
    }

    /// Strip the "-USD" suffix to get the coin name the SDK expects (e.g. "ETH").
    fn sdk_coin(symbol: &str) -> String {
        symbol
            .strip_suffix("-USD")
            .unwrap_or(symbol)
            .to_string()
    }
}

impl Render for BottomPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let handle = cx.entity().clone();

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(bg_panel())
            // Tab bar
            .child(
                TabBar::new("bottom-tabs")
                    .selected_index(self.tab_index())
                    .on_click(move |ix: &usize, _window, cx| {
                        let ix = *ix;
                        handle.update(cx, move |this, _cx| {
                            this.active_tab = match ix {
                                0 => BottomTab::Positions,
                                1 => BottomTab::OpenOrders,
                                2 => BottomTab::TradeHistory,
                                _ => BottomTab::Funds,
                            };
                        });
                    })
                    .child(Tab::new().label("Positions"))
                    .child(Tab::new().label("Orders"))
                    .child(Tab::new().label("History"))
                    .child(Tab::new().label("Funds")),
            )
            // Content area
            .child(
                div()
                    .flex_1()
                    .id("bottom-panel-scroll")
                    .overflow_y_scroll()
                    .child(match self.active_tab {
                        BottomTab::Positions => self.render_positions(cx).into_any_element(),
                        BottomTab::OpenOrders => self.render_orders(cx).into_any_element(),
                        BottomTab::TradeHistory => self.render_history().into_any_element(),
                        BottomTab::Funds => self.render_funds().into_any_element(),
                    }),
            )
    }
}

// Render helpers
impl BottomPanel {
    fn render_positions(&self, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let has_key = self.private_key.is_some();

        div()
            .w_full()
            .flex()
            .flex_col()
            // Header row
            .child(table_header(&[
                ("Symbol", 80.), ("Side", 60.), ("Size", 70.), ("Entry", 80.),
                ("Mark", 80.), ("PnL", 80.), ("Lev.", 50.), ("Action", 60.),
            ]))
            // Data rows
            .children(self.positions.iter().enumerate().map(|(i, pos)| {
                let side_text = match pos.side {
                    OrderSide::Buy => "Long",
                    OrderSide::Sell => "Short",
                };

                // Capture values for the close button click handler
                let coin = Self::sdk_coin(&pos.symbol);
                let size = pos.size;
                let private_key = self.private_key.clone();
                let network = self.network;

                table_row(i)
                    .child(table_cell(80., pos.symbol.clone(), text_muted()))
                    .child(table_cell(60., side_text, side_color(pos.side)))
                    .child(table_cell(70., format!("{:.3}", pos.size), text_muted()))
                    .child(table_cell(80., format!("{:.2}", pos.entry_price), text_muted()))
                    .child(table_cell(80., format!("{:.2}", pos.mark_price), text_muted()))
                    .child(table_cell(80., format!("{:+.2}", pos.unrealized_pnl), pnl_color(pos.unrealized_pnl)))
                    .child(table_cell(50., format!("{}x", pos.leverage), text_dim()))
                    .child(
                        div().w(px(60.)).child(
                            Button::new(SharedString::from(format!("close-{}", i)))
                                .label("Close")
                                .compact()
                                .outline()
                                .when(has_key, |btn| {
                                    btn.on_click(cx.listener(move |this, _, _window, cx| {
                                        let Some(ref key) = private_key else { return; };
                                        let wallet = match wallet_service::wallet_from_key(key) {
                                            Ok(w) => w,
                                            Err(e) => {
                                                tracing::error!("Failed to create wallet: {}", e);
                                                return;
                                            }
                                        };
                                        let coin = coin.clone();
                                        let network = network;
                                        let private_key_for_refetch = this.private_key.clone();
                                        let toast = this.toast.clone();

                                        cx.spawn(async move |this_handle, mut cx| {
                                            let mut service = ExchangeService::new(network);
                                            if let Err(e) = service.connect(wallet).await {
                                                tracing::error!("Failed to connect ExchangeService: {}", e);
                                                return;
                                            }
                                            if let Err(e) = service.market_close(&coin, Some(size)).await {
                                                tracing::error!("Failed to close position {}: {}", coin, e);
                                                let _ = cx.update_entity(&toast, |t: &mut Toast, cx| {
                                                    t.show(format!("Failed to close position: {}", e), ToastKind::Error, cx);
                                                });
                                                return;
                                            }
                                            tracing::info!("Position {} closed successfully", coin);
                                            let _ = cx.update_entity(&toast, |t: &mut Toast, cx| {
                                                t.show("Position closed", ToastKind::Success, cx);
                                            });

                                            // Re-fetch positions after close
                                            if let Some(ref key) = private_key_for_refetch {
                                                refetch_account_data(key, network, &this_handle, &mut cx).await;
                                            }
                                        })
                                        .detach();
                                    }))
                                }),
                        ),
                    )
            }))
            .when(self.positions.is_empty(), |el| {
                el.child(empty_state("No open positions"))
            })
    }

    fn render_orders(&self, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let has_key = self.private_key.is_some();

        div()
            .w_full()
            .flex()
            .flex_col()
            .child(table_header(&[
                ("Symbol", 80.), ("Side", 60.), ("Type", 60.), ("Price", 80.),
                ("Size", 70.), ("Filled", 70.), ("Action", 60.),
            ]))
            .children(self.open_orders.iter().enumerate().map(|(i, order)| {
                let side_text = match order.side {
                    OrderSide::Buy => "Buy",
                    OrderSide::Sell => "Sell",
                };
                let type_text = match order.order_type {
                    OrderType::Limit => "Limit",
                    OrderType::Market => "Market",
                    OrderType::TakeProfit => "TP",
                    OrderType::StopLoss => "SL",
                };

                // Capture values for the cancel button click handler
                let coin = Self::sdk_coin(&order.symbol);
                let oid: u64 = order.id.parse().unwrap_or(0);
                let private_key = self.private_key.clone();
                let network = self.network;

                table_row(i)
                    .child(table_cell(80., order.symbol.clone(), text_muted()))
                    .child(table_cell(60., side_text, side_color(order.side)))
                    .child(table_cell(60., type_text, text_dim()))
                    .child(table_cell(80., format!("{:.2}", order.price), text_muted()))
                    .child(table_cell(70., format!("{:.3}", order.size), text_muted()))
                    .child(table_cell(70., format!("{:.3}", order.filled), text_muted()))
                    .child(
                        div().w(px(60.)).child(
                            Button::new(SharedString::from(format!("cancel-{}", i)))
                                .label("Cancel")
                                .compact()
                                .outline()
                                .when(has_key, |btn| {
                                    btn.on_click(cx.listener(move |this, _, _window, cx| {
                                        let Some(ref key) = private_key else { return; };
                                        let wallet = match wallet_service::wallet_from_key(key) {
                                            Ok(w) => w,
                                            Err(e) => {
                                                tracing::error!("Failed to create wallet: {}", e);
                                                return;
                                            }
                                        };
                                        let coin = coin.clone();
                                        let network = network;
                                        let private_key_for_refetch = this.private_key.clone();
                                        let toast = this.toast.clone();

                                        cx.spawn(async move |this_handle, mut cx| {
                                            let mut service = ExchangeService::new(network);
                                            if let Err(e) = service.connect(wallet).await {
                                                tracing::error!("Failed to connect ExchangeService: {}", e);
                                                return;
                                            }
                                            if let Err(e) = service.cancel_order(&coin, oid).await {
                                                tracing::error!("Failed to cancel order {} (oid {}): {}", coin, oid, e);
                                                let _ = cx.update_entity(&toast, |t: &mut Toast, cx| {
                                                    t.show(format!("Failed to cancel order: {}", e), ToastKind::Error, cx);
                                                });
                                                return;
                                            }
                                            tracing::info!("Order {} (oid {}) cancelled successfully", coin, oid);
                                            let _ = cx.update_entity(&toast, |t: &mut Toast, cx| {
                                                t.show("Order cancelled", ToastKind::Success, cx);
                                            });

                                            // Re-fetch orders after cancel
                                            if let Some(ref key) = private_key_for_refetch {
                                                refetch_account_data(key, network, &this_handle, &mut cx).await;
                                            }
                                        })
                                        .detach();
                                    }))
                                }),
                        ),
                    )
            }))
            .when(self.open_orders.is_empty(), |el| {
                el.child(empty_state("No open orders"))
            })
    }

    fn render_history(&self) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(table_header(&[
                ("Time", 120.), ("Symbol", 80.), ("Side", 60.), ("Price", 80.),
                ("Size", 70.), ("Fee", 70.),
            ]))
            .children(self.trade_history.iter().enumerate().map(|(i, trade)| {
                let side_text = match trade.side {
                    OrderSide::Buy => "Buy",
                    OrderSide::Sell => "Sell",
                };
                table_row(i)
                    .child(table_cell(120., format_timestamp(trade.timestamp), text_dim()))
                    .child(table_cell(80., trade.symbol.clone(), text_muted()))
                    .child(table_cell(60., side_text, side_color(trade.side)))
                    .child(table_cell(80., format!("{:.2}", trade.price), text_muted()))
                    .child(table_cell(70., format!("{:.3}", trade.size), text_muted()))
                    .child(table_cell(70., format!("{:.4}", trade.fee), color_orange()))
            }))
            .when(self.trade_history.is_empty(), |el| {
                el.child(empty_state("No trade history"))
            })
    }

    fn render_funds(&self) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(12.))
            // PnL summary cards
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .child(stat_card(
                        "Total Balance",
                        &format!("${:.2}", self.pnl.total_balance),
                        text_primary(),
                    ))
                    .child(stat_card(
                        "Available",
                        &format!("${:.2}", self.pnl.available_balance),
                        color_green(),
                    ))
                    .child(stat_card(
                        "Margin Used",
                        &format!("${:.2}", self.pnl.margin_used),
                        color_yellow(),
                    ))
                    .child(stat_card(
                        "Total PnL",
                        &format!("${:+.2}", self.pnl.total_pnl),
                        pnl_color(self.pnl.total_pnl),
                    ))
                    .child(stat_card(
                        "Daily PnL",
                        &format!("${:+.2}", self.pnl.daily_pnl),
                        pnl_color(self.pnl.daily_pnl),
                    )),
            )
            // Balance table
            .child(table_header(&[
                ("Asset", 80.), ("Total", 100.), ("Available", 100.), ("In Margin", 100.),
            ]))
            .children(self.balances.iter().enumerate().map(|(i, bal)| {
                table_row(i)
                    .child(table_cell(80., bal.asset.clone(), text_muted()))
                    .child(table_cell(100., format!("{:.2}", bal.total), text_muted()))
                    .child(table_cell(100., format!("{:.2}", bal.available), color_green()))
                    .child(table_cell(100., format!("{:.2}", bal.in_margin), color_yellow()))
            }))
    }
}

/// Re-fetch positions, orders, and balances after a successful action.
async fn refetch_account_data(
    private_key: &str,
    network: Network,
    panel_handle: &gpui::WeakEntity<BottomPanel>,
    cx: &mut gpui::AsyncApp,
) {
    let address = match wallet_service::address_from_key(private_key) {
        Ok(addr) => addr,
        Err(e) => {
            tracing::error!("Failed to derive address for refetch: {}", e);
            return;
        }
    };

    let address_h160: ethers::types::H160 = match address.parse() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to parse address: {}", e);
            return;
        }
    };

    let info = match InfoService::new(network).await {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Failed to create InfoService for refetch: {}", e);
            return;
        }
    };

    // Fetch positions and PnL
    let positions_result = info.fetch_user_state(address_h160).await;
    let orders_result = info.fetch_open_orders(address_h160).await;
    let balances_result = info.fetch_balances(address_h160).await;

    let _ = panel_handle.update(cx, |panel, _cx| {
        if let Ok((positions, pnl)) = positions_result {
            panel.positions = positions;
            panel.pnl = pnl;
        }
        if let Ok(orders) = orders_result {
            panel.open_orders = orders;
        }
        if let Ok(balances) = balances_result {
            panel.balances = balances;
        }
    });
}
