use gpui::{rgb, Rgba};

// === Background colors ===
pub fn bg_primary() -> Rgba { rgb(0x1a1a2e) }
pub fn bg_panel() -> Rgba { rgb(0x16213e) }
pub fn bg_header() -> Rgba { rgb(0x0f3460) }
pub fn bg_row_alt() -> Rgba { rgb(0x1a2744) }
pub fn bg_hover() -> Rgba { rgb(0x1a3a6e) }
pub fn bg_card() -> Rgba { rgb(0x1c2a4a) }

// === Border colors ===
pub fn border_primary() -> Rgba { rgb(0x0f3460) }
pub fn border_accent() -> Rgba { rgb(0x1a1a4e) }
pub fn border_card() -> Rgba { rgb(0x1a4a80) }

// === Text colors ===
pub fn text_primary() -> Rgba { rgb(0xffffff) }
pub fn text_secondary() -> Rgba { rgb(0xdddddd) }
pub fn text_muted() -> Rgba { rgb(0xcccccc) }
pub fn text_dim() -> Rgba { rgb(0xaaaaaa) }
pub fn text_dimmer() -> Rgba { rgb(0x999999) }
pub fn text_dimmest() -> Rgba { rgb(0x888888) }
pub fn text_disabled() -> Rgba { rgb(0x666666) }

// === Semantic colors ===
pub fn color_green() -> Rgba { rgb(0x00ff88) }
pub fn color_red() -> Rgba { rgb(0xff4444) }
pub fn color_yellow() -> Rgba { rgb(0xffaa00) }
pub fn color_orange() -> Rgba { rgb(0xff8844) }
pub fn color_brand() -> Rgba { rgb(0xe94560) }
pub fn color_success() -> Rgba { rgb(0x53c28b) }

// === Helper functions ===

/// Returns green for positive values, red for negative.
pub fn pnl_color(value: f64) -> Rgba {
    if value >= 0.0 { color_green() } else { color_red() }
}

/// Returns green for Buy, red for Sell.
pub fn side_color(side: crate::models::OrderSide) -> Rgba {
    match side {
        crate::models::OrderSide::Buy => color_green(),
        crate::models::OrderSide::Sell => color_red(),
    }
}

/// Returns the row background color for zebra-striped tables.
pub fn row_bg(index: usize) -> Rgba {
    if index % 2 == 0 { bg_panel() } else { bg_row_alt() }
}
