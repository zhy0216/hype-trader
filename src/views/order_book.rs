use gpui::prelude::*;
use gpui::{div, px, rgb, SharedString};

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
            .bg(rgb(0x16213e))
            // Header
            .child(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .border_b_1()
                    .border_color(rgb(0x0f3460))
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(0xaaaaaa))
                            .child("Price"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(0xaaaaaa))
                            .child("Size"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(0xaaaaaa))
                            .child("Total"),
                    ),
            )
            // Asks (reversed - highest price at top, push to bottom of section)
            .child(
                div()
                    .flex_1()
                    .id("asks-scroll")
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(
                        self.data.asks.iter().enumerate().map(|(i, level)| {
                            let _bar_pct = (level.cumulative / max_cum * 100.0).min(100.0);
                            render_level(i, level.price, level.size, level.cumulative, rgb(0xff4444), "ask")
                        }),
                    ),
            )
            // Spread / current price
            .child(
                div()
                    .py(px(8.))
                    .px(px(10.))
                    .flex()
                    .justify_center()
                    .border_t_1()
                    .border_b_1()
                    .border_color(rgb(0x0f3460))
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(rgb(0xffffff))
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
                            let _bar_pct = (level.cumulative / max_cum * 100.0).min(100.0);
                            render_level(i, level.price, level.size, level.cumulative, rgb(0x00ff88), "bid")
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
    prefix: &str,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("{}-{}", prefix, index)))
        .w_full()
        .px(px(10.))
        .py(px(2.))
        .flex()
        .justify_between()
        .child(
            div()
                .text_size(px(12.))
                .text_color(text_color)
                .child(format!("{:.2}", price)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0xcccccc))
                .child(format!("{:.3}", size)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0x999999))
                .child(format!("{:.3}", cumulative)),
        )
}
