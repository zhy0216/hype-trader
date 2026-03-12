use gpui::prelude::*;
use gpui::{div, px, rgb, Entity};

use crate::models::*;
use crate::services::info_service::InfoService;
use crate::views::bottom_panel::BottomPanel;
use crate::views::candle_chart::CandleChart;
use crate::views::order_book::OrderBookView;
use crate::views::order_panel::OrderPanel;
use crate::views::symbol_list::SymbolList;
use crate::views::top_bar::TopBar;

pub struct MainView {
    top_bar: Entity<TopBar>,
    symbol_list: Entity<SymbolList>,
    candle_chart: Entity<CandleChart>,
    order_book: Entity<OrderBookView>,
    order_panel: Entity<OrderPanel>,
    bottom_panel: Entity<BottomPanel>,
}

impl MainView {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        // Create TopBar with defaults (Connecting state, no address yet)
        let top_bar = cx.new(|_cx| {
            TopBar::new(
                Network::Mainnet,
                ConnectionStatus::Connecting,
                ThemeMode::Dark,
                0.0,
                None,
            )
        });

        // Create SymbolList with empty symbols (will be populated async)
        let symbol_list = cx.new(|cx| SymbolList::new(window, cx));

        // Create CandleChart empty
        let candle_chart = cx.new(|_cx| CandleChart::new());

        // Create OrderBook empty
        let order_book = cx.new(|_cx| OrderBookView::new());

        // Create OrderPanel
        let order_panel = cx.new(|cx| OrderPanel::new(window, cx));

        // Create BottomPanel empty
        let bottom_panel = cx.new(|cx| BottomPanel::new(window, cx));

        // Clone entity handles for the async task
        let symbol_list_clone = symbol_list.clone();
        let candle_chart_clone = candle_chart.clone();
        let order_book_clone = order_book.clone();
        let top_bar_clone = top_bar.clone();

        // Spawn async task to fetch real data
        cx.spawn(async move |_this, cx| {
            // Create InfoService
            let info = match InfoService::new(Network::Mainnet).await {
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

            // Update top bar to connected
            let _ = cx.update_entity(&top_bar_clone, |bar, _cx| {
                bar.connection_status = ConnectionStatus::Connected;
            });
        })
        .detach();

        Self {
            top_bar,
            symbol_list,
            candle_chart,
            order_book,
            order_panel,
            bottom_panel,
        }
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
                    // Center: Chart + Order panel
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            // Chart
                            .child(div().flex_1().child(self.candle_chart.clone()))
                            // Order Panel
                            .child(self.order_panel.clone()),
                    )
                    // Right: OrderBook
                    .child(self.order_book.clone()),
            )
            // BottomPanel
            .child(div().h(px(250.)).child(self.bottom_panel.clone()))
    }
}
