# Hype Trader - Hyperliquid Desktop Trading Client

## Overview

A full-featured desktop trading client for Hyperliquid DEX, built with Rust + GPUI + gpui-component. Connects via the official `hyperliquid_rust_sdk`.

## Tech Stack

- **UI**: GPUI + gpui-component (60+ components)
- **API**: hyperliquid_rust_sdk (REST + WebSocket)
- **Async**: tokio
- **Crypto**: AES-256 for local key encryption
- **Language**: Rust

## Architecture

Three layers:

1. **UI Layer** - gpui-component views, pure presentation + user interaction
2. **State Layer** - Global AppState via GPUI Model, drives reactive UI updates
3. **Service Layer** - SDK calls, WebSocket subscriptions, background tasks

## Main Window Layout

```
+---------------------------------------------+
|  TopBar: Network | Account | Theme | Settings |
+--------+--------------------+---------------+
|        |                    |               |
| Symbol |   CandleChart      |  OrderBook    |
| List   |                    |  (bid/ask)    |
|        +--------------------+               |
|        |  OrderPanel         |               |
|        |  (limit/market/TP-SL)|              |
+--------+--------------------+---------------+
|  Tabs: Positions | Orders | History | Funds  |
+---------------------------------------------+
```

## State Design

```
AppState
├── network: Mainnet | Testnet
├── theme: Dark | Light
├── wallet: Option<WalletConfig>
├── connection_status: Connected | Connecting | Disconnected
├── market_data
│   ├── symbols: Vec<Symbol>
│   ├── selected_symbol: String
│   ├── orderbook: OrderBook
│   ├── trades: Vec<Trade>
│   └── candles: Vec<Candle>
├── account
│   ├── balances: Vec<Balance>
│   ├── positions: Vec<Position>
│   ├── open_orders: Vec<Order>
│   └── pnl: PnlSummary
└── ui_state
    ├── active_tab: BottomTab
    └── order_form: OrderFormState
```

## Service Modules

| Module | Responsibility |
|--------|---------------|
| `info_service` | Market queries, symbol info, candle data |
| `exchange_service` | Place/cancel/modify orders |
| `ws_service` | WebSocket: orderbook, trades, user events |
| `wallet_service` | Key loading, signing, network switching |

## Data Flow

- WebSocket push -> update AppState Model -> GPUI auto re-renders
- User places order -> OrderForm collects params -> exchange_service async submit -> result written to AppState
- Network switch -> disconnect current -> re-init SDK client -> re-subscribe

## Panel Details

### TopBar
- Left: App logo + name
- Center: Network dropdown (Mainnet/Testnet) with status indicator (green=connected)
- Right: Balance summary | Theme toggle | Settings gear

### SymbolList
- Search filter input
- VirtualList for all trading pairs
- Each row: pair name | last price | 24h change (red/green)
- Click to switch active pair

### OrderBook
- Top half: asks (red, price low to high)
- Center: current price (large, prominent)
- Bottom half: bids (green, price high to low)
- Each row: price | quantity | cumulative (horizontal bar background)

### CandleChart

Canvas structure:
```
+-------------------------------+
| Toolbar: 1m|5m|15m|1h|4h|1d | Indicators |
+-------------------------------+
|                               | Price axis
|  Candlestick main chart       | (Y-axis)
|  (overlay MA/EMA/BOLL)        |
|                               |
+-------------------------------+
|  Volume bars (sub-chart 1)    |
+-------------------------------+
|  Optional indicator (MACD/RSI/KDJ) |
+-------------------------------+
|  Time axis (X-axis)           |
+-------------------------------+
```

- Render with GPUI Canvas low-level drawing API
- Each candle: body rect (open/close) + upper/lower wicks (high/low)
- Green fill for bullish, red fill for bearish (theme-aware)
- Mouse wheel zoom, drag to pan, crosshair cursor with OHLCV tooltip
- Built-in indicators: MA(7/25/99), EMA, Bollinger Bands, MACD, RSI, KDJ
- Local cache for loaded candle data, auto-load history on scroll left
- WebSocket real-time update for latest candle

### OrderPanel
- Tabs: Limit | Market | TP/SL
- Buy/Sell toggle (green/red)
- Inputs: price, quantity, position % slider (25/50/75/100%)
- Estimated cost/return display
- Confirm button

### BottomPanel (Tabs)
- **Positions**: pair | side | size | entry price | mark price | unrealized PnL | close button
- **Open Orders**: pair | side | price | size | type | cancel button
- **Trade History**: time | pair | side | price | size | fee
- **Funds**: total balance | available | margin used | total PnL

All tables use gpui-component Table with sorting support.

## Login / Wallet Connection

### Startup Flow

```
App start
  |
  +- Check config file (~/.hype-trader/config.toml)
  |   +- Exists with key -> auto connect -> main view
  |   +- Missing -> show welcome page
  |
  +- Welcome page
      +- Option 1: Enter private key
      +- Option 2: Load from config file
      +- Option 3: Read-only mode (view market only)
```

### Security
- Private key input masked (password field)
- Key stored in memory only, unless user checks "Remember wallet"
- If remembered: AES-256 encrypted, written to config file
- Unlock password required on each startup
- Config file permission: 600

### Config File (`~/.hype-trader/config.toml`)

```toml
[network]
default = "mainnet"

[wallet]
encrypted_key = "..."
remember = true

[ui]
theme = "dark"
```

### Connection States
- Connecting -> loading spinner
- Connected -> main view, address shown as 0x1234...abcd
- Failed -> error toast, retry option
- Disconnected -> auto-retry 3 times with backoff, then manual reconnect prompt

### In-Session Switching
- Click address in TopBar -> dropdown: view address | switch network | disconnect
- Network switch shows confirmation dialog

## Theme Support
- Dark theme (default): dark backgrounds, standard trading color scheme
- Light theme: light backgrounds, adjusted colors
- Toggle via TopBar button
- Persisted in config file
