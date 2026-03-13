use gpui::prelude::*;
use gpui::{ElementId, SharedString};
use gpui_component::button::{Button, ButtonVariants as _};

/// A button that shows primary style when active, ghost when inactive.
pub fn toggle_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
) -> Button {
    Button::new(id)
        .label(label)
        .compact()
        .map(|b| if active { b.primary() } else { b.ghost() })
}
