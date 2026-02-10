use std::sync::Arc;

use kube::runtime::controller::Action;
use kube::{Api, Client, ResourceExt};
use tokio::time::Duration;

use crate::behavior::evaluate_tests;
use crate::classification::classify_stream;
use crate::recommendation::CycleTracker;
use crate::verdict::Verdict;

use super::crd::{CanaryGate, CanaryGatePhase, CanaryGateStatus};
use super::log_source::fetch_pod_logs;

/// Shared state for the operator controller.
pub struct OperatorState {
    pub client: Client,
}

/// Error type for reconciliation.
#[derive(Debug, thiserror::Error)]
pub enum ReconcileError {
    #[error("Kubernetes API error: {0}")]
    Kube(#[from] kube::Error),
    #[error("Missing spec field: {0}")]
    MissingField(String),
    #[error("Evaluation error: {0}")]
    Evaluation(String),
}

/// Reconcile a CanaryGate resource.
///
/// State machine: Pending → Evaluating → Promote/Hold/Rollback
pub async fn reconcile(
    gate: Arc<CanaryGate>,
    ctx: Arc<OperatorState>,
) -> Result<Action, ReconcileError> {
    let name = gate.name_any();
    let namespace = gate.namespace().unwrap_or_else(|| "default".to_string());

    tracing::info!("Reconciling CanaryGate {}/{}", namespace, name);

    let config: crate::config::Config = (&gate.spec).into();

    // Determine current phase
    let current_phase = gate
        .status
        .as_ref()
        .map(|s| s.phase.clone())
        .unwrap_or(CanaryGatePhase::Pending);

    match current_phase {
        CanaryGatePhase::Pending => {
            // Transition to Evaluating
            update_status(
                &ctx.client,
                &namespace,
                &name,
                CanaryGateStatus {
                    phase: CanaryGatePhase::Evaluating,
                    recommendation: None,
                    cycles: 0,
                    consecutive_passes: 0,
                    conditions: vec![],
                    reasoning: vec!["Starting evaluation".to_string()],
                },
            )
            .await?;
            Ok(Action::requeue(Duration::from_secs(5)))
        }
        CanaryGatePhase::Evaluating => {
            // Fetch pod logs
            let pod_selector = gate.spec.pod_selector.as_deref().unwrap_or("app=canary");

            let lines =
                fetch_pod_logs(ctx.client.clone(), &namespace, pod_selector, Some(300)).await?;

            // Run classification and evaluation
            let events = classify_stream(&lines, &config.logging.events);
            let evaluations = evaluate_tests(&config.tests, &events);

            let mut tracker = CycleTracker::new();

            // Restore previous cycle count from status
            if let Some(ref status) = gate.status {
                tracker.total_cycles = status.cycles;
                tracker.consecutive_passes = status.consecutive_passes;
            }

            tracker.record_cycle(&config.tests, &evaluations, &config.recommendation);

            let verdict = Verdict::from_tracker(&tracker);

            let new_phase = match verdict.recommendation {
                crate::recommendation::Recommendation::Promote => CanaryGatePhase::Promote,
                crate::recommendation::Recommendation::Hold => CanaryGatePhase::Evaluating,
                crate::recommendation::Recommendation::Rollback => CanaryGatePhase::Rollback,
            };

            let requeue_secs = if new_phase == CanaryGatePhase::Evaluating {
                30 // Continue evaluating
            } else {
                300 // Terminal state, check less frequently
            };

            update_status(
                &ctx.client,
                &namespace,
                &name,
                CanaryGateStatus {
                    phase: new_phase,
                    recommendation: Some(format!("{:?}", verdict.recommendation).to_lowercase()),
                    cycles: verdict.total_cycles,
                    consecutive_passes: verdict.consecutive_passes,
                    conditions: vec![],
                    reasoning: verdict.reasoning,
                },
            )
            .await?;

            Ok(Action::requeue(Duration::from_secs(requeue_secs)))
        }
        // Terminal states — requeue infrequently
        CanaryGatePhase::Promote | CanaryGatePhase::Hold | CanaryGatePhase::Rollback => {
            Ok(Action::requeue(Duration::from_secs(300)))
        }
    }
}

/// Handle reconciliation errors.
pub fn error_policy(
    _gate: Arc<CanaryGate>,
    _error: &ReconcileError,
    _ctx: Arc<OperatorState>,
) -> Action {
    tracing::error!("Reconciliation error: {:?}", _error);
    Action::requeue(Duration::from_secs(60))
}

/// Update the status subresource of a CanaryGate.
async fn update_status(
    client: &Client,
    namespace: &str,
    name: &str,
    status: CanaryGateStatus,
) -> Result<(), kube::Error> {
    let api: Api<CanaryGate> = Api::namespaced(client.clone(), namespace);

    let patch = serde_json::json!({
        "status": status
    });

    api.patch_status(
        name,
        &kube::api::PatchParams::apply("canary-gate-operator"),
        &kube::api::Patch::Merge(&patch),
    )
    .await?;

    Ok(())
}
