//! Feature tree component for the sidebar
//!
//! Displays the project's modules and features in a hierarchical tree view,
//! similar to Zed's file browser.

use gpui::*;
use legion_db::{Database, Feature, FeatureStatus, Module, Project};
use std::collections::HashSet;
use std::sync::Arc;

use crate::components::{ListItem, ListItemSpacing};
use crate::theme::Theme;

/// Event emitted when a feature is selected
pub struct FeatureSelected {
    pub feature_id: String,
}

/// Event emitted when user wants to open a project
pub struct OpenProjectRequested;

impl EventEmitter<FeatureSelected> for FeatureTree {}
impl EventEmitter<OpenProjectRequested> for FeatureTree {}

/// Flattened tree entry for uniform_list rendering
#[derive(Clone)]
enum TreeEntry {
    /// Project root node
    Project(Project),
    /// Module (folder) node
    Module { module: Module },
    /// Feature (file) node
    Feature { feature: Feature },
}

/// The feature tree sidebar component
pub struct FeatureTree {
    db: Arc<Database>,
    project: Option<Project>,
    modules: Vec<ModuleEntry>,
    entries: Vec<TreeEntry>,
    expanded_modules: HashSet<String>,
    selected_feature: Option<String>,
    focus_handle: FocusHandle,
    #[allow(dead_code)] // Reserved for uniform_list virtualization
    scroll_handle: UniformListScrollHandle,
}

struct ModuleEntry {
    module: Module,
    features: Vec<Feature>,
}

impl FeatureTree {
    pub fn new(db: Arc<Database>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut tree = Self {
            db,
            project: None,
            modules: Vec::new(),
            entries: Vec::new(),
            expanded_modules: HashSet::new(),
            selected_feature: None,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::new(),
        };

        tree.refresh(cx);
        tree
    }

    /// Set the active project and reload the tree
    pub fn set_project(&mut self, project: Project, cx: &mut Context<Self>) {
        self.project = Some(project);
        self.expanded_modules.clear();
        self.selected_feature = None;
        self.refresh(cx);
    }

    /// Clear the current project
    #[allow(dead_code)]
    pub fn clear_project(&mut self, cx: &mut Context<Self>) {
        self.project = None;
        self.modules.clear();
        self.entries.clear();
        self.expanded_modules.clear();
        self.selected_feature = None;
        cx.notify();
    }

    /// Refresh the tree from the database
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(project) = &self.project else {
            self.modules.clear();
            self.entries.clear();
            cx.notify();
            return;
        };

        let project_id = project.id.clone();
        let db = self.db.clone();

        // Load modules and features
        let result = db.with_connection(|conn| {
            let modules = Module::list_by_project(conn, &project_id)?;
            let mut entries = Vec::new();

            for module in modules {
                let features = Feature::list_by_module(conn, &module.id)?;
                entries.push(ModuleEntry { module, features });
            }

            Ok(entries)
        });

        match result {
            Ok(entries) => {
                self.modules = entries;
                self.rebuild_entries();
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to load feature tree: {}", e);
            }
        }
    }

    /// Rebuild the flattened entry list from modules
    fn rebuild_entries(&mut self) {
        self.entries.clear();

        // Add project root
        if let Some(project) = &self.project {
            self.entries.push(TreeEntry::Project(project.clone()));
        }

        // Add modules and their features
        for entry in &self.modules {
            self.entries.push(TreeEntry::Module {
                module: entry.module.clone(),
            });

            // Only add features if module is expanded
            if self.expanded_modules.contains(&entry.module.id) {
                for feature in &entry.features {
                    self.entries.push(TreeEntry::Feature {
                        feature: feature.clone(),
                    });
                }
            }
        }
    }

    /// Toggle module expansion
    fn toggle_module(&mut self, module_id: &str, cx: &mut Context<Self>) {
        if self.expanded_modules.contains(module_id) {
            self.expanded_modules.remove(module_id);
        } else {
            self.expanded_modules.insert(module_id.to_string());
        }
        self.rebuild_entries();
        cx.notify();
    }

    /// Select a feature
    fn select_feature(&mut self, feature_id: &str, cx: &mut Context<Self>) {
        self.selected_feature = Some(feature_id.to_string());
        cx.emit(FeatureSelected {
            feature_id: feature_id.to_string(),
        });
        cx.notify();
    }

    /// Get the currently selected feature
    #[allow(dead_code)]
    pub fn selected_feature(&self) -> Option<&str> {
        self.selected_feature.as_deref()
    }

    /// Create a new module in the current project
    #[allow(dead_code)]
    pub fn create_module(&mut self, name: &str, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let Some(project) = &self.project else {
            anyhow::bail!("No project selected");
        };

        let module = Module::new(&project.id, name);
        self.db.with_connection(|conn| module.insert(conn))?;
        self.refresh(cx);
        Ok(())
    }

    /// Create a new feature in a module
    #[allow(dead_code)]
    pub fn create_feature(
        &mut self,
        module_id: &str,
        title: &str,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<String> {
        let feature = Feature::new(module_id, title);
        let feature_id = feature.id.clone();
        self.db.with_connection(|conn| feature.insert(conn))?;

        // Expand the parent module and select the new feature
        self.expanded_modules.insert(module_id.to_string());
        self.select_feature(&feature_id, cx);
        self.refresh(cx);

        Ok(feature_id)
    }

    /// Render a single tree entry
    fn render_entry(&self, entry: &TreeEntry, cx: &mut Context<Self>) -> AnyElement {
        match entry {
            TreeEntry::Project(project) => self.render_project(project, cx),
            TreeEntry::Module { module } => self.render_module(module, cx),
            TreeEntry::Feature { feature } => self.render_feature(feature, cx),
        }
    }

    /// Render the project root node
    fn render_project(&self, project: &Project, _cx: &mut Context<Self>) -> AnyElement {
        ListItem::new(SharedString::from(format!("project-{}", project.id)))
            .indent_level(0)
            .spacing(ListItemSpacing::Default)
            .toggle(Some(true)) // Project is always expanded
            .start_slot(
                div()
                    .text_sm()
                    .text_color(Theme::text_muted())
                    .child("📁"),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(Theme::text())
                    .child(project.name.clone()),
            )
            .into_any_element()
    }

    /// Render a module (folder) node
    fn render_module(&self, module: &Module, cx: &mut Context<Self>) -> AnyElement {
        let module_id = module.id.clone();
        let is_expanded = self.expanded_modules.contains(&module_id);

        let module_id_for_click = module_id.clone();

        ListItem::new(SharedString::from(format!("module-{}", module_id)))
            .indent_level(1)
            .spacing(ListItemSpacing::Default)
            .toggle(Some(is_expanded))
            .on_toggle(cx.listener(move |this, _event, _window, cx| {
                this.toggle_module(&module_id_for_click, cx);
            }))
            .on_click(cx.listener({
                let module_id = module_id.clone();
                move |this, _event, _window, cx| {
                    this.toggle_module(&module_id, cx);
                }
            }))
            .start_slot(
                div()
                    .text_sm()
                    .text_color(Theme::text_muted())
                    .child(if is_expanded { "📂" } else { "📁" }),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(Theme::text())
                    .child(module.name.clone()),
            )
            .into_any_element()
    }

    /// Render a feature (file) node
    fn render_feature(&self, feature: &Feature, cx: &mut Context<Self>) -> AnyElement {
        let feature_id = feature.id.clone();
        let is_selected = self.selected_feature.as_deref() == Some(&feature_id);

        // Status icon based on feature status
        let status_icon = match feature.status {
            FeatureStatus::Active => ("●", Theme::success()),
            FeatureStatus::Paused => ("◐", Theme::warning()),
            FeatureStatus::Archived => ("○", Theme::text_muted()),
            FeatureStatus::Draft => ("◌", Theme::text_muted()),
        };

        ListItem::new(SharedString::from(format!("feature-{}", feature_id)))
            .indent_level(2)
            .spacing(ListItemSpacing::Default)
            .selected(is_selected)
            .on_click(cx.listener({
                let feature_id = feature_id.clone();
                move |this, _event, _window, cx| {
                    this.select_feature(&feature_id, cx);
                }
            }))
            .start_slot(
                div()
                    .text_sm()
                    .text_color(status_icon.1)
                    .child(status_icon.0),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if is_selected {
                        Theme::text()
                    } else {
                        Theme::text_editor()
                    })
                    .child(feature.title.clone()),
            )
            .into_any_element()
    }

    /// Render empty state when no project is loaded
    fn render_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(Theme::text_muted())
                    .child("No project open"),
            )
            .child(
                div()
                    .id("open-project-btn")
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(Theme::element())
                    .text_color(Theme::text())
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|s| s.bg(Theme::element_hover()))
                    .on_click(cx.listener(|_this, _event, _window, cx| {
                        tracing::info!("Open Project button clicked, emitting event");
                        cx.emit(OpenProjectRequested);
                    }))
                    .child("Open Project"),
            )
    }

    /// Render empty modules state
    fn render_empty_modules(&self) -> impl IntoElement {
        div()
            .px_3()
            .py_4()
            .text_sm()
            .text_color(Theme::text_muted())
            .child("No modules yet")
    }
}

impl Render for FeatureTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Empty state: no project
        if self.project.is_none() {
            return div()
                .id("feature-tree")
                .track_focus(&self.focus_handle)
                .size_full()
                .flex()
                .flex_col()
                .child(self.render_empty_state(cx))
                .into_any_element();
        }

        // Empty state: no modules
        if self.modules.is_empty() {
            return div()
                .id("feature-tree")
                .track_focus(&self.focus_handle)
                .size_full()
                .flex()
                .flex_col()
                .child(self.render_empty_modules())
                .into_any_element();
        }

        // Build rendered entries
        let entries: Vec<AnyElement> = self
            .entries
            .iter()
            .map(|entry| self.render_entry(entry, cx).into_any_element())
            .collect();

        div()
            .id("feature-tree")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .bg(Theme::surface())
            .children(entries)
            .into_any_element()
    }
}
