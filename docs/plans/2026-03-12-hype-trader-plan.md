# Hype Trader Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a full-featured Hyperliquid DEX desktop trading client with real-time market data, order management, and candlestick charts.

**Architecture:** Three-layer architecture (UI / State / Service). GPUI Model system drives reactive updates. WebSocket feeds push into state models, which auto-trigger UI re-renders. All async work runs on tokio.

**Tech Stack:** Rust, GPUI 0.2, gpui-component 0.5, hyperliquid_rust_sdk 0.6, tokio, alloy (wallet signing), aes-gcm (key encryption), toml (config)

---

## Phase 1: Project Skeleton

### Task 1: Initialize Cargo project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `rust-toolchain.toml`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "hype-trader"
version = "0.1.0"
edition = "2021"

[dependencies]
gpui = "0.2"
gpui_platform = "0.2"
gpui-component = "0.5"
hyperliquid_rust_sdk = "0.6"
tokio = { version = "1", features = ["full"] }
alloy = { version = "1.0", features = ["dyn-abi", "sol-types", "signer-local"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
aes-gcm = "0.10"
sha2 = "0.10"
rand = "0.8"
base64 = "0.22"
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
dirs = "6"
tracing = "0.1"
tracing-subscriber = "0.3"

[profile.release]
opt-level = 3
```

**Step 2: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
```

**Step 3: Create minimal main.rs that opens a window**

```rust
use gpui::prelude::*;
use gpui::{App, WindowOptions, WindowBounds, Bounds, Point, Size};

struct HypeTrader;

impl Render for HypeTrader {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        gpui_component::Root::new(
            gpui::div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child("Hype Trader - Loading...")
        )
    }
}

fn main() {
    App::new().run(|cx| {
        gpui_component::init(cx);

        cx.open_window(
            WindowOptions {
                bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    gpui::size(gpui::px(1400.), gpui::px(900.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_window, cx| cx.new_view(|_cx| HypeTrader),
        )
        .unwrap();
    });
}
```

**Step 4: Build and verify window opens**

Run: `cargo build 2>&1 | tail -20`
Then: `cargo run` (verify window appears, then close it)

**Step 5: Commit**

```bash
git init && git add -A && git commit -m "feat: initialize hype-trader project skeleton"
```

---

## Phase 2: Core Types & State

### Task 2: Define core domain types

**Files:**
- Create: `src/types.rs`
- Modify: `src/main.rs` (add module)

**Step 1: Create src/types.rs with all domain types**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    pub fn base_url(&self) -> hyperliquid_rust_sdk::BaseUrl {
        match self {
            Network::Mainnet => hyperliquid_rust_sdk::BaseUrl::Mainnet,
            Network::Testnet => hyperliquid_rust_sdk::BaseUrl::Testnet,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Network::Mainnet => "Mainnet",
            Network::Testnet => "Testnet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
    TakeProfitStopLoss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    Gtc,
    Ioc,
    PostOnly,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub last_price: f64,
    pub change_24h: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
    pub total: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookData {
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub mid_price: f64,
}

impl Default for OrderBookData {
    fn default() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
            mid_price: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandleData {
    pub time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    pub coin: String,
    pub side: OrderSide,
    pub size: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
    pub leverage: f64,
}

#[derive(Debug, Clone)]
pub struct OpenOrder {
    pub oid: u64,
    pub coin: String,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub order_type: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub time: u64,
    pub coin: String,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub closed_pnl: f64,
}

#[derive(Debug, Clone)]
pub struct AccountSummary {
    pub total_balance: f64,
    pub available_balance: f64,
    pub margin_used: f64,
    pub total_pnl: f64,
}

impl Default for AccountSummary {
    fn default() -> Self {
        Self {
            total_balance: 0.0,
            available_balance: 0.0,
            margin_used: 0.0,
            total_pnl: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Positions,
    OpenOrders,
    TradeHistory,
    Funds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleInterval {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl CandleInterval {
    pub fn as_str(&self) -> &str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1d",
        }
    }

    pub fn all() -> &'static [CandleInterval] {
        &[Self::M1, Self::M5, Self::M15, Self::H1, Self::H4, Self::D1]
    }
}
```

**Step 2: Add module to main.rs**

Add `mod types;` at top of `src/main.rs`.

**Step 3: Build to verify**

Run: `cargo build 2>&1 | tail -10`

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add core domain types"
```

---

### Task 3: Define AppState

**Files:**
- Create: `src/state.rs`
- Modify: `src/main.rs` (add module)

**Step 1: Create src/state.rs**

```rust
use crate::types::*;

#[derive(Debug)]
pub struct AppState {
    pub network: Network,
    pub theme: ThemeChoice,
    pub connection_status: ConnectionStatus,
    pub wallet_address: Option<String>,

    // Market data
    pub symbols: Vec<SymbolInfo>,
    pub selected_symbol: String,
    pub orderbook: OrderBookData,
    pub candles: Vec<CandleData>,
    pub candle_interval: CandleInterval,

    // Account
    pub positions: Vec<PositionInfo>,
    pub open_orders: Vec<OpenOrder>,
    pub trade_history: Vec<TradeRecord>,
    pub account_summary: AccountSummary,

    // UI
    pub active_tab: BottomTab,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    pub order_price: String,
    pub order_size: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: Network::Testnet,
            theme: ThemeChoice::Dark,
            connection_status: ConnectionStatus::Disconnected,
            wallet_address: None,
            symbols: Vec::new(),
            selected_symbol: "ETH".to_string(),
            orderbook: OrderBookData::default(),
            candles: Vec::new(),
            candle_interval: CandleInterval::H1,
            positions: Vec::new(),
            open_orders: Vec::new(),
            trade_history: Vec::new(),
            account_summary: AccountSummary::default(),
            active_tab: BottomTab::Positions,
            order_side: OrderSide::Buy,
            order_type: OrderType::Limit,
            order_price: String::new(),
            order_size: String::new(),
        }
    }
}
```

**Step 2: Add `mod state;` to main.rs**

**Step 3: Build to verify**

Run: `cargo build 2>&1 | tail -10`

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add AppState with all trading state"
```

---

## Phase 3: Config & Wallet

### Task 4: Config file handling

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

**Step 1: Create src/config.rs**

```rust
use crate::types::{Network, ThemeChoice};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub default: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletConfig {
    pub encrypted_key: Option<String>,
    pub nonce: Option<String>,
    pub remember: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                default: "testnet".into(),
            },
            wallet: WalletConfig {
                encrypted_key: None,
                nonce: None,
                remember: false,
            },
            ui: UiConfig {
                theme: "dark".into(),
            },
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        let dir = home.join(".hype-trader");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        // Set file permissions to 600 on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn default_network(&self) -> Network {
        match self.network.default.as_str() {
            "mainnet" => Network::Mainnet,
            _ => Network::Testnet,
        }
    }

    pub fn theme(&self) -> ThemeChoice {
        match self.ui.theme.as_str() {
            "light" => ThemeChoice::Light,
            _ => ThemeChoice::Dark,
        }
    }

    /// Encrypt and store a private key using a password
    pub fn encrypt_and_store_key(&mut self, private_key: &str, password: &str) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let key_bytes = hasher.finalize();

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, private_key.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        self.wallet.encrypted_key = Some(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &ciphertext,
        ));
        self.wallet.nonce = Some(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &nonce_bytes,
        ));
        self.wallet.remember = true;
        Ok(())
    }

    /// Decrypt stored private key using password
    pub fn decrypt_key(&self, password: &str) -> Result<String> {
        let encrypted = self
            .wallet
            .encrypted_key
            .as_ref()
            .context("No stored key")?;
        let nonce_b64 = self.wallet.nonce.as_ref().context("No stored nonce")?;

        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let key_bytes = hasher.finalize();

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;
        let nonce_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            nonce_b64,
        )?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            encrypted,
        )?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow::anyhow!("Decryption failed (wrong password?): {}", e))?;

        Ok(String::from_utf8(plaintext)?)
    }
}
```

**Step 2: Add `mod config;` to main.rs**

**Step 3: Build to verify**

Run: `cargo build 2>&1 | tail -20`

Expected: May need to fix base64 import style. The `base64` crate v0.22 uses `use base64::prelude::*` then `BASE64_STANDARD.encode(...)` / `BASE64_STANDARD.decode(...)`. Fix as needed.

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add config file with encrypted key storage"
```

---

## Phase 4: Service Layer

### Task 5: Hyperliquid service wrapper

**Files:**
- Create: `src/services/mod.rs`
- Create: `src/services/market_service.rs`
- Create: `src/services/exchange_service.rs`
- Modify: `src/main.rs`

**Step 1: Create src/services/mod.rs**

```rust
pub mod market_service;
pub mod exchange_service;
```

**Step 2: Create src/services/market_service.rs**

Wraps `InfoClient` for market data queries and WebSocket subscriptions.

```rust
use crate::types::*;
use anyhow::Result;
use hyperliquid_rust_sdk::{BaseUrl, InfoClient, Message, Subscription};
use tokio::sync::mpsc;

pub struct MarketService {
    info_client: InfoClient,
    base_url: BaseUrl,
}

impl MarketService {
    pub async fn new(network: Network) -> Result<Self> {
        let base_url = network.base_url();
        let info_client = InfoClient::new(None, Some(base_url.clone())).await?;
        Ok(Self {
            info_client,
            base_url,
        })
    }

    pub async fn get_symbols(&self) -> Result<Vec<SymbolInfo>> {
        let meta = self.info_client.meta().await?;
        let mids = self.info_client.all_mids().await?;

        let symbols = meta
            .universe
            .iter()
            .map(|asset| {
                let price = mids
                    .get(&asset.name)
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);
                SymbolInfo {
                    name: asset.name.clone(),
                    last_price: price,
                    change_24h: 0.0, // Will be updated via WS
                }
            })
            .collect();

        Ok(symbols)
    }

    pub async fn get_orderbook(&self, coin: &str) -> Result<OrderBookData> {
        let snapshot = self.info_client.l2_snapshot(coin.to_string()).await?;

        let mut bids = Vec::new();
        let mut asks = Vec::new();

        if snapshot.levels.len() >= 2 {
            let mut bid_total = 0.0;
            for level in &snapshot.levels[0] {
                let price = level.px.parse::<f64>().unwrap_or(0.0);
                let size = level.sz.parse::<f64>().unwrap_or(0.0);
                bid_total += size;
                bids.push(OrderBookLevel {
                    price,
                    size,
                    total: bid_total,
                });
            }

            let mut ask_total = 0.0;
            for level in &snapshot.levels[1] {
                let price = level.px.parse::<f64>().unwrap_or(0.0);
                let size = level.sz.parse::<f64>().unwrap_or(0.0);
                ask_total += size;
                asks.push(OrderBookLevel {
                    price,
                    size,
                    total: ask_total,
                });
            }
        }

        let mid = if !bids.is_empty() && !asks.is_empty() {
            (bids[0].price + asks[0].price) / 2.0
        } else {
            0.0
        };

        Ok(OrderBookData {
            bids,
            asks,
            mid_price: mid,
        })
    }

    pub async fn get_candles(
        &self,
        coin: &str,
        interval: CandleInterval,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<CandleData>> {
        let candles = self
            .info_client
            .candles_snapshot(
                coin.to_string(),
                interval.as_str().to_string(),
                start_time,
                end_time,
            )
            .await?;

        Ok(candles
            .iter()
            .map(|c| CandleData {
                time: c.time_open,
                open: c.open.parse().unwrap_or(0.0),
                high: c.high.parse().unwrap_or(0.0),
                low: c.low.parse().unwrap_or(0.0),
                close: c.close.parse().unwrap_or(0.0),
                volume: c.volume.parse().unwrap_or(0.0),
            })
            .collect())
    }

    pub async fn subscribe_orderbook(
        &mut self,
        coin: &str,
    ) -> Result<mpsc::UnboundedReceiver<Message>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.info_client
            .subscribe(
                Subscription::L2Book {
                    coin: coin.to_string(),
                },
                tx,
            )
            .await?;
        Ok(rx)
    }

    pub async fn subscribe_trades(
        &mut self,
        coin: &str,
    ) -> Result<mpsc::UnboundedReceiver<Message>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.info_client
            .subscribe(
                Subscription::Trades {
                    coin: coin.to_string(),
                },
                tx,
            )
            .await?;
        Ok(rx)
    }

    pub async fn subscribe_candles(
        &mut self,
        coin: &str,
        interval: CandleInterval,
    ) -> Result<mpsc::UnboundedReceiver<Message>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.info_client
            .subscribe(
                Subscription::Candle {
                    coin: coin.to_string(),
                    interval: interval.as_str().to_string(),
                },
                tx,
            )
            .await?;
        Ok(rx)
    }
}
```

**Step 3: Create src/services/exchange_service.rs**

```rust
use crate::types::*;
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use hyperliquid_rust_sdk::{
    BaseUrl, ClientCancelRequest, ClientLimit, ClientOrder, ClientOrderRequest, ExchangeClient,
    ExchangeResponseStatus, InfoClient,
};

pub struct ExchangeService {
    exchange_client: Option<ExchangeClient>,
    info_client: InfoClient,
    address: Option<Address>,
}

impl ExchangeService {
    pub async fn new(network: Network) -> Result<Self> {
        let base_url = network.base_url();
        let info_client = InfoClient::new(None, Some(base_url)).await?;
        Ok(Self {
            exchange_client: None,
            info_client,
            address: None,
        })
    }

    pub async fn connect_wallet(
        &mut self,
        private_key: &str,
        network: Network,
    ) -> Result<String> {
        let wallet: PrivateKeySigner = private_key
            .trim_start_matches("0x")
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid private key format"))?;

        let address = wallet.address();
        let exchange_client =
            ExchangeClient::new(None, wallet, Some(network.base_url()), None, None).await?;

        self.exchange_client = Some(exchange_client);
        self.address = Some(address);

        Ok(format!("{}", address))
    }

    pub fn address_short(&self) -> Option<String> {
        self.address.map(|a| {
            let s = format!("{}", a);
            format!("{}...{}", &s[..6], &s[s.len() - 4..])
        })
    }

    pub async fn get_positions(&self) -> Result<Vec<PositionInfo>> {
        let address = self.address.ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let state = self.info_client.user_state(address).await?;

        Ok(state
            .asset_positions
            .iter()
            .filter_map(|ap| {
                let pos = &ap.position;
                let size: f64 = pos.szi.parse().ok()?;
                if size.abs() < 1e-10 {
                    return None;
                }
                Some(PositionInfo {
                    coin: pos.coin.clone(),
                    side: if size > 0.0 {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    },
                    size: size.abs(),
                    entry_price: pos.entry_px.as_ref()?.parse().ok()?,
                    mark_price: 0.0, // updated from market data
                    unrealized_pnl: pos.unrealized_pnl.parse().unwrap_or(0.0),
                    leverage: pos
                        .leverage
                        .as_ref()
                        .and_then(|l| l.value.parse().ok())
                        .unwrap_or(1.0),
                })
            })
            .collect())
    }

    pub async fn get_open_orders(&self) -> Result<Vec<OpenOrder>> {
        let address = self.address.ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let orders = self.info_client.open_orders(address).await?;

        Ok(orders
            .iter()
            .map(|o| OpenOrder {
                oid: o.oid,
                coin: o.coin.clone(),
                side: if o.side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                price: o.limit_px.parse().unwrap_or(0.0),
                size: o.sz.parse().unwrap_or(0.0),
                order_type: "Limit".to_string(),
                timestamp: o.timestamp,
            })
            .collect())
    }

    pub async fn get_trade_history(&self) -> Result<Vec<TradeRecord>> {
        let address = self.address.ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let fills = self.info_client.user_fills(address).await?;

        Ok(fills
            .iter()
            .map(|f| TradeRecord {
                time: f.time,
                coin: f.coin.clone(),
                side: if f.side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                price: f.px.parse().unwrap_or(0.0),
                size: f.sz.parse().unwrap_or(0.0),
                fee: f.fee.parse().unwrap_or(0.0),
                closed_pnl: f.closed_pnl.parse().unwrap_or(0.0),
            })
            .collect())
    }

    pub async fn get_account_summary(&self) -> Result<AccountSummary> {
        let address = self.address.ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let state = self.info_client.user_state(address).await?;

        let summary = &state.margin_summary;
        Ok(AccountSummary {
            total_balance: summary.account_value.parse().unwrap_or(0.0),
            available_balance: state.withdrawable.parse().unwrap_or(0.0),
            margin_used: summary.total_margin_used.parse().unwrap_or(0.0),
            total_pnl: summary.total_ntl_pos.parse().unwrap_or(0.0),
        })
    }

    pub async fn place_limit_order(
        &self,
        coin: &str,
        is_buy: bool,
        price: f64,
        size: f64,
        tif: TimeInForce,
    ) -> Result<String> {
        let client = self
            .exchange_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let tif_str = match tif {
            TimeInForce::Gtc => "Gtc",
            TimeInForce::Ioc => "Ioc",
            TimeInForce::PostOnly => "Alo",
        };

        let order = ClientOrderRequest {
            asset: coin.to_string(),
            is_buy,
            reduce_only: false,
            limit_px: price,
            sz: size,
            cloid: None,
            order_type: ClientOrder::Limit(ClientLimit {
                tif: tif_str.to_string(),
            }),
        };

        let response = client.order(order, None).await?;
        match response {
            ExchangeResponseStatus::Ok(resp) => Ok(format!("Order placed: {:?}", resp.data)),
            ExchangeResponseStatus::Err(e) => Err(anyhow::anyhow!("Order failed: {}", e)),
        }
    }

    pub async fn cancel_order(&self, coin: &str, oid: u64) -> Result<String> {
        let client = self
            .exchange_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let cancel = ClientCancelRequest {
            asset: coin.to_string(),
            oid,
        };

        let response = client.cancel(cancel, None).await?;
        match response {
            ExchangeResponseStatus::Ok(_) => Ok("Order cancelled".to_string()),
            ExchangeResponseStatus::Err(e) => Err(anyhow::anyhow!("Cancel failed: {}", e)),
        }
    }
}
```

**Step 4: Add `mod services;` to main.rs**

**Step 5: Build to verify**

Run: `cargo build 2>&1 | tail -20`

Fix any type mismatches between SDK types and our types. The SDK types may vary slightly from what was documented — adapt field names as the compiler directs.

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add market and exchange service layers"
```

---

## Phase 5: UI - Welcome / Login View

### Task 6: Welcome view with wallet connection

**Files:**
- Create: `src/views/mod.rs`
- Create: `src/views/welcome.rs`
- Modify: `src/main.rs`

**Step 1: Create src/views/mod.rs**

```rust
pub mod welcome;
```

**Step 2: Create src/views/welcome.rs**

Build the welcome/login screen with network selection, private key input, and connect button.

```rust
use crate::types::Network;
use gpui::prelude::*;
use gpui_component::prelude::*;
use gpui_component::{button::Button, input::TextInput};

pub enum WelcomeEvent {
    Connect {
        network: Network,
        private_key: String,
    },
    ReadOnlyMode {
        network: Network,
    },
}

pub struct WelcomeView {
    network: Network,
    private_key_input: View<TextInput>,
    error_message: Option<String>,
    connecting: bool,
}

impl WelcomeView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let private_key_input = cx.new_view(|cx| {
            TextInput::new(cx)
                .placeholder("Enter private key (hex)")
                .masked(true)
        });

        Self {
            network: Network::Testnet,
            private_key_input,
            error_message: None,
            connecting: false,
        }
    }

    fn on_connect(&mut self, _: &gpui::ClickEvent, cx: &mut ViewContext<Self>) {
        let key = self.private_key_input.read(cx).text().to_string();
        if key.is_empty() {
            self.error_message = Some("Please enter a private key".into());
            cx.notify();
            return;
        }
        self.connecting = true;
        self.error_message = None;
        cx.emit(WelcomeEvent::Connect {
            network: self.network,
            private_key: key,
        });
        cx.notify();
    }

    fn on_read_only(&mut self, _: &gpui::ClickEvent, cx: &mut ViewContext<Self>) {
        cx.emit(WelcomeEvent::ReadOnlyMode {
            network: self.network,
        });
    }

    fn toggle_network(&mut self, _: &gpui::ClickEvent, cx: &mut ViewContext<Self>) {
        self.network = match self.network {
            Network::Mainnet => Network::Testnet,
            Network::Testnet => Network::Mainnet,
        };
        cx.notify();
    }
}

impl EventEmitter<WelcomeEvent> for WelcomeView {}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let network_label = self.network.display_name();

        gpui::div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            // Title
            .child(
                gpui::div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Hype Trader"),
            )
            // Network selector
            .child(
                Button::new("network-toggle")
                    .label(format!("Network: {}", network_label))
                    .on_click(cx.listener(Self::toggle_network)),
            )
            // Private key input
            .child(
                gpui::div()
                    .w(gpui::px(400.))
                    .child(self.private_key_input.clone()),
            )
            // Error message
            .children(self.error_message.as_ref().map(|msg| {
                gpui::div()
                    .text_color(gpui::red())
                    .child(msg.clone())
            }))
            // Connect button
            .child(
                Button::new("connect")
                    .label(if self.connecting {
                        "Connecting..."
                    } else {
                        "Connect Wallet"
                    })
                    .primary()
                    .disabled(self.connecting)
                    .on_click(cx.listener(Self::on_connect)),
            )
            // Read-only mode
            .child(
                Button::new("read-only")
                    .label("Read-only Mode (View Market Only)")
                    .ghost()
                    .on_click(cx.listener(Self::on_read_only)),
            )
    }
}
```

**Step 3: Add `mod views;` to main.rs and update HypeTrader to show WelcomeView**

**Step 4: Build and verify**

Run: `cargo build 2>&1 | tail -20`

Note: The exact gpui-component API for TextInput/Button may differ slightly. Adjust imports and method names based on compiler errors. Key things to check:
- `TextInput` vs `Input` naming
- How masked/password mode is set
- Button builder API

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add welcome view with wallet connection UI"
```

---

## Phase 6: UI - Main Trading View Shell

### Task 7: Main layout with resizable panels

**Files:**
- Create: `src/views/trading.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/main.rs`

**Step 1: Create src/views/trading.rs**

Build the main trading view shell with the top bar, three-column layout, and bottom tabs.

```rust
use crate::state::AppState;
use crate::types::*;
use gpui::prelude::*;
use gpui_component::prelude::*;
use gpui_component::button::Button;

pub struct TradingView {
    state: gpui::Model<AppState>,
}

impl TradingView {
    pub fn new(state: gpui::Model<AppState>, cx: &mut ViewContext<Self>) -> Self {
        Self { state }
    }

    fn render_top_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let network_label = state.network.display_name();
        let address = state
            .wallet_address
            .as_deref()
            .unwrap_or("Not connected");
        let balance = format!("{:.2} USDC", state.account_summary.total_balance);

        gpui::div()
            .w_full()
            .h(gpui::px(48.))
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .border_b_1()
            .border_color(gpui::rgb(0x333333))
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        gpui::div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Hype Trader"),
                    )
                    .child(
                        Button::new("network")
                            .label(network_label)
                            .sm(),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(balance)
                    .child(address.to_string())
                    .child(
                        Button::new("theme-toggle")
                            .label("Theme")
                            .ghost()
                            .sm()
                            .on_click(cx.listener(|this, _, cx| {
                                this.state.update(cx, |s, _| {
                                    s.theme = match s.theme {
                                        ThemeChoice::Dark => ThemeChoice::Light,
                                        ThemeChoice::Light => ThemeChoice::Dark,
                                    };
                                });
                                cx.notify();
                            })),
                    ),
            )
    }

    fn render_bottom_tabs(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let active = state.active_tab;

        let tab_btn = |id: &str, label: &str, tab: BottomTab, cx: &mut ViewContext<Self>| {
            let is_active = active == tab;
            Button::new(id)
                .label(label)
                .sm()
                .when(is_active, |b| b.primary())
                .when(!is_active, |b| b.ghost())
                .on_click(cx.listener(move |this, _, cx| {
                    this.state.update(cx, |s, _| s.active_tab = tab);
                    cx.notify();
                }))
        };

        gpui::div()
            .w_full()
            .flex()
            .gap_2()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(gpui::rgb(0x333333))
            .child(tab_btn("tab-pos", "Positions", BottomTab::Positions, cx))
            .child(tab_btn("tab-orders", "Orders", BottomTab::OpenOrders, cx))
            .child(tab_btn("tab-history", "History", BottomTab::TradeHistory, cx))
            .child(tab_btn("tab-funds", "Funds", BottomTab::Funds, cx))
    }

    fn render_bottom_content(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        match state.active_tab {
            BottomTab::Positions => self.render_positions_table(cx),
            BottomTab::OpenOrders => self.render_orders_table(cx),
            BottomTab::TradeHistory => self.render_history_table(cx),
            BottomTab::Funds => self.render_funds_view(cx),
        }
    }

    fn render_positions_table(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        gpui::div()
            .size_full()
            .p_4()
            .child(if state.positions.is_empty() {
                "No open positions".to_string()
            } else {
                format!("{} positions", state.positions.len())
            })
    }

    fn render_orders_table(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        gpui::div()
            .size_full()
            .p_4()
            .child(if state.open_orders.is_empty() {
                "No open orders".to_string()
            } else {
                format!("{} orders", state.open_orders.len())
            })
    }

    fn render_history_table(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        gpui::div()
            .size_full()
            .p_4()
            .child(if state.trade_history.is_empty() {
                "No trade history".to_string()
            } else {
                format!("{} trades", state.trade_history.len())
            })
    }

    fn render_funds_view(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let s = &state.account_summary;
        gpui::div()
            .size_full()
            .p_4()
            .flex()
            .gap_8()
            .child(format!("Total: {:.2}", s.total_balance))
            .child(format!("Available: {:.2}", s.available_balance))
            .child(format!("Margin: {:.2}", s.margin_used))
            .child(format!("PnL: {:.2}", s.total_pnl))
    }
}

impl Render for TradingView {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        gpui::div()
            .size_full()
            .flex()
            .flex_col()
            // Top bar
            .child(self.render_top_bar(cx))
            // Main content area (3 columns)
            .child(
                gpui::div()
                    .flex_1()
                    .flex()
                    // Left: Symbol list
                    .child(
                        gpui::div()
                            .w(gpui::px(200.))
                            .h_full()
                            .border_r_1()
                            .border_color(gpui::rgb(0x333333))
                            .p_2()
                            .child("Symbol List (TODO)"),
                    )
                    // Center: Chart + Order panel
                    .child(
                        gpui::div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(
                                gpui::div()
                                    .flex_1()
                                    .p_2()
                                    .child("Candle Chart (TODO)"),
                            )
                            .child(
                                gpui::div()
                                    .h(gpui::px(200.))
                                    .border_t_1()
                                    .border_color(gpui::rgb(0x333333))
                                    .p_2()
                                    .child("Order Panel (TODO)"),
                            ),
                    )
                    // Right: Order book
                    .child(
                        gpui::div()
                            .w(gpui::px(280.))
                            .h_full()
                            .border_l_1()
                            .border_color(gpui::rgb(0x333333))
                            .p_2()
                            .child("Order Book (TODO)"),
                    ),
            )
            // Bottom tabs
            .child(self.render_bottom_tabs(cx))
            // Bottom content
            .child(
                gpui::div()
                    .h(gpui::px(200.))
                    .child(self.render_bottom_content(cx)),
            )
    }
}
```

**Step 2: Add `pub mod trading;` to views/mod.rs**

**Step 3: Update main.rs to switch between WelcomeView and TradingView**

Wire up view switching: start with WelcomeView, on connect event switch to TradingView.

**Step 4: Build and verify**

Run: `cargo build 2>&1 | tail -20`

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add main trading view layout shell"
```

---

## Phase 7: Individual UI Panels

### Task 8: Symbol list panel

**Files:**
- Create: `src/views/symbol_list.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/trading.rs`

Build a searchable symbol list that displays coin names, prices, and 24h change. Uses gpui-component's list/virtual list. Clicking a symbol updates `selected_symbol` in AppState.

**Step 1: Create src/views/symbol_list.rs with search input + filtered virtual list**

**Step 2: Integrate into TradingView left column**

**Step 3: Build and verify**

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add symbol list panel with search"
```

---

### Task 9: Order book panel

**Files:**
- Create: `src/views/orderbook.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/trading.rs`

Build the order book showing asks (red, top), mid price, bids (green, bottom). Each row has price, size, total with horizontal bar background showing depth.

**Step 1: Create src/views/orderbook.rs with bid/ask rendering**

**Step 2: Integrate into TradingView right column**

**Step 3: Build and verify**

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add order book panel"
```

---

### Task 10: Order panel (place orders)

**Files:**
- Create: `src/views/order_panel.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/trading.rs`

Build the order entry form with: order type tabs (Limit/Market), buy/sell toggle, price input, size input, percentage slider, and submit button.

**Step 1: Create src/views/order_panel.rs with form inputs**

**Step 2: Integrate into TradingView center-bottom area**

**Step 3: Build and verify**

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add order entry panel"
```

---

### Task 11: Candlestick chart

**Files:**
- Create: `src/views/candle_chart.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/trading.rs`

Build the candlestick chart using GPUI Canvas. Includes:
- Interval toolbar (1m, 5m, 15m, 1h, 4h, 1d)
- Candlestick rendering (green bullish, red bearish)
- Volume bars below
- Y-axis price scale, X-axis time scale
- Mouse crosshair with OHLCV tooltip
- Scroll to zoom, drag to pan

This is the most complex component. Use gpui's `canvas()` element for custom drawing.

**Step 1: Create src/views/candle_chart.rs with interval toolbar + canvas rendering**

**Step 2: Implement candle drawing logic (body rect + wicks)**

**Step 3: Implement volume bars**

**Step 4: Implement zoom/pan with mouse events**

**Step 5: Implement crosshair overlay**

**Step 6: Integrate into TradingView center area**

**Step 7: Build and verify**

**Step 8: Commit**

```bash
git add -A && git commit -m "feat: add candlestick chart with zoom/pan"
```

---

### Task 12: Bottom panel tables (positions, orders, history, funds)

**Files:**
- Create: `src/views/bottom_panel.rs`
- Modify: `src/views/mod.rs`
- Modify: `src/views/trading.rs`

Replace placeholder text with proper tables for each tab. Use gpui-component Table with sortable columns.

**Step 1: Create positions table with close button**

**Step 2: Create open orders table with cancel button**

**Step 3: Create trade history table**

**Step 4: Create funds summary view**

**Step 5: Build and verify**

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add bottom panel tables for positions/orders/history/funds"
```

---

## Phase 8: Wire Up Data Flow

### Task 13: Connect services to UI

**Files:**
- Modify: `src/main.rs`
- Modify: `src/views/trading.rs`

**Step 1: On WelcomeView connect event, initialize MarketService and ExchangeService**

**Step 2: Fetch initial data (symbols, orderbook, candles, positions, orders) and populate AppState**

**Step 3: Start WebSocket subscriptions and spawn tokio tasks to push updates into AppState models**

**Step 4: Verify data flows from SDK -> State -> UI**

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: wire up data flow from SDK to UI"
```

---

### Task 14: Order submission and cancellation

**Files:**
- Modify: `src/views/order_panel.rs`
- Modify: `src/views/bottom_panel.rs`

**Step 1: Wire order panel submit to ExchangeService.place_limit_order**

**Step 2: Wire cancel buttons in orders table to ExchangeService.cancel_order**

**Step 3: Wire close buttons in positions table to market close**

**Step 4: Show success/error notifications**

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: wire up order placement and cancellation"
```

---

## Phase 9: Theme & Polish

### Task 15: Dark/Light theme support

**Files:**
- Modify: `src/main.rs`
- Modify: all view files to use theme colors

**Step 1: Set initial theme from config on startup (ThemeMode::Dark or Light)**

**Step 2: Wire theme toggle button to switch gpui-component theme via Theme::global_mut**

**Step 3: Ensure custom colors (orderbook red/green, chart candles) adapt to theme**

**Step 4: Persist theme choice to config file**

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add dark/light theme switching"
```

---

### Task 16: Network switching

**Files:**
- Modify: `src/views/trading.rs`
- Modify: `src/main.rs`

**Step 1: Add confirmation dialog on network switch**

**Step 2: Disconnect current services, re-initialize with new network**

**Step 3: Re-fetch all data and re-subscribe WebSockets**

**Step 4: Persist network choice to config**

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add network switching with confirmation"
```

---

### Task 17: Final polish and error handling

**Files:**
- All files

**Step 1: Add loading states (spinner on data fetch)**

**Step 2: Add error toasts for failed operations**

**Step 3: Add reconnection logic for WebSocket disconnects**

**Step 4: Test full flow: login -> view market -> place order -> cancel -> switch network**

**Step 5: Final commit**

```bash
git add -A && git commit -m "feat: polish UI, add error handling and reconnection"
```

---

## Execution Notes

- **Build frequently**: Run `cargo build` after every file change to catch type errors early.
- **SDK types may differ**: The hyperliquid SDK types (field names, method signatures) may differ slightly from what's documented. Always follow compiler errors.
- **gpui-component API**: The exact API may differ from research. Check `cargo doc --open` after adding the dependency to see actual available types.
- **Start simple**: Get each panel rendering with static/mock data first, then wire up real data.
- **Candle chart is hardest**: Task 11 will likely take the most iteration. Start with basic rendering before adding zoom/pan/crosshair.
