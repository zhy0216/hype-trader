use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{canvas, div, px, rgb, Bounds, MouseButton, Point, Pixels, ScrollDelta, SharedString};

use crate::components::theme::{bg_primary, border_primary, text_dimmest, text_dim, text_disabled, color_green, color_red};
use crate::components::toggle_button::toggle_button;
use crate::models::{Candle, CandleInterval};

pub struct CandleChart {
    pub candles: Vec<Candle>,
    pub interval: CandleInterval,
    pub visible_count: usize,
    pub scroll_offset: usize,
    pub show_ma7: bool,
    pub show_ma25: bool,
    pub show_ma99: bool,
    pub show_bb: bool,
    pub show_macd: bool,
    pub show_rsi: bool,
    // Interaction state
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_offset: usize,
    pub hover_position: Option<Point<Pixels>>,
    /// True while fetching data for a new symbol
    pub loading: bool,
    /// Tracks the chart area origin in window coordinates (updated each frame via canvas)
    pub chart_area_origin: Rc<Cell<(f32, f32)>>,
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
            show_bb: false,
            show_macd: false,
            show_rsi: false,
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_offset: 0,
            hover_position: None,
            loading: false,
            chart_area_origin: Rc::new(Cell::new((0.0, 0.0))),
        }
    }

    /// Compute dynamic candle width based on visible_count.
    /// When visible_count == 60 (default), width == 8.0.
    fn candle_width(&self) -> f32 {
        (8.0f32 * 60.0 / self.visible_count as f32).clamp(2.0, 20.0)
    }

    /// Clamp scroll_offset to valid range.
    fn clamp_scroll_offset(&mut self) {
        let max_offset = self.candles.len().saturating_sub(self.visible_count);
        self.scroll_offset = self.scroll_offset.min(max_offset);
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

    /// Calculate Bollinger Bands (middle, upper, lower) for all candles.
    /// Middle = SMA(period), Upper = SMA + mult * stddev, Lower = SMA - mult * stddev
    fn calculate_bollinger(
        &self,
        period: usize,
        std_dev_mult: f64,
    ) -> Vec<(Option<f64>, Option<f64>, Option<f64>)> {
        let len = self.candles.len();
        let ma = self.calculate_ma(period);
        let mut result = Vec::with_capacity(len);

        for i in 0..len {
            if let Some(sma) = ma[i] {
                // Calculate standard deviation over the last `period` candles
                let start = if i + 1 >= period { i + 1 - period } else { 0 };
                let slice = &self.candles[start..=i];
                let variance: f64 = slice
                    .iter()
                    .map(|c| {
                        let diff = c.close - sma;
                        diff * diff
                    })
                    .sum::<f64>()
                    / period as f64;
                let std_dev = variance.sqrt();
                let upper = sma + std_dev_mult * std_dev;
                let lower = sma - std_dev_mult * std_dev;
                result.push((Some(sma), Some(upper), Some(lower)));
            } else {
                result.push((None, None, None));
            }
        }

        result
    }

    /// Calculate Exponential Moving Average for all candles.
    /// Uses SMA of first `period` values as initial EMA, then applies
    /// multiplier = 2 / (period + 1).
    fn calculate_ema(&self, period: usize) -> Vec<Option<f64>> {
        let len = self.candles.len();
        if len == 0 || period == 0 {
            return vec![None; len];
        }
        let mut result = Vec::with_capacity(len);
        let multiplier = 2.0 / (period as f64 + 1.0);

        // Fill None until we have enough data for SMA seed
        for _ in 0..len.min(period.saturating_sub(1)) {
            result.push(None);
        }

        if len < period {
            return result;
        }

        // SMA of first `period` values as seed
        let sma: f64 = self.candles[..period].iter().map(|c| c.close).sum::<f64>() / period as f64;
        result.push(Some(sma));

        let mut prev_ema = sma;
        for i in period..len {
            let ema = self.candles[i].close * multiplier + prev_ema * (1.0 - multiplier);
            result.push(Some(ema));
            prev_ema = ema;
        }

        result
    }

    /// Calculate MACD: (macd_line, signal_line, histogram) for all candles.
    /// MACD Line = EMA(12) - EMA(26)
    /// Signal Line = EMA(9) of MACD Line
    /// Histogram = MACD Line - Signal Line
    fn calculate_macd(&self) -> Vec<(Option<f64>, Option<f64>, Option<f64>)> {
        let len = self.candles.len();
        let ema12 = self.calculate_ema(12);
        let ema26 = self.calculate_ema(26);

        // MACD line = EMA12 - EMA26
        let mut macd_line: Vec<Option<f64>> = Vec::with_capacity(len);
        for i in 0..len {
            match (ema12[i], ema26[i]) {
                (Some(e12), Some(e26)) => macd_line.push(Some(e12 - e26)),
                _ => macd_line.push(None),
            }
        }

        // Signal line = EMA(9) of MACD line values
        // We need to compute EMA manually on the macd_line values
        let signal_period: usize = 9;
        let multiplier = 2.0 / (signal_period as f64 + 1.0);
        let mut signal_line: Vec<Option<f64>> = vec![None; len];

        // Find the first run of `signal_period` consecutive Some values in macd_line
        let mut first_valid_run_start = None;
        let mut consecutive = 0usize;
        for i in 0..len {
            if macd_line[i].is_some() {
                consecutive += 1;
                if consecutive >= signal_period && first_valid_run_start.is_none() {
                    first_valid_run_start = Some(i + 1 - signal_period);
                }
            } else {
                consecutive = 0;
            }
        }

        if let Some(start) = first_valid_run_start {
            // SMA seed for signal
            let sma: f64 = (start..start + signal_period)
                .map(|i| macd_line[i].unwrap())
                .sum::<f64>()
                / signal_period as f64;
            let seed_idx = start + signal_period - 1;
            signal_line[seed_idx] = Some(sma);
            let mut prev = sma;
            for i in (seed_idx + 1)..len {
                if let Some(m) = macd_line[i] {
                    let s = m * multiplier + prev * (1.0 - multiplier);
                    signal_line[i] = Some(s);
                    prev = s;
                }
            }
        }

        // Histogram = MACD - Signal
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let hist = match (macd_line[i], signal_line[i]) {
                (Some(m), Some(s)) => Some(m - s),
                _ => None,
            };
            result.push((macd_line[i], signal_line[i], hist));
        }

        result
    }

    /// Calculate RSI using Wilder's smoothing method.
    /// Returns values 0-100 for each candle (None if not enough history).
    fn calculate_rsi(&self, period: usize) -> Vec<Option<f64>> {
        let len = self.candles.len();
        if len < 2 || period == 0 {
            return vec![None; len];
        }

        let mut result = vec![None; len];

        // Calculate price changes
        let mut gains = Vec::with_capacity(len);
        let mut losses = Vec::with_capacity(len);
        gains.push(0.0);
        losses.push(0.0);
        for i in 1..len {
            let change = self.candles[i].close - self.candles[i - 1].close;
            gains.push(change.max(0.0));
            losses.push((-change).max(0.0));
        }

        if len <= period {
            return result;
        }

        // Initial SMA of gains/losses over first `period` changes (indices 1..=period)
        let mut avg_gain: f64 = gains[1..=period].iter().sum::<f64>() / period as f64;
        let mut avg_loss: f64 = losses[1..=period].iter().sum::<f64>() / period as f64;

        // RSI at index `period`
        if avg_loss == 0.0 {
            result[period] = Some(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            result[period] = Some(100.0 - (100.0 / (1.0 + rs)));
        }

        // Wilder's smoothing for subsequent values
        for i in (period + 1)..len {
            avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;

            if avg_loss == 0.0 {
                result[i] = Some(100.0);
            } else {
                let rs = avg_gain / avg_loss;
                result[i] = Some(100.0 - (100.0 / (1.0 + rs)));
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
        color_green()
    } else {
        color_red()
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
        let macd_height: f32 = 80.0;
        let rsi_height: f32 = 60.0;
        let (price_min, price_max) = self.price_range();
        let price_range = (price_max - price_min).max(0.01);
        let vol_max = self.volume_max();
        let visible = self.visible_candles().to_vec();
        let (vis_start, vis_end) = self.visible_range();
        let candle_width: f32 = self.candle_width();
        let candle_gap: f32 = 2.0;
        let chart_px_padding: f32 = 8.0;
        let dot_size: f32 = 3.0;
        let current_interval = self.interval;

        // Chart area origin in window coordinates (from previous frame's canvas callback)
        let (chart_area_left_abs, chart_area_top_abs) = self.chart_area_origin.get();

        // Crosshair data: compute candle index and price from hover position
        let hover_info = self.hover_position.map(|pos| {
            let local_x = f32::from(pos.x) - chart_area_left_abs;
            let local_y = f32::from(pos.y) - chart_area_top_abs;
            let candle_step = candle_width + candle_gap;
            let candle_idx = if candle_step > 0.0 {
                (local_x / candle_step) as usize
            } else {
                0
            };
            let price_at_y = if local_y >= 0.0 && local_y <= chart_height {
                let ratio = 1.0 - (local_y as f64 / chart_height as f64);
                Some(price_min + ratio * price_range)
            } else {
                None
            };
            (pos, local_x, local_y, candle_idx, price_at_y)
        });

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

        // Bollinger Bands overlay
        let show_bb = self.show_bb;
        let bb_visible: Vec<(Option<f64>, Option<f64>, Option<f64>)> = if show_bb {
            let bb_all = self.calculate_bollinger(20, 2.0);
            bb_all
                .get(vis_start..vis_end)
                .unwrap_or(&[])
                .to_vec()
        } else {
            Vec::new()
        };

        let mut bb_overlay = div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(chart_height));

        if show_bb {
            for (i, (middle, upper, lower)) in bb_visible.iter().enumerate() {
                let x = chart_px_padding
                    + (i as f32) * (candle_width + candle_gap)
                    + (candle_width / 2.0)
                    - (dot_size / 2.0);

                // Middle band: gray
                if let Some(price) = middle {
                    let y_from_bottom =
                        ((price - price_min) / price_range * chart_height as f64) as f32;
                    let y_from_top = chart_height - y_from_bottom - (dot_size / 2.0);
                    bb_overlay = bb_overlay.child(
                        div()
                            .absolute()
                            .top(px(y_from_top))
                            .left(px(x))
                            .w(px(dot_size))
                            .h(px(dot_size))
                            .rounded(px(dot_size / 2.0))
                            .bg(text_dimmest()),
                    );
                }

                // Upper band: muted blue (semi-transparent appearance)
                if let Some(price) = upper {
                    let y_from_bottom =
                        ((price - price_min) / price_range * chart_height as f64) as f32;
                    let y_from_top = chart_height - y_from_bottom - (dot_size / 2.0);
                    bb_overlay = bb_overlay.child(
                        div()
                            .absolute()
                            .top(px(y_from_top))
                            .left(px(x))
                            .w(px(dot_size))
                            .h(px(dot_size))
                            .rounded(px(dot_size / 2.0))
                            .bg(rgb(0x2a5599)),
                    );
                }

                // Lower band: muted blue (semi-transparent appearance)
                if let Some(price) = lower {
                    let y_from_bottom =
                        ((price - price_min) / price_range * chart_height as f64) as f32;
                    let y_from_top = chart_height - y_from_bottom - (dot_size / 2.0);
                    bb_overlay = bb_overlay.child(
                        div()
                            .absolute()
                            .top(px(y_from_top))
                            .left(px(x))
                            .w(px(dot_size))
                            .h(px(dot_size))
                            .rounded(px(dot_size / 2.0))
                            .bg(rgb(0x2a5599)),
                    );
                }
            }
        }

        // MACD data
        let show_macd = self.show_macd;
        let macd_visible: Vec<(Option<f64>, Option<f64>, Option<f64>)> = if show_macd {
            let macd_all = self.calculate_macd();
            macd_all
                .get(vis_start..vis_end)
                .unwrap_or(&[])
                .to_vec()
        } else {
            Vec::new()
        };

        // RSI data
        let show_rsi = self.show_rsi;
        let rsi_visible: Vec<Option<f64>> = if show_rsi {
            let rsi_all = self.calculate_rsi(14);
            rsi_all
                .get(vis_start..vis_end)
                .unwrap_or(&[])
                .to_vec()
        } else {
            Vec::new()
        };

        div()
            .id("candle-chart-container")
            .w_full()
            .h_full()
            .relative()
            .flex()
            .flex_col()
            .bg(bg_primary())
            // Zoom: scroll wheel changes visible_count
            .on_scroll_wheel(cx.listener(move |this, event: &gpui::ScrollWheelEvent, _window, _cx| {
                let delta_y = match event.delta {
                    ScrollDelta::Pixels(pt) => f32::from(pt.y),
                    ScrollDelta::Lines(pt) => pt.y * 20.0,
                };
                // Scroll up (negative delta_y on most platforms) = zoom in = fewer candles
                // Scroll down (positive delta_y) = zoom out = more candles
                let old_count = this.visible_count;
                let zoom_step = (old_count as f32 * 0.1).max(2.0) as i32;
                let new_count = if delta_y > 0.0 {
                    // zoom out
                    (old_count as i32 + zoom_step).min(200) as usize
                } else {
                    // zoom in
                    (old_count as i32 - zoom_step).max(20) as usize
                };
                // Adjust scroll_offset to keep the center candle stable
                if old_count > 0 && new_count != old_count {
                    let right_edge = this.candles.len().saturating_sub(this.scroll_offset);
                    let center_candle = right_edge.saturating_sub(old_count / 2);
                    let new_right_edge = center_candle + new_count / 2;
                    this.scroll_offset = this.candles.len().saturating_sub(new_right_edge);
                }
                this.visible_count = new_count;
                this.clamp_scroll_offset();
            }))
            // Drag start
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, _window, _cx| {
                this.is_dragging = true;
                this.drag_start_x = f32::from(event.position.x);
                this.drag_start_offset = this.scroll_offset;
            }))
            // Drag end
            .on_mouse_up(MouseButton::Left, cx.listener(move |this, _event: &gpui::MouseUpEvent, _window, _cx| {
                this.is_dragging = false;
            }))
            // Mouse move: drag pan + crosshair tracking
            .on_mouse_move(cx.listener(move |this, event: &gpui::MouseMoveEvent, _window, cx| {
                // Update crosshair position
                this.hover_position = Some(event.position);

                // Handle drag panning
                if this.is_dragging {
                    let delta_x = f32::from(event.position.x) - this.drag_start_x;
                    let cw = this.candle_width();
                    let candle_step = cw + 2.0; // candle_gap = 2.0
                    let candle_delta = (delta_x / candle_step) as isize;
                    // Dragging right = revealing newer data = decrease offset
                    let new_offset = this.drag_start_offset as isize - candle_delta;
                    this.scroll_offset = new_offset.max(0) as usize;
                    this.clamp_scroll_offset();
                }
                cx.notify();
            }))
            // Hover leave: clear crosshair
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                if !hovered {
                    this.hover_position = None;
                    this.is_dragging = false;
                    cx.notify();
                }
            }))
            // Toolbar: interval selector buttons + MA toggles + indicator toggles
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .px(px(8.))
                    .py(px(6.))
                    .border_b_1()
                    .border_color(border_primary())
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
                            toggle_button(SharedString::from(format!("interval-{}", label)), label, is_active)
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
                            .bg(border_primary()),
                    )
                    // MA toggle buttons
                    .child({
                        let active = self.show_ma7;
                        toggle_button("toggle-ma7", "MA7", active)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_ma7 = !this.show_ma7;
                            }))
                    })
                    .child({
                        let active = self.show_ma25;
                        toggle_button("toggle-ma25", "MA25", active)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_ma25 = !this.show_ma25;
                            }))
                    })
                    .child({
                        let active = self.show_ma99;
                        toggle_button("toggle-ma99", "MA99", active)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_ma99 = !this.show_ma99;
                            }))
                    })
                    // Separator before indicator toggles
                    .child(
                        div()
                            .w(px(1.))
                            .h(px(16.))
                            .mx(px(4.))
                            .bg(border_primary()),
                    )
                    // BB toggle
                    .child({
                        let active = self.show_bb;
                        toggle_button("toggle-bb", "BB", active)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_bb = !this.show_bb;
                            }))
                    })
                    // MACD toggle
                    .child({
                        let active = self.show_macd;
                        toggle_button("toggle-macd", "MACD", active)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_macd = !this.show_macd;
                            }))
                    })
                    // RSI toggle
                    .child({
                        let active = self.show_rsi;
                        toggle_button("toggle-rsi", "RSI", active)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.show_rsi = !this.show_rsi;
                            }))
                    }),
            )
            // OHLCV info bar showing last candle data + MA values
            .child(self.render_ohlcv_bar(&visible, &ma_last_values))
            // Main chart area: candlesticks + MA overlay + BB overlay + crosshair
            .child({
                let origin_cell = self.chart_area_origin.clone();
                div()
                    .h(px(chart_height))
                    .w_full()
                    .relative()
                    .overflow_hidden()
                    // Invisible canvas to track chart area bounds in window coordinates
                    .child(
                        canvas(
                            move |bounds: Bounds<Pixels>, _window, _cx| {
                                origin_cell.set((f32::from(bounds.origin.x), f32::from(bounds.origin.y)));
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full()
                    )
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
                    .child(ma_overlay)
                    // Bollinger Bands overlay
                    .when(show_bb, |el| el.child(bb_overlay))
                    // Crosshair overlay
                    .map(|el| {
                        if let Some((_pos, local_x, local_y, candle_idx, price_at_y)) = hover_info {
                            let total_chart_width = (visible.len() as f32) * (candle_width + candle_gap) + chart_px_padding * 2.0;

                            // Snap crosshair X to candle center
                            let snapped_x = if candle_idx < visible.len() {
                                chart_px_padding + (candle_idx as f32) * (candle_width + candle_gap) + candle_width / 2.0
                            } else {
                                local_x
                            };

                            // Crosshair container
                            let mut crosshair = div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .h(px(chart_height));

                            // Vertical line at snapped candle center
                            if snapped_x >= 0.0 && snapped_x <= total_chart_width {
                                crosshair = crosshair.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .left(px(snapped_x))
                                        .w(px(1.0))
                                        .h(px(chart_height))
                                        .bg(gpui::rgba(0xffffff44)),
                                );
                            }

                            // Horizontal line at mouse Y
                            if local_y >= 0.0 && local_y <= chart_height {
                                crosshair = crosshair.child(
                                    div()
                                        .absolute()
                                        .top(px(local_y))
                                        .left_0()
                                        .w_full()
                                        .h(px(1.0))
                                        .bg(gpui::rgba(0xffffff44)),
                                );

                                // Price label at right edge
                                if let Some(price) = price_at_y {
                                    let label_y = (local_y - 8.0).max(0.0);
                                    crosshair = crosshair.child(
                                        div()
                                            .absolute()
                                            .top(px(label_y))
                                            .right(px(4.0))
                                            .bg(rgb(0x252a3a))
                                            .rounded(px(2.0))
                                            .px(px(4.0))
                                            .py(px(1.0))
                                            .text_size(px(10.0))
                                            .text_color(rgb(0xeaedf3))
                                            .child(format!("{:.2}", price)),
                                    );
                                }
                            }

                            // Candle info tooltip
                            if candle_idx < visible.len() {
                                let c = &visible[candle_idx];
                                let is_bullish = c.close >= c.open;
                                let color = if is_bullish { 0x22c55e } else { 0xef4444 };
                                let tooltip_x = (local_x + 12.0).min(total_chart_width - 140.0).max(0.0);
                                let tooltip_y = (local_y + 12.0).min(chart_height - 80.0).max(0.0);

                                crosshair = crosshair.child(
                                    div()
                                        .absolute()
                                        .top(px(tooltip_y))
                                        .left(px(tooltip_x))
                                        .bg(gpui::rgba(0x151822ee))
                                        .border_1()
                                        .border_color(rgb(0x2d3348))
                                        .rounded(px(4.0))
                                        .px(px(6.0))
                                        .py(px(4.0))
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.0))
                                        .text_size(px(10.0))
                                        .child(
                                            div().text_color(text_dimmest())
                                                .child(format_candle_time(c.time, current_interval))
                                        )
                                        .child(
                                            div().flex().gap(px(4.0))
                                                .child(div().text_color(text_dimmest()).child("O"))
                                                .child(div().text_color(rgb(color)).child(format!("{:.2}", c.open)))
                                                .child(div().text_color(text_dimmest()).child("H"))
                                                .child(div().text_color(rgb(color)).child(format!("{:.2}", c.high)))
                                        )
                                        .child(
                                            div().flex().gap(px(4.0))
                                                .child(div().text_color(text_dimmest()).child("L"))
                                                .child(div().text_color(rgb(color)).child(format!("{:.2}", c.low)))
                                                .child(div().text_color(text_dimmest()).child("C"))
                                                .child(div().text_color(rgb(color)).child(format!("{:.2}", c.close)))
                                        )
                                        .child(
                                            div().flex().gap(px(4.0))
                                                .child(div().text_color(text_dimmest()).child("Vol"))
                                                .child(div().text_color(text_dim()).child(format!("{:.0}", c.volume)))
                                        ),
                                );
                            }

                            el.child(crosshair)
                        } else {
                            el
                        }
                    })
            })
            // Price axis labels
            .child(self.render_price_axis(price_min, price_max))
            // Time axis labels
            .child(self.render_time_axis(&visible, candle_width, candle_gap, chart_px_padding))
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
                    .border_color(border_primary())
                    .children(visible.iter().enumerate().map(|(i, candle)| {
                        let vol_pct = candle.volume / vol_max;
                        let bar_h = (vol_pct * volume_height as f64).max(1.0) as f32;
                        let color = if candle.close >= candle.open {
                            rgb(0x133d2a)
                        } else {
                            rgb(0x3d1f13)
                        };
                        div()
                            .id(SharedString::from(format!("vol-{}", i)))
                            .w(px(candle_width))
                            .h(px(bar_h))
                            .bg(color)
                    })),
            )
            // MACD sub-chart
            .when(show_macd, |el| {
                el.child(self.render_macd_chart(
                    &macd_visible,
                    macd_height,
                    candle_width,
                    candle_gap,
                    chart_px_padding,
                    dot_size,
                ))
            })
            // RSI sub-chart
            .when(show_rsi, |el| {
                el.child(self.render_rsi_chart(
                    &rsi_visible,
                    rsi_height,
                    candle_width,
                    candle_gap,
                    chart_px_padding,
                    dot_size,
                ))
            })
            // Loading overlay
            .when(self.loading, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::rgba(0x0a0c14cc))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(text_dim())
                                .child("Loading..."),
                        ),
                )
            })
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
                color_green()
            } else {
                color_red()
            };

            let entries: Vec<(&str, String, gpui::Rgba)> = vec![
                ("O", format!("{:.2}", c.open), text_dim()),
                ("H", format!("{:.2}", c.high), text_dim()),
                ("L", format!("{:.2}", c.low), text_dim()),
                ("C", format!("{:.2}", c.close), price_color),
                ("Vol", format!("{:.0}", c.volume), text_dim()),
            ];

            for (label, val, color) in entries {
                container = container.child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(text_disabled())
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
                    .text_color(text_disabled())
                    .child(format!("{:.2}", price))
            }))
    }

    /// Render the time axis showing timestamps for visible candles.
    fn render_time_axis(
        &self,
        visible: &[Candle],
        candle_width: f32,
        candle_gap: f32,
        chart_px_padding: f32,
    ) -> impl IntoElement {
        // Show ~5-7 evenly spaced time labels
        let n = visible.len();
        let label_count = 6usize;
        let step = if n > label_count { n / label_count } else { 1 };

        let candle_step = candle_width + candle_gap;

        let mut container = div()
            .w_full()
            .flex()
            .px(px(chart_px_padding))
            .py(px(2.))
            .relative()
            .h(px(16.));

        for i in (0..n).step_by(step.max(1)) {
            let candle = &visible[i];
            let x = (i as f32) * candle_step + (candle_width / 2.0);
            let label = format_candle_time(candle.time, self.interval);

            container = container.child(
                div()
                    .absolute()
                    .left(px(chart_px_padding + x - 20.0))
                    .text_size(px(10.))
                    .text_color(text_disabled())
                    .child(label),
            );
        }

        container
    }

    /// Render the MACD sub-chart with histogram bars, MACD line, and signal line.
    fn render_macd_chart(
        &self,
        macd_visible: &[(Option<f64>, Option<f64>, Option<f64>)],
        height: f32,
        candle_width: f32,
        candle_gap: f32,
        chart_px_padding: f32,
        dot_size: f32,
    ) -> impl IntoElement {
        // Find the value range for MACD scaling
        let mut macd_min = f64::MAX;
        let mut macd_max = f64::MIN;
        for (macd_line, signal, histogram) in macd_visible {
            for v in [macd_line, signal, histogram] {
                if let Some(val) = v {
                    macd_min = macd_min.min(*val);
                    macd_max = macd_max.max(*val);
                }
            }
        }
        if macd_min >= macd_max {
            macd_min = -1.0;
            macd_max = 1.0;
        }
        // Add padding
        let range = macd_max - macd_min;
        macd_min -= range * 0.05;
        macd_max += range * 0.05;
        let macd_range = macd_max - macd_min;

        // The zero line position
        let zero_y = height - (((0.0 - macd_min) / macd_range) * height as f64) as f32;

        // Build MACD overlay with lines and histogram
        let mut macd_overlay = div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(height));

        // Zero line
        macd_overlay = macd_overlay.child(
            div()
                .absolute()
                .top(px(zero_y.clamp(0.0, height - 1.0)))
                .left_0()
                .w_full()
                .h(px(1.0))
                .bg(rgb(0x252a3a)),
        );

        for (i, (macd_line, signal, histogram)) in macd_visible.iter().enumerate() {
            let x = chart_px_padding
                + (i as f32) * (candle_width + candle_gap)
                + (candle_width / 2.0);

            // Histogram bar
            if let Some(hist) = histogram {
                let color = if *hist >= 0.0 {
                    color_green()
                } else {
                    color_red()
                };
                let val_y =
                    height - (((*hist - macd_min) / macd_range) * height as f64) as f32;
                let bar_top = val_y.min(zero_y);
                let bar_h = (val_y - zero_y).abs().max(1.0);
                macd_overlay = macd_overlay.child(
                    div()
                        .absolute()
                        .top(px(bar_top))
                        .left(px(x - candle_width / 2.0))
                        .w(px(candle_width))
                        .h(px(bar_h))
                        .bg(color),
                );
            }

            // MACD line dot
            if let Some(val) = macd_line {
                let y = height - (((val - macd_min) / macd_range) * height as f64) as f32
                    - (dot_size / 2.0);
                macd_overlay = macd_overlay.child(
                    div()
                        .absolute()
                        .top(px(y))
                        .left(px(x - dot_size / 2.0))
                        .w(px(dot_size))
                        .h(px(dot_size))
                        .rounded(px(dot_size / 2.0))
                        .bg(rgb(0x00aaff)),
                );
            }

            // Signal line dot
            if let Some(val) = signal {
                let y = height - (((val - macd_min) / macd_range) * height as f64) as f32
                    - (dot_size / 2.0);
                macd_overlay = macd_overlay.child(
                    div()
                        .absolute()
                        .top(px(y))
                        .left(px(x - dot_size / 2.0))
                        .w(px(dot_size))
                        .h(px(dot_size))
                        .rounded(px(dot_size / 2.0))
                        .bg(rgb(0xff6600)),
                );
            }
        }

        div()
            .h(px(height))
            .w_full()
            .relative()
            .border_t_1()
            .border_color(border_primary())
            // Label
            .child(
                div()
                    .absolute()
                    .top(px(2.0))
                    .left(px(4.0))
                    .text_size(px(10.))
                    .text_color(text_disabled())
                    .child("MACD"),
            )
            .child(macd_overlay)
    }

    /// Render the RSI sub-chart with the RSI line and reference lines at 30/70.
    fn render_rsi_chart(
        &self,
        rsi_visible: &[Option<f64>],
        height: f32,
        candle_width: f32,
        candle_gap: f32,
        chart_px_padding: f32,
        dot_size: f32,
    ) -> impl IntoElement {
        // RSI is always 0-100
        let rsi_min: f64 = 0.0;
        let rsi_max: f64 = 100.0;
        let rsi_range = rsi_max - rsi_min;

        let mut rsi_overlay = div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(height));

        // Reference line at 70 (overbought)
        let y70 = height - (((70.0 - rsi_min) / rsi_range) * height as f64) as f32;
        rsi_overlay = rsi_overlay.child(
            div()
                .absolute()
                .top(px(y70))
                .left_0()
                .w_full()
                .h(px(1.0))
                .bg(rgb(0x252a3a)),
        );

        // Reference line at 30 (oversold)
        let y30 = height - (((30.0 - rsi_min) / rsi_range) * height as f64) as f32;
        rsi_overlay = rsi_overlay.child(
            div()
                .absolute()
                .top(px(y30))
                .left_0()
                .w_full()
                .h(px(1.0))
                .bg(rgb(0x252a3a)),
        );

        // RSI line dots
        for (i, rsi_val) in rsi_visible.iter().enumerate() {
            if let Some(val) = rsi_val {
                let x = chart_px_padding
                    + (i as f32) * (candle_width + candle_gap)
                    + (candle_width / 2.0)
                    - (dot_size / 2.0);
                let y = height - (((val - rsi_min) / rsi_range) * height as f64) as f32
                    - (dot_size / 2.0);
                rsi_overlay = rsi_overlay.child(
                    div()
                        .absolute()
                        .top(px(y))
                        .left(px(x))
                        .w(px(dot_size))
                        .h(px(dot_size))
                        .rounded(px(dot_size / 2.0))
                        .bg(rgb(0xffaa00)),
                );
            }
        }

        // Get last RSI value for label
        let last_rsi = rsi_visible.iter().rev().find_map(|v| *v);
        let rsi_label = match last_rsi {
            Some(v) => format!("RSI(14) {:.1}", v),
            None => "RSI(14)".to_string(),
        };

        div()
            .h(px(height))
            .w_full()
            .relative()
            .border_t_1()
            .border_color(border_primary())
            // Label with value
            .child(
                div()
                    .absolute()
                    .top(px(2.0))
                    .left(px(4.0))
                    .text_size(px(10.))
                    .text_color(text_disabled())
                    .child(rsi_label),
            )
            // 70 label
            .child(
                div()
                    .absolute()
                    .top(px(y70 - 10.0))
                    .right(px(4.0))
                    .text_size(px(9.))
                    .text_color(rgb(0x454c5c))
                    .child("70"),
            )
            // 30 label
            .child(
                div()
                    .absolute()
                    .top(px(y30 - 10.0))
                    .right(px(4.0))
                    .text_size(px(9.))
                    .text_color(rgb(0x454c5c))
                    .child("30"),
            )
            .child(rsi_overlay)
    }
}

/// Format a candle timestamp (unix ms) as a human-readable time label.
fn format_candle_time(time_ms: u64, interval: CandleInterval) -> String {
    let secs = (time_ms / 1000) as i64;
    // Manual UTC breakdown (no chrono dependency)
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    // Compute year/month/day from days since epoch
    let (_year, month, day) = days_from_epoch(days_since_epoch);

    match interval {
        CandleInterval::D1 => format!("{:02}/{:02}", month, day),
        CandleInterval::H4 | CandleInterval::H1 => format!("{:02}/{:02} {:02}:{:02}", month, day, hours, minutes),
        _ => format!("{:02}:{:02}", hours, minutes),
    }
}

/// Convert days since Unix epoch to (year, month, day).
fn days_from_epoch(days: i64) -> (i64, u32, u32) {
    // Civil calendar algorithm from Howard Hinnant
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
