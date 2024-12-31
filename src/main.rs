mod logging;
mod mapper;
mod routes;

use std::{net::SocketAddr, process};

use clap::Parser;
use colored::Colorize;
use rustls::crypto;
use tokio::signal;
use tracing::{error, info, warn};
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

/// ret2traffic is a reversed proxy with REST-ful API.
#[derive(Parser)]
#[command(name = "ret2traffic", bin_name = "ret2traffic", version, about)]
enum Args {
    /// Start the server
    #[clap(alias("s"))]
    Serve {
        /// Host to listen on
        #[clap(long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[clap(long, default_value = "9969")]
        port: u16,
        /// Log in JSON format
        #[clap(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args {
        Args::Serve { host, port, json } => serve(&host, port, json).await,
    }
}

pub fn greet() {
    println!(
        "[START UP] {} {}",
        "Ret 2 Traffic".bold(),
        R2T_FULL_VERSION.dimmed()
    );
    println!(
        "----------------------------- {} -----------------------------",
        "server log starts here".to_uppercase().bold()
    );
}

async fn serve(host: &str, port: u16, json: bool) {
    greet();
    logging::init_logger(json);

    info!(">> Server initialization started <<");

    match crypto::aws_lc_rs::default_provider().install_default() {
        Ok(_) => info!("using `AWS Libcrypto` as default crypto backend."),
        Err(err) => {
            error!("`AWS Libcrypto` is not available: {:?}", err);
            warn!("try to use `ring` as default crypto backend.");
            crypto::ring::default_provider()
                .install_default()
                .inspect_err(|err| {
                    error!("`ring` is not available: {:?}", err);
                    error!("All crypto backend are not available, exiting...");
                    process::exit(1);
                })
                .ok();
            info!("using `ring` as default crypto backend.");
        }
    }
    info!("Constructing router...");
    let router = routes::initialize().await;
    info!(">> Server initialization finished <<");
    let addr_str = format!("{}:{}", &host, &port);

    let addr = tokio::net::TcpListener::bind(addr_str.clone())
        .await
        .expect("Failed to bind server address");
    info!("高性能ですから！\\ Φ ω Φ /");
    info!("Server started at [ {} ]", addr_str);
    axum::serve(
        addr,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Failed to start server.");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Termination `Ctrl+C` received, shutting down...");
            std::process::exit(0);
        },
        _ = terminate => {
            info!("Termination signal received, shutting down...");
            std::process::exit(0);
        },
    }
}
