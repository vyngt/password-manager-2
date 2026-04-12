use leptos::prelude::*;
use uuid::Uuid;

use crate::primitives::tokens::ToastVariant;

/// Internal toast data stored in the reactive list.
#[derive(Clone)]
pub struct ToastData {
    pub id: Uuid,
    pub variant: ToastVariant,
    pub message: String,
    pub action_label: Option<String>,
    pub on_action: Option<Callback<()>>,
    pub duration: u32,
    pub on_dismiss: Option<Callback<()>>,
}

/// Input struct for creating a toast. Use builder methods for convenience.
pub struct ToastInput {
    pub variant: ToastVariant,
    pub message: String,
    pub action_label: Option<String>,
    pub on_action: Option<Callback<()>>,
    pub duration: Option<u32>,
    pub on_dismiss: Option<Callback<()>>,
}

impl ToastInput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            variant: ToastVariant::Default,
            message: message.into(),
            action_label: None,
            on_action: None,
            duration: None,
            on_dismiss: None,
        }
    }

    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn action(mut self, label: impl Into<String>, callback: Callback<()>) -> Self {
        self.action_label = Some(label.into());
        self.on_action = Some(callback);
        self
    }

    pub fn duration(mut self, ms: u32) -> Self {
        self.duration = Some(ms);
        self
    }

    pub fn on_dismiss(mut self, callback: Callback<()>) -> Self {
        self.on_dismiss = Some(callback);
        self
    }
}

/// Reactive state shared via context. Provides `show` and `dismiss`.
#[derive(Clone, Copy)]
pub struct ToastState {
    pub toasts: RwSignal<Vec<ToastData>>,
    /// IDs of toasts currently playing their exit animation.
    pub dismissing: RwSignal<Vec<Uuid>>,
}
