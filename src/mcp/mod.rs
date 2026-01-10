use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerInfo},
    tool, tool_handler, tool_router,
    schemars::JsonSchema,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::Database;
use crate::models::*;

#[derive(Clone)]
pub struct McpServer {
    db: Database,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTaskContextRequest {
    #[schemars(description = "The task ID to get context for")]
    pub task_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddImplementationNoteRequest {
    #[schemars(description = "The task ID to add a note to")]
    pub task_id: String,
    #[schemars(description = "The content of the implementation note")]
    pub content: String,
    #[schemars(description = "List of files changed")]
    #[serde(default)]
    pub files_changed: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompleteTaskRequest {
    #[schemars(description = "The task ID to mark as complete")]
    pub task_id: String,
}

#[derive(Debug, Serialize)]
pub struct TaskContext {
    pub task: Task,
    pub feature_title: String,
    pub feature_story: Option<String>,
    pub feature_details: Option<String>,
}

impl McpServer {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            tool_router: Self::tool_router(),
        }
    }

    fn parse_uuid(s: &str) -> Result<Uuid, McpError> {
        Uuid::parse_str(s)
            .map_err(|e| McpError::invalid_params(format!("Invalid UUID: {}", e), None))
    }
}

#[tool_router]
impl McpServer {
    #[tool(description = "Get the context for an assigned task, including feature details")]
    async fn get_task_context(
        &self,
        params: Parameters<GetTaskContextRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let task_id = Self::parse_uuid(&req.task_id)?;

        let task = self.db.get_task(task_id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .ok_or_else(|| McpError::invalid_params("Task not found", None))?;

        let session = self.db.get_session(task.session_id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .ok_or_else(|| McpError::internal_error("Session not found", None))?;

        let feature = self.db.get_feature(session.feature_id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .ok_or_else(|| McpError::internal_error("Feature not found", None))?;

        let context = TaskContext {
            task,
            feature_title: feature.title,
            feature_story: feature.story,
            feature_details: feature.details,
        };

        let json = serde_json::to_string_pretty(&context)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Add an implementation note to a task")]
    async fn add_implementation_note(
        &self,
        params: Parameters<AddImplementationNoteRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let task_id = Self::parse_uuid(&req.task_id)?;

        let note = self.db.create_note_for_task(task_id, CreateImplementationNoteInput {
            content: req.content,
            files_changed: req.files_changed,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!("Note added with id: {}", note.id))]))
    }

    #[tool(description = "Mark a task as complete")]
    async fn complete_task(
        &self,
        params: Parameters<CompleteTaskRequest>,
    ) -> Result<CallToolResult, McpError> {
        let req = params.0;
        let task_id = Self::parse_uuid(&req.task_id)?;

        let updated = self.db.update_task(task_id, UpdateTaskInput {
            status: Some(TaskStatus::Completed),
            worktree_path: None,
            branch: None,
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if !updated {
            return Err(McpError::invalid_params("Task not found", None));
        }

        Ok(CallToolResult::success(vec![Content::text("Task marked as complete")]))
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("RocketManifest MCP server for AI-assisted feature development".into()),
            ..Default::default()
        }
    }
}

pub async fn run_stdio_server(db: Database) -> anyhow::Result<()> {
    use tokio::io::{stdin, stdout};

    tracing::info!("Starting MCP server via stdio");

    let service = McpServer::new(db);
    let server = service.serve((stdin(), stdout())).await?;

    let quit_reason = server.waiting().await?;
    tracing::info!("MCP server stopped: {:?}", quit_reason);

    Ok(())
}
