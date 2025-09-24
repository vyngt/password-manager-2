use leptos::prelude::RwSignal;
use uuid::Uuid;

use crate::primitives::color::RgbColor;

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub duration_ms: u64,
    pub color: RgbColor,
}

#[derive(Clone, Debug)]
pub struct ToastInput {
    pub title: String,
    pub description: Option<String>,
    pub duration_ms: Option<u64>,
    pub color: RgbColor,
}

impl ToastInput {
    pub fn new<T: Into<String>>(
        title: T,
        description: Option<T>,
        duration_ms: Option<u64>,
        color: RgbColor,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.map(Into::into),
            duration_ms,
            color,
        }
    }
}

#[derive(Clone)]
pub struct ToastState {
    pub toasts: RwSignal<Vec<Toast>>,
}
