use kube::{api::LogParams, Api, Client};

use crate::ingestion::RawLogLine;

/// Fetch pod logs from Kubernetes pods matching a label selector.
///
/// Returns raw log lines suitable for the classification pipeline.
pub async fn fetch_pod_logs(
    client: Client,
    namespace: &str,
    label_selector: &str,
    since_seconds: Option<i64>,
) -> Result<Vec<RawLogLine>, kube::Error> {
    let pods: Api<k8s_openapi::api::core::v1::Pod> = Api::namespaced(client, namespace);
    let pod_list = pods
        .list(&kube::api::ListParams::default().labels(label_selector))
        .await?;

    let mut all_lines = Vec::new();
    let mut global_line_number = 1;

    for pod in pod_list {
        let pod_name = pod.metadata.name.unwrap_or_default();

        let mut log_params = LogParams::default();
        if let Some(since) = since_seconds {
            log_params.since_seconds = Some(since);
        }

        match pods.logs(&pod_name, &log_params).await {
            Ok(log_output) => {
                for line in log_output.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    let is_json = line.starts_with('{');
                    all_lines.push(RawLogLine {
                        content: line.to_string(),
                        line_number: global_line_number,
                        timestamp: None,
                        is_json,
                        source: Some(format!("pod:{}", pod_name)),
                    });
                    global_line_number += 1;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch logs from pod {}: {}", pod_name, e);
            }
        }
    }

    Ok(all_lines)
}
