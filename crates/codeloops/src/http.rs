use crate::{AppResult, config::Config, outbox};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use session_history::{
    History,
    model::{Capture, MAX_CAPTURE_BYTES, Query},
};

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl From<session_history::Error> for ApiError {
    fn from(error: session_history::Error) -> Self {
        Self {
            code: error.code().into(),
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "invalid_request" => StatusCode::BAD_REQUEST,
            "unknown_id" => StatusCode::NOT_FOUND,
            "delivery_conflict" => StatusCode::CONFLICT,
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "unavailable_artifact" => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({"error":self}))).into_response()
    }
}

fn authorize(config: &Config, headers: &HeaderMap) -> Result<(), ApiError> {
    if headers.get("authorization").and_then(|h| h.to_str().ok())
        != Some(&format!("Bearer {}", config.token))
    {
        return Err(ApiError {
            code: "unauthorized".into(),
            message: "application credential required".into(),
        });
    }
    Ok(())
}

fn parse<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(v)| v).map_err(|e| ApiError {
        code: "invalid_request".into(),
        message: e.body_text(),
    })
}

async fn query(
    State(config): State<Config>,
    headers: HeaderMap,
    body: Result<Json<Query>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    authorize(&config, &headers)?;
    let q = parse(body)?;
    tokio::task::spawn_blocking(move || History::open(config.archive())?.query(q))
        .await
        .map_err(|_| ApiError {
            code: "storage_failure".into(),
            message: "query worker failed".into(),
        })?
        .map(Json)
        .map_err(ApiError::from)
}

async fn ingest(
    State(config): State<Config>,
    headers: HeaderMap,
    body: Result<Json<Capture>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    authorize(&config, &headers)?;
    let c = parse(body)?;
    tokio::task::spawn_blocking(move || -> session_history::Result<Value> {
        Ok(serde_json::to_value(
            History::open(config.archive())?.ingest(c)?,
        )?)
    })
    .await
    .map_err(|_| ApiError {
        code: "storage_failure".into(),
        message: "ingest worker failed".into(),
    })?
    .map(Json)
    .map_err(ApiError::from)
}

async fn health(State(config): State<Config>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    authorize(&config, &headers)?;
    tokio::task::spawn_blocking(move || outbox::health(&config))
        .await
        .map_err(|_| ApiError {
            code: "storage_failure".into(),
            message: "health worker failed".into(),
        })?
        .map(Json)
        .map_err(|e| ApiError {
            code: "storage_failure".into(),
            message: e.to_string(),
        })
}

pub async fn serve(config: Config) -> AppResult<()> {
    History::open(config.archive())?;
    let listener = tokio::net::TcpListener::bind(config.address).await?;
    eprintln!(
        "CodeLoops listening at http://{}; data {}",
        listener.local_addr()?,
        config.root.display()
    );
    let worker_config = config.clone();
    let worker = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            interval.tick().await;
            let config = worker_config.clone();
            match tokio::task::spawn_blocking(move || outbox::flush(&config)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("capture flush failed: {error}"),
                Err(error) => eprintln!("capture worker failed: {error}"),
            }
        }
    });
    let router = Router::new()
        .route("/v1/history/query", post(query))
        .route("/v1/history/ingest", post(ingest))
        .route("/v1/health", post(health))
        .layer(DefaultBodyLimit::max(MAX_CAPTURE_BYTES))
        .with_state(config);
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    worker.abort();
    result?;
    Ok(())
}

#[derive(Clone)]
pub struct Client {
    config: Config,
    http: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> AppResult<Self> {
        Ok(Self {
            config,
            http: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
        })
    }

    pub async fn post(&self, path: &str, body: &impl Serialize) -> Result<Value, ApiError> {
        let response = self
            .http
            .post(self.config.url(path))
            .bearer_auth(&self.config.token)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError {
                code: "service_unavailable".into(),
                message: e.to_string(),
            })?;
        let status = response.status();
        let value: Value = response.json().await.map_err(|e| ApiError {
            code: "invalid_response".into(),
            message: e.to_string(),
        })?;
        if !status.is_success() {
            return Err(
                serde_json::from_value(value["error"].clone()).unwrap_or(ApiError {
                    code: "invalid_response".into(),
                    message: format!("HTTP {status}"),
                }),
            );
        }
        Ok(value)
    }
}
