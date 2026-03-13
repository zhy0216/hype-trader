use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, Entity, Subscription};

use crate::models::*;
use crate::services::info_service::InfoService;
use crate::services::wallet_service;
use crate::services::ws_service::{WsService, WsUpdate};
use crate::views::bottom_panel::BottomPanel;
use crate::views::candle_chart::{CandleChart, IntervalChanged};
use crate::views::order_book::OrderBookView;
use crate::views::order_panel::OrderPanel;
use crate::views::symbol_list::{SymbolList, SymbolSelected};
use crate::views::toast::{Toast, ToastKind};
use crate::views::top_bar::TopBar;

pub struct MainView {
    top_bar: Entity<TopBar>,
    symbol_list: Entity<SymbolList>,
    candle_chart: Entity<CandleChart>,
    order_book: Entity<OrderBookView>,
    order_panel: Entity<OrderPanel>,
    bottom_panel: Entity<BottomPanel>,
    toast: Entity<Toast>,
    is_loading: bool,
    wallet_connected: bool,
    pub private_key: Option<String>,
    pub network: Network,
    _symbol_subscription: Subscription,
    _interval_subscription: Subscription,
    /// Channel to tell the WS task to switch symbol subscriptions
    symbol_switch_tx: tokio::sync::mpsc::UnboundedSender<String>,
    /// Channel to tell the WS task to switch candle interval
    interval_switch_tx: tokio::sync::mpsc::UnboundedSender<CandleInterval>,
    /// Current coin (e.g. "ETH") for REST fetches on interval change
    current_coin: String,
    /// Shared InfoService for REST fetches (reused across symbol switches)
    info_service: Arc<tokio::sync::OnceCell<InfoService>>,
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

        // Create Toast entity (shared with child panels)
        let toast = cx.new(|_cx| Toast::new());

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

        // Create OrderPanel (pass toast handle)
        let order_panel = cx.new(|cx| OrderPanel::new(wallet_connected, private_key.clone(), network, toast.clone(), window, cx));

        // Create BottomPanel empty (pass toast handle)
        let bottom_panel = cx.new(|cx| BottomPanel::new(private_key.clone(), network, toast.clone(), window, cx));

        // Subscribe to symbol selection changes
        let _symbol_subscription =
            cx.subscribe_in(&symbol_list, window, Self::on_symbol_selected);

        // Subscribe to interval changes from the candle chart
        let _interval_subscription =
            cx.subscribe_in(&candle_chart, window, Self::on_interval_changed);

        // Clone entity handles for the async task
        let symbol_list_clone = symbol_list.clone();
        let candle_chart_clone = candle_chart.clone();
        let order_book_clone = order_book.clone();
        let top_bar_clone = top_bar.clone();
        let bottom_panel_clone = bottom_panel.clone();
        let toast_clone = toast.clone();
        let pk_clone = private_key.clone();

        // Channel for symbol switch commands from on_symbol_selected → WS task
        let (symbol_switch_tx, mut symbol_switch_rx) =
            tokio::sync::mpsc::unbounded_channel::<String>();

        // Channel for interval switch commands from on_interval_changed → WS task
        let (interval_switch_tx, mut interval_switch_rx) =
            tokio::sync::mpsc::unbounded_channel::<CandleInterval>();

        // Shared InfoService: created once, reused for all REST fetches
        let info_service: Arc<tokio::sync::OnceCell<InfoService>> = Arc::new(tokio::sync::OnceCell::new());
        let info_cell_clone = info_service.clone();

        // Spawn async task to fetch real data, then start WebSocket subscriptions
        cx.spawn(async move |this, cx| {
            // Create InfoService once and store in the shared cell
            let info = match InfoService::new(network).await {
                Ok(info) => info,
                Err(e) => {
                    tracing::error!("Failed to create InfoService: {}", e);
                    return;
                }
            };
            let _ = info_cell_clone.set(info);
            let info = info_cell_clone.get().unwrap();

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

            // Initial data loaded, hide loading overlay
            let _ = this.update(cx, |view, cx| {
                view.is_loading = false;
                cx.notify();
            });

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

                // Track current coin for filtering stale WS updates
                let mut current_coin = "ETH".to_string();
                let mut current_interval = CandleInterval::H1;

                // Subscribe to L2 book for the default symbol
                let mut l2_sub_id = None;
                match ws.subscribe_l2_book(&current_coin, tx.clone()).await {
                    Ok(id) => l2_sub_id = Some(id),
                    Err(e) => tracing::error!("Failed to subscribe to L2 book: {}", e),
                }

                // Subscribe to candles for default symbol + interval
                let mut candle_sub_id = None;
                match ws.subscribe_candles(&current_coin, CandleInterval::H1, tx.clone()).await {
                    Ok(id) => candle_sub_id = Some(id),
                    Err(e) => tracing::error!("Failed to subscribe to candles: {}", e),
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
                // Show reconnected toast (only after a reconnect, not the first connect)
                if backoff_secs > 3 {
                    let _ = cx.update_entity(&toast_clone, |t, cx| {
                        t.show("Reconnected", ToastKind::Success, cx);
                    });
                }
                backoff_secs = 3;

                tracing::info!("WebSocket subscriptions active, entering recv loop");

                // Receive loop: route WsUpdate messages to UI entities.
                // Also listen for symbol switch commands to resubscribe.
                // Keep `ws` alive in scope so subscriptions stay open.
                loop {
                    tokio::select! {
                        update = rx.recv() => {
                            let Some(update) = update else { break };
                            match update {
                                WsUpdate::OrderBookUpdate(coin, book) => {
                                    if coin == current_coin {
                                        let _ = cx.update_entity(&order_book_clone, |view, cx| {
                                            view.data = book;
                                            cx.notify();
                                        });
                                    }
                                }
                                WsUpdate::CandleUpdate(coin, candle) => {
                                    if coin == current_coin {
                                        let _ = cx.update_entity(&candle_chart_clone, |chart, cx| {
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
                                            cx.notify();
                                        });
                                    }
                                }
                                WsUpdate::AllMids(mids) => {
                                    let _ = cx.update_entity(&symbol_list_clone, |list, cx| {
                                        list.update_prices(&mids);
                                        cx.notify();
                                    });
                                }
                                WsUpdate::OrderUpdate(info) => {
                                    tracing::info!("Order update: {}", info);
                                }
                                WsUpdate::UserFill(fill) => {
                                    let _ = cx.update_entity(&bottom_panel_clone, |panel, cx| {
                                        panel.trade_history.insert(0, fill);
                                        cx.notify();
                                    });
                                }
                                WsUpdate::TradesUpdate(_) => {}
                            }
                        }
                        new_coin = symbol_switch_rx.recv() => {
                            let Some(new_coin) = new_coin else { break };
                            if new_coin == current_coin {
                                continue;
                            }
                            let wst0 = std::time::Instant::now();
                            tracing::info!("[ws-switch] {} -> {}: start", current_coin, new_coin);

                            // Unsubscribe old L2 book and candle subs
                            if let Some(id) = l2_sub_id.take() {
                                ws.unsubscribe(id).await;
                            }
                            if let Some(id) = candle_sub_id.take() {
                                ws.unsubscribe(id).await;
                            }
                            tracing::info!("[ws-switch] unsubscribed old: {:?}", wst0.elapsed());

                            current_coin = new_coin;

                            // Subscribe to new symbol
                            match ws.subscribe_l2_book(&current_coin, tx.clone()).await {
                                Ok(id) => l2_sub_id = Some(id),
                                Err(e) => tracing::error!("[ws-switch] subscribe L2 book FAILED for {}: {}", current_coin, e),
                            }
                            tracing::info!("[ws-switch] l2_book subscribed: {:?}", wst0.elapsed());
                            match ws.subscribe_candles(&current_coin, current_interval, tx.clone()).await {
                                Ok(id) => candle_sub_id = Some(id),
                                Err(e) => tracing::error!("[ws-switch] subscribe candles FAILED for {}: {}", current_coin, e),
                            }
                            tracing::info!("[ws-switch] candles subscribed: {:?} TOTAL", wst0.elapsed());
                        }
                        new_interval = interval_switch_rx.recv() => {
                            let Some(new_interval) = new_interval else { break };
                            if new_interval == current_interval {
                                continue;
                            }
                            let wst0 = std::time::Instant::now();
                            tracing::info!("[ws-interval] {:?} -> {:?}: start", current_interval, new_interval);

                            // Unsubscribe old candle sub only
                            if let Some(id) = candle_sub_id.take() {
                                ws.unsubscribe(id).await;
                            }

                            current_interval = new_interval;

                            // Subscribe to candles with new interval
                            match ws.subscribe_candles(&current_coin, current_interval, tx.clone()).await {
                                Ok(id) => candle_sub_id = Some(id),
                                Err(e) => tracing::error!("[ws-interval] subscribe candles FAILED: {}", e),
                            }
                            tracing::info!("[ws-interval] candles resubscribed: {:?}", wst0.elapsed());
                        }
                    }
                }

                // If we exit the recv loop, WS connection was lost
                tracing::warn!("WebSocket disconnected, reconnecting in {}s...", backoff_secs);
                let _ = ws.unsubscribe_all().await;
                let _ = cx.update_entity(&top_bar_clone, |bar, _cx| {
                    bar.connection_status = ConnectionStatus::Disconnected;
                });
                let _ = cx.update_entity(&toast_clone, |t, cx| {
                    t.show("Connection lost", ToastKind::Info, cx);
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
            toast,
            is_loading: true,
            wallet_connected,
            private_key,
            network,
            _symbol_subscription,
            _interval_subscription,
            symbol_switch_tx,
            interval_switch_tx,
            current_coin: "ETH".to_string(),
            info_service,
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

        // Update current coin
        self.current_coin = coin.clone();

        // Tell the WS task to switch L2 book + candle subscriptions
        let _ = self.symbol_switch_tx.send(coin.clone());

        // Update order panel symbol
        let order_panel = self.order_panel.clone();
        cx.update_entity(&order_panel, |panel, _cx| {
            panel.symbol = symbol_name;
        });

        // Clone entity handles for async task
        let order_book_clone = self.order_book.clone();
        let candle_chart_clone = self.candle_chart.clone();
        let info_cell = self.info_service.clone();

        // Show loading state immediately and get current interval
        let current_interval = cx.read_entity(&self.candle_chart, |chart, _cx| chart.interval);
        cx.update_entity(&self.candle_chart, |chart, cx| { chart.loading = true; cx.notify(); });
        cx.update_entity(&self.order_book, |view, cx| { view.loading = true; cx.notify(); });

        // Spawn async task to re-fetch orderbook and candles in parallel
        cx.spawn_in(window, async move |_this, cx| {
            let t0 = std::time::Instant::now();

            let Some(info) = info_cell.get() else {
                tracing::error!("InfoService not yet initialized");
                return;
            };
            tracing::info!("[switch {}] info_service ready: {:?}", coin, t0.elapsed());

            // Fetch orderbook and candles in parallel
            let (book_result, candles_result) = tokio::join!(
                info.fetch_orderbook(&coin),
                info.fetch_candles(&coin, current_interval, 100),
            );
            tracing::info!("[switch {}] REST fetches done: {:?}", coin, t0.elapsed());

            if let Ok(book) = book_result {
                let _ = cx.update(|_window, cx| {
                    order_book_clone.update(cx, |view, cx| {
                        view.data = book;
                        view.loading = false;
                        cx.notify();
                    });
                });
                tracing::info!("[switch {}] orderbook UI updated: {:?}", coin, t0.elapsed());
            } else if let Err(e) = book_result {
                let _ = cx.update(|_window, cx| {
                    order_book_clone.update(cx, |view, cx| { view.loading = false; cx.notify(); });
                });
                tracing::error!("[switch {}] fetch orderbook FAILED: {} ({:?})", coin, e, t0.elapsed());
            }

            if let Ok(candles) = candles_result {
                let _ = cx.update(|_window, cx| {
                    candle_chart_clone.update(cx, |chart, cx| {
                        chart.candles = candles;
                        chart.loading = false;
                        cx.notify();
                    });
                });
                tracing::info!("[switch {}] candles UI updated: {:?}", coin, t0.elapsed());
            } else if let Err(e) = candles_result {
                let _ = cx.update(|_window, cx| {
                    candle_chart_clone.update(cx, |chart, cx| { chart.loading = false; cx.notify(); });
                });
                tracing::error!("[switch {}] fetch candles FAILED: {} ({:?})", coin, e, t0.elapsed());
            }

            tracing::info!("[switch {}] TOTAL: {:?}", coin, t0.elapsed());
        })
        .detach();
    }

    /// Called when the user selects a different candle interval in the CandleChart.
    fn on_interval_changed(
        &mut self,
        _candle_chart: &Entity<CandleChart>,
        event: &IntervalChanged,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let interval = event.0;
        let coin = self.current_coin.clone();

        tracing::info!("Interval changed to {:?} for coin {}", interval, coin);

        // Tell the WS task to switch candle interval subscription
        let _ = self.interval_switch_tx.send(interval);

        // Show loading state and fetch new candle data
        let candle_chart_clone = self.candle_chart.clone();
        let info_cell = self.info_service.clone();

        cx.update_entity(&self.candle_chart, |chart, cx| { chart.loading = true; cx.notify(); });

        cx.spawn_in(window, async move |_this, cx| {
            let Some(info) = info_cell.get() else {
                tracing::error!("InfoService not yet initialized");
                return;
            };

            match info.fetch_candles(&coin, interval, 100).await {
                Ok(candles) => {
                    let _ = cx.update(|_window, cx| {
                        candle_chart_clone.update(cx, |chart, cx| {
                            chart.candles = candles;
                            chart.scroll_offset = 0;
                            chart.loading = false;
                            cx.notify();
                        });
                    });
                }
                Err(e) => {
                    let _ = cx.update(|_window, cx| {
                        candle_chart_clone.update(cx, |chart, cx| { chart.loading = false; cx.notify(); });
                    });
                    tracing::error!("Failed to fetch candles for interval {:?}: {}", interval, e);
                }
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
        let is_loading = self.is_loading;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(crate::components::theme::bg_primary())
            .relative()
            // TopBar
            .child(self.top_bar.clone())
            // Toast notification (below TopBar, above content)
            .child(self.toast.clone())
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
            .child(
                div()
                    .h(px(250.))
                    .border_t_1()
                    .border_color(crate::components::theme::border_primary())
                    .child(self.bottom_panel.clone()),
            )
            // Loading overlay
            .when(is_loading, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(crate::components::theme::bg_primary())
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(12.))
                                .child(
                                    div()
                                        .text_color(crate::components::theme::color_brand())
                                        .text_size(px(20.))
                                        .child("Hype Trader"),
                                )
                                .child(
                                    div()
                                        .text_color(crate::components::theme::text_dim())
                                        .text_size(px(14.))
                                        .child("Loading..."),
                                ),
                        ),
                )
            })
    }
}
