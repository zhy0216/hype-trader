use gpui::prelude::*;
use gpui::{div, Entity, EventEmitter};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};

use crate::models::Network;
use crate::services::wallet_service;

/// Events emitted by WelcomeView to signal screen transitions.
pub enum WelcomeEvent {
    /// User connected with a valid wallet. Carries (private_key, network).
    ConnectWallet { private_key: String, network: Network },
    /// User chose read-only browsing. Carries the selected network.
    BrowseReadOnly { network: Network },
}

impl EventEmitter<WelcomeEvent> for WelcomeView {}

pub struct WelcomeView {
    network: Network,
    key_input: Entity<InputState>,
    error_message: Option<String>,
}

impl WelcomeView {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        let key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter private key (0x...)")
                .masked(true)
        });
        Self {
            network: Network::Mainnet,
            key_input,
            error_message: None,
        }
    }
}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgb(0x1a1a2e))
            .child(
                div()
                    .w(gpui::px(420.))
                    .p(gpui::px(32.))
                    .rounded(gpui::px(12.))
                    .bg(gpui::rgb(0x16213e))
                    .border_1()
                    .border_color(gpui::rgb(0x0f3460))
                    .flex()
                    .flex_col()
                    .gap(gpui::px(20.))
                    // Title
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(gpui::px(24.))
                                    .text_color(gpui::rgb(0xe94560))
                                    .child("Hype Trader"),
                            ),
                    )
                    // Subtitle
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(gpui::rgb(0x888888))
                                    .child("Hyperliquid Trading Client"),
                            ),
                    )
                    // Network selector
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(gpui::px(6.))
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(gpui::rgb(0xaaaaaa))
                                    .child("Network"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(gpui::px(8.))
                                    .child(
                                        Button::new("mainnet")
                                            .label("Mainnet")
                                            .compact()
                                            .map(|btn| {
                                                if self.network == Network::Mainnet {
                                                    btn.primary()
                                                } else {
                                                    btn.ghost()
                                                }
                                            })
                                            .on_click(cx.listener(|this, _, _w, _cx| {
                                                this.network = Network::Mainnet;
                                            })),
                                    )
                                    .child(
                                        Button::new("testnet")
                                            .label("Testnet")
                                            .compact()
                                            .map(|btn| {
                                                if self.network == Network::Testnet {
                                                    btn.primary()
                                                } else {
                                                    btn.ghost()
                                                }
                                            })
                                            .on_click(cx.listener(|this, _, _w, _cx| {
                                                this.network = Network::Testnet;
                                            })),
                                    ),
                            ),
                    )
                    // Private key input
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(gpui::px(6.))
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(gpui::rgb(0xaaaaaa))
                                    .child("Private Key"),
                            )
                            .child(Input::new(&self.key_input)),
                    )
                    // Error message (if any)
                    .when_some(self.error_message.clone(), |el, msg| {
                        el.child(
                            div()
                                .text_size(gpui::px(12.))
                                .text_color(gpui::rgb(0xff4444))
                                .child(msg),
                        )
                    })
                    // Connect button
                    .child(
                        Button::new("connect")
                            .label("Connect Wallet")
                            .primary()
                            .w_full()
                            .on_click(cx.listener(|this, _, _w, cx| {
                                let key_text = this.key_input.read(cx).value();
                                let key_str = key_text.to_string().trim().to_string();
                                if key_str.is_empty() {
                                    this.error_message = Some("Please enter a private key.".into());
                                    return;
                                }
                                match wallet_service::address_from_key(&key_str) {
                                    Ok(_) => {
                                        this.error_message = None;
                                        cx.emit(WelcomeEvent::ConnectWallet {
                                            private_key: key_str,
                                            network: this.network,
                                        });
                                    }
                                    Err(e) => {
                                        this.error_message = Some(format!("Invalid key: {}", e));
                                    }
                                }
                            })),
                    )
                    // Divider text
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(gpui::px(12.))
                                    .text_color(gpui::rgb(0x666666))
                                    .child("or"),
                            ),
                    )
                    // Read-only mode
                    .child(
                        Button::new("readonly")
                            .label("Browse Market (Read-only)")
                            .ghost()
                            .w_full()
                            .on_click(cx.listener(|this, _, _w, cx| {
                                cx.emit(WelcomeEvent::BrowseReadOnly {
                                    network: this.network,
                                });
                            })),
                    ),
            )
    }
}
