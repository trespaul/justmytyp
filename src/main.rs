use axum::{
    Json, Router,
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, decompression::RequestDecompressionLayer, timeout::TimeoutLayer,
};

use crate::{config::Config, upload::upload};

mod config;
mod pdf;
mod upload;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    let config_result = Config::init();

    env_logger::Builder::new()
        .filter_level(Config::get().loglevel)
        .init();

    if let Err(e) = config_result {
        log::warn!("Failed to load config: {e}; using defaults.");
    };

    log::debug!("Loaded config: {:#?}", Config::get());

    let app = Router::new()
        .route("/", post(handler))
        .fallback(Redirect::to("/"))
        .layer(
            ServiceBuilder::new()
                .layer(RequestDecompressionLayer::new())
                .layer(CompressionLayer::new()),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(Config::get().timeout),
        ));

    let listener = tokio::net::TcpListener::bind(&Config::get().bindaddress).await;

    match listener {
        Ok(r) => {
            log::info!("Listening on {}.", Config::get().bindaddress);
            axum::serve(r, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap();
        }
        Err(e) => {
            log::error!("Failed to bind TCP listener: {e}");
            return;
        }
    };
}

async fn handler(Json(body): Json<RequestMessage>) -> impl IntoResponse {
    log::info!("Incoming request.");
    log::debug!("Request body: {:#?}", body);

    let config = Config::get();

    let compiled = pdf::compile(body.template, body.input.to_string(), config);

    match compiled {
        Ok(document) => match &config.s3 {
            Some(c) => {
                let upload_result = upload(c, body.name.clone(), document).await;

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
                log::debug!("Responding with: {:#?}", response);

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
                // (StatusCode::CREATED, Json(m)).into_response()
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
