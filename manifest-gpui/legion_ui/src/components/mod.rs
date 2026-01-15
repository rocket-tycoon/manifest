//! Reusable UI components for Legion
//!
//! These components are inspired by Zed's UI library and provide
//! consistent styling and behavior across the application.

mod disclosure;
mod list_item;
mod resize_handle;

// Disclosure is available for standalone use but ListItem handles toggles inline
#[allow(unused_imports)]
pub use disclosure::Disclosure;
pub use list_item::{ListItem, ListItemSpacing};
pub use resize_handle::{ResizeAxis, ResizeDrag, ResizeHandle};
