use gpui::prelude::*;
use gpui::{div, px, rgb, SharedString};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::tab::{Tab, TabBar};

use crate::models::*;

pub struct BottomPanel {
    pub active_tab: BottomTab,
    pub positions: Vec<Position>,
    pub open_orders: Vec<OpenOrder>,
    pub trade_history: Vec<TradeHistory>,
    pub balances: Vec<Balance>,
    pub pnl: PnlSummary,
}

impl BottomPanel {
    pub fn new(_window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> Self {
        Self {
            active_tab: BottomTab::Positions,
            positions: Vec::new(),
            open_orders: Vec::new(),
            trade_history: Vec::new(),
            balances: Vec::new(),
            pnl: PnlSummary::default(),
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
            .bg(rgb(0x16213e))
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
                        BottomTab::Positions => self.render_positions().into_any_element(),
                        BottomTab::OpenOrders => self.render_orders().into_any_element(),
                        BottomTab::TradeHistory => self.render_history().into_any_element(),
                        BottomTab::Funds => self.render_funds().into_any_element(),
                    }),
            )
    }
}

// Render helpers
impl BottomPanel {
    fn render_positions(&self) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            // Header row
            .child(table_header(&[
                "Symbol", "Side", "Size", "Entry", "Mark", "PnL", "Lev.", "Action",
            ]))
            // Data rows
            .children(self.positions.iter().enumerate().map(|(i, pos)| {
                let pnl_color = if pos.unrealized_pnl >= 0.0 {
                    rgb(0x00ff88)
                } else {
                    rgb(0xff4444)
                };
                let side_color = match pos.side {
                    OrderSide::Buy => rgb(0x00ff88),
                    OrderSide::Sell => rgb(0xff4444),
                };
                let side_text = match pos.side {
                    OrderSide::Buy => "Long",
                    OrderSide::Sell => "Short",
                };
                div()
                    .w_full()
                    .px(px(10.))
                    .py(px(4.))
                    .flex()
                    .items_center()
                    .bg(if i % 2 == 0 {
                        rgb(0x16213e)
                    } else {
                        rgb(0x1a2744)
                    })
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(pos.symbol.clone()),
                    )
                    .child(
                        div()
                            .w(px(60.))
                            .text_size(px(12.))
                            .text_color(side_color)
                            .child(side_text),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.3}", pos.size)),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.2}", pos.entry_price)),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.2}", pos.mark_price)),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(pnl_color)
                            .child(format!("{:+.2}", pos.unrealized_pnl)),
                    )
                    .child(
                        div()
                            .w(px(50.))
                            .text_size(px(12.))
                            .text_color(rgb(0xaaaaaa))
                            .child(format!("{}x", pos.leverage)),
                    )
                    .child(
                        div().w(px(60.)).child(
                            Button::new(SharedString::from(format!("close-{}", i)))
                                .label("Close")
                                .compact()
                                .ghost(),
                        ),
                    )
            }))
            .when(self.positions.is_empty(), |el| {
                el.child(empty_state("No open positions"))
            })
    }

    fn render_orders(&self) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(table_header(&[
                "Symbol", "Side", "Type", "Price", "Size", "Filled", "Action",
            ]))
            .children(self.open_orders.iter().enumerate().map(|(i, order)| {
                let side_color = match order.side {
                    OrderSide::Buy => rgb(0x00ff88),
                    OrderSide::Sell => rgb(0xff4444),
                };
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
                div()
                    .w_full()
                    .px(px(10.))
                    .py(px(4.))
                    .flex()
                    .items_center()
                    .bg(if i % 2 == 0 {
                        rgb(0x16213e)
                    } else {
                        rgb(0x1a2744)
                    })
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(order.symbol.clone()),
                    )
                    .child(
                        div()
                            .w(px(60.))
                            .text_size(px(12.))
                            .text_color(side_color)
                            .child(side_text),
                    )
                    .child(
                        div()
                            .w(px(60.))
                            .text_size(px(12.))
                            .text_color(rgb(0xaaaaaa))
                            .child(type_text),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.2}", order.price)),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.3}", order.size)),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .text_size(px(12.))
                            .text_color(rgb(0xaaaaaa))
                            .child(format!("{:.3}", order.filled)),
                    )
                    .child(
                        div().w(px(60.)).child(
                            Button::new(SharedString::from(format!("cancel-{}", i)))
                                .label("Cancel")
                                .compact()
                                .ghost(),
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
                "Time", "Symbol", "Side", "Price", "Size", "Fee",
            ]))
            .children(self.trade_history.iter().enumerate().map(|(i, trade)| {
                let side_color = match trade.side {
                    OrderSide::Buy => rgb(0x00ff88),
                    OrderSide::Sell => rgb(0xff4444),
                };
                let side_text = match trade.side {
                    OrderSide::Buy => "Buy",
                    OrderSide::Sell => "Sell",
                };
                div()
                    .w_full()
                    .px(px(10.))
                    .py(px(4.))
                    .flex()
                    .items_center()
                    .bg(if i % 2 == 0 {
                        rgb(0x16213e)
                    } else {
                        rgb(0x1a2744)
                    })
                    .child(
                        div()
                            .w(px(120.))
                            .text_size(px(12.))
                            .text_color(rgb(0xaaaaaa))
                            .child(format_timestamp(trade.timestamp)),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(trade.symbol.clone()),
                    )
                    .child(
                        div()
                            .w(px(60.))
                            .text_size(px(12.))
                            .text_color(side_color)
                            .child(side_text),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.2}", trade.price)),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.3}", trade.size)),
                    )
                    .child(
                        div()
                            .w(px(70.))
                            .text_size(px(12.))
                            .text_color(rgb(0xff8844))
                            .child(format!("{:.4}", trade.fee)),
                    )
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
                        rgb(0xffffff),
                    ))
                    .child(stat_card(
                        "Available",
                        &format!("${:.2}", self.pnl.available_balance),
                        rgb(0x00ff88),
                    ))
                    .child(stat_card(
                        "Margin Used",
                        &format!("${:.2}", self.pnl.margin_used),
                        rgb(0xffaa00),
                    ))
                    .child(stat_card(
                        "Total PnL",
                        &format!("${:+.2}", self.pnl.total_pnl),
                        if self.pnl.total_pnl >= 0.0 {
                            rgb(0x00ff88)
                        } else {
                            rgb(0xff4444)
                        },
                    ))
                    .child(stat_card(
                        "Daily PnL",
                        &format!("${:+.2}", self.pnl.daily_pnl),
                        if self.pnl.daily_pnl >= 0.0 {
                            rgb(0x00ff88)
                        } else {
                            rgb(0xff4444)
                        },
                    )),
            )
            // Balance table
            .child(table_header(&["Asset", "Total", "Available", "In Margin"]))
            .children(self.balances.iter().enumerate().map(|(i, bal)| {
                div()
                    .w_full()
                    .px(px(10.))
                    .py(px(4.))
                    .flex()
                    .items_center()
                    .bg(if i % 2 == 0 {
                        rgb(0x16213e)
                    } else {
                        rgb(0x1a2744)
                    })
                    .child(
                        div()
                            .w(px(80.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(bal.asset.clone()),
                    )
                    .child(
                        div()
                            .w(px(100.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcccccc))
                            .child(format!("{:.2}", bal.total)),
                    )
                    .child(
                        div()
                            .w(px(100.))
                            .text_size(px(12.))
                            .text_color(rgb(0x00ff88))
                            .child(format!("{:.2}", bal.available)),
                    )
                    .child(
                        div()
                            .w(px(100.))
                            .text_size(px(12.))
                            .text_color(rgb(0xffaa00))
                            .child(format!("{:.2}", bal.in_margin)),
                    )
            }))
    }
}

fn table_header(headers: &[&str]) -> impl IntoElement {
    let widths = [120., 80., 70., 80., 80., 80., 60., 60.];
    let items: Vec<_> = headers
        .iter()
        .enumerate()
        .map(|(i, header)| {
            let w = widths.get(i).copied().unwrap_or(80.);
            div()
                .w(px(w))
                .text_size(px(11.))
                .text_color(rgb(0x888888))
                .child(header.to_string())
        })
        .collect();

    div()
        .w_full()
        .px(px(10.))
        .py(px(6.))
        .flex()
        .items_center()
        .border_b_1()
        .border_color(rgb(0x0f3460))
        .children(items)
}

fn stat_card(label: &str, value: &str, value_color: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(10.))
        .rounded(px(6.))
        .bg(rgb(0x0f3460))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(0x888888))
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(14.))
                .text_color(value_color)
                .child(value.to_string()),
        )
}

fn empty_state(message: &str) -> impl IntoElement {
    div()
        .w_full()
        .py(px(20.))
        .flex()
        .justify_center()
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(0x666666))
                .child(message.to_string()),
        )
}

fn format_timestamp(ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let diff_secs = (now.saturating_sub(ts)) / 1000;
    if diff_secs < 60 {
        format!("{}s ago", diff_secs)
    } else if diff_secs < 3600 {
        format!("{}m ago", diff_secs / 60)
    } else if diff_secs < 86400 {
        format!("{}h ago", diff_secs / 3600)
    } else {
        format!("{}d ago", diff_secs / 86400)
    }
}
