use gpui::*;
use legion_db::{Database, Project};
use rfd::AsyncFileDialog;
use std::path::PathBuf;
use std::sync::Arc;

use crate::actions::{OpenProject, OpenRecentProject};
use crate::theme::Theme;
use crate::workspace::Workspace;

/// Event emitted when a project is opened
pub struct ProjectOpened {
    pub project: Project,
}

impl EventEmitter<ProjectOpened> for LegionApp {}

pub struct LegionApp {
    db: Arc<Database>,
    workspace: Entity<Workspace>,
}

impl LegionApp {
    pub fn new(
        db: Arc<Database>,
        initial_project: Option<Project>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let workspace = cx.new(|cx| Workspace::new(db.clone(), window, cx));

        let app = Self { db, workspace };

        // Auto-open initial project if provided
        if let Some(project) = initial_project {
            tracing::info!("Auto-opening initial project: {}", project.name);
            app.workspace.update(cx, |workspace, cx| {
                workspace.set_project(project, cx);
            });
        }

        app
    }

    /// Get the workspace entity
    pub fn workspace(&self) -> &Entity<Workspace> {
        &self.workspace
    }

    /// Open a project by its ID from the database
    pub fn open_project_by_id(&mut self, project_id: &str, cx: &mut Context<Self>) {
        let db = self.db.clone();
        let project_id = project_id.to_string();

        match db.with_connection(|conn| Project::find_by_id(conn, &project_id)) {
            Ok(Some(project)) => {
                self.workspace.update(cx, |workspace, cx| {
                    workspace.set_project(project.clone(), cx);
                });
                cx.emit(ProjectOpened { project });
                cx.notify();
            }
            Ok(None) => {
                tracing::warn!("Project not found: {}", project_id);
            }
            Err(e) => {
                tracing::error!("Failed to load project: {}", e);
            }
        }
    }

    fn open_recent_project(
        &mut self,
        action: &OpenRecentProject,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        tracing::info!("Opening recent project: {}", action.project_id);
        self.open_project_by_id(&action.project_id, cx);
    }

    fn open_project(&mut self, _: &OpenProject, window: &mut Window, cx: &mut Context<Self>) {
        tracing::info!("open_project handler called in LegionApp");
        let db = self.db.clone();

        cx.spawn_in(window, async |this, cx| {
            tracing::info!("Opening folder picker dialog");

            let folder = AsyncFileDialog::new()
                .set_title("Open Project")
                .pick_folder()
                .await;

            if let Some(folder) = folder {
                let path = folder.path().to_path_buf();
                tracing::info!("Selected folder: {:?}", path);
                Self::create_and_open_project(this, db, path, cx)?;
            } else {
                tracing::info!("User cancelled dialog");
            }

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn create_and_open_project(
        this: WeakEntity<Self>,
        db: Arc<Database>,
        path: PathBuf,
        cx: &mut AsyncWindowContext,
    ) -> anyhow::Result<()> {
        // Extract project name from path
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let path_str = path.to_string_lossy().to_string();

        // Check if project already exists
        let existing = db.with_connection(|conn| Project::find_by_path(conn, &path_str))?;

        let project = if let Some(existing) = existing {
            existing
        } else {
            let project = Project::new(&name, &path_str);
            db.with_connection(|conn| project.insert(conn))?;
            project
        };

        // Update workspace with new project
        this.update(cx, |app, cx| {
            app.workspace.update(cx, |workspace, cx| {
                workspace.set_project(project.clone(), cx);
            });
            cx.emit(ProjectOpened { project });
            cx.notify();
        })?;

        Ok(())
    }
}

impl Render for LegionApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(Theme::background())
            .text_color(Theme::text())
            .on_action(cx.listener(Self::open_project))
            .on_action(cx.listener(Self::open_recent_project))
            .child(self.workspace.clone())
    }
}
