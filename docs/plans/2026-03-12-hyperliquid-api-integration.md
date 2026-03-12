# Hyperliquid API Integration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace all mock data with real Hyperliquid API calls using `hyperliquid_rust_sdk`, enabling live market data, account queries, trading, and real-time WebSocket updates.

**Architecture:** Services wrap `hyperliquid_rust_sdk` clients (`InfoClient` for market+account data, `ExchangeClient` for trading). SDK types (String-based) are converted to app models (f64-based) in the service layer. WebSocket subscriptions push real-time updates to the UI via GPUI's entity system.

**Tech Stack:** Rust, hyperliquid_rust_sdk 0.6, ethers 2 (wallet), tokio (async), GPUI (UI)

---

### Task 1: Update Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update Cargo.toml**

Add `ethers` dependency, remove `alloy`:

```toml
[dependencies]
# ... keep existing deps ...
ethers = { version = "2", features = ["signers"] }
# REMOVE: alloy = { ... }
```

**Step 2: Verify it compiles (expect errors from alloy removal)**

Run: `cargo check 2>&1 | head -20`
Expected: Errors in `wallet_service.rs` about missing `alloy` — that's correct, we fix it in Task 2.

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: switch from alloy to ethers for wallet signing"
```

---

### Task 2: Migrate wallet_service to ethers

**Files:**
- Modify: `src/services/wallet_service.rs`

**Step 1: Replace alloy import with ethers in `address_from_key`**

The only alloy usage is in `address_from_key()`. Replace:

```rust
pub fn address_from_key(private_key: &str) -> Result<String> {
    let key = private_key.strip_prefix("0x").unwrap_or(private_key);
    let wallet: ethers::signers::LocalWallet = key.parse()
        .context("invalid private key")?;
    Ok(format!("{:?}", wallet.address()))
}
```

Also add a helper to get a `LocalWallet` for SDK use:

```rust
pub fn wallet_from_key(private_key: &str) -> Result<ethers::signers::LocalWallet> {
    let key = private_key.strip_prefix("0x").unwrap_or(private_key);
    let wallet: ethers::signers::LocalWallet = key.parse()
        .context("invalid private key")?;
    Ok(wallet)
}
```

Remove any `use std::str::FromStr;` if it was only for alloy (check — it may still be needed by ethers `.parse()`).

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Clean compilation (or only unrelated warnings).

**Step 3: Run existing tests**

Run: `cargo test -p hype-trader -- wallet`
Expected: All 4 wallet tests pass.

**Step 4: Commit**

```bash
git add src/services/wallet_service.rs
git commit -m "refactor: migrate wallet_service from alloy to ethers"
```

---

### Task 3: Implement InfoService with real SDK

**Files:**
- Modify: `src/services/info_service.rs`
- Modify: `src/models.rs` (add `CandleInterval::to_sdk_string`)

**Step 1: Add SDK interval mapping to models.rs**

Add to `CandleInterval` impl:

```rust
pub fn to_sdk_string(&self) -> &'static str {
    match self {
        Self::M1 => "1m",
        Self::M5 => "5m",
        Self::M15 => "15m",
        Self::H1 => "1h",
        Self::H4 => "4h",
        Self::D1 => "1d",
    }
}
```

**Step 2: Rewrite info_service.rs**

Replace entire file with:

```rust
use anyhow::Result;
use ethers::types::H160;
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use crate::models::*;

pub struct InfoService {
    client: InfoClient,
}

impl InfoService {
    pub async fn new(network: Network) -> Result<Self> {
        let base_url = match network {
            Network::Mainnet => BaseUrl::Mainnet,
            Network::Testnet => BaseUrl::Testnet,
        };
        let client = InfoClient::new(None, Some(base_url)).await?;
        Ok(Self { client })
    }

    /// Fetch all perpetual trading symbols with mid prices
    pub async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        let meta = self.client.meta().await?;
        let mids = self.client.all_mids().await?;

        let symbols = meta.universe.iter().map(|asset| {
            let mid_price = mids.get(&asset.name)
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            Symbol {
                name: format!("{}-USD", asset.name),
                base: asset.name.clone(),
                quote: "USD".to_string(),
                last_price: mid_price,
                change_24h: 0.0,   // not available from meta
                volume_24h: 0.0,   // not available from meta
            }
        }).collect();

        Ok(symbols)
    }

    /// Fetch L2 order book snapshot for a coin (e.g. "ETH")
    pub async fn fetch_orderbook(&self, coin: &str) -> Result<OrderBook> {
        let snapshot = self.client.l2_snapshot(coin.to_string()).await?;

        let mut bids = Vec::new();
        let mut asks = Vec::new();

        // levels[0] = bids, levels[1] = asks
        if let Some(bid_levels) = snapshot.levels.get(0) {
            let mut cum = 0.0;
            for level in bid_levels {
                let price = level.px.parse::<f64>().unwrap_or(0.0);
                let size = level.sz.parse::<f64>().unwrap_or(0.0);
                cum += size;
                bids.push(OrderBookLevel { price, size, cumulative: cum });
            }
        }
        if let Some(ask_levels) = snapshot.levels.get(1) {
            let mut cum = 0.0;
            for level in ask_levels {
                let price = level.px.parse::<f64>().unwrap_or(0.0);
                let size = level.sz.parse::<f64>().unwrap_or(0.0);
                cum += size;
                asks.push(OrderBookLevel { price, size, cumulative: cum });
            }
        }

        let last_price = bids.first().map(|b| b.price).unwrap_or(0.0);

        Ok(OrderBook { bids, asks, last_price })
    }

    /// Fetch candle data for a coin
    pub async fn fetch_candles(&self, coin: &str, interval: CandleInterval, limit: usize) -> Result<Vec<Candle>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;

        let interval_ms: u64 = match interval {
            CandleInterval::M1 => 60_000,
            CandleInterval::M5 => 300_000,
            CandleInterval::M15 => 900_000,
            CandleInterval::H1 => 3_600_000,
            CandleInterval::H4 => 14_400_000,
            CandleInterval::D1 => 86_400_000,
        };
        let start_time = now - (limit as u64) * interval_ms;

        let snapshot = self.client.candles_snapshot(
            coin.to_string(),
            interval.to_sdk_string().to_string(),
            start_time,
            now,
        ).await?;

        let candles = snapshot.iter().map(|c| {
            Candle {
                time: c.time_open,
                open: c.open.parse::<f64>().unwrap_or(0.0),
                high: c.high.parse::<f64>().unwrap_or(0.0),
                low: c.low.parse::<f64>().unwrap_or(0.0),
                close: c.close.parse::<f64>().unwrap_or(0.0),
                volume: c.vlm.parse::<f64>().unwrap_or(0.0),
            }
        }).collect();

        Ok(candles)
    }

    /// Fetch recent trades for a coin
    pub async fn fetch_recent_trades(&self, coin: &str) -> Result<Vec<Trade>> {
        let trades = self.client.recent_trades(coin.to_string()).await?;

        let result = trades.iter().map(|t| {
            Trade {
                time: t.time,
                price: t.px.parse::<f64>().unwrap_or(0.0),
                size: t.sz.parse::<f64>().unwrap_or(0.0),
                is_buy: t.side == "B",
            }
        }).collect();

        Ok(result)
    }

    /// Fetch user positions and margin summary
    pub async fn fetch_user_state(&self, address: H160) -> Result<(Vec<Position>, PnlSummary)> {
        let state = self.client.user_state(address).await?;

        let positions: Vec<Position> = state.asset_positions.iter().filter_map(|ap| {
            let d = &ap.position;
            let szi: f64 = d.szi.parse().ok()?;
            if szi.abs() < 1e-10 { return None; }

            let side = if szi > 0.0 { OrderSide::Buy } else { OrderSide::Sell };
            let entry_price = d.entry_px.as_ref()?.parse::<f64>().ok()?;
            let unrealized_pnl = d.unrealized_pnl.parse::<f64>().unwrap_or(0.0);
            let margin_used = d.margin_used.parse::<f64>().unwrap_or(0.0);
            let leverage_val = if margin_used > 0.0 {
                d.position_value.parse::<f64>().unwrap_or(0.0).abs() / margin_used
            } else {
                1.0
            };

            // mark_price = entry + unrealized_pnl / size
            let mark_price = if szi.abs() > 1e-10 {
                entry_price + unrealized_pnl / szi
            } else {
                entry_price
            };

            Some(Position {
                symbol: format!("{}-USD", d.coin),
                side,
                size: szi.abs(),
                entry_price,
                mark_price,
                unrealized_pnl,
                leverage: leverage_val,
            })
        }).collect();

        let ms = &state.margin_summary;
        let total_balance = ms.account_value.parse::<f64>().unwrap_or(0.0);
        let margin_used = ms.total_margin_used.parse::<f64>().unwrap_or(0.0);
        let available = total_balance - margin_used;
        let total_raw_usd = ms.total_raw_usd.parse::<f64>().unwrap_or(0.0);

        let pnl = PnlSummary {
            total_pnl: total_balance - total_raw_usd,
            daily_pnl: 0.0,  // not directly available from this endpoint
            total_balance,
            available_balance: available,
            margin_used,
        };

        Ok((positions, pnl))
    }

    /// Fetch open orders for a user
    pub async fn fetch_open_orders(&self, address: H160) -> Result<Vec<OpenOrder>> {
        let orders = self.client.open_orders(address).await?;

        let result = orders.iter().map(|o| {
            let side = if o.side == "B" { OrderSide::Buy } else { OrderSide::Sell };
            OpenOrder {
                id: o.oid.to_string(),
                symbol: format!("{}-USD", o.coin),
                side,
                order_type: OrderType::Limit,
                price: o.limit_px.parse::<f64>().unwrap_or(0.0),
                size: o.sz.parse::<f64>().unwrap_or(0.0),
                filled: 0.0,
                timestamp: o.timestamp,
            }
        }).collect();

        Ok(result)
    }

    /// Fetch user fill/trade history
    pub async fn fetch_trade_history(&self, address: H160) -> Result<Vec<TradeHistory>> {
        let fills = self.client.user_fills(address).await?;

        let result = fills.iter().map(|f| {
            let side = if f.side == "B" { OrderSide::Buy } else { OrderSide::Sell };
            TradeHistory {
                id: f.oid.to_string(),
                symbol: format!("{}-USD", f.coin),
                side,
                price: f.px.parse::<f64>().unwrap_or(0.0),
                size: f.sz.parse::<f64>().unwrap_or(0.0),
                fee: f.fee.parse::<f64>().unwrap_or(0.0),
                timestamp: f.time,
            }
        }).collect();

        Ok(result)
    }

    /// Fetch user balances
    pub async fn fetch_balances(&self, address: H160) -> Result<Vec<Balance>> {
        let state = self.client.user_state(address).await?;
        let ms = &state.margin_summary;
        let total = ms.account_value.parse::<f64>().unwrap_or(0.0);
        let margin = ms.total_margin_used.parse::<f64>().unwrap_or(0.0);

        Ok(vec![Balance {
            asset: "USDC".to_string(),
            total,
            available: total - margin,
            in_margin: margin,
        }])
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Clean (warnings OK).

**Step 4: Commit**

```bash
git add src/services/info_service.rs src/models.rs
git commit -m "feat: implement InfoService with real Hyperliquid SDK"
```

---

### Task 4: Implement ExchangeService with real SDK

**Files:**
- Modify: `src/services/exchange_service.rs`

**Step 1: Rewrite exchange_service.rs**

Replace entire file:

```rust
use anyhow::{Result, bail};
use ethers::signers::LocalWallet;
use hyperliquid_rust_sdk::{
    BaseUrl, ExchangeClient, ExchangeResponseStatus,
    ClientOrderRequest, ClientOrder, ClientLimit, ClientTrigger,
    ClientCancelRequest, MarketOrderParams, MarketCloseParams,
};
use crate::models::*;

pub struct ExchangeService {
    client: Option<ExchangeClient>,
    base_url: BaseUrl,
}

impl ExchangeService {
    pub fn new(network: Network) -> Self {
        let base_url = match network {
            Network::Mainnet => BaseUrl::Mainnet,
            Network::Testnet => BaseUrl::Testnet,
        };
        Self { client: None, base_url }
    }

    /// Connect with a wallet, creating the ExchangeClient
    pub async fn connect(&mut self, wallet: LocalWallet) -> Result<()> {
        let client = ExchangeClient::new(
            None,
            wallet,
            Some(self.base_url),
            None,
            None,
        ).await?;
        self.client = Some(client);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    fn client(&self) -> Result<&ExchangeClient> {
        self.client.as_ref().ok_or_else(|| anyhow::anyhow!("exchange not connected"))
    }

    /// Place a limit order
    pub async fn place_limit_order(
        &self,
        coin: &str,
        is_buy: bool,
        price: f64,
        size: f64,
        reduce_only: bool,
    ) -> Result<String> {
        let client = self.client()?;
        let order = ClientOrderRequest {
            asset: coin.to_string(),
            is_buy,
            reduce_only,
            limit_px: price,
            sz: size,
            cloid: None,
            order_type: ClientOrder::Limit(ClientLimit {
                tif: "Gtc".to_string(),
            }),
        };

        match client.order(order, None).await? {
            ExchangeResponseStatus::Ok(resp) => {
                Ok(format!("{:?}", resp))
            }
            ExchangeResponseStatus::Err(e) => bail!("order failed: {}", e),
        }
    }

    /// Place a market order
    pub async fn place_market_order(
        &self,
        coin: &str,
        is_buy: bool,
        size: f64,
    ) -> Result<String> {
        let client = self.client()?;
        let params = MarketOrderParams {
            asset: coin,
            is_buy,
            sz: size,
            px: None,
            slippage: None,
            cloid: None,
            wallet: None,
        };

        match client.market_open(params).await? {
            ExchangeResponseStatus::Ok(resp) => {
                Ok(format!("{:?}", resp))
            }
            ExchangeResponseStatus::Err(e) => bail!("market order failed: {}", e),
        }
    }

    /// Place a TP/SL trigger order
    pub async fn place_trigger_order(
        &self,
        coin: &str,
        is_buy: bool,
        trigger_price: f64,
        size: f64,
        is_tp: bool,
    ) -> Result<String> {
        let client = self.client()?;
        let order = ClientOrderRequest {
            asset: coin.to_string(),
            is_buy,
            reduce_only: true,
            limit_px: trigger_price,
            sz: size,
            cloid: None,
            order_type: ClientOrder::Trigger(ClientTrigger {
                is_market: true,
                trigger_px: trigger_price,
                tpsl: if is_tp { "tp".to_string() } else { "sl".to_string() },
            }),
        };

        match client.order(order, None).await? {
            ExchangeResponseStatus::Ok(resp) => Ok(format!("{:?}", resp)),
            ExchangeResponseStatus::Err(e) => bail!("trigger order failed: {}", e),
        }
    }

    /// Cancel a specific order by OID
    pub async fn cancel_order(&self, coin: &str, oid: u64) -> Result<()> {
        let client = self.client()?;
        let cancel = ClientCancelRequest {
            asset: coin.to_string(),
            oid,
        };

        match client.cancel(cancel, None).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("cancel failed: {}", e),
        }
    }

    /// Cancel multiple orders
    pub async fn bulk_cancel(&self, cancels: Vec<(String, u64)>) -> Result<()> {
        let client = self.client()?;
        let requests: Vec<ClientCancelRequest> = cancels.into_iter().map(|(asset, oid)| {
            ClientCancelRequest { asset, oid }
        }).collect();

        match client.bulk_cancel(requests, None).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("bulk cancel failed: {}", e),
        }
    }

    /// Update leverage for a coin
    pub async fn update_leverage(&self, coin: &str, leverage: u32, is_cross: bool) -> Result<()> {
        let client = self.client()?;
        match client.update_leverage(leverage, coin, is_cross, None).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("update leverage failed: {}", e),
        }
    }

    /// Market close a position
    pub async fn market_close(&self, coin: &str, size: Option<f64>) -> Result<()> {
        let client = self.client()?;
        let params = MarketCloseParams {
            asset: coin,
            sz: size,
            px: None,
            slippage: None,
            cloid: None,
            wallet: None,
        };

        match client.market_close(params).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("market close failed: {}", e),
        }
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Clean.

**Step 3: Commit**

```bash
git add src/services/exchange_service.rs
git commit -m "feat: implement ExchangeService with real Hyperliquid SDK"
```

---

### Task 5: Implement WsService with real WebSocket subscriptions

**Files:**
- Modify: `src/services/ws_service.rs`

**Step 1: Rewrite ws_service.rs**

```rust
use anyhow::Result;
use ethers::types::H160;
use hyperliquid_rust_sdk::{
    BaseUrl, InfoClient, Subscription,
    Message as HlMessage,
};
use tokio::sync::mpsc;
use crate::models::*;

/// Messages pushed from WebSocket to the UI layer
#[derive(Debug)]
pub enum WsUpdate {
    OrderBookUpdate(OrderBook),
    TradesUpdate(Vec<Trade>),
    CandleUpdate(Candle),
    AllMids(std::collections::HashMap<String, f64>),
    OrderUpdate(String),  // raw status for now
    UserFill(TradeHistory),
}

pub struct WsService {
    client: InfoClient,
    active_subs: Vec<u32>,
}

impl WsService {
    pub async fn new(network: Network) -> Result<Self> {
        let base_url = match network {
            Network::Mainnet => BaseUrl::Mainnet,
            Network::Testnet => BaseUrl::Testnet,
        };
        let client = InfoClient::with_reconnect(None, Some(base_url)).await?;
        Ok(Self { client, active_subs: Vec::new() })
    }

    /// Unsubscribe all active subscriptions
    pub async fn unsubscribe_all(&mut self) -> Result<()> {
        for sub_id in self.active_subs.drain(..) {
            let _ = self.client.unsubscribe(sub_id).await;
        }
        Ok(())
    }

    /// Subscribe to L2 order book updates for a coin
    pub async fn subscribe_l2_book(
        &mut self,
        coin: &str,
        tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::L2Book { coin: coin.to_string() },
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::L2Book(l2) = msg {
                    let data = &l2.data;
                    let mut bids = Vec::new();
                    let mut asks = Vec::new();

                    if let Some(bid_levels) = data.levels.get(0) {
                        let mut cum = 0.0;
                        for level in bid_levels {
                            let price = level.px.parse::<f64>().unwrap_or(0.0);
                            let size = level.sz.parse::<f64>().unwrap_or(0.0);
                            cum += size;
                            bids.push(OrderBookLevel { price, size, cumulative: cum });
                        }
                    }
                    if let Some(ask_levels) = data.levels.get(1) {
                        let mut cum = 0.0;
                        for level in ask_levels {
                            let price = level.px.parse::<f64>().unwrap_or(0.0);
                            let size = level.sz.parse::<f64>().unwrap_or(0.0);
                            cum += size;
                            asks.push(OrderBookLevel { price, size, cumulative: cum });
                        }
                    }

                    let last_price = bids.first().map(|b| b.price).unwrap_or(0.0);
                    let _ = tx.send(WsUpdate::OrderBookUpdate(OrderBook { bids, asks, last_price }));
                }
            }
        });

        Ok(sub_id)
    }

    /// Subscribe to trade updates for a coin
    pub async fn subscribe_trades(
        &mut self,
        coin: &str,
        tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::Trades { coin: coin.to_string() },
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::Trades(trades) = msg {
                    let converted: Vec<Trade> = trades.data.iter().map(|t| {
                        Trade {
                            time: t.time,
                            price: t.px.parse::<f64>().unwrap_or(0.0),
                            size: t.sz.parse::<f64>().unwrap_or(0.0),
                            is_buy: t.side == "B",
                        }
                    }).collect();
                    let _ = tx.send(WsUpdate::TradesUpdate(converted));
                }
            }
        });

        Ok(sub_id)
    }

    /// Subscribe to candle updates for a coin
    pub async fn subscribe_candles(
        &mut self,
        coin: &str,
        interval: CandleInterval,
        tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::Candle {
                coin: coin.to_string(),
                interval: interval.to_sdk_string().to_string(),
            },
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::Candle(candle) = msg {
                    let c = &candle.data;
                    let converted = Candle {
                        time: c.time_open,
                        open: c.open.parse::<f64>().unwrap_or(0.0),
                        high: c.high.parse::<f64>().unwrap_or(0.0),
                        low: c.low.parse::<f64>().unwrap_or(0.0),
                        close: c.close.parse::<f64>().unwrap_or(0.0),
                        volume: c.volume.parse::<f64>().unwrap_or(0.0),
                    };
                    let _ = tx.send(WsUpdate::CandleUpdate(converted));
                }
            }
        });

        Ok(sub_id)
    }

    /// Subscribe to all mid prices
    pub async fn subscribe_all_mids(
        &mut self,
        tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::AllMids,
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::AllMids(mids) = msg {
                    let parsed: std::collections::HashMap<String, f64> = mids.data.mids.iter()
                        .filter_map(|(k, v)| {
                            Some((k.clone(), v.parse::<f64>().ok()?))
                        })
                        .collect();
                    let _ = tx.send(WsUpdate::AllMids(parsed));
                }
            }
        });

        Ok(sub_id)
    }

    /// Subscribe to user order updates (requires login)
    pub async fn subscribe_order_updates(
        &mut self,
        user: H160,
        tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::OrderUpdates { user },
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::OrderUpdates(updates) = msg {
                    for update in &updates.data {
                        let _ = tx.send(WsUpdate::OrderUpdate(
                            format!("{} {} {}", update.order.coin, update.status, update.order.oid),
                        ));
                    }
                }
            }
        });

        Ok(sub_id)
    }

    /// Subscribe to user fills (requires login)
    pub async fn subscribe_user_fills(
        &mut self,
        user: H160,
        tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::UserFills { user },
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::UserFills(fills) = msg {
                    for f in &fills.data.fills {
                        let side = if f.side == "B" { OrderSide::Buy } else { OrderSide::Sell };
                        let _ = tx.send(WsUpdate::UserFill(TradeHistory {
                            id: f.oid.to_string(),
                            symbol: format!("{}-USD", f.coin),
                            side,
                            price: f.px.parse::<f64>().unwrap_or(0.0),
                            size: f.sz.parse::<f64>().unwrap_or(0.0),
                            fee: f.fee.parse::<f64>().unwrap_or(0.0),
                            timestamp: f.time,
                        }));
                    }
                }
            }
        });

        Ok(sub_id)
    }
}
```

**Step 2: Update `src/services/mod.rs`**

Verify it still exports all modules (no change needed if `ws_service` is already listed).

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Clean.

**Step 4: Commit**

```bash
git add src/services/ws_service.rs
git commit -m "feat: implement WsService with real WebSocket subscriptions"
```

---

### Task 6: Update MainView to use async services

**Files:**
- Modify: `src/views/main_view.rs`
- Modify: `src/main.rs`

**Step 1: Update main.rs to set up a tokio runtime**

GPUI runs its own event loop, so we need a way to run async tasks. Create a shared tokio runtime:

```rust
mod models;
mod services;
mod state;
mod views;

use gpui::prelude::*;
use gpui::{div, Application, Bounds, Entity, WindowBounds, WindowOptions};
use views::main_view::MainView;

struct HypeTrader {
    main_view: Entity<MainView>,
}

impl HypeTrader {
    fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let main_view = cx.new(|cx| MainView::new(window, cx));
        Self { main_view }
    }
}

impl Render for HypeTrader {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div().size_full().child(self.main_view.clone())
    }
}

fn main() {
    // Create tokio runtime for async SDK calls
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    // Store runtime handle globally for async tasks
    let _guard = rt.enter();

    Application::new().run(|cx| {
        gpui_component::init(cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    gpui::size(gpui::px(1400.), gpui::px(900.)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let inner_view = cx.new(|cx| HypeTrader::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(inner_view, window, cx))
            },
        )
        .unwrap();
    });
}
```

**Step 2: Update MainView to load data via services on init**

This is the most complex part. MainView needs to:
1. Spawn async task on construction to fetch initial data
2. Update its child entities when data arrives

Replace `MainView::new` to kick off async data loading. The mock data stays as initial defaults — async tasks update them once data arrives.

Key pattern for GPUI async:
```rust
// In MainView::new
cx.spawn(|this, mut cx| async move {
    let info = InfoService::new(Network::Mainnet).await?;
    let symbols = info.fetch_symbols().await?;
    this.update(&mut cx, |view, cx| {
        // update child entities with real data
        cx.notify();
    })?;
    Ok::<_, anyhow::Error>(())
}).detach();
```

The full implementation of `MainView::new` should:
- Keep mock data as initial state (so UI renders immediately)
- Spawn async task to fetch symbols, orderbook, candles, trades
- On success, update the child entities

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Clean.

**Step 4: Commit**

```bash
git add src/main.rs src/views/main_view.rs
git commit -m "feat: wire MainView to async services for live data loading"
```

---

### Task 7: Build and smoke test

**Step 1: Full build**

Run: `cargo build --release`
Expected: Clean build.

**Step 2: Quick manual test**

Run the app and verify:
- Symbol list loads real trading pairs from Hyperliquid
- Order book shows real bid/ask levels
- Candle chart shows real historical data
- No panics or crashes

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: complete Hyperliquid API integration"
```

---

## Implementation Notes

- **Coin naming:** SDK uses `"ETH"`, app uses `"ETH-USD"`. Convert in service layer: `coin.strip_suffix("-USD")` when calling SDK, `format!("{}-USD", coin)` when returning.
- **String → f64:** All SDK numeric fields are Strings. Always use `.parse::<f64>().unwrap_or(0.0)`.
- **Error handling:** Services return `anyhow::Result`. UI layer should catch and display errors gracefully.
- **BaseUrl:** SDK's `BaseUrl` enum maps directly: `BaseUrl::Mainnet` / `BaseUrl::Testnet`.
- **Tokio runtime:** GPUI has its own event loop. Create a tokio `Runtime` in `main()` and enter it so `tokio::spawn` works inside GPUI's `cx.spawn`.
