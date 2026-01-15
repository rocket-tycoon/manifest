//! Manifest Terminal Application
//!
//! A GPUI application with a feature explorer panel, feature editor, and terminal.

mod active_context;
mod context_file;

use std::cell::Cell;
use std::sync::Arc;

use active_context::ActiveFeatureContext;

use feature_editor::{Event as EditorEvent, FeatureEditor};
use feature_panel::{Event as PanelEvent, FeaturePanel};
use gpui::{
    App, Application, Bounds, Context, CursorStyle, Entity, Focusable, KeyBinding, Menu, MenuItem,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render, Styled,
    TitlebarOptions, Window, WindowBounds, WindowOptions, actions, div, point, prelude::*, px,
    relative, size,
};
use manifest_core::db::Database;
use parking_lot::Mutex;
use terminal::mappings::colors::TerminalColors;
use terminal_view::TerminalView;
use uuid::Uuid;

/// Convert manifest_core types to manifest_client types for feature_panel compatibility.
mod convert {
    use manifest_client::{Feature, FeatureState};
    use manifest_core::models::{FeatureState as CoreState, FeatureTreeNode};

    fn convert_state(state: CoreState) -> FeatureState {
        match state {
            CoreState::Proposed => FeatureState::Proposed,
            CoreState::Specified => FeatureState::Specified,
            CoreState::Implemented => FeatureState::Implemented,
            CoreState::Deprecated => FeatureState::Deprecated,
        }
    }

    pub fn tree_node_to_feature(node: FeatureTreeNode) -> Feature {
        Feature {
            id: node.feature.id,
            project_id: node.feature.project_id,
            parent_id: node.feature.parent_id,
            title: node.feature.title,
            details: node.feature.details,
            desired_details: node.feature.desired_details,
            state: convert_state(node.feature.state),
            priority: node.feature.priority,
            created_at: node.feature.created_at.to_rfc3339(),
            updated_at: node.feature.updated_at.to_rfc3339(),
            children: node
                .children
                .into_iter()
                .map(tree_node_to_feature)
                .collect(),
        }
    }
}

actions!(app, [Quit, Open, OpenRecent, Save]);

/// Pane colors (Pigs in Space theme).
mod colors {
    use gpui::Hsla;

    pub fn divider() -> Hsla {
        Hsla {
            h: 210.0 / 360.0,
            s: 0.10,
            l: 0.25,
            a: 1.0,
        }
    }

    pub fn divider_hover() -> Hsla {
        Hsla {
            h: 220.0 / 360.0,
            s: 1.0,
            l: 0.75,
            a: 0.5,
        }
    }
}

/// Minimum pane height in pixels.
const MIN_PANE_HEIGHT: f32 = 100.0;
/// Divider hit area size.
const DIVIDER_HITBOX_SIZE: f32 = 8.0;
/// Visual divider thickness.
const DIVIDER_SIZE: f32 = 1.0;

/// Set up the application menus.
fn set_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "Manifest".into(),
            items: vec![MenuItem::action("Quit Manifest", Quit)],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open...", Open),
                MenuItem::action("Open Recent", OpenRecent),
                MenuItem::separator(),
                MenuItem::action("Save", Save),
            ],
        },
    ]);
}

/// Root application view with feature panel, editor, and terminal.
struct ManifestApp {
    feature_panel: Entity<FeaturePanel>,
    feature_editor: Entity<FeatureEditor>,
    terminal_view: Entity<TerminalView>,
    /// Flex values for editor/terminal split [editor_flex, terminal_flex].
    pane_flexes: Arc<Mutex<Vec<f32>>>,
    /// Whether the user is currently dragging the divider.
    dragging_divider: Arc<Cell<bool>>,
    /// Y position when drag started (for computing delta).
    drag_start_y: Arc<Cell<f32>>,
    /// Total height of the split pane area (updated during render).
    split_area_height: Arc<Cell<f32>>,
}

impl ManifestApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let feature_panel = cx.new(|cx| FeaturePanel::new(cx));
        let feature_editor = cx.new(|cx| FeatureEditor::new(cx));
        let terminal_view = cx.new(|cx| TerminalView::new(window, cx));

        // Subscribe to feature panel selection events
        let feature_editor_clone = feature_editor.clone();
        cx.subscribe(
            &feature_panel,
            move |_this, _panel, event: &PanelEvent, cx| {
                if let PanelEvent::FeatureSelected(feature_id) = event {
                    Self::on_feature_selected(*feature_id, &feature_editor_clone, cx);
                }
            },
        )
        .detach();

        // Subscribe to editor events for dirty close handling
        cx.subscribe(
            &feature_editor,
            |_this, _editor, event: &EditorEvent, cx| {
                match event {
                    EditorEvent::FeatureSaved(id) => {
                        eprintln!("Feature {} saved", id);
                    }
                    EditorEvent::SaveFailed(id, err) => {
                        eprintln!("Failed to save feature {}: {}", id, err);
                    }
                    EditorEvent::DirtyCloseRequested(idx) => {
                        // TODO: Show prompt dialog
                        eprintln!("Dirty close requested for tab {}", idx);
                    }
                }
            },
        )
        .detach();

        // Focus the terminal on startup
        let focus_handle = terminal_view.focus_handle(cx);
        focus_handle.focus(window, cx);

        // Fetch features in background
        let feature_panel_clone = feature_panel.clone();
        let background_executor = cx.background_executor().clone();
        cx.spawn(async move |_this, cx| {
            let result = background_executor
                .spawn(async move { Self::fetch_features() })
                .await;

            match result {
                Ok(features) => {
                    eprintln!("Loaded {} features", features.len());
                    cx.update_entity(&feature_panel_clone, |panel, cx| {
                        panel.set_features(features, cx);
                    });
                }
                Err(e) => {
                    eprintln!("Failed to load features: {}", e);
                    cx.update_entity(&feature_panel_clone, |panel, cx| {
                        panel.set_error(e, cx);
                    });
                }
            }
        })
        .detach();

        ManifestApp {
            feature_panel,
            feature_editor,
            terminal_view,
            pane_flexes: Arc::new(Mutex::new(vec![1.0, 1.0])), // Equal split
            dragging_divider: Arc::new(Cell::new(false)),
            drag_start_y: Arc::new(Cell::new(0.0)),
            split_area_height: Arc::new(Cell::new(600.0)),
        }
    }

    /// Handle feature selection from the panel.
    fn on_feature_selected(feature_id: Uuid, editor: &Entity<FeatureEditor>, cx: &mut App) {
        let editor_clone = editor.clone();
        let background_executor = cx.background_executor().clone();

        cx.spawn(async move |cx| {
            let result = background_executor
                .spawn(async move {
                    let db = Database::open_default()?;
                    db.get_feature(feature_id)
                })
                .await;

            match result {
                Ok(Some(feature)) => {
                    // Write to context file for MCP server
                    if let Err(e) = context_file::write_context(feature.id, &feature.title) {
                        eprintln!("Failed to write context file: {}", e);
                    }

                    // Update global active feature context
                    cx.update(|cx| {
                        ActiveFeatureContext::set(
                            ActiveFeatureContext {
                                feature_id: Some(feature.id),
                                feature_title: Some(feature.title.clone()),
                                feature_details: feature.details.clone(),
                            },
                            cx,
                        );
                    });

                    // Update editor
                    cx.update_entity(&editor_clone, |editor, cx| {
                        editor.open_feature(feature.id, feature.title, feature.details, cx);
                    });
                }
                Ok(None) => {
                    eprintln!("Feature not found: {}", feature_id);
                }
                Err(e) => {
                    eprintln!("Failed to load feature: {}", e);
                }
            }
        })
        .detach();
    }

    /// Fetch features directly from the database (blocking, runs on background thread).
    fn fetch_features() -> Result<Vec<manifest_client::Feature>, String> {
        let db = Database::open_default().map_err(|e| format!("Failed to open database: {}", e))?;
        db.migrate()
            .map_err(|e| format!("Failed to migrate database: {}", e))?;

        let project_path = "/Users/alastair/Documents/work/rocket-tycoon/RocketManifest";

        // Try to find project by directory
        if let Ok(Some(project_with_dirs)) = db.get_project_by_directory(project_path) {
            eprintln!(
                "Found project '{}' for directory",
                project_with_dirs.project.name
            );
            match db.get_feature_tree(project_with_dirs.project.id) {
                Ok(features) => {
                    eprintln!(
                        "Loaded {} features from '{}'",
                        features.len(),
                        project_with_dirs.project.name
                    );
                    let converted: Vec<_> = features
                        .into_iter()
                        .map(convert::tree_node_to_feature)
                        .collect();
                    return Ok(converted);
                }
                Err(e) => {
                    eprintln!("Error fetching features: {}", e);
                }
            }
        }

        // Fallback: find first project with features
        let projects = db
            .get_all_projects()
            .map_err(|e| format!("Failed to fetch projects: {}", e))?;

        for project in &projects {
            match db.get_feature_tree(project.id) {
                Ok(features) if !features.is_empty() => {
                    eprintln!(
                        "Found {} features in project '{}'",
                        features.len(),
                        project.name
                    );
                    let converted: Vec<_> = features
                        .into_iter()
                        .map(convert::tree_node_to_feature)
                        .collect();
                    return Ok(converted);
                }
                Ok(_) => continue,
                Err(e) => {
                    eprintln!("Error fetching features for '{}': {}", project.name, e);
                }
            }
        }

        Err("No projects with features found".into())
    }

    /// Handle mouse down on divider.
    fn on_divider_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.dragging_divider.set(true);
            // Convert Pixels to f32 using division
            self.drag_start_y.set(event.position.y / px(1.0));

            // Double-click resets to equal split
            if event.click_count >= 2 {
                let mut flexes = self.pane_flexes.lock();
                *flexes = vec![1.0, 1.0];
            }
        }
    }

    /// Handle mouse up (stop dragging).
    fn on_divider_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.dragging_divider.set(false);
    }

    /// Handle mouse move while dragging.
    fn on_divider_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dragging_divider.get() {
            return;
        }

        // Convert Pixels to f32 using division
        let current_y = event.position.y / px(1.0);
        let delta_y = current_y - self.drag_start_y.get();
        let total_height = self.split_area_height.get();

        if total_height <= 0.0 {
            return;
        }

        // Convert pixel delta to flex delta
        let flex_delta = delta_y / total_height;

        let mut flexes = self.pane_flexes.lock();
        let total_flex: f32 = flexes.iter().sum();

        // Calculate new flex values
        let new_editor_flex = (flexes[0] + flex_delta * total_flex).max(0.1);
        let new_terminal_flex = (flexes[1] - flex_delta * total_flex).max(0.1);

        // Check minimum heights
        let editor_height =
            (new_editor_flex / (new_editor_flex + new_terminal_flex)) * total_height;
        let terminal_height =
            (new_terminal_flex / (new_editor_flex + new_terminal_flex)) * total_height;

        if editor_height >= MIN_PANE_HEIGHT && terminal_height >= MIN_PANE_HEIGHT {
            flexes[0] = new_editor_flex;
            flexes[1] = new_terminal_flex;
            self.drag_start_y.set(current_y);
            cx.notify();
        }
    }
}

impl Render for ManifestApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_color: gpui::Hsla = TerminalColors::background().into();

        // Get current flex values as ratios (0.0-1.0)
        let flexes = self.pane_flexes.lock().clone();
        let total_flex: f32 = flexes.iter().sum();
        let editor_ratio = flexes[0] / total_flex;
        let terminal_ratio = flexes[1] / total_flex;

        let is_dragging = self.dragging_divider.get();
        let divider_color = if is_dragging {
            colors::divider_hover()
        } else {
            colors::divider()
        };

        div()
            .id("manifest-app")
            .size_full()
            .bg(bg_color)
            .flex()
            .flex_row()
            // Left: Feature panel (fixed 250px)
            .child(self.feature_panel.clone())
            // Right: Split pane area
            .child(
                div()
                    .id("split-pane-area")
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_divider_mouse_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_divider_mouse_up))
                    .on_mouse_move(cx.listener(Self::on_divider_mouse_move))
                    // Top: Feature editor
                    .child(
                        div()
                            .id("editor-pane")
                            .flex_grow()
                            .flex_shrink()
                            .flex_basis(relative(editor_ratio))
                            .min_h(px(MIN_PANE_HEIGHT))
                            .w_full()
                            .child(self.feature_editor.clone()),
                    )
                    // Divider
                    .child(
                        div()
                            .id("pane-divider")
                            .h(px(DIVIDER_HITBOX_SIZE))
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor(CursorStyle::ResizeRow)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(Self::on_divider_mouse_down),
                            )
                            .child(div().h(px(DIVIDER_SIZE)).w_full().bg(divider_color)),
                    )
                    // Bottom: Terminal
                    .child(
                        div()
                            .id("terminal-pane")
                            .flex_grow()
                            .flex_shrink()
                            .flex_basis(relative(terminal_ratio))
                            .min_h(px(MIN_PANE_HEIGHT))
                            .w_full()
                            .child(self.terminal_view.clone()),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // Initialize global active feature context
        cx.set_global(ActiveFeatureContext::default());

        // Register global actions
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &Open, _cx| {
            eprintln!("Open action triggered");
        });
        cx.on_action(|_: &OpenRecent, _cx| {
            eprintln!("Open Recent action triggered");
        });

        // Set up application menus
        set_menus(cx);

        // Bind global keys
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-o", Open, None),
        ]);

        // Register feature editor key bindings
        feature_editor::register_bindings(cx);

        // Open the main window
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(100.0), px(100.0)),
                size: size(px(1200.0), px(800.0)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Manifest".into()),
                appears_transparent: false,
                ..Default::default()
            }),
            focus: true,
            show: true,
            kind: gpui::WindowKind::Normal,
            is_movable: true,
            app_id: Some("com.manifest.app".into()),
            ..Default::default()
        };

        cx.open_window(window_options, |window, cx| {
            cx.new(|cx| ManifestApp::new(window, cx))
        })
        .ok();

        cx.activate(true);
    });
}
