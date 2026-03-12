use gpui::prelude::*;
use gpui::{div, px, rgb, SharedString};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::models::{Candle, CandleInterval};

pub struct CandleChart {
    pub candles: Vec<Candle>,
    pub interval: CandleInterval,
    pub visible_count: usize,
    pub scroll_offset: usize,
}

impl CandleChart {
    pub fn new() -> Self {
        Self {
            candles: Vec::new(),
            interval: CandleInterval::H1,
            visible_count: 60,
            scroll_offset: 0,
        }
    }

    fn visible_candles(&self) -> &[Candle] {
        if self.candles.is_empty() {
            return &[];
        }
        let end = self.candles.len().saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(self.visible_count);
        &self.candles[start..end]
    }

    fn price_range(&self) -> (f64, f64) {
        let visible = self.visible_candles();
        if visible.is_empty() {
            return (0.0, 100.0);
        }
        let min = visible.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        let max = visible.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let padding = (max - min) * 0.05;
        (min - padding, max + padding)
    }

    fn volume_max(&self) -> f64 {
        let visible = self.visible_candles();
        visible
            .iter()
            .map(|c| c.volume)
            .fold(0.0_f64, f64::max)
            .max(1.0)
    }
}

/// Render a single candlestick as a vertical column of divs.
///
/// Since gpui 0.2 does not support absolute positioning, each candle is
/// represented as a flex column with three sections stacked top-to-bottom:
///   1. Top spacer  (transparent, pushes content down from the top)
///   2. Upper wick  (thin colored bar from high to body top)
///   3. Body        (wide colored bar from open to close)
///   4. Lower wick  (thin colored bar from body bottom to low)
///   5. Bottom spacer (transparent, fills remaining space to chart bottom)
///
/// Heights are computed proportionally to the chart area.
fn render_candle(
    index: usize,
    candle: &Candle,
    chart_height: f32,
    price_min: f64,
    price_range: f64,
    candle_width: f32,
) -> impl IntoElement {
    let is_bullish = candle.close >= candle.open;
    let color = if is_bullish {
        rgb(0x00ff88)
    } else {
        rgb(0xff4444)
    };

    let body_top = if is_bullish { candle.close } else { candle.open };
    let body_bottom = if is_bullish { candle.open } else { candle.close };

    // Convert prices to pixel heights from bottom of chart area
    let high_px = ((candle.high - price_min) / price_range * chart_height as f64) as f32;
    let low_px = ((candle.low - price_min) / price_range * chart_height as f64) as f32;
    let body_top_px = ((body_top - price_min) / price_range * chart_height as f64) as f32;
    let body_bottom_px = ((body_bottom - price_min) / price_range * chart_height as f64) as f32;

    // Section heights (from top of chart to bottom):
    // top_spacer = chart_height - high_px (space above the high)
    // upper_wick = high_px - body_top_px
    // body = body_top_px - body_bottom_px
    // lower_wick = body_bottom_px - low_px
    // bottom_spacer = low_px (space below the low)
    let top_spacer = (chart_height - high_px).max(0.0);
    let upper_wick = (high_px - body_top_px).max(0.0);
    let body_h = (body_top_px - body_bottom_px).max(1.0);
    let lower_wick = (body_bottom_px - low_px).max(0.0);
    let bottom_spacer = low_px.max(0.0);

    let wick_width: f32 = 1.0;
    div()
        .id(SharedString::from(format!("candle-{}", index)))
        .w(px(candle_width))
        .h(px(chart_height))
        .flex()
        .flex_col()
        .items_center()
        // Top spacer
        .child(div().w(px(candle_width)).h(px(top_spacer)))
        // Upper wick
        .child(div().w(px(wick_width)).h(px(upper_wick)).bg(color))
        // Body
        .child(div().w(px(candle_width)).h(px(body_h)).bg(color))
        // Lower wick
        .child(div().w(px(wick_width)).h(px(lower_wick)).bg(color))
        // Bottom spacer
        .child(div().w(px(candle_width)).h(px(bottom_spacer)))
}

impl Render for CandleChart {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let chart_height: f32 = 400.0;
        let volume_height: f32 = 80.0;
        let (price_min, price_max) = self.price_range();
        let price_range = (price_max - price_min).max(0.01);
        let vol_max = self.volume_max();
        let visible = self.visible_candles().to_vec();
        let candle_width: f32 = 8.0;
        let candle_gap: f32 = 2.0;

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1a1a2e))
            // Toolbar: interval selector buttons
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .px(px(8.))
                    .py(px(6.))
                    .border_b_1()
                    .border_color(rgb(0x0f3460))
                    .children(
                        [
                            CandleInterval::M1,
                            CandleInterval::M5,
                            CandleInterval::M15,
                            CandleInterval::H1,
                            CandleInterval::H4,
                            CandleInterval::D1,
                        ]
                        .into_iter()
                        .map(|interval| {
                            let label = interval.label();
                            let is_active = interval == self.interval;
                            Button::new(SharedString::from(format!("interval-{}", label)))
                                .label(label)
                                .compact()
                                .map(|b| {
                                    if is_active {
                                        b.primary()
                                    } else {
                                        b.ghost()
                                    }
                                })
                                .on_click(cx.listener(move |this, _, _, _| {
                                    this.interval = interval;
                                }))
                        }),
                    ),
            )
            // OHLCV info bar showing last candle data
            .child(self.render_ohlcv_bar(&visible))
            // Main chart area: candlesticks
            .child(
                div()
                    .h(px(chart_height))
                    .w_full()
                    .flex()
                    .items_end()
                    .px(px(8.))
                    .gap(px(candle_gap))
                    .children(visible.iter().enumerate().map(|(i, candle)| {
                        render_candle(i, candle, chart_height, price_min, price_range, candle_width)
                    })),
            )
            // Price axis labels
            .child(self.render_price_axis(price_min, price_max))
            // Volume bars
            .child(
                div()
                    .h(px(volume_height))
                    .w_full()
                    .flex()
                    .items_end()
                    .gap(px(candle_gap))
                    .px(px(8.))
                    .border_t_1()
                    .border_color(rgb(0x0f3460))
                    .children(visible.iter().enumerate().map(|(i, candle)| {
                        let vol_pct = candle.volume / vol_max;
                        let bar_h = (vol_pct * volume_height as f64).max(1.0) as f32;
                        let color = if candle.close >= candle.open {
                            rgb(0x1a6644)
                        } else {
                            rgb(0x664422)
                        };
                        div()
                            .id(SharedString::from(format!("vol-{}", i)))
                            .w(px(candle_width))
                            .h(px(bar_h))
                            .bg(color)
                    })),
            )
    }
}

impl CandleChart {
    fn render_ohlcv_bar(&self, visible: &[Candle]) -> impl IntoElement {
        let mut container = div()
            .flex()
            .items_center()
            .gap(px(16.))
            .px(px(8.))
            .py(px(4.));

        if let Some(c) = visible.last() {
            let price_color = if c.close >= c.open {
                rgb(0x00ff88)
            } else {
                rgb(0xff4444)
            };

            let entries: Vec<(&str, String, gpui::Rgba)> = vec![
                ("O", format!("{:.2}", c.open), rgb(0xaaaaaa)),
                ("H", format!("{:.2}", c.high), rgb(0xaaaaaa)),
                ("L", format!("{:.2}", c.low), rgb(0xaaaaaa)),
                ("C", format!("{:.2}", c.close), price_color),
                ("Vol", format!("{:.0}", c.volume), rgb(0xaaaaaa)),
            ];

            for (label, val, color) in entries {
                container = container.child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(0x666666))
                                .child(label.to_string()),
                        )
                        .child(div().text_size(px(11.)).text_color(color).child(val)),
                );
            }
        }

        container
    }

    fn render_price_axis(&self, price_min: f64, price_max: f64) -> impl IntoElement {
        div()
            .flex()
            .justify_between()
            .px(px(8.))
            .py(px(2.))
            .children((0..5).map(|i| {
                let price = price_min + (i as f64 / 4.0) * (price_max - price_min);
                div()
                    .text_size(px(10.))
                    .text_color(rgb(0x666666))
                    .child(format!("{:.2}", price))
            }))
    }
}
