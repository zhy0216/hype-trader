use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, rgb};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub visible: bool,
}

impl Toast {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            kind: ToastKind::Info,
            visible: false,
        }
    }

    pub fn show(
        &mut self,
        message: impl Into<String>,
        kind: ToastKind,
        cx: &mut gpui::Context<Self>,
    ) {
        self.message = message.into();
        self.kind = kind;
        self.visible = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let _ = this.update(cx, |t, cx| {
                t.visible = false;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for Toast {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        if !self.visible {
            return div();
        }

        let (bg_color, border_color, icon) = match self.kind {
            ToastKind::Success => (rgb(0x0f2a1a), rgb(0x22c55e), "OK"),
            ToastKind::Error => (rgb(0x2a0f0f), rgb(0xef4444), "!!"),
            ToastKind::Info => (rgb(0x0f1a2a), rgb(0x6366f1), "i"),
        };

        div()
            .w_full()
            .flex()
            .justify_center()
            .py(px(4.))
            .child(
                div()
                    .px(px(16.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .bg(bg_color)
                    .border_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(border_color)
                            .child(icon),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(0xeaedf3))
                            .child(self.message.clone()),
                    ),
            )
    }
}
