use gpui::*;
use legion_db::{Database, Project};
use std::sync::Arc;

use super::feature_tree::{FeatureSelected, FeatureTree, OpenProjectRequested};
use crate::theme::Theme;

/// Event emitted when a feature is selected in the sidebar
pub struct SidebarFeatureSelected {
    pub feature_id: String,
}

/// Event emitted when user wants to open a project from the sidebar
pub struct SidebarOpenProjectRequested;

impl EventEmitter<SidebarFeatureSelected> for Sidebar {}
impl EventEmitter<SidebarOpenProjectRequested> for Sidebar {}

/// The sidebar with feature tree
pub struct Sidebar {
    #[allow(dead_code)]
    db: Arc<Database>,
    feature_tree: Entity<FeatureTree>,
    focus_handle: FocusHandle,
}

impl Sidebar {
    pub fn new(db: Arc<Database>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let feature_tree = cx.new(|cx| FeatureTree::new(db.clone(), window, cx));

        // Subscribe to feature selection and re-emit
        cx.subscribe(&feature_tree, |_this, _tree, event: &FeatureSelected, cx| {
            cx.emit(SidebarFeatureSelected {
                feature_id: event.feature_id.clone(),
            });
        })
        .detach();

        // Subscribe to open project requests and re-emit
        cx.subscribe(&feature_tree, |_this, _tree, _event: &OpenProjectRequested, cx| {
            cx.emit(SidebarOpenProjectRequested);
        })
        .detach();

        Self {
            db,
            feature_tree,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Set the active project
    pub fn set_project(&mut self, project: Project, cx: &mut Context<Self>) {
        self.feature_tree.update(cx, |tree, cx| {
            tree.set_project(project, cx);
        });
        cx.notify();
    }

    /// Get the feature tree view for external access
    #[allow(dead_code)]
    pub fn feature_tree(&self) -> &Entity<FeatureTree> {
        &self.feature_tree
    }

    /// Refresh all data
    #[allow(dead_code)]
    pub fn refresh(&self, cx: &mut Context<Self>) {
        self.feature_tree.update(cx, |tree, cx| {
            tree.refresh(cx);
        });
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .w(px(260.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(Theme::surface())
            .border_r_1()
            .border_color(Theme::border())
            // Feature tree fills the sidebar
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.feature_tree.clone()),
            )
            // Action buttons at bottom
            .child(
                div()
                    .p_2()
                    .border_t_1()
                    .border_color(Theme::border())
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("new-module-btn")
                            .flex_1()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .bg(Theme::element())
                            .border_1()
                            .border_color(Theme::border())
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(Theme::element_hover()).text_color(Theme::text()))
                            .child("+ Module"),
                    )
                    .child(
                        div()
                            .id("new-feature-btn")
                            .flex_1()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .bg(Theme::element())
                            .border_1()
                            .border_color(Theme::border())
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(Theme::element_hover()).text_color(Theme::text()))
                            .child("+ Feature"),
                    ),
            )
    }
}
