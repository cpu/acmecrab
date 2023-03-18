use acmecrab::error::Error::DNSError;
use acmecrab::{Config, Shared};
use anyhow::{anyhow, Result};
use is_terminal::IsTerminal;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_init();

    let mut first_args = std::env::args().take(2);
    let (program_name, config_file) = (
        first_args.next().unwrap_or("acmecrab".to_string()),
        first_args.next(),
    );

    let config = config_init(&program_name, config_file)?;
    let txt_store = config.txt_store().await?;

    if std::io::stdout().is_terminal() {
        println!("{}", acmecrab::crab::CRAB);
    }

    tracing::info!("DNS listening on UDP {}", &config.dns_udp_bind_addr);
    tracing::info!("DNS listening on TCP {}", &config.dns_tcp_bind_addr);
    let dns_server = acmecrab::dns::new(config.clone(), txt_store.clone()).await?;
    let dns_handle = tokio::spawn(dns_server.block_until_done());

    tracing::info!("API listening on {}", &config.api_bind_addr);
    let api_server = acmecrab::api::new(config.clone(), txt_store.clone());
    let api_handle = tokio::spawn(api_server);

    // TODO(XXX): proper graceful shutdown.
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("quitting from signal");
        },
        Ok(dns_res) = dns_handle => {
            if let Err(err) = dns_res {
                return Err(DNSError(err).into())
            }
        }
        Ok(api_res) = api_handle => {
            if let Err(err) = api_res {
                return Err(err.into())
            }
        }
    }
    tracing::info!("goodbye");
    Ok(())
}

fn tracing_init() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "acmecrab=info".into()),
        )
        .init();
}

fn config_init(program_name: &str, config_file: Option<String>) -> Result<Shared> {
    match config_file {
        None => Err(anyhow!("usage: {program_name} /path/to/config.json")),
        Some(config_file) => {
            tracing::debug!("loaded config from {config_file}");
            let config = Config::try_from_file(&config_file)?;
            Ok(Arc::new(config))
        }
    }
}
