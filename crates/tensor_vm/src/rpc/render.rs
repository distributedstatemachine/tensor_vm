use crate::chain::JobState;
use crate::faucet::Faucet;
use crate::hash::hex;
use crate::telemetry::TelemetrySnapshot;
use serde_json::{Value, json};

pub(super) fn telemetry_dashboard_html(snapshot: &TelemetrySnapshot) -> String {
    html_document(
        "TensorVM Telemetry",
        format!(
            "<section><h1>Telemetry Dashboard</h1><dl>{}</dl></section>",
            metric_rows(&[
                (
                    "Block Finality Rate",
                    format!("{:.6}", snapshot.block_finality_rate),
                ),
                (
                    "Average Block Time",
                    format!("{:.6}", snapshot.average_block_time),
                ),
                (
                    "Data Availability Rate",
                    format!("{:.6}", snapshot.data_availability_rate),
                ),
                (
                    "Invalid Receipts Submitted",
                    snapshot.invalid_receipts_submitted.to_string(),
                ),
                (
                    "Validator Disagreement Rate",
                    format!("{:.6}", snapshot.validator_disagreement_rate),
                ),
                ("Total TensorWork", snapshot.total_tensor_work.to_string()),
                (
                    "Max Miner Work Share",
                    format!("{:.6}", snapshot.max_miner_work_share),
                ),
                (
                    "GPU Utilization",
                    format!("{:.6}", snapshot.estimated_gpu_utilization),
                ),
                (
                    "Hardware Classes",
                    snapshot.hardware_class_participation.to_string(),
                ),
            ]),
        ),
    )
}

pub(super) fn faucet_page_html(faucet: Option<&Faucet>) -> String {
    let rows = match faucet {
        Some(faucet) => metric_rows(&[
            ("Balance", faucet.balance().to_string()),
            ("Drip Amount", faucet.drip_amount().to_string()),
        ]),
        None => metric_rows(&[("Status", "Not configured".to_owned())]),
    };
    html_document(
        "TensorVM Faucet",
        format!("<section><h1>Faucet</h1><dl>{rows}</dl></section>"),
    )
}

pub(super) fn job_value(job: &JobState) -> Value {
    match job {
        JobState::TensorOp(job) => json!({
            "job_id": hex(&job.job_id),
            "primitive_type": "tensor_op",
            "epoch": job.epoch,
            "m": job.m,
            "k": job.k,
            "n": job.n,
            "deadline_block": job.deadline_block,
            "reward_weight": job.reward_weight,
        }),
        JobState::LinearTrainingStep(job) => json!({
            "job_id": hex(&job.job_id),
            "primitive_type": "linear_training_step",
            "model_id": hex(&job.model_id),
            "step": job.step,
            "input_shape": job.input_shape,
            "weight_shape": job.weight_shape,
            "target_shape": job.target_shape,
            "deadline_block": job.deadline_block,
            "reward_weight": job.reward_weight,
        }),
        JobState::GraphExecution(job) => json!({
            "job_id": hex(&job.job_id),
            "primitive_type": "graph_execution",
            "epoch": job.epoch,
            "graph_id": hex(&job.graph_id),
            "input_roots": job.input_roots.iter().map(|(name, root)| {
                (name.clone(), json!(hex(root)))
            }).collect::<serde_json::Map<_, _>>(),
            "field_params": job.field_params,
            "deadline_block": job.deadline_block,
            "reward_weight": job.reward_weight,
            "declared_tensor_work_units": job.declared_tensor_work_units,
        }),
    }
}

fn metric_rows(rows: &[(&str, String)]) -> String {
    rows.iter()
        .map(|(name, value)| format!("<dt>{name}</dt><dd>{value}</dd>"))
        .collect::<Vec<_>>()
        .join("")
}

fn html_document(title: &str, body: String) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>body{{font-family:system-ui,sans-serif;margin:0;background:#f7f7f4;color:#151515}}main{{max-width:960px;margin:0 auto;padding:32px}}section{{border-top:1px solid #d8d8d0;padding:20px 0}}dl{{display:grid;grid-template-columns:minmax(160px,260px)1fr;gap:8px 16px}}dt{{font-weight:700}}dd{{margin:0}}code{{font-size:12px;word-break:break-all}}</style></head><body><main>{body}</main></body></html>"
    )
}
