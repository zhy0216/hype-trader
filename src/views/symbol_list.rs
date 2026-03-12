use gpui::prelude::*;
use gpui::{div, px, rgb, Entity, EventEmitter, SharedString};
use gpui_component::input::{Input, InputState};

use crate::models::Symbol;

/// Emitted when the user clicks a symbol in the list.
/// Carries the full symbol name (e.g. "ETH-USD").
pub struct SymbolSelected(pub String);

impl EventEmitter<SymbolSelected> for SymbolList {}

pub struct SymbolList {
    pub symbols: Vec<Symbol>,
    pub selected: String,
    pub filter: String,
    filter_input: Entity<InputState>,
}

impl SymbolList {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let filter_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search symbol...")
        });
        Self {
            symbols: Vec::new(),
            selected: "ETH-USD".to_string(),
            filter: String::new(),
            filter_input,
        }
    }

    fn filtered_symbols(&self) -> Vec<&Symbol> {
        let filter = self.filter.to_uppercase();
        self.symbols
            .iter()
            .filter(|s| {
                filter.is_empty()
                    || s.name.to_uppercase().contains(&filter)
                    || s.base.to_uppercase().contains(&filter)
            })
            .collect()
    }
}

impl Render for SymbolList {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let filtered = self.filtered_symbols();
        let selected = self.selected.clone();

        div()
            .w(px(200.))
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0x16213e))
            // Header
            .child(
                div()
                    .p(px(8.))
                    .border_b_1()
                    .border_color(rgb(0x0f3460))
                    .child(Input::new(&self.filter_input)),
            )
            // Symbol rows - scrollable
            .child(
                div()
                    .id("symbol-list-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(filtered.into_iter().map(|symbol| {
                        let is_selected = symbol.name == selected;
                        let name = symbol.name.clone();
                        let change_color = if symbol.change_24h >= 0.0 {
                            rgb(0x00ff88)
                        } else {
                            rgb(0xff4444)
                        };
                        let change_str = format!("{:+.2}%", symbol.change_24h);
                        let price_str = format_price(symbol.last_price);
                        let sym_name = name.clone();

                        div()
                            .id(SharedString::from(format!("sym-{}", name)))
                            .w_full()
                            .px(px(10.))
                            .py(px(6.))
                            .flex()
                            .justify_between()
                            .items_center()
                            .cursor_pointer()
                            .bg(if is_selected {
                                rgb(0x0f3460)
                            } else {
                                rgb(0x16213e)
                            })
                            .hover(|s| s.bg(rgb(0x1a3a6e)))
                            .on_click(cx.listener(move |this, _, _w, cx| {
                                this.selected = sym_name.clone();
                                cx.emit(SymbolSelected(sym_name.clone()));
                            }))
                            // Left: symbol name
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(if is_selected {
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0xcccccc)
                                    })
                                    .child(name),
                            )
                            // Right: price + change
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(0xdddddd))
                                            .child(price_str),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
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
