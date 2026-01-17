use gpui::{
    actions, div, prelude::*, px, App, Context, Entity, FocusHandle, Focusable, KeyBinding,
    SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonRounded, ButtonVariant, ButtonVariants},
    input::{Input, InputEvent, InputState},
    text::markdown,
    ActiveTheme, Sizable,
};
use manifest_client::{ManifestClient, Session, SessionStatus, Task, TaskStatus};
use uuid::Uuid;

// Define editor actions
actions!(feature_editor, [Save, Edit, Cancel]);

/// Events emitted by the FeatureEditor.
#[derive(Clone, Debug)]
pub enum Event {
    /// Feature was saved successfully.
    FeatureSaved(Uuid),
    /// Save failed with error message.
    SaveFailed(Uuid, String),
}

/// Colors for the editor (Pigs in Space theme).
mod colors {
    use gpui::{Hsla, Rgba};

    pub fn background() -> Hsla {
        Hsla {
            h: 210.0 / 360.0,
            s: 0.13,
            l: 0.15,
            a: 1.0,
        }
    }

    pub fn panel_background() -> Hsla {
        Hsla {
            h: 212.0 / 360.0,
            s: 0.15,
            l: 0.12,
            a: 1.0,
        }
    }

    /// Header background - matches feature panel header (#15191e)
    pub fn header_background() -> Rgba {
        Rgba {
            r: 0.082,
            g: 0.098,
            b: 0.118,
            a: 1.0,
        }
    }

    /// Header border color - matches feature panel header (#2d333a)
    pub fn header_border() -> Rgba {
        Rgba {
            r: 0.176,
            g: 0.200,
            b: 0.227,
            a: 1.0,
        }
    }

    /// Header text color - matches feature panel header (#c2d6ea)
    pub fn header_text() -> Rgba {
        Rgba {
            r: 0.761,
            g: 0.839,
            b: 0.918,
            a: 1.0,
        }
    }

    pub fn task_pending() -> Hsla {
        // Gray for pending
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.5,
            a: 1.0,
        }
    }

    pub fn task_running() -> Hsla {
        // Blue for running
        Hsla {
            h: 210.0 / 360.0,
            s: 0.8,
            l: 0.6,
            a: 1.0,
        }
    }

    pub fn task_completed() -> Hsla {
        // Green for completed
        Hsla {
            h: 120.0 / 360.0,
            s: 0.5,
            l: 0.5,
            a: 1.0,
        }
    }

    pub fn task_failed() -> Hsla {
        // Red for failed
        Hsla {
            h: 0.0 / 360.0,
            s: 0.7,
            l: 0.5,
            a: 1.0,
        }
    }

    pub fn dirty_indicator() -> Hsla {
        // Yellow/amber for dirty state
        Hsla {
            h: 45.0 / 360.0,
            s: 0.95,
            l: 0.60,
            a: 1.0,
        }
    }
}

/// A feature waiting to be opened (set from async context, opened in render).
#[derive(Clone)]
struct PendingFeature {
    id: Uuid,
    title: String,
    details: Option<String>,
}

/// Single-feature editor view with title, details, and tasks panel.
pub struct FeatureEditor {
    /// Currently loaded feature ID.
    feature_id: Option<Uuid>,
    /// Title input state.
    title_input: Option<Entity<InputState>>,
    /// Details input state.
    details_input: Option<Entity<InputState>>,
    /// Original title for dirty detection.
    original_title: SharedString,
    /// Original details for dirty detection.
    original_details: SharedString,
    /// Is title dirty?
    title_dirty: bool,
    /// Is details dirty?
    details_dirty: bool,
    /// Whether we're in edit mode.
    is_editing: bool,
    /// Tasks for the current feature's active session.
    tasks: Vec<Task>,
    /// Active session (if any).
    active_session: Option<Session>,
    /// Focus handle for keyboard input.
    focus_handle: FocusHandle,
    /// API client for saving.
    client: ManifestClient,
    /// Feature pending to be opened (set from async, opened in render with window access).
    pending_feature: Option<PendingFeature>,
}

impl FeatureEditor {
    /// Create a new empty editor.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            feature_id: None,
            title_input: None,
            details_input: None,
            original_title: "".into(),
            original_details: "".into(),
            title_dirty: false,
            details_dirty: false,
            is_editing: false,
            tasks: Vec::new(),
            active_session: None,
            focus_handle: cx.focus_handle(),
            client: ManifestClient::localhost(),
            pending_feature: None,
        }
    }

    /// Queue a feature to be opened (can be called from async context without window).
    pub fn load_feature(
        &mut self,
        feature_id: Uuid,
        title: String,
        details: Option<String>,
        cx: &mut Context<Self>,
    ) {
        // Queue for opening in render (when we have window access)
        self.pending_feature = Some(PendingFeature {
            id: feature_id,
            title,
            details,
        });
        cx.notify();
    }

    /// Open a feature for editing.
    fn open_feature(
        &mut self,
        feature_id: Uuid,
        title: String,
        details: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title_str = title.clone();
        let details_str = details.clone().unwrap_or_default();

        // Create title input (single line)
        let title_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(false)
                .default_value(title_str.clone())
        });

        // Create details input (multi-line)
        let details_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .default_value(details_str.clone())
        });

        // Subscribe to title input changes
        let title_entity = title_input.clone();
        cx.subscribe_in(
            &title_input,
            window,
            move |this, _state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(ref input) = this.title_input {
                        if *input == title_entity {
                            this.update_title_dirty(cx);
                            cx.notify();
                        }
                    }
                }
            },
        )
        .detach();

        // Subscribe to details input changes
        let details_entity = details_input.clone();
        cx.subscribe_in(
            &details_input,
            window,
            move |this, _state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(ref input) = this.details_input {
                        if *input == details_entity {
                            this.update_details_dirty(cx);
                            cx.notify();
                        }
                    }
                }
            },
        )
        .detach();

        self.feature_id = Some(feature_id);
        self.title_input = Some(title_input);
        self.details_input = Some(details_input);
        self.original_title = title_str.into();
        self.original_details = details_str.into();
        self.title_dirty = false;
        self.details_dirty = false;
        self.is_editing = false;

        // Load tasks for this feature
        self.load_tasks(feature_id, cx);

        cx.notify();
    }

    /// Load tasks for a feature's active session.
    fn load_tasks(&mut self, feature_id: Uuid, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let background = cx.background_executor().clone();

        cx.spawn(async move |this, cx| {
            let result = background
                .spawn(async move {
                    // Get sessions for the feature
                    let sessions = client.get_feature_sessions(&feature_id)?;

                    // Find active session
                    let active_session = sessions
                        .into_iter()
                        .find(|s| s.status == SessionStatus::Active);

                    if let Some(session) = active_session {
                        let tasks = client.get_session_tasks(&session.id)?;
                        Ok::<_, manifest_client::ClientError>((Some(session), tasks))
                    } else {
                        Ok((None, Vec::new()))
                    }
                })
                .await;

            if let Ok((session, tasks)) = result {
                if let Some(this) = this.upgrade() {
                    cx.update_entity(&this, |this: &mut FeatureEditor, cx| {
                        this.active_session = session;
                        this.tasks = tasks;
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Check if the editor has a feature loaded.
    pub fn has_feature(&self) -> bool {
        self.feature_id.is_some()
    }

    /// Check if content is dirty.
    pub fn is_dirty(&self) -> bool {
        self.title_dirty || self.details_dirty
    }

    /// Update title dirty state.
    fn update_title_dirty(&mut self, cx: &App) {
        if let Some(ref input) = self.title_input {
            let current = input.read(cx).value();
            self.title_dirty = current != self.original_title;
        }
    }

    /// Update details dirty state.
    fn update_details_dirty(&mut self, cx: &App) {
        if let Some(ref input) = self.details_input {
            let current = input.read(cx).value();
            self.details_dirty = current != self.original_details;
        }
    }

    /// Save the current feature.
    pub fn save_current(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(feature_id) = self.feature_id else {
            return;
        };

        if !self.is_dirty() {
            self.is_editing = false;
            cx.notify();
            return;
        }

        // Extract current values
        let title = self
            .title_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string());
        let details = self
            .details_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string());

        let client = self.client.clone();

        // Update originals optimistically
        if let Some(ref t) = title {
            self.original_title = t.clone().into();
        }
        if let Some(ref d) = details {
            self.original_details = d.clone().into();
        }
        self.title_dirty = false;
        self.details_dirty = false;
        self.is_editing = false;
        cx.notify();

        // Save in background
        cx.background_executor()
            .spawn(async move { client.update_feature_full(&feature_id, title, details) })
            .detach_and_log_err(cx);

        cx.emit(Event::FeatureSaved(feature_id));
    }

    /// Enter edit mode.
    fn enter_edit_mode(&mut self, cx: &mut Context<Self>) {
        self.is_editing = true;
        cx.notify();
    }

    /// Cancel editing and revert to original values.
    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Revert inputs to original values
        if let Some(ref title_input) = self.title_input {
            title_input.update(cx, |state, cx| {
                state.set_value(self.original_title.to_string(), window, cx);
            });
        }
        if let Some(ref details_input) = self.details_input {
            details_input.update(cx, |state, cx| {
                state.set_value(self.original_details.to_string(), window, cx);
            });
        }
        self.title_dirty = false;
        self.details_dirty = false;
        self.is_editing = false;
        cx.notify();
    }

    // --- Action handlers ---

    fn on_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        self.save_current(window, cx);
    }

    fn on_edit(&mut self, _: &Edit, _window: &mut Window, cx: &mut Context<Self>) {
        self.enter_edit_mode(cx);
    }

    fn on_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_edit(window, cx);
    }
}

impl Focusable for FeatureEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::EventEmitter<Event> for FeatureEditor {}

impl Render for FeatureEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Process any pending feature that was queued from async context
        if let Some(pending) = self.pending_feature.take() {
            self.open_feature(pending.id, pending.title, pending.details, window, cx);
        }

        div()
            .id("feature-editor")
            .size_full()
            .flex()
            .flex_col()
            .bg(colors::background())
            .track_focus(&self.focus_handle)
            .key_context("FeatureEditor")
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_edit))
            .on_action(cx.listener(Self::on_cancel))
            // Top: Feature header (always visible)
            .child(self.render_feature_header(cx))
            // Bottom: Content area (tasks + details or empty state)
            .child(self.render_content_area(cx))
    }
}

impl FeatureEditor {
    fn render_feature_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_editing = self.is_editing;
        let is_dirty = self.is_dirty();
        let has_feature = self.has_feature();

        div()
            .id("feature-header")
            .w_full()
            .h(px(32.0)) // Match feature panel header height
            .px(px(12.0))
            .bg(colors::header_background())
            .border_b_1()
            .border_color(colors::header_border())
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            // Left: Title and dirty indicator
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .font_family("IBM Plex Sans")
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(colors::header_text())
                            .child("FEATURE"),
                    )
                    .when(is_dirty, |d| {
                        d.child(
                            div()
                                .font_family("IBM Plex Sans")
                                .text_size(px(11.0))
                                .text_color(colors::dirty_indicator())
                                .child("• Unsaved"),
                        )
                    }),
            )
            // Right: Buttons (only show when feature is loaded)
            .when(has_feature, |d| {
                d.child(if is_editing {
                    // Edit mode: Cancel and Save buttons
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .mr(px(4.0))
                        .child(
                            Button::new("cancel-btn")
                                .label("Cancel")
                                .xsmall()
                                .rounded(ButtonRounded::Small)
                                .with_variant(ButtonVariant::Ghost)
                                .font_family("IBM Plex Sans")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.cancel_edit(window, cx);
                                })),
                        )
                        .child(
                            Button::new("save-btn")
                                .label("Save")
                                .xsmall()
                                .rounded(ButtonRounded::Small)
                                .with_variant(ButtonVariant::Primary)
                                .font_family("IBM Plex Sans")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_current(window, cx);
                                })),
                        )
                        .into_any_element()
                } else {
                    // View mode: Edit button
                    div()
                        .mr(px(4.0))
                        .child(
                            Button::new("edit-btn")
                                .label("Edit")
                                .xsmall()
                                .rounded(ButtonRounded::Small)
                                .with_variant(ButtonVariant::Ghost)
                                .font_family("IBM Plex Sans")
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.enter_edit_mode(cx);
                                })),
                        )
                        .into_any_element()
                })
            })
    }

    fn render_content_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_feature = self.has_feature();

        div()
            .id("content-area")
            .flex_1()
            .w_full()
            .flex()
            .flex_row()
            .overflow_hidden()
            // Left: Feature details or empty state (3/4 width)
            .child(if has_feature {
                self.render_feature_details(cx).into_any_element()
            } else {
                self.render_empty_state(cx).into_any_element()
            })
            // Right: Tasks panel (1/4 width, always visible)
            .child(self.render_tasks_panel(cx))
    }

    fn render_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("editor-empty-state")
            .w_3_4()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .font_family("IBM Plex Sans")
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a feature to edit"),
            )
    }

    fn render_tasks_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("tasks-panel")
            .w_1_4()
            .min_w(px(180.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(colors::panel_background())
            .border_l_1()
            .border_color(cx.theme().border)
            // Header
            .child(
                div()
                    .id("tasks-header")
                    .w_full()
                    .px(px(12.0))
                    .py(px(10.0))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .font_family("IBM Plex Sans")
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(cx.theme().foreground)
                            .child("Tasks"),
                    ),
            )
            // Task list
            .child(
                div()
                    .id("tasks-list")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(if self.tasks.is_empty() {
                        vec![self.render_no_tasks(cx).into_any_element()]
                    } else {
                        self.tasks
                            .iter()
                            .enumerate()
                            .map(|(idx, task)| self.render_task(idx, task, cx).into_any_element())
                            .collect()
                    }),
            )
    }

    fn render_no_tasks(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("no-tasks")
            .w_full()
            .p(px(12.0))
            .child(
                div()
                    .font_family("IBM Plex Sans")
                    .text_size(px(12.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.active_session.is_some() {
                        "No tasks in session"
                    } else {
                        "No active session"
                    }),
            )
    }

    fn render_task(&self, idx: usize, task: &Task, cx: &mut Context<Self>) -> impl IntoElement {
        let status_color = match task.status {
            TaskStatus::Pending => colors::task_pending(),
            TaskStatus::Running => colors::task_running(),
            TaskStatus::Completed => colors::task_completed(),
            TaskStatus::Failed => colors::task_failed(),
        };

        let status_label = match task.status {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "done",
            TaskStatus::Failed => "failed",
        };

        div()
            .id(format!("task-{}", idx))
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(cx.theme().border)
            .hover(|s| s.bg(cx.theme().list_hover))
            .flex()
            .flex_col()
            .gap(px(4.0))
            // Title row
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    // Status indicator
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded_full()
                            .bg(status_color),
                    )
                    // Title
                    .child(
                        div()
                            .flex_1()
                            .font_family("IBM Plex Sans")
                            .text_size(px(12.0))
                            .text_color(cx.theme().foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(task.title.clone()),
                    ),
            )
            // Status label
            .child(
                div()
                    .pl(px(16.0))
                    .font_family("IBM Plex Sans")
                    .text_size(px(10.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(status_label),
            )
    }

    fn render_feature_details(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_editing = self.is_editing;

        div()
            .id("feature-details")
            .w_3_4()
            .h_full()
            .overflow_y_scroll()
            .p(px(24.0))
            .flex()
            .flex_col()
            .gap(px(20.0))
            // Title section
            .child(
                div()
                    .id("title-section")
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .font_family("IBM Plex Sans")
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(cx.theme().muted_foreground)
                            .child("TITLE"),
                    )
                    .child(if is_editing {
                        // Edit mode - IBM Plex Mono with panel background
                        if let Some(ref title_input) = self.title_input {
                            div()
                                .w_full()
                                .bg(colors::panel_background())
                                .rounded(px(4.0))
                                .p(px(8.0))
                                .font_family("IBM Plex Mono")
                                .child(
                                    Input::new(title_input)
                                        .appearance(false)
                                        .w_full(),
                                )
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }
                    } else {
                        // View mode - IBM Plex Sans
                        div()
                            .font_family("IBM Plex Sans")
                            .text_size(px(14.0))
                            .text_color(cx.theme().foreground)
                            .child(self.original_title.clone())
                            .into_any_element()
                    }),
            )
            // Details section
            .child(
                div()
                    .id("details-section")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .font_family("IBM Plex Sans")
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(cx.theme().muted_foreground)
                            .child("DETAILS"),
                    )
                    .child(if is_editing {
                        // Edit mode - IBM Plex Mono with panel background
                        if let Some(ref details_input) = self.details_input {
                            div()
                                .flex_1()
                                .w_full()
                                .bg(colors::panel_background())
                                .rounded(px(4.0))
                                .p(px(8.0))
                                .font_family("IBM Plex Mono")
                                .child(
                                    Input::new(details_input)
                                        .appearance(false)
                                        .w_full()
                                        .h_full(),
                                )
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }
                    } else {
                        // View mode - render as markdown
                        if self.original_details.is_empty() {
                            div()
                                .font_family("IBM Plex Sans")
                                .text_size(px(13.0))
                                .text_color(cx.theme().muted_foreground)
                                .child("No details")
                                .into_any_element()
                        } else {
                            markdown(self.original_details.clone())
                                .selectable(true)
                                .into_any_element()
                        }
                    }),
            )
    }
}

/// Register key bindings for the feature editor.
pub fn register_bindings(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("cmd-s", Save, Some("FeatureEditor"))]);
}
