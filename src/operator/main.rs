use anyhow::Result;
use kube::Client;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        "Starting canary-gate-operator v{}",
        env!("CARGO_PKG_VERSION")
    );

    let client = Client::try_default()
        .await
        .expect("Failed to create Kubernetes client");

    canary_gate::operator::controller::run(client).await?;

    Ok(())
}
