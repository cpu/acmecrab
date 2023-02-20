use crate::api::routes;
use crate::config::SharedConfig;
use crate::txt_store::DynTxtStore;
use std::future::Future;
use std::net::SocketAddr;

#[derive(Clone)]
pub(super) struct AppState {
    pub config: SharedConfig,
    pub txt_store: DynTxtStore,
}

pub(crate) fn new(
    config: SharedConfig,
    txt_store: DynTxtStore,
) -> impl Future<Output = hyper::Result<()>> {
    axum::Server::bind(&config.api_bind_addr).serve(
        routes::new(AppState { config, txt_store })
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
}
