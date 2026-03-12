use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, rgb, Entity, Subscription};

use crate::models::*;
use crate::services::info_service::InfoService;
use crate::services::wallet_service;
use crate::services::ws_service::{WsService, WsUpdate};
use crate::views::bottom_panel::BottomPanel;
use crate::views::candle_chart::CandleChart;
use crate::views::order_book::OrderBookView;
use crate::views::order_panel::OrderPanel;
use crate::views::symbol_list::{SymbolList, SymbolSelected};
use crate::views::top_bar::TopBar;

pub struct MainView {
    top_bar: Entity<TopBar>,
    symbol_list: Entity<SymbolList>,
    candle_chart: Entity<CandleChart>,
    order_book: Entity<OrderBookView>,
    order_panel: Entity<OrderPanel>,
    bottom_panel: Entity<BottomPanel>,
    wallet_connected: bool,
    pub private_key: Option<String>,
    pub network: Network,
    _symbol_subscription: Subscription,
}

impl MainView {
    pub fn new(
        private_key: Option<String>,
        network: Network,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let wallet_connected = private_key.is_some();

        // Derive address from key if provided
        let address: Option<String> = private_key.as_ref().and_then(|key| {
            wallet_service::address_from_key(key)
                .ok()
                .map(|addr| wallet_service::format_address(&addr))
        });

        // Create TopBar with defaults (Connecting state, address if available)
        let top_bar = cx.new(|_cx| {
            TopBar::new(
                network,
                ConnectionStatus::Connecting,
                ThemeMode::Dark,
                0.0,
                address,
            )
        });

        // Create SymbolList with empty symbols (will be populated async)
        let symbol_list = cx.new(|cx| SymbolList::new(window, cx));

        // Create CandleChart empty
        let candle_chart = cx.new(|_cx| CandleChart::new());

        // Create OrderBook empty
        let order_book = cx.new(|_cx| OrderBookView::new());

        // Create OrderPanel
        let order_panel = cx.new(|cx| OrderPanel::new(wallet_connected, private_key.clone(), network, window, cx));

        // Create BottomPanel empty
        let bottom_panel = cx.new(|cx| BottomPanel::new(private_key.clone(), network, window, cx));

        // Subscribe to symbol selection changes
        let _symbol_subscription =
            cx.subscribe_in(&symbol_list, window, Self::on_symbol_selected);

        // Clone entity handles for the async task
        let symbol_list_clone = symbol_list.clone();
        let candle_chart_clone = candle_chart.clone();
        let order_book_clone = order_book.clone();
        let top_bar_clone = top_bar.clone();
        let bottom_panel_clone = bottom_panel.clone();
        let pk_clone = private_key.clone();

        // Spawn async task to fetch real data, then start WebSocket subscriptions
        cx.spawn(async move |_this, cx| {
            // Create InfoService
            let info = match InfoService::new(network).await {
                Ok(info) => info,
                Err(e) => {
                    tracing::error!("Failed to create InfoService: {}", e);
                    return;
                }
            };

            // Fetch symbols
            match info.fetch_symbols().await {
                Ok(symbols) => {
                    let _ = cx.update_entity(&symbol_list_clone, |list, _cx| {
                        list.symbols = symbols;
                    });
                }
                Err(e) => tracing::error!("Failed to fetch symbols: {}", e),
            }

            // Fetch orderbook for default symbol (ETH)
            match info.fetch_orderbook("ETH").await {
                Ok(book) => {
                    let _ = cx.update_entity(&order_book_clone, |view, _cx| {
                        view.data = book;
                    });
                }
                Err(e) => tracing::error!("Failed to fetch orderbook: {}", e),
            }

            // Fetch candles for default symbol
            match info.fetch_candles("ETH", CandleInterval::H1, 100).await {
                Ok(candles) => {
                    let _ = cx.update_entity(&candle_chart_clone, |chart, _cx| {
                        chart.candles = candles;
                    });
                }
                Err(e) => tracing::error!("Failed to fetch candles: {}", e),
            }

            // --- WebSocket real-time subscriptions with reconnect logic ---
            let mut backoff_secs = 3u64;
            loop {
                // Set status to Connecting
                let _ = cx.update_entity(&top_bar_clone, |bar, _cx| {
                    bar.connection_status = ConnectionStatus::Connecting;
                });

                let ws_result = WsService::new(network).await;
                let mut ws = match ws_result {
                    Ok(ws) => ws,
                    Err(e) => {
                        tracing::error!("WsService creation failed: {}", e);
                        let _ = cx.update_entity(&top_bar_clone, |bar, _cx| {
                            bar.connection_status = ConnectionStatus::Disconnected;
                        });
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(30);
                        continue;
                    }
                };

                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WsUpdate>();

                // Subscribe to L2 book for the default symbol
                if let Err(e) = ws.subscribe_l2_book("ETH", tx.clone()).await {
                    tracing::error!("Failed to subscribe to L2 book: {}", e);
                }

                // Subscribe to candles for default symbol + interval
                if let Err(e) = ws.subscribe_candles("ETH", CandleInterval::H1, tx.clone()).await {
                    tracing::error!("Failed to subscribe to candles: {}", e);
                }

                // Subscribe to all mids for live price updates on symbol list
                if let Err(e) = ws.subscribe_all_mids(tx.clone()).await {
                    tracing::error!("Failed to subscribe to all mids: {}", e);
                }

                // If wallet is connected, subscribe to user-specific feeds
                if let Some(ref key) = pk_clone {
                    match wallet_service::address_from_key(key) {
                        Ok(addr_str) => {
                            match addr_str.parse::<ethers::types::H160>() {
                                Ok(addr) => {
                                    if let Err(e) = ws.subscribe_order_updates(addr, tx.clone()).await {
                                        tracing::error!("Failed to subscribe to order updates: {}", e);
                                    }
                                    if let Err(e) = ws.subscribe_user_fills(addr, tx.clone()).await {
                                        tracing::error!("Failed to subscribe to user fills: {}", e);
                                    }
                                }
                                Err(e) => tracing::error!("Failed to parse address for WS subscriptions: {}", e),
                            }
                        }
                        Err(e) => tracing::error!("Failed to derive address for WS subscriptions: {}", e),
                    }
                }

                // Set Connected and reset backoff on successful setup
                let _ = cx.update_entity(&top_bar_clone, |bar, _cx| {
                    bar.connection_status = ConnectionStatus::Connected;
                });
                backoff_secs = 3;

                tracing::info!("WebSocket subscriptions active, entering recv loop");

                // Receive loop: route WsUpdate messages to UI entities.
                // Keep `ws` alive in scope so subscriptions stay open.
                while let Some(update) = rx.recv().await {
                    match update {
                        WsUpdate::OrderBookUpdate(book) => {
                            let _ = cx.update_entity(&order_book_clone, |view, _cx| {
                                view.data = book;
                            });
                        }
                        WsUpdate::CandleUpdate(candle) => {
                            let _ = cx.update_entity(&candle_chart_clone, |chart, _cx| {
                                if let Some(last) = chart.candles.last() {
                                    if last.time == candle.time {
                                        // Same candle period: replace the last candle
                                        let len = chart.candles.len();
                                        chart.candles[len - 1] = candle;
                                    } else {
                                        chart.candles.push(candle);
                                    }
                                } else {
                                    chart.candles.push(candle);
                                }
                            });
                        }
                        WsUpdate::AllMids(mids) => {
                            let _ = cx.update_entity(&symbol_list_clone, |list, _cx| {
                                for symbol in &mut list.symbols {
                                    // Symbol base is the coin name (e.g. "ETH")
                                    if let Some(&price) = mids.get(&symbol.base) {
                                        symbol.last_price = price;
                                    }
                                }
                            });
                        }
                        WsUpdate::OrderUpdate(info) => {
                            tracing::info!("Order update: {}", info);
                        }
                        WsUpdate::UserFill(fill) => {
                            let _ = cx.update_entity(&bottom_panel_clone, |panel, _cx| {
                                panel.trade_history.insert(0, fill);
                            });
                        }
                        WsUpdate::TradesUpdate(_) => {
                            // Not routed to UI currently
                        }
                    }
                }

                // If we exit the recv loop, WS connection was lost
                tracing::warn!("WebSocket disconnected, reconnecting in {}s...", backoff_secs);
                let _ = ws.unsubscribe_all().await;
                let _ = cx.update_entity(&top_bar_clone, |bar, _cx| {
                    bar.connection_status = ConnectionStatus::Disconnected;
                });
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(30);
            }
        })
        .detach();

        Self {
            top_bar,
            symbol_list,
            candle_chart,
            order_book,
            order_panel,
            bottom_panel,
            wallet_connected,
            private_key,
            network,
            _symbol_subscription,
        }
    }

    /// Called when the user selects a different symbol in the SymbolList.
    /// Spawns an async task to re-fetch orderbook and candles for the new coin.
    fn on_symbol_selected(
        &mut self,
        _symbol_list: &Entity<SymbolList>,
        event: &SymbolSelected,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let symbol_name = event.0.clone();
        // Extract the base coin (e.g. "ETH" from "ETH-USD") for SDK calls
        let coin = symbol_name
            .strip_suffix("-USD")
            .unwrap_or(&symbol_name)
            .to_string();

        tracing::info!("Symbol selected: {} (coin: {})", symbol_name, coin);

        // Update order panel symbol
        let order_panel = self.order_panel.clone();
        cx.update_entity(&order_panel, |panel, _cx| {
            panel.symbol = symbol_name;
        });

        // Clone entity handles for async task
        let order_book_clone = self.order_book.clone();
        let candle_chart_clone = self.candle_chart.clone();
        let network = self.network;

        // Spawn async task to re-fetch orderbook and candles for the new symbol
        cx.spawn_in(window, async move |_this, cx| {
            let info = match InfoService::new(network).await {
                Ok(info) => info,
                Err(e) => {
                    tracing::error!("Failed to create InfoService for symbol switch: {}", e);
                    return;
                }
            };

            // Fetch orderbook
            match info.fetch_orderbook(&coin).await {
                Ok(book) => {
                    let _ = cx.update(|_window, cx| {
                        order_book_clone.update(cx, |view, _cx| {
                            view.data = book;
                        });
                    });
                }
                Err(e) => tracing::error!("Failed to fetch orderbook for {}: {}", coin, e),
            }

            // Fetch candles
            match info.fetch_candles(&coin, CandleInterval::H1, 100).await {
                Ok(candles) => {
                    let _ = cx.update(|_window, cx| {
                        candle_chart_clone.update(cx, |chart, _cx| {
                            chart.candles = candles;
                        });
                    });
                }
                Err(e) => tracing::error!("Failed to fetch candles for {}: {}", coin, e),
            }
        })
        .detach();
    }
}

impl Render for MainView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1a1a2e))
            // TopBar
            .child(self.top_bar.clone())
            // Main content area (flex row)
            .child(
                div()
                    .flex_1()
                    .flex()
                    // Left: SymbolList
                    .child(self.symbol_list.clone())
                    // Center: Chart + Order panel (if wallet connected)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            // Chart
                            .child(div().flex_1().child(self.candle_chart.clone()))
                            // Order Panel (only when wallet is connected)
                            .when(self.wallet_connected, |el| {
                                el.child(self.order_panel.clone())
                            }),
                    )
                    // Right: OrderBook
                    .child(self.order_book.clone()),
            )
            // BottomPanel
            .child(div().h(px(250.)).child(self.bottom_panel.clone()))
    }
}
