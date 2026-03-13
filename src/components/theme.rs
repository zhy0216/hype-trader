use gpui::{rgb, Rgba};

// === Background colors ===
pub fn bg_primary() -> Rgba { rgb(0x0f1118) }      // Deep dark base
pub fn bg_panel() -> Rgba { rgb(0x151822) }         // Slightly lighter panel
pub fn bg_header() -> Rgba { rgb(0x1a1e2c) }        // Header/card background
pub fn bg_row_alt() -> Rgba { rgb(0x1c2030) }       // Alternating row
pub fn bg_hover() -> Rgba { rgb(0x252a3a) }         // Hover state
pub fn bg_card() -> Rgba { rgb(0x1a1e2c) }          // Card surfaces
pub fn bg_input() -> Rgba { rgb(0x0d0f16) }         // Input field background
pub fn bg_elevated() -> Rgba { rgb(0x1f2435) }      // Elevated surfaces (modals, dropdowns)

// === Border colors ===
pub fn border_primary() -> Rgba { rgb(0x252a3a) }   // Subtle border
pub fn border_accent() -> Rgba { rgb(0x2d3348) }    // Slightly more visible border
pub fn border_card() -> Rgba { rgb(0x2d3348) }      // Card border
pub fn border_focus() -> Rgba { rgb(0x3b82f6) }     // Focus ring color

// === Text colors ===
pub fn text_primary() -> Rgba { rgb(0xeaedf3) }     // Primary text (slightly off-white)
pub fn text_secondary() -> Rgba { rgb(0xb0b8c9) }   // Secondary text
pub fn text_muted() -> Rgba { rgb(0x8b95a9) }       // Muted text
pub fn text_dim() -> Rgba { rgb(0x6b7280) }         // Dim labels
pub fn text_dimmer() -> Rgba { rgb(0x565e6e) }      // Even dimmer
pub fn text_dimmest() -> Rgba { rgb(0x454c5c) }     // Barely visible
pub fn text_disabled() -> Rgba { rgb(0x3a4050) }    // Disabled state

// === Semantic colors ===
pub fn color_green() -> Rgba { rgb(0x22c55e) }      // Softer green (not neon)
pub fn color_red() -> Rgba { rgb(0xef4444) }        // Standard red
pub fn color_yellow() -> Rgba { rgb(0xeab308) }     // Warning yellow
pub fn color_orange() -> Rgba { rgb(0xf59e0b) }     // Fee/orange accent
pub fn color_brand() -> Rgba { rgb(0x6366f1) }      // Indigo brand color
pub fn color_success() -> Rgba { rgb(0x22c55e) }    // Same as green

// Buy/Sell specific colors (slightly different shades for clarity)
pub fn color_buy() -> Rgba { rgb(0x22c55e) }        // Green for buy/long
pub fn color_sell() -> Rgba { rgb(0xef4444) }       // Red for sell/short
pub fn color_buy_bg() -> Rgba { rgb(0x0f2a1a) }     // Dark green tinted background
pub fn color_sell_bg() -> Rgba { rgb(0x2a0f0f) }    // Dark red tinted background

// === Helper functions ===

/// Returns green for positive values, red for negative.
pub fn pnl_color(value: f64) -> Rgba {
    if value >= 0.0 { color_green() } else { color_red() }
}

/// Returns green for Buy, red for Sell.
pub fn side_color(side: crate::models::OrderSide) -> Rgba {
    match side {
        crate::models::OrderSide::Buy => color_buy(),
        crate::models::OrderSide::Sell => color_sell(),
    }
}

/// Returns the row background color for zebra-striped tables.
pub fn row_bg(index: usize) -> Rgba {
    if index % 2 == 0 { bg_panel() } else { bg_row_alt() }
}
