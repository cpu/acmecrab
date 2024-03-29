use crate::config::Shared;
use crate::dns::handlers::Handler;
use crate::txt_store::DynTxtStore;
use tokio::net::{TcpListener, UdpSocket};
use trust_dns_server::ServerFuture;

/// Construct a server future for a ACME Crab DNS server with the given [Shared] config. The server
/// will respond to TXT record requests using the [`DynTxtStore`].  
///
/// # Errors
///
/// Returns [`crate::error::Error::IO`] if the DNS server UDP or TCP sockets specified in the
/// [Shared] config can't be bound (e.g. because they are already in use).
pub async fn new(config: Shared, txt_store: DynTxtStore) -> anyhow::Result<ServerFuture<Handler>> {
    let udp_addr = config.dns_udp_bind_addr;
    let tcp_addr = config.dns_tcp_bind_addr;
    let tcp_timeout = config.dns_tcp_timeout;
    let dns_handler = Handler::new(config, txt_store);
    let mut dns_server = ServerFuture::new(dns_handler);
    dns_server.register_socket(UdpSocket::bind(udp_addr).await?);
    dns_server.register_listener(TcpListener::bind(tcp_addr).await?, tcp_timeout);
    Ok(dns_server)
}
