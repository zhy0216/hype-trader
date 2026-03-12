use gpui::prelude::*;
use gpui::{div, px, rgb, SharedString};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::models::{Candle, CandleInterval};

pub struct CandleChart {
    pub candles: Vec<Candle>,
    pub interval: CandleInterval,
    pub visible_count: usize,
    pub scroll_offset: usize,
    pub show_ma7: bool,
    pub show_ma25: bool,
    pub show_ma99: bool,
}

impl CandleChart {
    pub fn new() -> Self {
        Self {
            candles: Vec::new(),
            interval: CandleInterval::H1,
            visible_count: 60,
            scroll_offset: 0,
            show_ma7: true,
            show_ma25: true,
            show_ma99: true,
        }
    }

    /// Returns the start..end index range into self.candles for visible candles.
    fn visible_range(&self) -> (usize, usize) {
        if self.candles.is_empty() {
            return (0, 0);
        }
        let end = self.candles.len().saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(self.visible_count);
        (start, end)
    }

    fn visible_candles(&self) -> &[Candle] {
        let (start, end) = self.visible_range();
        if start == end {
            return &[];
        }
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

    /// Calculate the simple moving average for all candles.
    /// Returns a Vec with one entry per candle: None if not enough history,
    /// Some(average) otherwise.
    fn calculate_ma(&self, period: usize) -> Vec<Option<f64>> {
        let len = self.candles.len();
        let mut result = Vec::with_capacity(len);
        let mut sum = 0.0;

        for i in 0..len {
            sum += self.candles[i].close;
            if i >= period {
                sum -= self.candles[i - period].close;
            }
            if i + 1 >= period {
                result.push(Some(sum / period as f64));
            } else {
                result.push(None);
            }
        }

        result
    }
}

/// Render a single candlestick as a vertical column of divs.
///
/// Each candle is represented as a flex column with sections stacked top-to-bottom:
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

/// MA indicator definition: period, color, show flag, label.
struct MaIndicator {
    period: usize,
    color: u32,
    show: bool,
    label: &'static str,
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
        let (vis_start, vis_end) = self.visible_range();
        let candle_width: f32 = 8.0;
        let candle_gap: f32 = 2.0;
        let chart_px_padding: f32 = 8.0;
        let dot_size: f32 = 3.0;

        // Calculate MA values for the full candle array
        let ma_indicators = vec![
            MaIndicator {
                period: 7,
                color: 0xffff00,
                show: self.show_ma7,
                label: "MA7",
            },
            MaIndicator {
                period: 25,
                color: 0x00aaff,
                show: self.show_ma25,
                label: "MA25",
            },
            MaIndicator {
                period: 99,
                color: 0xff00ff,
                show: self.show_ma99,
                label: "MA99",
            },
        ];

        // Pre-calculate all MA vectors
        let ma_values: Vec<(Vec<Option<f64>>, u32, bool)> = ma_indicators
            .iter()
            .map(|ind| (self.calculate_ma(ind.period), ind.color, ind.show))
            .collect();

        // Slice MA values for visible range and get last values for OHLCV bar
        let ma_visible: Vec<(Vec<Option<f64>>, u32)> = ma_values
            .iter()
            .filter(|(_, _, show)| *show)
            .map(|(vals, color, _)| (vals.get(vis_start..vis_end).unwrap_or(&[]).to_vec(), *color))
            .collect();

        // Last visible MA values for OHLCV bar display
        let ma_last_values: Vec<(&str, Option<f64>, u32)> = ma_indicators
            .iter()
            .zip(ma_values.iter())
            .filter(|(ind, _)| ind.show)
            .map(|(ind, (vals, _, _))| {
                let last_val = if vis_end > 0 && vis_end <= vals.len() {
                    vals[vis_end - 1]
                } else {
                    None
                };
                (ind.label, last_val, ind.color)
            })
            .collect();

        // Build the MA overlay: absolute-positioned dots on top of the chart area
        let mut ma_overlay = div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(chart_height));

        for (vals, color) in &ma_visible {
            for (i, ma_val) in vals.iter().enumerate() {
                if let Some(price) = ma_val {
                    // Y position: distance from top of chart
                    let y_from_bottom =
                        ((price - price_min) / price_range * chart_height as f64) as f32;
                    let y_from_top = chart_height - y_from_bottom - (dot_size / 2.0);
                    // X position: same as candle column position
                    let x = chart_px_padding
                        + (i as f32) * (candle_width + candle_gap)
                        + (candle_width / 2.0)
                        - (dot_size / 2.0);

                    ma_overlay = ma_overlay.child(
                        div()
                            .absolute()
                            .top(px(y_from_top))
                            .left(px(x))
                            .w(px(dot_size))
                            .h(px(dot_size))
                            .rounded(px(dot_size / 2.0))
                            .bg(rgb(*color)),
                    );
                }
            }
        }

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1a1a2e))
            // Toolbar: interval selector buttons + MA toggles
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
                    )
                    // Separator
                    .child(
                        div()
                            .w(px(1.))
                            .h(px(16.))
                            .mx(px(4.))
                            .bg(rgb(0x0f3460)),
                    )
                    // MA toggle buttons
                    .child({
                        let active = self.show_ma7;
                        Button::new("toggle-ma7")
                            .label("MA7")
                            .compact()
                            .map(|b| if active { b.primary() } else { b.ghost() })
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_ma7 = !this.show_ma7;
                            }))
                    })
                    .child({
                        let active = self.show_ma25;
                        Button::new("toggle-ma25")
                            .label("MA25")
                            .compact()
                            .map(|b| if active { b.primary() } else { b.ghost() })
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_ma25 = !this.show_ma25;
                            }))
                    })
                    .child({
                        let active = self.show_ma99;
                        Button::new("toggle-ma99")
                            .label("MA99")
                            .compact()
                            .map(|b| if active { b.primary() } else { b.ghost() })
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_ma99 = !this.show_ma99;
                            }))
                    }),
            )
            // OHLCV info bar showing last candle data + MA values
            .child(self.render_ohlcv_bar(&visible, &ma_last_values))
            // Main chart area: candlesticks + MA overlay
            .child(
                div()
                    .h(px(chart_height))
                    .w_full()
                    .relative()
                    .child(
                        div()
                            .h(px(chart_height))
                            .w_full()
                            .flex()
                            .items_end()
                            .px(px(chart_px_padding))
                            .gap(px(candle_gap))
                            .children(visible.iter().enumerate().map(|(i, candle)| {
                                render_candle(
                                    i,
                                    candle,
                                    chart_height,
                                    price_min,
                                    price_range,
                                    candle_width,
                                )
                            })),
                    )
                    // MA dots overlay
                    .child(ma_overlay),
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
    fn render_ohlcv_bar(
        &self,
        visible: &[Candle],
        ma_values: &[(&str, Option<f64>, u32)],
    ) -> impl IntoElement {
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

            // MA values
            for (label, val, color) in ma_values {
                if let Some(v) = val {
                    container = container.child(
                        div()
                            .flex()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(*color))
                                    .child(label.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(*color))
                                    .child(format!("{:.2}", v)),
                            ),
                    );
                }
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
