use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{sync::Arc, time::Duration};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, decompression::RequestDecompressionLayer, timeout::TimeoutLayer,
};

use crate::{config::Config, pdf::world::World, upload::upload};

mod config;
mod pdf;
mod upload;

#[cfg(test)]
mod tests;

struct AppState {
    config: Config,
    world: World,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .filter_level(Config::default().loglevel)
        .init();

    let config = Config::init();
    let world = World::new(config.rootdir.clone(), config.cachedir.clone());

    let state = Arc::new(AppState { config, world });

    log::set_max_level(state.config.loglevel);

    log::debug!("Loaded config: {:#?}", state.config);

    let app = Router::new()
        .route("/", post(handler))
        .with_state(state.clone())
        .fallback(Redirect::to("/"))
        .layer(
            ServiceBuilder::new()
                .layer(RequestDecompressionLayer::new())
                .layer(CompressionLayer::new()),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.timeout),
        ));

    let listener = tokio::net::TcpListener::bind(&state.config.bindaddress).await;

    match listener {
        Ok(r) => {
            log::info!("Listening on {}.", &state.config.bindaddress);
            axum::serve(r, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap();
        }
        Err(e) => {
            log::error!(
                "Failed to bind TCP listener on {}: {}",
                state.config.bindaddress,
                e
            );
            return;
        }
    }
}

async fn handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RequestMessage>,
) -> impl IntoResponse {
    log::info!("Incoming request.");
    log::debug!("Request body: {body:#?}");

    let compiled = pdf::compile(&state.world, body.template, body.input.to_string());

    #[allow(clippy::single_match_else)]
    match compiled {
        Ok(document) => match &state.config.s3 {
            Some(c) => {
                let upload_result = upload(c, &make_filename(&body.name, &state), document).await;

                let response = match upload_result {
                    Ok(url) => {
                        log::info!("Responding to request with URL of uploaded PDF.");
                        AppResult::Success(SuccessMessage::Uploaded(url))
                    }
                    Err(e) => {
                        log::error!("Could not upload to S3: {e}");
                        AppResult::Error(
                            ["Could not upload to S3.".to_string(), e.to_string()].to_vec(),
                        )
                    }
                };
                log::debug!("Responding with: {response:#?}");

                response
            }
            None => {
                log::info!("Responding to request with PDF in body.");
                AppResult::Success(SuccessMessage::InResponse(body.name, document))
            }
        },
        Err(e) => {
            log::warn!("Failed to produce PDF: {e:#?}");
            AppResult::Error(e.iter().map(|d| d.message.to_string()).collect())
        }
    }
}

#[derive(Serialize, Debug)]
enum AppResult {
    Success(SuccessMessage),
    Error(Vec<String>),
}

#[derive(Serialize, Debug)]
enum SuccessMessage {
    Uploaded(String),
    InResponse(String, Vec<u8>),
}

impl IntoResponse for AppResult {
    fn into_response(self) -> Response {
        match self {
            AppResult::Success(m) => match m {
                SuccessMessage::Uploaded(message) => (
                    StatusCode::CREATED,
                    [(header::CONTENT_TYPE, "text/plain")],
                    message,
                )
                    .into_response(),
                SuccessMessage::InResponse(name, body) => (
                    StatusCode::CREATED,
                    [
                        (header::CONTENT_TYPE, "application/pdf"),
                        (header::CONTENT_DISPOSITION, &format!("filename={name}.pdf")),
                    ],
                    body,
                )
                    .into_response(),
            },
            AppResult::Error(m) => (StatusCode::INTERNAL_SERVER_ERROR, Json(m)).into_response(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct RequestMessage {
    name: String,
    template: String,
    input: Box<RawValue>,
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to instantiate Ctrl+C handler.");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to instantiate signal handler.")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

fn make_filename(name: &str, state: &Arc<AppState>) -> String {
    let timestampformat = state
        .config
        .timestampformat_parsed
        .as_ref()
        .expect("BUG: could not unwrap timestamp.");

    let now = time::UtcDateTime::now();

    let timestamp = now.format(timestampformat).unwrap();

    format!("{timestamp}-{name}.pdf")
}
