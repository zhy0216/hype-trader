use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{div, px, Entity, EventEmitter, Rgba, SharedString};
use gpui_component::input::{Input, InputState};

use crate::components::theme::*;
use crate::models::Symbol;

const FLASH_DURATION_MS: u128 = 800;
const PINNED_BASES: &[&str] = &["BTC", "ETH", "SOL", "SUI", "HYPER"];

/// Emitted when the user clicks a symbol in the list.
/// Carries the full symbol name (e.g. "ETH-USD").
pub struct SymbolSelected(pub String);

impl EventEmitter<SymbolSelected> for SymbolList {}

pub struct SymbolList {
    pub symbols: Vec<Symbol>,
    pub selected: String,
    pub filter: String,
    filter_input: Entity<InputState>,
    prev_prices: HashMap<String, f64>,
    flash_states: HashMap<String, (bool, Instant)>, // (is_up, when)
    animation_scheduled: bool,
}

impl SymbolList {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let filter_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search...")
        });
        Self {
            symbols: Vec::new(),
            selected: "ETH-USD".to_string(),
            filter: String::new(),
            filter_input,
            prev_prices: HashMap::new(),
            flash_states: HashMap::new(),
            animation_scheduled: false,
        }
    }

    /// Update prices from AllMids, detecting changes for flash animation.
    pub fn update_prices(&mut self, mids: &HashMap<String, f64>) {
        let now = Instant::now();
        for symbol in &mut self.symbols {
            if let Some(&new_price) = mids.get(&symbol.base) {
                if let Some(&prev_price) = self.prev_prices.get(&symbol.name) {
                    if new_price != prev_price {
                        let is_up = new_price > prev_price;
                        self.flash_states.insert(symbol.name.clone(), (is_up, now));
                    }
                }
                self.prev_prices.insert(symbol.name.clone(), new_price);
                symbol.last_price = new_price;
            }
        }
    }

    fn flash_intensity(&self, name: &str) -> Option<(bool, f32)> {
        if let Some(&(is_up, start)) = self.flash_states.get(name) {
            let elapsed = start.elapsed().as_millis();
            if elapsed < FLASH_DURATION_MS {
                let t = 1.0 - (elapsed as f32 / FLASH_DURATION_MS as f32);
                return Some((is_up, t));
            }
        }
        None
    }

    fn filtered_symbols(&self) -> Vec<&Symbol> {
        let filter = self.filter.to_uppercase();
        if filter.is_empty() {
            // Show only pinned symbols when no search query
            self.symbols
                .iter()
                .filter(|s| PINNED_BASES.iter().any(|&b| s.base.eq_ignore_ascii_case(b)))
                .collect()
        } else {
            // Show all matching symbols when searching
            self.symbols
                .iter()
                .filter(|s| {
                    s.name.to_uppercase().contains(&filter)
                        || s.base.to_uppercase().contains(&filter)
                })
                .collect()
        }
    }
}

impl Render for SymbolList {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected.clone();

        // Collect filtered symbols into owned data to release borrow on self
        struct SymRow {
            name: String,
            last_price: f64,
            change_24h: f64,
            is_selected: bool,
            flash: Option<(bool, f32)>,
        }
        let rows: Vec<SymRow> = self
            .filtered_symbols()
            .into_iter()
            .map(|s| SymRow {
                name: s.name.clone(),
                last_price: s.last_price,
                change_24h: s.change_24h,
                is_selected: s.name == selected,
                flash: self.flash_intensity(&s.name),
            })
            .collect();

        let has_active_flash = rows.iter().any(|r| r.flash.is_some());

        // Schedule animation re-renders for smooth fade
        if has_active_flash && !self.animation_scheduled {
            self.animation_scheduled = true;
            cx.spawn(async move |this, cx| {
                for _ in 0..6 {
                    tokio::time::sleep(Duration::from_millis(130)).await;
                    let should_continue = this.update(cx, |list, cx| {
                        let active = list
                            .flash_states
                            .values()
                            .any(|(_, start)| start.elapsed().as_millis() < FLASH_DURATION_MS);
                        cx.notify();
                        active
                    });
                    if !should_continue.unwrap_or(false) {
                        break;
                    }
                }
                let _ = this.update(cx, |list, cx| {
                    list.animation_scheduled = false;
                    list.flash_states
                        .retain(|_, (_, start)| start.elapsed().as_millis() < FLASH_DURATION_MS);
                    cx.notify();
                });
            })
            .detach();
        }

        div()
            .w(px(220.))
            .h_full()
            .flex()
            .flex_col()
            .bg(bg_panel())
            .border_r_1()
            .border_color(border_primary())
            // Header with search
            .child(
                div()
                    .px(px(10.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(border_primary())
                    .child(Input::new(&self.filter_input)),
            )
            // Column headers
            .child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(6.))
                    .flex()
                    .justify_between()
                    .border_b_1()
                    .border_color(border_primary())
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(text_dimmest())
                            .child("PAIR"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(text_dimmest())
                            .child("PRICE / 24H"),
                    ),
            )
            // Symbol rows - scrollable
            .child(
                div()
                    .id("symbol-list-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(rows.into_iter().map(|row| {
                        let is_selected = row.is_selected;
                        let name = row.name.clone();
                        let change_color = pnl_color(row.change_24h);
                        let change_str = format!("{:+.2}%", row.change_24h);
                        let price_str = format_price(row.last_price);
                        let sym_name = name.clone();

                        // Flash animation
                        let flash = row.flash;

                        let price_color = match flash {
                            Some((true, t)) => {
                                lerp_color(text_primary(), color_green(), t)
                            }
                            Some((false, t)) => {
                                lerp_color(text_primary(), color_red(), t)
                            }
                            None => text_primary(),
                        };

                        let base_bg = if is_selected { bg_hover() } else { bg_panel() };
                        let row_bg = match flash {
                            Some((true, t)) => {
                                lerp_color(base_bg, color_buy_bg(), t)
                            }
                            Some((false, t)) => {
                                lerp_color(base_bg, color_sell_bg(), t)
                            }
                            None => base_bg,
                        };

                        div()
                            .id(SharedString::from(format!("sym-{}", name)))
                            .w_full()
                            .px(px(12.))
                            .py(px(7.))
                            .flex()
                            .justify_between()
                            .items_center()
                            .cursor_pointer()
                            .bg(row_bg)
                            .when(is_selected, |el| {
                                el.border_l_2().border_color(color_brand())
                            })
                            .hover(|s| s.bg(bg_hover()))
                            .on_click(cx.listener(move |this, _, _w, cx| {
                                this.selected = sym_name.clone();
                                cx.emit(SymbolSelected(sym_name.clone()));
                            }))
                            // Left: symbol name
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(if is_selected {
                                        text_primary()
                                    } else {
                                        text_secondary()
                                    })
                                    .child(name),
                            )
                            // Right: price + change
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap(px(1.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(price_color)
                                            .child(price_str),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(change_color)
                                            .child(change_str),
                                    ),
                            )
                    })),
            )
    }
}

fn format_price(price: f64) -> String {
    if price >= 1000.0 {
        format!("{:.1}", price)
    } else if price >= 1.0 {
        format!("{:.2}", price)
    } else {
        format!("{:.4}", price)
    }
}

/// Linearly interpolate between two colors.
fn lerp_color(a: Rgba, b: Rgba, t: f32) -> Rgba {
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}
