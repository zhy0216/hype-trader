# Hype Trader Completion Roadmap

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the Hype Trader from "all services work" to a fully functional, production-ready desktop trading client.

**Architecture:** Three-layer architecture (UI → State → Services) is already in place. All Hyperliquid SDK services are implemented. Work focuses on wiring flows together, adding real-time updates, hardening the network layer, and polishing UX.

**Tech Stack:** Rust, GPUI 0.2, gpui-component 0.5, hyperliquid_rust_sdk 0.6, ethers 2, tokio

---

## Priority Overview

| Phase | Priority | Description | Effort |
|-------|----------|-------------|--------|
| P0 | Critical | Login flow + Config persistence | Medium |
| P1 | Critical | Order submission + action wiring | Medium |
| P2 | High | WebSocket real-time data pipeline | Large |
| P3 | High | Network resilience (reconnect, error states) | Medium |
| P4 | Medium | Symbol switching data reload | Medium |
| P5 | Medium | UI feedback (loading, toast, error) | Medium |
| P6 | Low | Technical indicators on chart | Large |
| P7 | Low | Settings dialog | Small |
| P8 | Low | Test coverage | Medium |

---

## Phase P0: Login Flow + Config Persistence

**Why first:** Without this, the app can't authenticate users or remember wallets. Everything else depends on having a wallet connected.

### Task P0.1: Wire WelcomeView → MainView routing

**Files:**
- Modify: `src/main.rs`
- Modify: `src/views/main_view.rs`
- Modify: `src/views/welcome_view.rs`

**What to do:**

1. Add an `AppScreen` enum (`Welcome | Trading`) to `main.rs` or a new `src/app.rs`.
2. `HypeTrader` holds the current screen state and conditionally renders `WelcomeView` or `MainView`.
3. `WelcomeView.connect` button: validate private key via `WalletService::address_from_key()`, on success emit a callback/event that `HypeTrader` listens to, switching screen to `Trading` and passing the key + network to `MainView`.
4. "Browse Market (Read-only)" button: switch to `MainView` with no wallet (read-only mode, hide OrderPanel).
5. `MainView::new()` accepts `Option<String>` (private_key) and `Network` parameters instead of hardcoded `Network::Mainnet`.

### Task P0.2: Config load on startup, save on connect

**Files:**
- Modify: `src/main.rs`
- Modify: `src/views/welcome_view.rs`
- Modify: `src/services/config_service.rs`

**What to do:**

1. On app launch, call `config_service::load_config()`. If config has an encrypted wallet key, pre-populate WelcomeView or auto-connect.
2. On successful connect with "Remember" checked: encrypt key via `WalletService::encrypt_key()`, save to config via `save_config()`.
3. On disconnect/logout: if user unchecks remember, clear the wallet from config and re-save.

---

## Phase P1: Order Submission + Action Wiring

**Why second:** The OrderPanel submit button and BottomPanel close/cancel buttons currently do nothing. This makes the app actually usable for trading.

### Task P1.1: Wire OrderPanel submit to ExchangeService

**Files:**
- Modify: `src/views/order_panel.rs`
- Modify: `src/views/main_view.rs`

**What to do:**

1. `OrderPanel` needs access to the private key and network (passed from MainView or via a shared state handle).
2. On "Buy/Long" or "Sell/Short" click: read `price_input` and `size_input` values, parse to f64.
3. Based on `order_type`, call `ExchangeService::place_limit_order()` / `place_market_order()` / `place_trigger_order()` in a `cx.spawn()` async block.
4. On success: clear the form inputs.
5. On error: display error (see P5 for toast, initially just `tracing::error!`).

### Task P1.2: Wire BottomPanel close/cancel buttons

**Files:**
- Modify: `src/views/bottom_panel.rs`
- Modify: `src/views/main_view.rs`

**What to do:**

1. Positions tab "Close" button: call `ExchangeService::market_close(coin, size)` on click.
2. Open Orders tab "Cancel" button: call `ExchangeService::cancel_order(coin, oid)` on click.
3. After success, re-fetch positions/orders via `InfoService` and update the panel data.
4. Both need access to the wallet (private key) — pass via callback or shared handle.

### Task P1.3: Wire percentage buttons in OrderPanel

**Files:**
- Modify: `src/views/order_panel.rs`

**What to do:**

1. The 25%/50%/75%/100% buttons should calculate size based on available balance.
2. On click: `available_balance * pct / current_price` → set `size_input` value.
3. Requires passing available balance info from MainView/BottomPanel to OrderPanel.

---

## Phase P2: WebSocket Real-Time Data Pipeline

**Why third:** Currently the app loads data once at startup. Without real-time updates, orderbook/chart/positions are stale. The WsService is fully implemented but not connected to the UI.

### Task P2.1: Start WebSocket subscriptions after login

**Files:**
- Modify: `src/views/main_view.rs`

**What to do:**

1. After `InfoService` fetches initial data successfully, create `WsService::new(network)`.
2. Create an `mpsc::unbounded_channel::<WsUpdate>()`.
3. Subscribe to: `l2_book` (selected symbol), `trades` (selected symbol), `candles` (selected symbol + interval), `all_mids`.
4. If wallet connected: also subscribe to `order_updates` and `user_fills`.
5. Spawn a GPUI task that polls the receiver and dispatches updates to the relevant UI entities.

### Task P2.2: Route WsUpdate messages to UI components

**Files:**
- Modify: `src/views/main_view.rs`

**What to do:**

1. In the GPUI polling task:
   - `WsUpdate::OrderBookUpdate(book)` → `cx.update_entity(&order_book, |v, _| v.data = book)`
   - `WsUpdate::CandleUpdate(candle)` → update or append to `candle_chart.candles`
   - `WsUpdate::TradesUpdate(trades)` → update `recent_trades` display (if added)
   - `WsUpdate::AllMids(mids)` → update `symbol_list` prices
   - `WsUpdate::OrderUpdate(_)` → re-fetch open orders
   - `WsUpdate::UserFill(fill)` → prepend to `bottom_panel.trade_history`

---

## Phase P3: Network Resilience

**Why fourth:** Once we have live WebSocket connections, we need to handle disconnects gracefully. Without this, the app silently stops updating.

### Task P3.1: Connection status tracking

**Files:**
- Modify: `src/views/main_view.rs`
- Modify: `src/views/top_bar.rs`

**What to do:**

1. Track `ConnectionStatus` properly: set `Connecting` before `InfoService::new()`, `Connected` on success, `Disconnected` on failure.
2. TopBar already renders the status — just make sure it reflects the real state.
3. When `Disconnected`: show a reconnect button in TopBar or auto-reconnect after delay.

### Task P3.2: WebSocket reconnect logic

**Files:**
- Modify: `src/views/main_view.rs`
- Modify: `src/services/ws_service.rs`

**What to do:**

1. Detect when the WsService receiver channel closes (all senders dropped → WebSocket disconnected).
2. On disconnect: set status to `Disconnected`, wait 3 seconds, attempt to re-create WsService and re-subscribe.
3. Exponential backoff: 3s → 6s → 12s → max 30s between retries.
4. `WsService` already uses `InfoClient::with_reconnect()` — verify this handles TCP-level reconnects. If not, add a `reconnect()` method.

### Task P3.3: Service call error handling

**Files:**
- Modify: `src/views/main_view.rs`
- Modify: `src/views/order_panel.rs`
- Modify: `src/views/bottom_panel.rs`

**What to do:**

1. Wrap all `InfoService` / `ExchangeService` calls in proper error handling.
2. On transient errors (network timeout): retry once after 1s.
3. On permanent errors (invalid order): display to user (see P5).
4. Never silently drop errors — at minimum `tracing::error!`.

---

## Phase P4: Symbol Switching Data Reload

**Why fifth:** Currently changing the selected symbol doesn't reload data. Users see stale data when clicking a different pair.

### Task P4.1: SymbolList selection triggers data reload

**Files:**
- Modify: `src/views/symbol_list.rs`
- Modify: `src/views/main_view.rs`

**What to do:**

1. `SymbolList` click handler emits a callback with the selected symbol name.
2. `MainView` receives it and:
   a. Unsubscribe current WebSocket subs (`ws.unsubscribe_all()`).
   b. Re-fetch orderbook + candles for new symbol via `InfoService`.
   c. Re-subscribe WebSocket for new symbol's l2_book, trades, candles.
   d. Update `OrderPanel.symbol`.

---

## Phase P5: UI Feedback (Loading, Toast, Errors)

**Why sixth:** Without loading states and error messages, users don't know what's happening.

### Task P5.1: Loading state

**Files:**
- Modify: `src/views/main_view.rs`

**What to do:**

1. Add an `is_loading: bool` flag. Set `true` before async fetches, `false` after.
2. While loading: overlay a simple "Loading..." text centered over the content area.
3. Individual components can also show "Loading..." in their empty states.

### Task P5.2: Error/success toast

**Files:**
- Create: `src/views/toast.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/main_view.rs`

**What to do:**

1. Simple `Toast` struct: `message: String, kind: ToastKind (Success|Error|Info), visible: bool`.
2. Show as a small colored bar at the top of MainView, auto-hide after 3 seconds.
3. Use for: "Order placed", "Order cancelled", "Connection lost", "Failed to place order: ..."

---

## Phase P6: Technical Indicators on Chart

**Why seventh:** Nice-to-have but not critical for basic trading functionality.

### Task P6.1: Moving Average (MA / EMA)

**Files:**
- Modify: `src/views/candle_chart.rs`

**What to do:**

1. Calculate MA(7), MA(25), MA(99) from candle close prices.
2. Render as colored lines overlaid on the candlestick chart.
3. Add toggle buttons to show/hide each indicator.

### Task P6.2: Volume-weighted indicators

**Files:**
- Modify: `src/views/candle_chart.rs`

**What to do:**

1. Bollinger Bands (20-period, 2 std dev): upper/lower/middle lines.
2. MACD (12, 26, 9): separate sub-chart below volume.
3. RSI (14-period): separate sub-chart.

### Task P6.3: Chart interaction

**Files:**
- Modify: `src/views/candle_chart.rs`

**What to do:**

1. Mouse wheel → zoom in/out (adjust visible candle range).
2. Click-drag → pan left/right.
3. Crosshair on hover → show OHLCV tooltip.

---

## Phase P7: Settings Dialog

**Why eighth:** Low priority — most settings (network, theme) are already in TopBar.

### Task P7.1: Settings modal

**Files:**
- Create: `src/views/settings.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/top_bar.rs`

**What to do:**

1. Modal dialog with: Network selection, Theme selection, Default leverage, Wallet management (disconnect / change key).
2. Save settings to config on close.
3. TopBar settings button opens this modal.

---

## Phase P8: Test Coverage

**Why last:** The app works — tests prevent regressions as we iterate.

### Task P8.1: Service layer unit tests

**Files:**
- Create: `src/services/info_service_test.rs` or inline `#[cfg(test)]` modules
- Create: `src/services/exchange_service_test.rs`

**What to do:**

1. InfoService: mock the SDK client, test String→f64 parsing, test symbol name conversion (ETH ↔ ETH-USD).
2. ExchangeService: test order parameter construction.
3. ConfigService: test save/load roundtrip with temp directory.

### Task P8.2: Model tests

**Files:**
- Modify: `src/models.rs` (add `#[cfg(test)]` module)

**What to do:**

1. Test `CandleInterval::to_sdk_string()` for all variants.
2. Test `AppConfig` serialization/deserialization roundtrip.
3. Test `OrderFormState::default()` values.

---

## Execution Summary

```
P0 (Login flow)       ██░░░░░░░░  Must-have, unlocks user auth
P1 (Order wiring)     ██░░░░░░░░  Must-have, unlocks actual trading
P2 (WebSocket pipe)   ███░░░░░░░  High, unlocks real-time data
P3 (Network resil.)   ██░░░░░░░░  High, prevents silent failures
P4 (Symbol switch)    ██░░░░░░░░  Medium, basic UX expectation
P5 (UI feedback)      ██░░░░░░░░  Medium, user knows what's happening
P6 (Indicators)       ████░░░░░░  Low, nice-to-have for traders
P7 (Settings)         █░░░░░░░░░  Low, most config already works
P8 (Tests)            ██░░░░░░░░  Low, but prevents regressions
```

**Recommended approach:** Execute P0 → P1 → P2 → P3 sequentially (they build on each other). P4/P5 can be parallelized. P6/P7/P8 are independent and can be done in any order.
