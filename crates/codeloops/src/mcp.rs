use crate::{
    AppResult,
    http::{ApiError, Client},
};
use rmcp::{
    ServerHandler, ServiceExt, handler::server::wrapper::Parameters, model::CallToolResult, tool,
    tool_handler, tool_router,
};
use serde::Deserialize;
use session_history::model::{Capture, Query};

#[derive(Clone)]
pub struct Server {
    client: Client,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct QueryInput {
    pub request: Query,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CaptureInput {
    pub capture: Capture,
}

fn result(value: Result<serde_json::Value, ApiError>) -> CallToolResult {
    match value {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => CallToolResult::structured_error(serde_json::json!({"error": error})),
    }
}

#[tool_router]
impl Server {
    #[tool(
        description = "Retrieve durable session history: list/search/show/entry/captures; \
            checkpoint metadata; compare HEAD/index/worktree layers; \
            changes by session_id or entry_id plus workspace_id; \
            file bytes by checkpoint_id and base64 path; \
            artifact chunks for tool input/output/error and patches. \
            Export by session_id returns an immutable manifest_hash; retrieve it \
            and every artifact in its inventory through artifact queries. \
            Follow next_cursor/next_offset. Check boundary and checkpoint coverage: \
            observations do not attribute authorship."
    )]
    async fn history_query(&self, Parameters(input): Parameters<QueryInput>) -> CallToolResult {
        result(self.client.post("/v1/history/query", &input.request).await)
    }

    #[tool(description = "Ingest a version-1 capture envelope. \
            Persist a UUID delivery_id before retrying. \
            Reusing a delivery ID with different content is a conflict. \
            Source sequences order revisions within one installation.")]
    async fn history_ingest(&self, Parameters(input): Parameters<CaptureInput>) -> CallToolResult {
        result(self.client.post("/v1/history/ingest", &input.capture).await)
    }
}

#[tool_handler(name = "codeloops-history", version = "0.1.0")]
impl ServerHandler for Server {}

pub async fn serve(client: Client) -> AppResult<()> {
    Server { client }
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
