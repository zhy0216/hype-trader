use anyhow::Result;
use ethers::types::H160;
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use serde::Deserialize;
use crate::models::*;

/// Asset context returned by the Hyperliquid `metaAndAssetCtxs` endpoint.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PerpsAssetCtx {
    day_ntl_vlm: String,
    mark_px: String,
    prev_day_px: String,
}

pub struct InfoService {
    client: InfoClient,
    base_url: String,
}

impl InfoService {
    pub async fn new(network: Network) -> Result<Self> {
        let base_url = match network {
            Network::Mainnet => BaseUrl::Mainnet,
            Network::Testnet => BaseUrl::Testnet,
        };
        let url_str = match network {
            Network::Mainnet => "https://api.hyperliquid.xyz/info",
            Network::Testnet => "https://api.hyperliquid-testnet.xyz/info",
        };
        let client = InfoClient::new(None, Some(base_url)).await?;
        Ok(Self { client, base_url: url_str.to_string() })
    }

    /// Fetch all perpetual trading symbols with 24h change and volume
    pub async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        // Call metaAndAssetCtxs to get prev_day_px, mark_px, and day_ntl_vlm
        let http = reqwest::Client::new();
        let resp: serde_json::Value = http
            .post(&self.base_url)
            .json(&serde_json::json!({"type": "metaAndAssetCtxs"}))
            .send()
            .await?
            .json()
            .await?;

        // Response is [meta, [assetCtx, ...]]
        let universe = resp.get(0)
            .and_then(|m| m.get("universe"))
            .and_then(|u| u.as_array());
        let ctxs = resp.get(1)
            .and_then(|c| c.as_array());

        let (universe, ctxs) = match (universe, ctxs) {
            (Some(u), Some(c)) => (u, c),
            _ => return self.fetch_symbols_fallback().await,
        };

        let symbols = universe.iter().zip(ctxs.iter()).filter_map(|(asset, ctx)| {
            let name = asset.get("name")?.as_str()?;
            let ctx: PerpsAssetCtx = serde_json::from_value(ctx.clone()).ok()?;

            let mark_px = ctx.mark_px.parse::<f64>().unwrap_or(0.0);
            let prev_day_px = ctx.prev_day_px.parse::<f64>().unwrap_or(0.0);
            let volume = ctx.day_ntl_vlm.parse::<f64>().unwrap_or(0.0);

            let change_24h = if prev_day_px > 0.0 {
                ((mark_px - prev_day_px) / prev_day_px) * 100.0
            } else {
                0.0
            };

            Some(Symbol {
                name: format!("{}-USD", name),
                base: name.to_string(),
                quote: "USD".to_string(),
                last_price: mark_px,
                change_24h,
                volume_24h: volume,
            })
        }).collect();

        Ok(symbols)
    }

    /// Fallback using basic meta + all_mids if metaAndAssetCtxs fails
    async fn fetch_symbols_fallback(&self) -> Result<Vec<Symbol>> {
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
                change_24h: 0.0,
                volume_24h: 0.0,
            }
        }).collect();

        Ok(symbols)
    }

    /// Fetch L2 order book snapshot for a coin (e.g. "ETH")
    pub async fn fetch_orderbook(&self, coin: &str) -> Result<OrderBook> {
        let snapshot = self.client.l2_snapshot(coin.to_string()).await?;

        let mut bids = Vec::new();
        let mut asks = Vec::new();

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
            daily_pnl: 0.0,
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
