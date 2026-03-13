use gpui::prelude::*;
use gpui::{div, px, SharedString};

use crate::components::theme::*;
use crate::models::OrderBook as OrderBookData;

pub struct OrderBookView {
    pub data: OrderBookData,
}

impl OrderBookView {
    pub fn new() -> Self {
        Self {
            data: OrderBookData::default(),
        }
    }

    fn max_cumulative(&self) -> f64 {
        let max_bid = self.data.bids.last().map(|l| l.cumulative).unwrap_or(1.0);
        let max_ask = self.data.asks.last().map(|l| l.cumulative).unwrap_or(1.0);
        max_bid.max(max_ask).max(1.0)
    }
}

impl Render for OrderBookView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let max_cum = self.max_cumulative();
        let last_price = self.data.last_price;

        div()
            .w(px(280.))
            .h_full()
            .flex()
            .flex_col()
            .bg(bg_panel())
            .border_l_1()
            .border_color(border_primary())
            // Title
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(border_primary())
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(text_muted())
                            .child("Order Book"),
                    ),
            )
            // Column Header
            .child(
                div()
                    .px(px(12.))
                    .py(px(5.))
                    .border_b_1()
                    .border_color(border_primary())
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(text_dimmest())
                            .child("PRICE"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(text_dimmest())
                            .child("SIZE"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(text_dimmest())
                            .child("TOTAL"),
                    ),
            )
            // Asks (reversed - highest price at top)
            .child(
                div()
                    .flex_1()
                    .id("asks-scroll")
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(
                        self.data.asks.iter().enumerate().map(|(i, level)| {
                            let bar_pct = (level.cumulative / max_cum * 100.0).min(100.0);
                            render_level(i, level.price, level.size, level.cumulative, color_sell(), color_sell_bg(), bar_pct, "ask")
                        }),
                    ),
            )
            // Spread / current price
            .child(
                div()
                    .py(px(8.))
                    .px(px(12.))
                    .flex()
                    .justify_center()
                    .items_center()
                    .gap(px(8.))
                    .bg(bg_header())
                    .border_t_1()
                    .border_b_1()
                    .border_color(border_accent())
                    .child(
                        div()
                            .text_size(px(16.))
                            .text_color(text_primary())
                            .child(format!("{:.2}", last_price)),
                    ),
            )
            // Bids
            .child(
                div()
                    .flex_1()
                    .id("bids-scroll")
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(
                        self.data.bids.iter().enumerate().map(|(i, level)| {
                            let bar_pct = (level.cumulative / max_cum * 100.0).min(100.0);
                            render_level(i, level.price, level.size, level.cumulative, color_buy(), color_buy_bg(), bar_pct, "bid")
                        }),
                    ),
            )
    }
}

fn render_level(
    index: usize,
    price: f64,
    size: f64,
    cumulative: f64,
    text_color: gpui::Rgba,
    _bg_tint: gpui::Rgba,
    _bar_pct: f64,
    prefix: &str,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("{}-{}", prefix, index)))
        .w_full()
        .px(px(12.))
        .py(px(2.))
        .flex()
        .justify_between()
        .hover(|s| s.bg(bg_hover()))
        .child(
            div()
                .text_size(px(12.))
                .text_color(text_color)
                .child(format!("{:.2}", price)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(text_secondary())
                .child(format!("{:.3}", size)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(text_dim())
                .child(format!("{:.3}", cumulative)),
        )
}
