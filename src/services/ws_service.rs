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
    /// (coin, book) – coin identifies which symbol this update is for
    OrderBookUpdate(String, OrderBook),
    TradesUpdate(Vec<Trade>),
    /// (coin, candle) – coin identifies which symbol this update is for
    CandleUpdate(String, Candle),
    AllMids(std::collections::HashMap<String, f64>),
    OrderUpdate(String),
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

    pub async fn unsubscribe_all(&mut self) -> Result<()> {
        for sub_id in self.active_subs.drain(..) {
            let _ = self.client.unsubscribe(sub_id).await;
        }
        Ok(())
    }

    /// Unsubscribe a single subscription by ID.
    pub async fn unsubscribe(&mut self, sub_id: u32) {
        let _ = self.client.unsubscribe(sub_id).await;
        self.active_subs.retain(|&id| id != sub_id);
    }

    pub async fn subscribe_l2_book(
        &mut self, coin: &str, tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(
            Subscription::L2Book { coin: coin.to_string() },
            ws_tx,
        ).await?;
        self.active_subs.push(sub_id);

        let coin_name = coin.to_string();
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
                    let _ = tx.send(WsUpdate::OrderBookUpdate(coin_name.clone(), OrderBook { bids, asks, last_price }));
                }
            }
        });
        Ok(sub_id)
    }

    pub async fn subscribe_trades(
        &mut self, coin: &str, tx: mpsc::UnboundedSender<WsUpdate>,
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

    pub async fn subscribe_candles(
        &mut self, coin: &str, interval: CandleInterval, tx: mpsc::UnboundedSender<WsUpdate>,
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

        let coin_name = coin.to_string();
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
                    let _ = tx.send(WsUpdate::CandleUpdate(coin_name.clone(), converted));
                }
            }
        });
        Ok(sub_id)
    }

    pub async fn subscribe_all_mids(
        &mut self, tx: mpsc::UnboundedSender<WsUpdate>,
    ) -> Result<u32> {
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel();
        let sub_id = self.client.subscribe(Subscription::AllMids, ws_tx).await?;
        self.active_subs.push(sub_id);

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if let HlMessage::AllMids(mids) = msg {
                    let parsed: std::collections::HashMap<String, f64> = mids.data.mids.iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.parse::<f64>().ok()?)))
                        .collect();
                    let _ = tx.send(WsUpdate::AllMids(parsed));
                }
            }
        });
        Ok(sub_id)
    }

    pub async fn subscribe_order_updates(
        &mut self, user: H160, tx: mpsc::UnboundedSender<WsUpdate>,
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

    pub async fn subscribe_user_fills(
        &mut self, user: H160, tx: mpsc::UnboundedSender<WsUpdate>,
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
