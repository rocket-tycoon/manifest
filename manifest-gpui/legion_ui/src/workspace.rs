//! Workspace - Main application container with editor, sidebar, and terminal panel.
//!
//! The workspace coordinates the main UI components and manages agent sessions.
//! It now supports multiple agent terminals in a tabbed interface.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::*;
use legion_agent::{AgentEvent, AgentOrchestrator, AgentState, AgentTerminal, ContextLayers, TaskContext};
use legion_db::{Database, Feature, Project, Session, Task as DbTask, TaskCriterion};
use legion_git::WorktreeManager;
use rfd::AsyncFileDialog;

use crate::components::{ResizeAxis, ResizeDrag, ResizeHandle};
use crate::editor::{FeatureEditor, StartSession};
use crate::sidebar::{Sidebar, SidebarFeatureSelected, SidebarOpenProjectRequested};
use crate::terminal::TerminalView;
use crate::theme::Theme;

// Sidebar size constants
const DEFAULT_SIDEBAR_WIDTH: f32 = 260.0;
const MIN_SIDEBAR_WIDTH: f32 = 180.0;
const MAX_SIDEBAR_WIDTH: f32 = 500.0;

// Terminal size constants
const DEFAULT_TERMINAL_HEIGHT: f32 = 200.0;
const MIN_TERMINAL_HEIGHT: f32 = 100.0;
const MAX_TERMINAL_HEIGHT: f32 = 600.0;

/// Event emitted when the workspace wants to open a feature
pub struct OpenFeature {
    pub feature: Feature,
}

impl EventEmitter<OpenFeature> for Workspace {}

/// Represents an agent terminal tab in the terminal panel.
pub struct AgentTab {
    /// The task ID this agent is working on
    pub task_id: String,
    /// Display title for the tab
    pub title: String,
    /// The terminal view for this agent
    pub terminal_view: Entity<TerminalView>,
    /// The underlying agent terminal entity
    pub agent_terminal: Entity<AgentTerminal>,
    /// Current state of the agent
    pub state: AgentState,
}

/// Terminal panel mode - either default shell or agent tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMode {
    /// Default shell terminal
    Shell,
    /// Agent terminals (tabbed)
    Agents,
}

pub struct Workspace {
    db: Arc<Database>,
    sidebar: Entity<Sidebar>,
    editor: Entity<FeatureEditor>,
    /// Default shell terminal
    shell_terminal: Entity<TerminalView>,
    /// Agent terminals by task ID
    agent_tabs: HashMap<String, AgentTab>,
    /// Order of agent tabs for display
    agent_tab_order: Vec<String>,
    /// Currently active agent tab (task_id)
    active_agent_tab: Option<String>,
    /// Current terminal mode
    terminal_mode: TerminalMode,
    /// Agent orchestrator for managing sessions
    orchestrator: Option<Entity<AgentOrchestrator>>,
    active_project: Option<Project>,
    active_feature: Option<Feature>,
    focus_handle: FocusHandle,
    // Panel sizes (pixels)
    sidebar_width: f32,
    terminal_height: f32,
}

impl Workspace {
    pub fn new(db: Arc<Database>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| Sidebar::new(db.clone(), window, cx));
        let editor = cx.new(|cx| FeatureEditor::new(db.clone(), window, cx));
        let shell_terminal = cx.new(|cx| TerminalView::new(None, window, cx));

        // Initialize the shell terminal (spawn PTY)
        shell_terminal.update(cx, |terminal, cx| {
            terminal.initialize(window, cx);
        });

        // Subscribe to feature selection from sidebar
        cx.subscribe(
            &sidebar,
            |this, _sidebar, event: &SidebarFeatureSelected, cx| {
                this.open_feature(&event.feature_id, cx);
            },
        )
        .detach();

        // Subscribe to start session events from editor
        cx.subscribe(&editor, |this, _editor, event: &StartSession, cx| {
            this.start_session(&event.feature_id, cx);
        })
        .detach();

        // Subscribe to open project requests from sidebar
        cx.subscribe(
            &sidebar,
            |this, _sidebar, _event: &SidebarOpenProjectRequested, cx| {
                this.open_project_dialog(cx);
            },
        )
        .detach();

        Self {
            db,
            sidebar,
            editor,
            shell_terminal,
            agent_tabs: HashMap::new(),
            agent_tab_order: Vec::new(),
            active_agent_tab: None,
            terminal_mode: TerminalMode::Shell,
            orchestrator: None,
            active_project: None,
            active_feature: None,
            focus_handle: cx.focus_handle(),
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            terminal_height: DEFAULT_TERMINAL_HEIGHT,
        }
    }

    /// Set the active project
    pub fn set_project(&mut self, mut project: Project, cx: &mut Context<Self>) {
        // Touch project to update last_opened_at
        if let Err(e) = self.db.with_connection(|conn| project.touch(conn)) {
            tracing::error!("Failed to update project last_opened_at: {}", e);
        }

        self.active_project = Some(project.clone());
        self.active_feature = None;

        // Update sidebar with new project
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_project(project, cx);
        });

        // Clear editor
        self.editor.update(cx, |editor, cx| {
            editor.clear(cx);
        });

        cx.notify();
    }

    /// Get current project
    #[allow(dead_code)]
    pub fn active_project(&self) -> Option<&Project> {
        self.active_project.as_ref()
    }

    /// Open project dialog triggered from sidebar button
    fn open_project_dialog(&mut self, cx: &mut Context<Self>) {
        tracing::info!("Opening project dialog from sidebar button");
        let db = self.db.clone();

        cx.spawn(async move |this, cx| {
            let folder = AsyncFileDialog::new()
                .set_title("Open Project")
                .pick_folder()
                .await;

            if let Some(folder) = folder {
                let path = folder.path().to_path_buf();
                tracing::info!("Selected folder: {:?}", path);

                // Extract project name from path
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled")
                    .to_string();

                let path_str = path.to_string_lossy().to_string();

                // Check if project already exists or create new
                let project = db.with_connection(|conn| {
                    if let Some(existing) = Project::find_by_path(conn, &path_str)? {
                        Ok(existing)
                    } else {
                        let project = Project::new(&name, &path_str);
                        project.insert(conn)?;
                        Ok(project)
                    }
                })?;

                // Update workspace with the project
                this.update(cx, |workspace, cx| {
                    workspace.set_project(project, cx);
                })?;
            } else {
                tracing::info!("User cancelled dialog");
            }

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    /// Open a feature by ID
    fn open_feature(&mut self, feature_id: &str, cx: &mut Context<Self>) {
        let db = self.db.clone();
        let feature_id = feature_id.to_string();

        match db.with_connection(|conn| Feature::find_by_id(conn, &feature_id)) {
            Ok(Some(feature)) => {
                self.active_feature = Some(feature.clone());

                // Update the editor
                self.editor.update(cx, |editor, cx| {
                    editor.open(feature.clone(), cx);
                });

                cx.emit(OpenFeature { feature });
                cx.notify();
            }
            Ok(None) => {
                tracing::warn!("Feature not found: {}", feature_id);
            }
            Err(e) => {
                tracing::error!("Failed to load feature: {}", e);
            }
        }
    }

    /// Start a new session for a feature
    fn start_session(&mut self, feature_id: &str, cx: &mut Context<Self>) {
        let Some(project) = &self.active_project else {
            tracing::warn!("Cannot start session: no active project");
            return;
        };

        let project_path = PathBuf::from(&project.path);
        let db = self.db.clone();
        let feature_id = feature_id.to_string();

        tracing::info!("Starting session for feature: {}", feature_id);

        // Create session and task in database
        let session_result = db.with_connection(|conn| {
            // Load feature
            let feature = Feature::find_by_id(conn, &feature_id)?
                .ok_or_else(|| anyhow::anyhow!("Feature not found"))?;

            // Create session
            let session = Session::new(&feature_id, &feature.title);
            session.insert(conn)?;

            // Create a single task for now (future: parse feature into multiple tasks)
            let task = DbTask::new(&session.id, &feature.title)
                .with_description(feature.content.clone().unwrap_or_default());
            task.insert(conn)?;

            Ok::<_, anyhow::Error>((session, task, feature))
        });

        let (session, task, feature) = match session_result {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Failed to create session: {}", e);
                return;
            }
        };

        // Create worktree for the task
        let worktree_mgr = WorktreeManager::new(&project_path);
        let worktree = match worktree_mgr.create(&task.id, "main") {
            Ok(wt) => wt,
            Err(e) => {
                tracing::error!("Failed to create worktree: {}", e);
                return;
            }
        };

        // Update task with worktree info
        let mut task = task;
        if let Err(e) = db.with_connection(|conn| {
            task.start(
                conn,
                worktree.branch.clone(),
                worktree.path.to_string_lossy().to_string(),
            )
        }) {
            tracing::error!("Failed to update task: {}", e);
        }

        // Build context for the agent
        let task_context = TaskContext::from_task(
            &task,
            &db.with_connection(|conn| TaskCriterion::list_by_task(conn, &task.id))
                .unwrap_or_default(),
        );
        let context =
            ContextLayers::new(task_context).with_feature_context(feature.content.unwrap_or_default());

        // Spawn the agent terminal
        let task_id = task.id.clone();
        let task_title = task.title.clone();
        let worktree_path = worktree.path.clone();

        cx.spawn(async move |this, cx| {
            // Create terminal in background
            let builder =
                legion_terminal::Terminal::create_pty_sync(Some(worktree_path.clone()), 0)
                    .map_err(|e| anyhow::anyhow!("Failed to create PTY: {}", e))?;

            // Build terminal entity and agent
            this.update(cx, |workspace, cx| {
                let terminal = cx.new(|cx| builder.build(cx));

                // Create agent terminal
                let agent_terminal = cx.new(|cx| {
                    // Subscribe to terminal events
                    cx.subscribe(
                        &terminal,
                        |this: &mut AgentTerminal, _terminal, event: &legion_terminal::Event, cx| {
                            this.handle_terminal_event(event, cx);
                        },
                    )
                    .detach();

                    AgentTerminal::new_internal(
                        terminal.clone(),
                        task_id.clone(),
                        context.clone(),
                        worktree_path.clone(),
                        "claude".to_string(),
                    )
                });

                // Subscribe to agent events
                let tid = task_id.clone();
                cx.subscribe(&agent_terminal, move |workspace, _agent, event: &AgentEvent, cx| {
                    workspace.handle_agent_event(&tid, event, cx);
                })
                .detach();

                // Create terminal view from the terminal
                let terminal_view =
                    cx.new(|cx| {
                        // Create a placeholder window - the view will get the real window on render
                        TerminalView::from_terminal_entity(terminal.clone(), cx)
                    });

                // Add to agent tabs
                workspace.agent_tabs.insert(
                    task_id.clone(),
                    AgentTab {
                        task_id: task_id.clone(),
                        title: task_title.clone(),
                        terminal_view,
                        agent_terminal: agent_terminal.clone(),
                        state: AgentState::Initializing,
                    },
                );
                workspace.agent_tab_order.push(task_id.clone());
                workspace.active_agent_tab = Some(task_id.clone());
                workspace.terminal_mode = TerminalMode::Agents;

                // Start the agent by injecting the prompt
                agent_terminal.update(cx, |agent, cx| {
                    agent.inject_prompt(cx);
                });

                cx.notify();
            })?;

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    /// Handle events from an agent terminal
    fn handle_agent_event(&mut self, task_id: &str, event: &AgentEvent, cx: &mut Context<Self>) {
        match event {
            AgentEvent::StateChanged(state) => {
                if let Some(tab) = self.agent_tabs.get_mut(task_id) {
                    tab.state = *state;
                }
                cx.notify();
            }
            AgentEvent::Exited { success } => {
                tracing::info!(
                    "Agent {} exited with success={}",
                    task_id,
                    success
                );
                if let Some(tab) = self.agent_tabs.get_mut(task_id) {
                    tab.state = if *success {
                        AgentState::Complete
                    } else {
                        AgentState::Failed
                    };
                }
                cx.notify();
            }
            AgentEvent::Wakeup => {
                cx.notify();
            }
        }
    }

    /// Switch to a specific agent tab
    pub fn select_agent_tab(&mut self, task_id: &str, cx: &mut Context<Self>) {
        if self.agent_tabs.contains_key(task_id) {
            self.active_agent_tab = Some(task_id.to_string());
            self.terminal_mode = TerminalMode::Agents;
            cx.notify();
        }
    }

    /// Switch to shell terminal
    pub fn select_shell_terminal(&mut self, cx: &mut Context<Self>) {
        self.terminal_mode = TerminalMode::Shell;
        cx.notify();
    }

    /// Close an agent tab
    pub fn close_agent_tab(&mut self, task_id: &str, cx: &mut Context<Self>) {
        self.agent_tabs.remove(task_id);
        self.agent_tab_order.retain(|id| id != task_id);

        // Update active tab
        if self.active_agent_tab.as_deref() == Some(task_id) {
            self.active_agent_tab = self.agent_tab_order.last().cloned();
        }

        // Switch to shell if no more agent tabs
        if self.agent_tabs.is_empty() {
            self.terminal_mode = TerminalMode::Shell;
        }

        cx.notify();
    }

    /// Get the sidebar view
    #[allow(dead_code)]
    pub fn sidebar(&self) -> &Entity<Sidebar> {
        &self.sidebar
    }

    /// Get currently active feature
    #[allow(dead_code)]
    pub fn active_feature(&self) -> Option<&Feature> {
        self.active_feature.as_ref()
    }

    /// Render the terminal tabs
    #[allow(dead_code)]
    fn render_terminal_tabs(&self, _cx: &App) -> impl IntoElement {
        let shell_active = self.terminal_mode == TerminalMode::Shell;

        div()
            .h(px(32.0))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(Theme::border())
            // Shell tab
            .child(
                div()
                    .id("shell-tab")
                    .h(px(26.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_t_md()
                    .text_xs()
                    .cursor_pointer()
                    .when(shell_active, |el| {
                        el.bg(Theme::background()).text_color(Theme::text())
                    })
                    .when(!shell_active, |el| {
                        el.text_color(Theme::text_muted())
                            .hover(|el| el.bg(Theme::ghost_element_hover()))
                    })
                    .child("Terminal"),
            )
            // Agent tabs
            .children(self.agent_tab_order.iter().map(|task_id| {
                let is_active = self.active_agent_tab.as_deref() == Some(task_id);
                let tab = self.agent_tabs.get(task_id).unwrap();

                let state_indicator = match tab.state {
                    AgentState::Initializing | AgentState::Injecting => "⏳",
                    AgentState::Running => "🔄",
                    AgentState::Complete => "✅",
                    AgentState::Failed => "❌",
                };

                div()
                    .id(SharedString::from(format!("agent-tab-{}", task_id)))
                    .h(px(26.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_t_md()
                    .text_xs()
                    .cursor_pointer()
                    .when(is_active, |el| {
                        el.bg(Theme::background()).text_color(Theme::text())
                    })
                    .when(!is_active, |el| {
                        el.text_color(Theme::text_muted())
                            .hover(|el| el.bg(Theme::ghost_element_hover()))
                    })
                    .child(state_indicator)
                    .child(tab.title.clone())
            }))
    }

    /// Render the active terminal content
    fn render_terminal_content(&self) -> impl IntoElement {
        match self.terminal_mode {
            TerminalMode::Shell => div()
                .flex_1()
                .overflow_hidden()
                .child(self.shell_terminal.clone()),
            TerminalMode::Agents => {
                if let Some(task_id) = &self.active_agent_tab {
                    if let Some(tab) = self.agent_tabs.get(task_id) {
                        return div()
                            .flex_1()
                            .overflow_hidden()
                            .child(tab.terminal_view.clone());
                    }
                }
                // Fallback to shell if no active agent
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.shell_terminal.clone())
            }
        }
    }

    /// Handle resize drag events
    fn handle_resize_drag(
        &mut self,
        event: &DragMoveEvent<ResizeDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.drag(cx).axis {
            ResizeAxis::Horizontal => {
                // Sidebar width is the x position of the drag
                let new_width: f32 = event.event.position.x.into();
                self.sidebar_width = new_width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
            }
            ResizeAxis::Vertical => {
                // Get window bounds to calculate terminal height from bottom
                let window_bounds = window.bounds();
                let window_height: f32 = window_bounds.size.height.into();
                let y_pos: f32 = event.event.position.y.into();
                // Terminal is at the bottom, so new height = window_height - y_position
                // Account for tab bar height (~36px)
                let content_top = 36.0_f32;
                let new_height = (window_height - y_pos).max(0.0);
                self.terminal_height = new_height.clamp(
                    MIN_TERMINAL_HEIGHT,
                    MAX_TERMINAL_HEIGHT.min(window_height - content_top - MIN_TERMINAL_HEIGHT),
                );
            }
        }
        cx.notify();
    }

    /// Handle tab click events
    fn handle_tab_click(&mut self, task_id: Option<String>, cx: &mut Context<Self>) {
        match task_id {
            None => self.select_shell_terminal(cx),
            Some(id) => self.select_agent_tab(&id, cx),
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_width = self.sidebar_width;
        let terminal_height = self.terminal_height;
        let terminal_mode = self.terminal_mode;
        let active_agent_tab = self.active_agent_tab.clone();
        let agent_tab_order = self.agent_tab_order.clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_row()
            // Sidebar (with dynamic width)
            .child(
                div()
                    .w(px(sidebar_width))
                    .h_full()
                    .flex_shrink_0()
                    .child(self.sidebar.clone()),
            )
            // Sidebar resize handle (double-click to reset)
            .child(
                div()
                    .id("sidebar-resize-wrapper")
                    .child(ResizeHandle::new("sidebar-resize", ResizeAxis::Horizontal))
                    .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                        if event.click_count() == 2 {
                            this.sidebar_width = DEFAULT_SIDEBAR_WIDTH;
                            cx.notify();
                        }
                    })),
            )
            // Main content area
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    // Editor area
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .min_h_0()
                            // Tab bar
                            .child(
                                div()
                                    .h(px(36.0))
                                    .w_full()
                                    .bg(Theme::surface())
                                    .border_b_1()
                                    .border_color(Theme::border())
                                    .flex()
                                    .items_center()
                                    .px_2()
                                    .when_some(self.active_feature.as_ref(), |el, feature| {
                                        el.child(
                                            div()
                                                .h(px(28.0))
                                                .px_3()
                                                .flex()
                                                .items_center()
                                                .bg(Theme::background())
                                                .rounded_t_md()
                                                .text_sm()
                                                .text_color(Theme::text())
                                                .child(feature.title.clone()),
                                        )
                                    }),
                            )
                            // Editor content (using FeatureEditor view)
                            .child(self.editor.clone()),
                    )
                    // Terminal resize handle (double-click to reset)
                    .child(
                        div()
                            .id("terminal-resize-wrapper")
                            .child(ResizeHandle::new("terminal-resize", ResizeAxis::Vertical))
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.click_count() == 2 {
                                    this.terminal_height = DEFAULT_TERMINAL_HEIGHT;
                                    cx.notify();
                                }
                            })),
                    )
                    // Terminal panel (with dynamic height and tabs)
                    .child(
                        div()
                            .h(px(terminal_height))
                            .flex_shrink_0()
                            .bg(Theme::surface())
                            .flex()
                            .flex_col()
                            // Terminal tabs header
                            .child(
                                div()
                                    .h(px(32.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .border_b_1()
                                    .border_color(Theme::border())
                                    // Shell tab
                                    .child(
                                        div()
                                            .id("shell-tab")
                                            .h(px(26.0))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .rounded_t_md()
                                            .text_xs()
                                            .cursor_pointer()
                                            .when(terminal_mode == TerminalMode::Shell, |el| {
                                                el.bg(Theme::background()).text_color(Theme::text())
                                            })
                                            .when(terminal_mode != TerminalMode::Shell, |el| {
                                                el.text_color(Theme::text_muted())
                                                    .hover(|el| el.bg(Theme::ghost_element_hover()))
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.handle_tab_click(None, cx);
                                            }))
                                            .child("Terminal"),
                                    )
                                    // Agent tabs
                                    .children(agent_tab_order.iter().map(|task_id| {
                                        let is_active = active_agent_tab.as_deref() == Some(task_id);
                                        let tab = self.agent_tabs.get(task_id).unwrap();
                                        let task_id_clone = task_id.clone();

                                        let state_indicator = match tab.state {
                                            AgentState::Initializing | AgentState::Injecting => "⏳",
                                            AgentState::Running => "🔄",
                                            AgentState::Complete => "✅",
                                            AgentState::Failed => "❌",
                                        };

                                        div()
                                            .id(SharedString::from(format!("agent-tab-{}", task_id)))
                                            .h(px(26.0))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .rounded_t_md()
                                            .text_xs()
                                            .cursor_pointer()
                                            .when(is_active, |el| {
                                                el.bg(Theme::background()).text_color(Theme::text())
                                            })
                                            .when(!is_active, |el| {
                                                el.text_color(Theme::text_muted())
                                                    .hover(|el| el.bg(Theme::ghost_element_hover()))
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.handle_tab_click(Some(task_id_clone.clone()), cx);
                                            }))
                                            .child(state_indicator)
                                            .child(tab.title.clone())
                                    })),
                            )
                            // Terminal content
                            .child(self.render_terminal_content()),
                    ),
            )
            // Handle drag events for resizing
            .on_drag_move(cx.listener(Self::handle_resize_drag))
    }
}
