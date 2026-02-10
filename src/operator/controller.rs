use std::sync::Arc;

use futures::StreamExt;
use kube::runtime::watcher::Config as WatcherConfig;
use kube::runtime::Controller;
use kube::{Api, Client};

use super::crd::CanaryGate;
use super::reconciler::{error_policy, reconcile, OperatorState};

/// Run the CanaryGate CRD controller.
///
/// This starts the controller loop that watches for CanaryGate resources
/// and reconciles them through the evaluation pipeline.
pub async fn run(client: Client) -> anyhow::Result<()> {
    let api: Api<CanaryGate> = Api::all(client.clone());

    let state = Arc::new(OperatorState { client });

    tracing::info!("Starting CanaryGate controller");

    Controller::new(api, WatcherConfig::default())
        .run(reconcile, error_policy, state)
        .for_each(|res| async {
            match res {
                Ok(obj) => tracing::debug!("Reconciled {:?}", obj),
                Err(e) => tracing::error!("Reconciliation failed: {:?}", e),
            }
        })
        .await;

    Ok(())
}
