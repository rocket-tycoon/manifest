//! Global active feature context for cross-component state sharing.

use gpui::{App, Global};
use uuid::Uuid;

/// Global context for the currently active/selected feature.
///
/// Any component can access this via `ActiveFeatureContext::get(cx)` to know
/// which feature is currently being edited. This enables workflows where
/// terminals can operate on the active feature without explicit communication.
#[derive(Clone, Debug, Default)]
pub struct ActiveFeatureContext {
    pub feature_id: Option<Uuid>,
    pub feature_title: Option<String>,
    pub feature_details: Option<String>,
}

impl Global for ActiveFeatureContext {}

impl ActiveFeatureContext {
    /// Get the active feature context (returns default if not set).
    pub fn get(cx: &App) -> Self {
        cx.try_global::<Self>().cloned().unwrap_or_default()
    }

    /// Update the active feature context.
    pub fn set(ctx: Self, cx: &mut App) {
        cx.set_global(ctx);
    }

    /// Clear the active feature (no feature selected).
    pub fn clear(cx: &mut App) {
        cx.set_global(Self::default());
    }
}
