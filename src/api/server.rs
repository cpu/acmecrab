use crate::api::routes;
use crate::config::Shared;
use crate::txt_store::DynTxtStore;
use std::future::Future;
use std::net::SocketAddr;

#[derive(Clone)]
pub(super) struct AppState {
    pub config: Shared,
    pub txt_store: DynTxtStore,
}

/// Construct a [`Future`] for a new API server with the given [Shared] [Config][`crate::config::Config`].
/// Its update API will mutate TXT records in the [`DynTxtStore`]
pub fn new(config: Shared, txt_store: DynTxtStore) -> impl Future<Output = hyper::Result<()>> {
    axum::Server::bind(&config.api_bind_addr).serve(
        routes::new(AppState { config, txt_store })
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
}
