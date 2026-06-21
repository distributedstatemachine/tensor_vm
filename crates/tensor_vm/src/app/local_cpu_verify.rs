use super::KeyValueReportWriter;
use crate::NodeStore;

pub fn verify_local_cpu_store(data_dir: &str, json: bool) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let status = store
        .status()
        .map_err(|error| format!("failed to inspect node store {data_dir}: {error}"))?;
    let latest_block_height = chain
        .blocks()
        .last()
        .map(|block| block.height)
        .unwrap_or_default();
    let finalized_block_count = chain
        .blocks()
        .iter()
        .filter(|block| chain.is_block_finalized(&block.hash()))
        .count();
    let block_count = chain.blocks().len();
    let ready = block_count > 0
        && chain.state().height() == latest_block_height.saturating_add(1)
        && finalized_block_count <= block_count;
    let report = LocalCpuVerifyReport {
        command: "local_cpu_verify",
        data_dir,
        structured_verifier_ready: true,
        ready,
        height: chain.state().height(),
        latest_block_height,
        block_count,
        compact_block_count: status.block_count,
        finalized_block_count,
        node_store_ready: true,
    };
    if json {
        serde_json::to_string(&report)
            .map_err(|error| format!("failed to serialize local CPU verify report: {error}"))
    } else {
        Ok(report.to_key_value_report())
    }
}

#[derive(serde::Serialize)]
struct LocalCpuVerifyReport<'a> {
    command: &'static str,
    data_dir: &'a str,
    structured_verifier_ready: bool,
    ready: bool,
    height: u64,
    latest_block_height: u64,
    block_count: usize,
    compact_block_count: usize,
    finalized_block_count: usize,
    node_store_ready: bool,
}

impl LocalCpuVerifyReport<'_> {
    fn to_key_value_report(&self) -> String {
        let mut report = KeyValueReportWriter::new();
        report.field("command", self.command);
        report.field("data_dir", self.data_dir);
        report.field("structured_verifier_ready", self.structured_verifier_ready);
        report.field("ready", self.ready);
        report.field("height", self.height);
        report.field("latest_block_height", self.latest_block_height);
        report.field("block_count", self.block_count);
        report.field("compact_block_count", self.compact_block_count);
        report.field("finalized_block_count", self.finalized_block_count);
        report.field("node_store_ready", self.node_store_ready);
        report.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::KeyValueReport;

    #[test]
    fn local_cpu_verify_key_value_report_is_parseable() {
        let report = LocalCpuVerifyReport {
            command: "local_cpu_verify",
            data_dir: "/var/lib/tensorvm",
            structured_verifier_ready: true,
            ready: true,
            height: 2,
            latest_block_height: 1,
            block_count: 2,
            compact_block_count: 2,
            finalized_block_count: 2,
            node_store_ready: true,
        }
        .to_key_value_report();

        let fields = KeyValueReport::parse_strict(&report).expect("verify report must parse");
        assert_eq!(fields.value("command"), Some("local_cpu_verify"));
        assert_eq!(fields.value("data_dir"), Some("/var/lib/tensorvm"));
        assert_eq!(fields.value("ready"), Some("true"));
        assert_eq!(fields.value("finalized_block_count"), Some("2"));
    }
}
