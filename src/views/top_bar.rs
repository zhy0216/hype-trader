use gpui::prelude::*;
use gpui::div;
use crate::components::theme::*;
use crate::components::toggle_button::toggle_button;
use crate::components::status_dot::status_dot;

use crate::models::{ConnectionStatus, Network, ThemeMode};
use gpui_component::button::{Button, ButtonVariants as _};

pub struct TopBar {
    pub network: Network,
    pub connection_status: ConnectionStatus,
    pub theme: ThemeMode,
    pub balance: f64,
    pub address: Option<String>,
}

impl TopBar {
    pub fn new(
        network: Network,
        connection_status: ConnectionStatus,
        theme: ThemeMode,
        balance: f64,
        address: Option<String>,
    ) -> Self {
        Self {
            network,
            connection_status,
            theme,
            balance,
            address,
        }
    }
}

impl Render for TopBar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let status_color = match self.connection_status {
            ConnectionStatus::Connected => color_green(),
            ConnectionStatus::Connecting => color_yellow(),
            ConnectionStatus::Disconnected => color_red(),
        };

        div()
            .h(gpui::px(48.))
            .w_full()
            .bg(bg_header())
            .border_b_1()
            .border_color(border_accent())
            .flex()
            .items_center()
            .justify_between()
            .px(gpui::px(16.))
            // Left section - branding
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(12.))
                    .child(
                        div()
                            .text_size(gpui::px(18.))
                            .text_color(color_brand())
                            .child("Hype Trader"),
                    ),
            )
            // Center section - network toggle + connection status
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(8.))
                    // Status dot
                    .child(status_dot(status_color))
                    .child(
                        toggle_button("mainnet-btn", "Mainnet", self.network == Network::Mainnet)
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.network = Network::Mainnet;
                            })),
                    )
                    .child(
                        toggle_button("testnet-btn", "Testnet", self.network == Network::Testnet)
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.network = Network::Testnet;
                            })),
                    ),
            )
            // Right section - balance + address + theme toggle + settings
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(12.))
                    .child(
                        div()
                            .text_size(gpui::px(14.))
                            .text_color(color_green())
                            .child(format!("${:.2}", self.balance)),
                    )
                    .when_some(self.address.clone(), |el, addr| {
                        el.child(
                            div()
                                .text_size(gpui::px(12.))
                                .text_color(text_dim())
                                .child(addr),
                        )
                    })
                    .child(
                        Button::new("theme-toggle")
                            .label(match self.theme {
                                ThemeMode::Dark => "Light",
                                ThemeMode::Light => "Dark",
                            })
                            .compact()
                            .ghost()
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.theme = match this.theme {
                                    ThemeMode::Dark => ThemeMode::Light,
                                    ThemeMode::Light => ThemeMode::Dark,
                                };
                            })),
                    )
                    .child(
                        Button::new("settings-btn")
                            .label("Settings")
                            .compact()
                            .ghost(),
                    ),
            )
    }
}
