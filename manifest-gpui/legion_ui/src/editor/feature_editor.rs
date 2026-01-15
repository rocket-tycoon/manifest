use gpui::*;
use gpui::prelude::*;
use legion_db::{Database, Feature, FeatureStatus, Session, SessionStatus, Task};
use std::sync::Arc;

use super::MarkdownRenderer;
use crate::theme::Theme;

/// Event emitted when the user wants to start a session
pub struct StartSession {
    pub feature_id: String,
}

impl EventEmitter<StartSession> for FeatureEditor {}

/// Feature editor component displaying feature content and controls
pub struct FeatureEditor {
    db: Arc<Database>,
    feature: Option<Feature>,
    sessions: Vec<Session>,
    active_tasks: Vec<Task>,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
}

impl FeatureEditor {
    pub fn new(db: Arc<Database>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            db,
            feature: None,
            sessions: Vec::new(),
            active_tasks: Vec::new(),
            focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
        }
    }

    /// Open a feature for editing
    pub fn open(&mut self, feature: Feature, cx: &mut Context<Self>) {
        let feature_id = feature.id.clone();
        self.feature = Some(feature);
        self.load_sessions(&feature_id);
        cx.notify();
    }

    /// Close the current feature
    #[allow(dead_code)]
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.feature = None;
        self.sessions.clear();
        self.active_tasks.clear();
        cx.notify();
    }

    /// Clear the editor (alias for close)
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.close(cx);
    }

    /// Load sessions for the current feature
    fn load_sessions(&mut self, feature_id: &str) {
        match self.db.with_connection(|conn| Session::list_by_feature(conn, feature_id)) {
            Ok(sessions) => {
                self.sessions = sessions;
            }
            Err(e) => {
                tracing::error!("Failed to load sessions: {}", e);
            }
        }
    }

    /// Get the currently open feature
    pub fn feature(&self) -> Option<&Feature> {
        self.feature.as_ref()
    }

    /// Render the header with feature title and status
    fn render_header(&self, feature: &Feature) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .pb_4()
            .border_b_1()
            .border_color(Theme::border())
            // Title
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::text())
                    .child(feature.title.clone())
            )
            // Status and metadata row
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    // Status badge
                    .child(self.render_status_badge(&feature.status))
                    // Version badge
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_xs()
                            .bg(Theme::element())
                            .text_color(Theme::text_muted())
                            .child(format!("v{}", feature.iteration))
                    )
                    // Session count
                    .when(!self.sessions.is_empty(), |el| {
                        el.child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_xs()
                                .bg(Theme::element())
                                .text_color(Theme::text_muted())
                                .child(format!("{} sessions", self.sessions.len()))
                        )
                    })
            )
    }

    /// Render a status badge
    fn render_status_badge(&self, status: &FeatureStatus) -> impl IntoElement {
        let (bg, text_color, label) = match status {
            FeatureStatus::Draft => (Theme::element(), Theme::text_muted(), "DRAFT"),
            FeatureStatus::Active => (Theme::success_bg(), Theme::success(), "ACTIVE"),
            FeatureStatus::Paused => (Theme::warning_bg(), Theme::warning(), "PAUSED"),
            FeatureStatus::Archived => (Theme::surface(), Theme::text_muted(), "ARCHIVED"),
        };

        div()
            .px_2()
            .py_1()
            .rounded_md()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .bg(bg)
            .text_color(text_color)
            .child(label)
    }

    /// Render the action bar with session controls
    fn render_actions(&self, feature: &Feature, cx: &mut Context<Self>) -> impl IntoElement {
        let feature_id = feature.id.clone();
        let has_active_session = self.sessions.iter().any(|s| {
            matches!(s.status, SessionStatus::Active | SessionStatus::Created)
        });

        div()
            .flex()
            .flex_row()
            .gap_2()
            .py_3()
            .border_b_1()
            .border_color(Theme::border())
            // New session button
            .when(!has_active_session, |el| {
                el.child(
                    div()
                        .id("start-session-btn")
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(Theme::element())
                        .text_color(Theme::text())
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .hover(|s| s.bg(Theme::element_hover()))
                        .on_click(cx.listener(move |_this, _event, _window, cx| {
                            cx.emit(StartSession { feature_id: feature_id.clone() });
                        }))
                        .child("Start Session")
                )
            })
            // Show active session indicator
            .when(has_active_session, |el| {
                el.child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(Theme::success_bg())
                        .text_color(Theme::success())
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w_2()
                                .h_2()
                                .rounded_full()
                                .bg(Theme::success())
                        )
                        .child("Session Active")
                )
            })
            // Edit button
            .child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(Theme::element())
                    .text_color(Theme::text())
                    .text_sm()
                    .hover(|s| s.bg(Theme::element_hover()))
                    .child("Edit")
            )
    }

    /// Render the markdown content
    fn render_content(&self, feature: &Feature) -> impl IntoElement {
        let content = feature.content.as_deref().unwrap_or("*No content yet. Click Edit to add a description.*");

        div()
            .flex_1()
            .py_4()
            .child(MarkdownRenderer::render(content))
    }

    /// Render session history
    fn render_sessions(&self) -> impl IntoElement {
        if self.sessions.is_empty() {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(Theme::border())
            // Header
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(Theme::text_muted())
                    .child("SESSION HISTORY")
            )
            // Session list
            .children(self.sessions.iter().map(|session| {
                let status_color = match session.status {
                    SessionStatus::Created => Theme::text_muted(),
                    SessionStatus::Active => Theme::success(),
                    SessionStatus::Review => Theme::warning(),
                    SessionStatus::Squashed => Theme::info(),
                    SessionStatus::Failed => Theme::error(),
                };

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_2()
                    .rounded_md()
                    .hover(|s| s.bg(Theme::element_hover()))
                    .cursor_pointer()
                    .child(
                        div()
                            .w_2()
                            .h_2()
                            .rounded_full()
                            .bg(status_color)
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(Theme::text())
                            .child(session.title.clone())
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .child(session.status.as_str())
                    )
            }))
            .into_any_element()
    }

    /// Render the empty state
    fn render_empty(&self) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_color(Theme::text_muted())
                            .text_lg()
                            .child("No feature selected")
                    )
                    .child(
                        div()
                            .text_color(Theme::text_placeholder())
                            .text_sm()
                            .child("Select a feature from the sidebar to view its content")
                    )
            )
    }
}

impl Render for FeatureEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = if let Some(feature) = &self.feature {
            div()
                .id("feature-editor-content")
                .size_full()
                .flex()
                .flex_col()
                .p_4()
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
                // Header
                .child(self.render_header(feature))
                // Actions
                .child(self.render_actions(feature, cx))
                // Content
                .child(self.render_content(feature))
                // Sessions
                .child(self.render_sessions())
                .into_any_element()
        } else {
            self.render_empty().into_any_element()
        };

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(Theme::background())
            .child(content)
    }
}
