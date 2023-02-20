use crate::api::api_error::APIError;
use crate::api::model::{UpdateRecordRequest, UpdateRecordResult};
use crate::api::server::AppState;
use crate::error::Error;
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::WithRejection;
use serde_json::json;
use std::net::SocketAddr;
use std::str::FromStr;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use trust_dns_server::client::rr::Name;

pub(super) fn new(state: AppState) -> Router {
    Router::new()
        .route("/healthcheck", get(health_check))
        .route("/register", post(register))
        .route("/update", post(update))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(state.config.api_timeout))
        .with_state(state)
}

#[allow(clippy::unused_async)]
async fn health_check() -> impl IntoResponse {
    Json(json!({"ok":"healthy"}))
}

#[allow(clippy::unused_async)]
async fn register() -> APIError {
    Error::NotImplemented.into()
}

async fn update(
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    WithRejection(Json(payload), _): WithRejection<Json<UpdateRecordRequest>, APIError>,
) -> Result<Json<UpdateRecordResult>, APIError> {
    let client_addr = client_addr.ip();
    let subdomain: Name = Name::from_str(&payload.subdomain)?;

    if !state.config.update_permitted(client_addr, &subdomain) {
        tracing::debug!("rejected update from {client_addr} for \"{subdomain}\"",);
        return Err(Error::AuthForbidden(client_addr, subdomain.into()).into());
    }

    match &payload.valid_dns01() {
        Err(err) => {
            tracing::debug!("rejected update from {client_addr} for \"{subdomain}\": {err}",);
            Err(Error::InvalidDNS01.into())
        }
        Ok(_) => {
            let domain: Name = (&state.config.domain).into();
            let fqdn = &subdomain.append_domain(&domain)?;
            tracing::info!("accepted update from {client_addr} for \"{fqdn}\"");
            state
                .txt_store
                .write()
                .await
                .add_txt(fqdn.into(), payload.txt.clone())
                .await?;
            Ok(Json(UpdateRecordResult { txt: payload.txt }))
        }
    }
}
