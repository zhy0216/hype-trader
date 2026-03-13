# UI Components Extraction Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract reusable UI components from views to eliminate code duplication and establish a consistent component library.

**Architecture:** Create a `src/components/` module containing pure helper functions that return `impl IntoElement`. These are not GPUI `Render` views — they're composable element builders (like the existing `table_header()`, `stat_card()`, `empty_state()` in bottom_panel.rs). Each component is a function that takes data and returns styled elements.

**Tech Stack:** Rust, GPUI, gpui_component

---

## File Structure

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `src/components/mod.rs` | Re-export all component modules |
| Create | `src/components/theme.rs` | Color constants + helper fns (`pnl_color`, `side_color`) |
| Create | `src/components/toggle_button.rs` | `toggle_button()` — Button with active/inactive primary/ghost states |
| Create | `src/components/table.rs` | `table_header()`, `table_cell()`, `table_row()`, `empty_state()` — extracted from bottom_panel.rs |
| Create | `src/components/input_field.rs` | `input_field()` — Label + Input wrapper |
| Create | `src/components/stat_card.rs` | `stat_card()` — Label + value display card, extracted from bottom_panel.rs |
| Create | `src/components/status_dot.rs` | `status_dot()` — Small colored circle indicator |
| Create | `src/components/pnl_text.rs` | `pnl_text()` — Value with conditional red/green coloring |
| Modify | `src/main.rs` | Add `mod components;` |
| Modify | `src/views/top_bar.rs` | Use theme colors, toggle_button, status_dot |
| Modify | `src/views/order_panel.rs` | Use theme colors, toggle_button, input_field |
| Modify | `src/views/bottom_panel.rs` | Use theme colors, table components, pnl_text, stat_card |
| Modify | `src/views/welcome_view.rs` | Use theme colors, input_field (no toggle_button — welcome uses `.outline()` for inactive, not `.ghost()`) |
| Modify | `src/views/symbol_list.rs` | Use theme colors, pnl_text |
| Modify | `src/views/order_book.rs` | Use theme colors, table_cell |
| Modify | `src/views/candle_chart.rs` | Use theme colors, toggle_button |

---

## Chunk 1: Foundation — Theme Colors + Module Setup

### Task 1: Create components module and theme colors

**Files:**
- Create: `src/components/mod.rs`
- Create: `src/components/theme.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/components/theme.rs`**

Define all colors as functions (not `const` — `gpui::rgb()` is not a `const fn`). Group by semantic meaning.

```rust
use gpui::{rgb, Rgba};

// === Background colors ===
pub fn bg_primary() -> Rgba { rgb(0x1a1a2e) }      // darkest background (app bg)
pub fn bg_panel() -> Rgba { rgb(0x16213e) }         // panel/sidebar background
pub fn bg_header() -> Rgba { rgb(0x0f3460) }        // header bar, card bg, selected row
pub fn bg_row_alt() -> Rgba { rgb(0x1a2744) }       // alternating table row
pub fn bg_hover() -> Rgba { rgb(0x1a3a6e) }         // hover state
pub fn bg_card() -> Rgba { rgb(0x1c2a4a) }          // card background (welcome)

// === Border colors ===
pub fn border_primary() -> Rgba { rgb(0x0f3460) }   // standard border/divider
pub fn border_accent() -> Rgba { rgb(0x1a1a4e) }    // top bar bottom border
pub fn border_card() -> Rgba { rgb(0x1a4a80) }      // card border (welcome)

// === Text colors ===
pub fn text_primary() -> Rgba { rgb(0xffffff) }      // primary white text
pub fn text_secondary() -> Rgba { rgb(0xdddddd) }    // labels, prices
pub fn text_muted() -> Rgba { rgb(0xcccccc) }        // table cell text
pub fn text_dim() -> Rgba { rgb(0xaaaaaa) }          // dimmed text, sub-labels
pub fn text_dimmer() -> Rgba { rgb(0x999999) }       // cumulative text
pub fn text_dimmest() -> Rgba { rgb(0x888888) }      // table headers
pub fn text_disabled() -> Rgba { rgb(0x666666) }     // empty state

// === Semantic colors ===
pub fn color_green() -> Rgba { rgb(0x00ff88) }       // buy, positive PnL, connected
pub fn color_red() -> Rgba { rgb(0xff4444) }         // sell, negative PnL, disconnected, error
pub fn color_yellow() -> Rgba { rgb(0xffaa00) }      // warning, connecting, margin
pub fn color_orange() -> Rgba { rgb(0xff8844) }      // fees
pub fn color_brand() -> Rgba { rgb(0xe94560) }       // brand accent (Hype Trader)
pub fn color_success() -> Rgba { rgb(0x53c28b) }     // success messages

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
```

- [ ] **Step 2: Create `src/components/mod.rs`**

```rust
pub mod theme;
```

- [ ] **Step 3: Add `mod components;` to `src/main.rs`**

Add the line `mod components;` alongside the existing `mod views;`.

- [ ] **Step 4: Build to verify**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation (warnings OK, no errors)

- [ ] **Step 5: Commit**

```bash
git add src/components/mod.rs src/components/theme.rs src/main.rs
git commit -m "feat: add components module with theme color constants"
```

---

## Chunk 2: Core Components — toggle_button, table, input_field, stat_card, status_dot, pnl_text

### Task 2: Create toggle_button component

**Files:**
- Create: `src/components/toggle_button.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/toggle_button.rs`**

```rust
use gpui::prelude::*;
use gpui::SharedString;
use gpui_component::button::{Button, ButtonVariants as _};

/// A button that shows primary style when active, ghost when inactive.
pub fn toggle_button(id: impl Into<SharedString>, label: &str, active: bool) -> Button {
    Button::new(id)
        .label(label)
        .compact()
        .map(|b| if active { b.primary() } else { b.ghost() })
}
```

- [ ] **Step 2: Add to `src/components/mod.rs`**

```rust
pub mod theme;
pub mod toggle_button;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build 2>&1 | head -20`
Expected: Compiles successfully

### Task 3: Create table components

**Files:**
- Create: `src/components/table.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/table.rs`**

Extract and generalize `table_header()`, `empty_state()`, `format_timestamp()` from bottom_panel.rs. Add new `table_cell()` and `table_row()`.

```rust
use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::*;

/// Renders a table header row with column labels and widths.
pub fn table_header(headers: &[(&str, f32)]) -> Div {
    let items: Vec<_> = headers
        .iter()
        .map(|(header, w)| {
            div()
                .w(px(*w))
                .text_size(px(11.))
                .text_color(text_dimmest())
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
        .border_color(border_primary())
        .children(items)
}

/// A single table cell with fixed width.
pub fn table_cell(width: f32, text: impl Into<String>, color: gpui::Rgba) -> Div {
    div()
        .w(px(width))
        .text_size(px(12.))
        .text_color(color)
        .child(text.into())
}

/// A standard table data row with zebra-striping.
pub fn table_row(index: usize) -> Div {
    div()
        .w_full()
        .px(px(10.))
        .py(px(4.))
        .flex()
        .items_center()
        .bg(row_bg(index))
}

/// Empty state placeholder for tables with no data.
pub fn empty_state(message: &str) -> Div {
    div()
        .w_full()
        .py(px(20.))
        .flex()
        .justify_center()
        .child(
            div()
                .text_size(px(13.))
                .text_color(text_disabled())
                .child(message.to_string()),
        )
}

/// Formats a unix-ms timestamp as relative time (e.g. "5m ago").
pub fn format_timestamp(ts: u64) -> String {
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
```

- [ ] **Step 2: Add to `src/components/mod.rs`**

- [ ] **Step 3: Build to verify**

### Task 4: Create input_field component

**Files:**
- Create: `src/components/input_field.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/input_field.rs`**

```rust
use gpui::prelude::*;
use gpui::{div, px, Entity, Div};
use gpui_component::input::{Input, InputState};

use super::theme::*;

/// A labeled input field (label on top, input below).
/// Uses px(13.) and text_secondary() for labels — standardized from welcome_view style.
pub fn input_field(label: &str, input: &Entity<InputState>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(13.))
                .text_color(text_secondary())
                .child(label.to_string()),
        )
        .child(Input::new(input))
}
```

- [ ] **Step 2: Add to `src/components/mod.rs`**

- [ ] **Step 3: Build to verify**

### Task 5: Create stat_card component

**Files:**
- Create: `src/components/stat_card.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/stat_card.rs`**

```rust
use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::*;

/// A small card displaying a label and a colored value.
pub fn stat_card(label: &str, value: &str, value_color: gpui::Rgba) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(10.))
        .rounded(px(6.))
        .bg(bg_header())
        .child(
            div()
                .text_size(px(11.))
                .text_color(text_dimmest())
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(14.))
                .text_color(value_color)
                .child(value.to_string()),
        )
}
```

- [ ] **Step 2: Add to `src/components/mod.rs`**

- [ ] **Step 3: Build to verify**

### Task 6: Create status_dot component

**Files:**
- Create: `src/components/status_dot.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/status_dot.rs`**

```rust
use gpui::prelude::*;
use gpui::{div, px, Div, Rgba};

/// A small colored circle indicator.
pub fn status_dot(color: Rgba) -> Div {
    div()
        .w(px(8.))
        .h(px(8.))
        .rounded(px(4.))
        .bg(color)
}
```

- [ ] **Step 2: Add to `src/components/mod.rs`**

- [ ] **Step 3: Build to verify**

### Task 7: Create pnl_text component

**Files:**
- Create: `src/components/pnl_text.rs`
- Modify: `src/components/mod.rs`

- [ ] **Step 1: Create `src/components/pnl_text.rs`**

```rust
use gpui::prelude::*;
use gpui::{div, px, Div};

use super::theme::pnl_color;

/// Displays a value with green (positive) or red (negative) coloring.
/// format options: "signed" (+1.23), "percent" (+1.23%), default (1.23)
pub fn pnl_text(value: f64, format: &str, font_size: f32) -> Div {
    let text = match format {
        "signed" => format!("{:+.2}", value),
        "percent" => format!("{:+.2}%", value),
        _ => format!("{:.2}", value),
    };
    div()
        .text_size(px(font_size))
        .text_color(pnl_color(value))
        .child(text)
}
```

- [ ] **Step 2: Update `src/components/mod.rs` to final state**

```rust
pub mod input_field;
pub mod pnl_text;
pub mod stat_card;
pub mod status_dot;
pub mod table;
pub mod theme;
pub mod toggle_button;
```

- [ ] **Step 3: Build to verify all components compile**

Run: `cargo build 2>&1 | head -20`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/components/
git commit -m "feat: add toggle_button, table, input_field, stat_card, status_dot, pnl_text components"
```

---

## Chunk 3: Refactor Views to Use Components

### Task 8: Refactor top_bar.rs

**Files:**
- Modify: `src/views/top_bar.rs`

- [ ] **Step 1: Replace hardcoded colors and patterns**

Changes:
- Import `crate::components::theme::*`, `toggle_button`, `status_dot`
- Replace `gpui::rgb(0x00ff88)` etc. with `color_green()`, `color_red()`, `color_yellow()`
- Replace network toggle buttons with `toggle_button("mainnet-btn", "Mainnet", self.network == Network::Mainnet)`
- Replace status dot div with `status_dot(status_color)`
- Replace hardcoded bg/border colors with theme constants

The structure and behavior remain identical — only the color/component references change.

- [ ] **Step 2: Build to verify**

Run: `cargo build 2>&1 | head -20`

### Task 9: Refactor order_panel.rs

**Files:**
- Modify: `src/views/order_panel.rs`

- [ ] **Step 1: Replace hardcoded colors and patterns**

Changes:
- Import `crate::components::{theme::*, toggle_button::toggle_button, input_field::input_field}`
- Replace 3 order type toggle buttons with `toggle_button()` calls + `.on_click()`. Note: the TP/SL button uses `matches!(self.order_type, OrderType::TakeProfit | OrderType::StopLoss)` for its active bool
- Replace 2 Buy/Sell buttons with `toggle_button()` calls + `.w_full().on_click()` (these need `.w_full()` chained after `toggle_button()`)
- Replace 2 label+input patterns with `input_field("Price", &self.price_input)` and `input_field("Size", &self.size_input)`
- Replace `rgb(0x16213e)` with `bg_panel()`, `rgb(0xaaaaaa)` with `text_dim()`

- [ ] **Step 2: Build to verify**

### Task 10: Refactor bottom_panel.rs

**Files:**
- Modify: `src/views/bottom_panel.rs`

- [ ] **Step 1: Replace with component calls**

This is the biggest refactor. Changes:
- Import `crate::components::{theme::*, table::*, stat_card::stat_card, pnl_text::pnl_text}`
- Remove local `table_header()`, `stat_card()`, `empty_state()`, `format_timestamp()` free functions (now in components)
- Update `table_header()` calls to new signature with explicit widths per table:
  - Positions: `table_header(&[("Symbol", 80.), ("Side", 60.), ("Size", 70.), ("Entry", 80.), ("Mark", 80.), ("PnL", 80.), ("Lev.", 50.), ("Action", 60.)])`
  - Orders: `table_header(&[("Symbol", 80.), ("Side", 60.), ("Type", 60.), ("Price", 80.), ("Size", 70.), ("Filled", 70.), ("Action", 60.)])`
  - History: `table_header(&[("Time", 120.), ("Symbol", 80.), ("Side", 60.), ("Price", 80.), ("Size", 70.), ("Fee", 70.)])`
  - Funds: `table_header(&[("Asset", 80.), ("Total", 100.), ("Available", 100.), ("In Margin", 100.)])`
- Replace all `div().w(px(80.)).text_size(px(12.)).text_color(rgb(0xcccccc)).child(...)` patterns with `table_cell(80., text, text_muted())`
- Replace all row wrapper divs with `table_row(i)`
- Replace `stat_card()` calls to use imported version (same signature, uses theme colors internally)
- Replace PnL conditional colors with `pnl_color(value)` from theme
- Replace `side_color` logic with `side_color(pos.side)` from theme

- [ ] **Step 2: Build to verify**

### Task 11: Refactor welcome_view.rs

**Files:**
- Modify: `src/views/welcome_view.rs`

- [ ] **Step 1: Replace hardcoded colors and patterns**

Changes:
- Import `crate::components::{theme::*, input_field::input_field}`
- Do NOT use `toggle_button()` for welcome_view — its inactive buttons use `.outline().text_color(hsla(...))` not `.ghost()`. Keep using `Button` directly but reference theme color functions
- Replace 3 label+input wrappers (Private Key, Password, Encryption Password) with `input_field()` calls
- Replace all `gpui::rgb(0x...)` with theme color functions

- [ ] **Step 2: Build to verify**

### Task 12: Refactor symbol_list.rs

**Files:**
- Modify: `src/views/symbol_list.rs`

- [ ] **Step 1: Replace hardcoded colors**

Changes:
- Import `crate::components::theme::*`
- Replace `rgb(0x00ff88)` / `rgb(0xff4444)` change color logic with `pnl_color(symbol.change_24h)`
- Replace background colors with `bg_panel()`, `bg_header()`, `bg_hover()`
- Replace text colors with `text_muted()`, `text_primary()`, `text_secondary()`
- Replace border color with `border_primary()`

- [ ] **Step 2: Build to verify**

### Task 13: Refactor order_book.rs

**Files:**
- Modify: `src/views/order_book.rs`

- [ ] **Step 1: Replace hardcoded colors**

Changes:
- Import `crate::components::theme::*`
- Replace all `rgb(0x...)` in `render_level()` and the main render with theme constants
- Replace `rgb(0xff4444)` / `rgb(0x00ff88)` passed to `render_level()` with `color_red()` / `color_green()`
- Replace background, border, text colors with theme constants

- [ ] **Step 2: Build to verify**

### Task 14: Refactor candle_chart.rs

**Files:**
- Modify: `src/views/candle_chart.rs`

- [ ] **Step 1: Replace toggle button patterns and colors**

Changes:
- Import `crate::components::{theme::*, toggle_button::toggle_button}`
- Replace all 8+ indicator toggle buttons (MA7, MA25, MA99, BB, MACD, RSI, interval buttons) with `toggle_button()` calls + `.on_click()`
- Replace hardcoded `rgb(0x...)` colors with theme constants where applicable (backgrounds, borders, text)
- Note: chart-specific drawing colors (candle body, wick, MA line colors) should stay inline as they are chart-specific, not part of the shared theme

- [ ] **Step 2: Build to verify**

- [ ] **Step 3: Final full build + commit**

Run: `cargo build 2>&1 | head -20`
Expected: Clean compilation

```bash
git add -u
git commit -m "refactor: use reusable components across all views"
```

---

## Summary of Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| Hardcoded `rgb()` calls | ~60 | ~10 (chart-specific only) |
| Toggle button boilerplate | ~20 instances × 6 lines | ~20 instances × 1 line |
| Table cell boilerplate | ~32 instances × 4 lines | ~32 instances × 1 line |
| Input field wrappers | ~5 instances × 8 lines | ~5 instances × 1 line |
| Total estimated line reduction | — | ~200-300 lines removed |
| New component files | — | 7 files, ~150 lines total |
