use gpui::*;
use gpui::prelude::*;
use legion_db::{Database, Project};
use std::sync::Arc;

use crate::theme::Theme;

/// Event emitted when a project is selected
pub struct ProjectSelected {
    pub project: Project,
}

impl EventEmitter<ProjectSelected> for ProjectPicker {}

/// Dropdown for selecting the active project
pub struct ProjectPicker {
    db: Arc<Database>,
    projects: Vec<Project>,
    selected: Option<Project>,
    is_open: bool,
    focus_handle: FocusHandle,
}

impl ProjectPicker {
    pub fn new(db: Arc<Database>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut picker = Self {
            db,
            projects: Vec::new(),
            selected: None,
            is_open: false,
            focus_handle: cx.focus_handle(),
        };

        picker.refresh(cx);
        picker
    }

    /// Refresh the project list from database
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let db = self.db.clone();

        match db.with_connection(|conn| Project::list_all(conn)) {
            Ok(projects) => {
                // Auto-select first project if none selected
                if self.selected.is_none() && !projects.is_empty() {
                    let first = projects[0].clone();
                    self.selected = Some(first.clone());
                    cx.emit(ProjectSelected { project: first });
                }
                self.projects = projects;
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to load projects: {}", e);
            }
        }
    }

    /// Select a project
    fn select(&mut self, project: Project, cx: &mut Context<Self>) {
        self.selected = Some(project.clone());
        self.is_open = false;
        cx.emit(ProjectSelected { project });
        cx.notify();
    }

    /// Toggle dropdown open/closed
    fn toggle(&mut self, cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
        cx.notify();
    }

    /// Get current selection
    pub fn selected(&self) -> Option<&Project> {
        self.selected.as_ref()
    }

    /// Create a new project
    #[allow(dead_code)]
    pub fn create_project(
        &mut self,
        name: &str,
        path: &str,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let project = Project::new(name, path);
        self.db.with_connection(|conn| project.insert(conn))?;
        self.refresh(cx);
        Ok(())
    }
}

impl Render for ProjectPicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_name = self
            .selected
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("Select Project");

        div()
            .track_focus(&self.focus_handle)
            .w_full()
            .flex()
            .flex_col()
            // Selected project button
            .child(
                div()
                    .id("project-picker-button")
                    .w_full()
                    .h(px(36.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(Theme::element())
                    .border_1()
                    .border_color(Theme::border())
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.border_color(Theme::border_focused()))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.toggle(cx);
                    }))
                    .child(
                        div()
                            .text_sm()
                            .text_color(Theme::text())
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(selected_name.to_string())
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(Theme::text_muted())
                            .child(if self.is_open { "▲" } else { "▼" })
                    )
            )
            // Dropdown list
            .when(self.is_open, |el| {
                let items: Vec<AnyElement> = self
                    .projects
                    .iter()
                    .map(|project| {
                        let is_selected = self
                            .selected
                            .as_ref()
                            .map(|s| s.id == project.id)
                            .unwrap_or(false);

                        let project_clone = project.clone();

                        div()
                            .id(SharedString::from(format!("project-{}", project.id)))
                            .w_full()
                            .h(px(32.0))
                            .px_3()
                            .flex()
                            .items_center()
                            .bg(if is_selected {
                                Theme::element_active()
                            } else {
                                Theme::element()
                            })
                            .hover(|s| s.bg(Theme::element_hover()))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.select(project_clone.clone(), cx);
                            }))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(Theme::text())
                                    .child(project.name.clone())
                            )
                            .into_any_element()
                    })
                    .collect();

                el.child(
                    div()
                        .id("project-dropdown")
                        .absolute()
                        .top(px(40.0))
                        .left_0()
                        .w_full()
                        .max_h(px(200.0))
                        .overflow_y_scroll()
                        .bg(Theme::surface())
                        .border_1()
                        .border_color(Theme::border())
                        .rounded_md()
                        .shadow_lg()
                        .children(items)
                )
            })
    }
}
