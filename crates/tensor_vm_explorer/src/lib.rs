use std::fmt::Write;

#[cfg(all(feature = "ratzilla-ui", target_arch = "wasm32"))]
pub mod ratzilla_ui;

pub const DEFAULT_EXPLORER_LISTEN: &str = "127.0.0.1:8080";
pub const DEFAULT_EXPLORER_WS_URL: &str = "ws://127.0.0.1:8545/explorer/ws";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerSummary {
    pub height: u64,
    pub epoch: u64,
    pub block_count: usize,
    pub miner_count: usize,
    pub validator_count: usize,
    pub job_count: usize,
    pub model_count: usize,
    pub attestation_count: usize,
    pub receipt_count: usize,
    pub settled_receipt_count: usize,
    pub data_unavailable_receipt_count: usize,
    pub data_unavailability_slash_count: usize,
    pub data_unavailability_slashed_amount_total: u64,
    pub validator_audit_assignment_count: usize,
    pub validator_audit_result_count: usize,
    pub validator_audit_slash_count: usize,
    pub validator_audit_slashed_amount_total: u64,
    pub finalized_block_count: usize,
    pub treasury_balance: u64,
    pub pending_receipt_reward_count: usize,
    pub pending_proposer_reward_count: usize,
    pub pending_challenge_reward_count: usize,
    pub pending_credit_reward_count: usize,
    pub total_reward_balance: u64,
}

impl ExplorerSummary {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"height\":{},\"epoch\":{},\"block_count\":{},\"miner_count\":{},\"validator_count\":{},\"job_count\":{},\"model_count\":{},\"attestation_count\":{},\"receipt_count\":{},\"settled_receipt_count\":{},\"data_unavailable_receipt_count\":{},\"data_unavailability_slash_count\":{},\"data_unavailability_slashed_amount_total\":{},\"validator_audit_assignment_count\":{},\"validator_audit_result_count\":{},\"validator_audit_slash_count\":{},\"validator_audit_slashed_amount_total\":{},\"finalized_block_count\":{},\"treasury_balance\":{},\"pending_receipt_reward_count\":{},\"pending_proposer_reward_count\":{},\"pending_challenge_reward_count\":{},\"pending_credit_reward_count\":{},\"total_reward_balance\":{}}}",
            self.height,
            self.epoch,
            self.block_count,
            self.miner_count,
            self.validator_count,
            self.job_count,
            self.model_count,
            self.attestation_count,
            self.receipt_count,
            self.settled_receipt_count,
            self.data_unavailable_receipt_count,
            self.data_unavailability_slash_count,
            self.data_unavailability_slashed_amount_total,
            self.validator_audit_assignment_count,
            self.validator_audit_result_count,
            self.validator_audit_slash_count,
            self.validator_audit_slashed_amount_total,
            self.finalized_block_count,
            self.treasury_balance,
            self.pending_receipt_reward_count,
            self.pending_proposer_reward_count,
            self.pending_challenge_reward_count,
            self.pending_credit_reward_count,
            self.total_reward_balance
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerBlock {
    pub height: u64,
    pub epoch: u64,
    pub hash: String,
    pub proposer: String,
    pub state_root: String,
    pub timestamp: u64,
}

impl ExplorerBlock {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"height\":{},\"epoch\":{},\"hash\":\"{}\",\"proposer\":\"{}\",\"state_root\":\"{}\",\"timestamp\":{}}}",
            self.height,
            self.epoch,
            escape_json(&self.hash),
            escape_json(&self.proposer),
            escape_json(&self.state_root),
            self.timestamp
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerAccount {
    pub address: String,
    pub is_miner: bool,
    pub is_validator: bool,
    pub balance: u64,
    pub reward_balance: u64,
    pub stake: u64,
    pub reputation: i64,
    pub settled_tensor_work: u64,
    pub pending_tensor_work: u64,
}

impl ExplorerAccount {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"address\":\"{}\",\"is_miner\":{},\"is_validator\":{},\"balance\":{},\"reward_balance\":{},\"stake\":{},\"reputation\":{},\"settled_tensor_work\":{},\"pending_tensor_work\":{}}}",
            escape_json(&self.address),
            self.is_miner,
            self.is_validator,
            self.balance,
            self.reward_balance,
            self.stake,
            self.reputation,
            self.settled_tensor_work,
            self.pending_tensor_work
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerMiner {
    pub address: String,
    pub operator_id: String,
    pub stake: u64,
    pub reputation: i64,
    pub settled_tensor_work: u64,
    pub pending_tensor_work: u64,
    pub hardware_class: String,
    pub gpu_utilization_bps: u64,
    pub reward_balance: u64,
}

impl ExplorerMiner {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"address\":\"{}\",\"operator_id\":\"{}\",\"stake\":{},\"reputation\":{},\"settled_tensor_work\":{},\"pending_tensor_work\":{},\"hardware_class\":\"{}\",\"gpu_utilization_bps\":{},\"reward_balance\":{}}}",
            escape_json(&self.address),
            escape_json(&self.operator_id),
            self.stake,
            self.reputation,
            self.settled_tensor_work,
            self.pending_tensor_work,
            escape_json(&self.hardware_class),
            self.gpu_utilization_bps,
            self.reward_balance
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerValidator {
    pub address: String,
    pub stake: u64,
    pub reputation: i64,
    pub valid_attestations: u64,
    pub missed_assignments: u64,
    pub reward_balance: u64,
}

impl ExplorerValidator {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"address\":\"{}\",\"stake\":{},\"reputation\":{},\"valid_attestations\":{},\"missed_assignments\":{},\"reward_balance\":{}}}",
            escape_json(&self.address),
            self.stake,
            self.reputation,
            self.valid_attestations,
            self.missed_assignments,
            self.reward_balance
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerReceipt {
    pub receipt_id: String,
    pub job_id: String,
    pub primitive_type: String,
    pub miner: String,
    pub tensor_work_units: u64,
    pub attestation_count: usize,
    pub validator_attestations: Vec<String>,
    pub settled: bool,
}

impl ExplorerReceipt {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"receipt_id\":\"{}\",\"job_id\":\"{}\",\"primitive_type\":\"{}\",\"miner\":\"{}\",\"tensor_work_units\":{},\"attestation_count\":{},\"validator_attestations\":{},\"settled\":{}}}",
            escape_json(&self.receipt_id),
            escape_json(&self.job_id),
            escape_json(&self.primitive_type),
            escape_json(&self.miner),
            self.tensor_work_units,
            self.attestation_count,
            json_string_array(&self.validator_attestations),
            self.settled
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerJob {
    pub job_id: String,
    pub primitive_type: String,
    pub deadline_block: u64,
    pub detail: String,
}

impl ExplorerJob {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"job_id\":\"{}\",\"primitive_type\":\"{}\",\"deadline_block\":{},\"detail\":\"{}\"}}",
            escape_json(&self.job_id),
            escape_json(&self.primitive_type),
            self.deadline_block,
            escape_json(&self.detail)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerOverview {
    pub summary: ExplorerSummary,
    pub blocks: Vec<ExplorerBlock>,
    pub miners: Vec<ExplorerMiner>,
    pub validators: Vec<ExplorerValidator>,
    pub receipts: Vec<ExplorerReceipt>,
    pub jobs: Vec<ExplorerJob>,
}

impl ExplorerOverview {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"type\":\"overview\",\"summary\":{},\"blocks\":{},\"miners\":{},\"validators\":{},\"receipts\":{},\"jobs\":{}}}",
            self.summary.to_json(),
            json_array(&self.blocks, ExplorerBlock::to_json),
            json_array(&self.miners, ExplorerMiner::to_json),
            json_array(&self.validators, ExplorerValidator::to_json),
            json_array(&self.receipts, ExplorerReceipt::to_json),
            json_array(&self.jobs, ExplorerJob::to_json)
        )
    }
}

pub fn blocks_json(blocks: &[ExplorerBlock]) -> String {
    format!(
        "{{\"type\":\"blocks\",\"blocks\":{}}}",
        json_array(blocks, ExplorerBlock::to_json)
    )
}

pub fn miners_json(miners: &[ExplorerMiner]) -> String {
    format!(
        "{{\"type\":\"miners\",\"miners\":{}}}",
        json_array(miners, ExplorerMiner::to_json)
    )
}

pub fn validators_json(validators: &[ExplorerValidator]) -> String {
    format!(
        "{{\"type\":\"validators\",\"validators\":{}}}",
        json_array(validators, ExplorerValidator::to_json)
    )
}

pub fn receipts_json(receipts: &[ExplorerReceipt]) -> String {
    format!(
        "{{\"type\":\"receipts\",\"receipts\":{}}}",
        json_array(receipts, ExplorerReceipt::to_json)
    )
}

pub fn jobs_json(jobs: &[ExplorerJob]) -> String {
    format!(
        "{{\"type\":\"jobs\",\"jobs\":{}}}",
        json_array(jobs, ExplorerJob::to_json)
    )
}

pub fn account_json(account: &ExplorerAccount) -> String {
    format!("{{\"type\":\"account\",\"account\":{}}}", account.to_json())
}

pub fn explorer_health_json(ws_url: &str) -> String {
    format!(
        "{{\"service\":\"tensorvm-explorer\",\"tensorvm_explorer_ready\":true,\"websocket_url\":\"{}\"}}",
        escape_json(ws_url)
    )
}

pub fn explorer_shell_html(ws_url: &str) -> String {
    let html_ws = escape_html(ws_url);
    let js_ws = escape_js_string(ws_url);
    let template = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>TensorVM Explorer</title>
<style>
* { box-sizing: border-box; }
:root { color-scheme: dark; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace; background: #05080d; color: #dbe7f0; }
body { margin: 0; min-height: 100vh; background: #05080d; }
button, input { font: inherit; letter-spacing: 0; }
code { color: #7dd3fc; }
[hidden] { display: none !important; }
.terminal { min-height: calc(100vh - 24px); margin: 12px; display: flex; flex-direction: column; border: 1px solid #293847; background: #070b10; box-shadow: inset 0 0 0 1px #0d141c; }
.topbar { display: grid; grid-template-columns: minmax(190px, 1fr) auto minmax(220px, 42vw); gap: 12px; align-items: center; padding: 10px 12px; border-bottom: 1px solid #293847; background: #0c121a; }
.brand { color: #f8fafc; font-weight: 800; }
.status { color: #9be9a8; text-align: center; font-size: 13px; }
.ws { color: #98a8b8; overflow: hidden; text-align: right; text-overflow: ellipsis; white-space: nowrap; }
.tabs { display: flex; gap: 1px; padding: 0 12px; border-bottom: 1px solid #293847; background: #0a1017; }
.tabs button { min-height: 38px; border: 0; border-left: 1px solid transparent; border-right: 1px solid transparent; background: transparent; color: #a6b6c6; padding: 0 12px; cursor: pointer; }
.tabs button.active { background: #9be9a8; color: #071015; font-weight: 800; }
.layout { flex: 1; display: grid; grid-template-columns: minmax(240px, 300px) minmax(0, 1fr); min-height: 0; }
.sidebar { border-right: 1px solid #293847; padding: 12px; overflow: auto; }
.screen { min-width: 0; overflow: auto; padding: 12px; }
.metrics { display: grid; gap: 8px; }
.metric { display: flex; justify-content: space-between; gap: 16px; border: 1px solid #263747; border-radius: 4px; background: #0b1118; padding: 8px 10px; }
.metric span, .panel-title, th { color: #91a4b7; }
.metric strong { color: #f8fafc; font-variant-numeric: tabular-nums; }
.panel-title { margin: 16px 0 8px; font-size: 12px; text-transform: uppercase; }
.terminal-lines { margin: 0; border: 1px solid #263747; border-radius: 4px; background: #05080d; color: #cbd5e1; min-height: 92px; padding: 10px; white-space: pre-wrap; }
.window { border: 1px solid #2d4052; border-radius: 4px; background: #070b10; margin-bottom: 12px; }
.window-title { display: flex; justify-content: space-between; gap: 12px; border-bottom: 1px solid #243241; padding: 8px 10px; color: #f8fafc; background: #0b1118; }
.window-title span:last-child { color: #91a4b7; }
.table-wrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th, td { padding: 7px 8px; border-bottom: 1px solid #1d2a36; text-align: left; white-space: nowrap; }
td { color: #dbe7f0; }
td.mono { color: #7dd3fc; }
.empty { color: #91a4b7; }
.command { display: flex; gap: 10px; align-items: center; border-top: 1px solid #293847; background: #0c121a; padding: 10px 12px; }
.prompt { color: #9be9a8; font-weight: 800; }
.command input { flex: 1; min-width: 0; border: 1px solid #33485d; border-radius: 4px; background: #05080d; color: #f8fafc; padding: 8px 10px; }
.command button { border: 1px solid #9be9a8; border-radius: 4px; background: #9be9a8; color: #071015; cursor: pointer; font-weight: 800; padding: 8px 12px; }
@media (max-width: 900px) {
  .terminal { min-height: 100vh; margin: 0; border-left: 0; border-right: 0; }
  .topbar { grid-template-columns: 1fr; }
  .status, .ws { text-align: left; }
  .layout { grid-template-columns: 1fr; }
  .sidebar { border-right: 0; border-bottom: 1px solid #293847; }
  .tabs { overflow-x: auto; }
  .command { align-items: stretch; flex-direction: column; }
  .command button { width: 100%; }
}
</style>
</head>
<body>
<div class="terminal" data-ui="ratzilla-tui">
  <header class="topbar">
    <div class="brand">TensorVM Explorer</div>
    <div class="status" id="status">connecting</div>
    <div class="ws">WS <code id="ws-url">__HTML_WS__</code></div>
  </header>
  <nav class="tabs" aria-label="Explorer views">
    <button class="active" data-view="overview">Overview</button>
    <button data-view="blocks">Blocks</button>
    <button data-view="operators">Operators</button>
    <button data-view="work">Work</button>
  </nav>
  <div class="layout">
    <aside class="sidebar">
      <div class="metrics" id="metrics"></div>
      <div class="panel-title">Operator Status</div>
      <pre class="terminal-lines" id="operator-lines">waiting for operator set</pre>
      <div class="panel-title">Account</div>
      <pre class="terminal-lines" id="account-output">address lookup idle</pre>
    </aside>
    <main class="screen">
      <section class="window" data-panel="blocks">
        <div class="window-title"><span>Latest Blocks</span><span id="block-count">0 rows</span></div>
        <div class="table-wrap"><table id="blocks"></table></div>
      </section>
      <section class="window" data-panel="operators">
        <div class="window-title"><span>Miners</span><span id="miner-count">0 rows</span></div>
        <div class="table-wrap"><table id="miners"></table></div>
      </section>
      <section class="window" data-panel="operators">
        <div class="window-title"><span>Validators</span><span id="validator-count">0 rows</span></div>
        <div class="table-wrap"><table id="validators"></table></div>
      </section>
      <section class="window" data-panel="work">
        <div class="window-title"><span>Receipts</span><span id="receipt-count">0 rows</span></div>
        <div class="table-wrap"><table id="receipts"></table></div>
      </section>
      <section class="window" data-panel="work">
        <div class="window-title"><span>Jobs</span><span id="job-count">0 rows</span></div>
        <div class="table-wrap"><table id="jobs"></table></div>
      </section>
    </main>
  </div>
  <footer class="command">
    <span class="prompt">tensorvm&gt;</span>
    <input id="account-input" placeholder="lookup account address">
    <button id="account-button">Lookup</button>
  </footer>
</div>
<script>
const WS_URL = "__JS_WS__";
const statusEl = document.getElementById("status");
const tabs = Array.from(document.querySelectorAll("[data-view]"));
const panels = Array.from(document.querySelectorAll("[data-panel]"));
function escapeText(value) {
  return String(value ?? "").replace(/[&<>"']/g, char => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;"
  }[char]));
}
function setStatus(text) { statusEl.textContent = text; }
function shortHex(value) {
  const text = String(value ?? "");
  return text.length > 24 ? text.slice(0, 12) + "..." + text.slice(-8) : text;
}
function cell(value, code=false) {
  const text = code ? shortHex(value) : value;
  return `<td${code ? ' class="mono"' : ""}>${escapeText(text)}</td>`;
}
function renderTable(id, countId, headers, rows) {
  const head = `<thead><tr>${headers.map(h => `<th>${escapeText(h)}</th>`).join("")}</tr></thead>`;
  const bodyRows = rows.length ? rows.join("") : `<tr><td class="empty" colspan="${headers.length}">waiting for chain data</td></tr>`;
  document.getElementById(id).innerHTML = `${head}<tbody>${bodyRows}</tbody>`;
  document.getElementById(countId).textContent = `${rows.length} row${rows.length === 1 ? "" : "s"}`;
}
function metric(label, value) {
  return `<div class="metric"><span>${escapeText(label)}</span><strong>${escapeText(value)}</strong></div>`;
}
function setView(view) {
  tabs.forEach(tab => tab.classList.toggle("active", tab.dataset.view === view));
  panels.forEach(panel => {
    panel.hidden = view !== "overview" && panel.dataset.panel !== view;
  });
}
tabs.forEach(tab => tab.addEventListener("click", () => setView(tab.dataset.view)));
function renderOverview(data) {
  if (!data || !data.summary) {
    setStatus("invalid explorer payload");
    return;
  }
  const s = data.summary;
  document.getElementById("metrics").innerHTML = [
    metric("height", s.height), metric("epoch", s.epoch), metric("blocks", s.block_count),
    metric("miners", s.miner_count), metric("validators", s.validator_count),
    metric("receipts", `${s.settled_receipt_count}/${s.receipt_count}`),
    metric("jobs", s.job_count), metric("models", s.model_count),
    metric("attest", s.attestation_count), metric("rewards", s.total_reward_balance)
  ].join("");
  document.getElementById("operator-lines").textContent = [
    `miners      ${s.miner_count}`,
    `validators  ${s.validator_count}`,
    `finalized   ${s.finalized_block_count}/${s.block_count}`,
    `treasury    ${s.treasury_balance}`
  ].join("\n");
  renderTable("blocks", "block-count", ["Height", "Epoch", "Hash", "Proposer", "State Root", "Time"], (data.blocks || []).map(b =>
    `<tr>${cell(b.height)}${cell(b.epoch)}${cell(b.hash, true)}${cell(b.proposer, true)}${cell(b.state_root, true)}${cell(b.timestamp)}</tr>`));
  renderTable("miners", "miner-count", ["Address", "Stake", "Settled Work", "Pending", "Hardware", "Rewards"], (data.miners || []).map(m =>
    `<tr>${cell(m.address, true)}${cell(m.stake)}${cell(m.settled_tensor_work)}${cell(m.pending_tensor_work)}${cell(m.hardware_class)}${cell(m.reward_balance)}</tr>`));
  renderTable("validators", "validator-count", ["Address", "Stake", "Valid", "Missed", "Rewards"], (data.validators || []).map(v =>
    `<tr>${cell(v.address, true)}${cell(v.stake)}${cell(v.valid_attestations)}${cell(v.missed_assignments)}${cell(v.reward_balance)}</tr>`));
  renderTable("receipts", "receipt-count", ["Receipt", "Job", "Type", "Miner", "Work", "Attest", "Settled"], (data.receipts || []).map(r =>
    `<tr>${cell(r.receipt_id, true)}${cell(r.job_id, true)}${cell(r.primitive_type)}${cell(r.miner, true)}${cell(r.tensor_work_units)}${cell(r.attestation_count)}${cell(r.settled)}</tr>`));
  renderTable("jobs", "job-count", ["Job", "Type", "Deadline", "Detail"], (data.jobs || []).map(j =>
    `<tr>${cell(j.job_id, true)}${cell(j.primitive_type)}${cell(j.deadline_block)}${cell(j.detail)}</tr>`));
}
function wsRequest(payload, onData) {
  const ws = new WebSocket(WS_URL);
  ws.onopen = () => ws.send(JSON.stringify(payload));
  ws.onmessage = event => {
    try {
      onData(JSON.parse(event.data));
    } catch (error) {
      setStatus("parse error");
    }
    ws.close();
  };
  ws.onerror = () => setStatus("websocket error");
  ws.onclose = () => {};
}
function refresh() {
  wsRequest({type: "overview", block_limit: 14, receipt_limit: 24, job_limit: 24}, data => {
    renderOverview(data);
    setStatus("live " + new Date().toLocaleTimeString());
  });
}
function lookupAccount() {
  const address = document.getElementById("account-input").value.trim();
  if (!address) return;
  wsRequest({type: "account", address}, data => {
    document.getElementById("account-output").textContent = JSON.stringify(data.account, null, 2);
  });
}
document.getElementById("account-button").onclick = lookupAccount;
document.getElementById("account-input").addEventListener("keydown", event => {
  if (event.key === "Enter") lookupAccount();
});
setView("overview");
refresh();
setInterval(refresh, 3000);
</script>
</body>
</html>"#;
    template
        .replace("__HTML_WS__", &html_ws)
        .replace("__JS_WS__", &js_ws)
}

fn json_array<T>(items: &[T], render: fn(&T) -> String) -> String {
    let parts = items.iter().map(render).collect::<Vec<_>>();
    format!("[{}]", parts.join(","))
}

fn json_string_array(items: &[String]) -> String {
    let parts = items
        .iter()
        .map(|item| format!("\"{}\"", escape_json(item)))
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(","))
}

pub fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn escape_js_string(value: &str) -> String {
    escape_json(value)
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_json_and_shell_include_live_websocket_contract() {
        let summary = ExplorerSummary {
            height: 2,
            epoch: 0,
            block_count: 2,
            miner_count: 10,
            validator_count: 5,
            job_count: 2,
            model_count: 1,
            attestation_count: 30,
            receipt_count: 10,
            settled_receipt_count: 10,
            data_unavailable_receipt_count: 1,
            data_unavailability_slash_count: 1,
            data_unavailability_slashed_amount_total: 10,
            validator_audit_assignment_count: 3,
            validator_audit_result_count: 2,
            validator_audit_slash_count: 1,
            validator_audit_slashed_amount_total: 55,
            finalized_block_count: 2,
            treasury_balance: 3,
            pending_receipt_reward_count: 7,
            pending_proposer_reward_count: 4,
            pending_challenge_reward_count: 2,
            pending_credit_reward_count: 1,
            total_reward_balance: 100,
        };
        assert!(summary.to_json().contains("\"settled_receipt_count\":10"));
        assert!(
            summary
                .to_json()
                .contains("\"data_unavailable_receipt_count\":1")
        );
        assert!(
            summary
                .to_json()
                .contains("\"data_unavailability_slash_count\":1")
        );
        assert!(
            summary
                .to_json()
                .contains("\"data_unavailability_slashed_amount_total\":10")
        );
        assert!(
            summary
                .to_json()
                .contains("\"validator_audit_assignment_count\":3")
        );
        assert!(
            summary
                .to_json()
                .contains("\"validator_audit_slash_count\":1")
        );
        assert!(
            summary
                .to_json()
                .contains("\"validator_audit_slashed_amount_total\":55")
        );
        assert!(
            summary
                .to_json()
                .contains("\"pending_receipt_reward_count\":7")
        );
        assert!(
            summary
                .to_json()
                .contains("\"pending_proposer_reward_count\":4")
        );
        assert!(
            summary
                .to_json()
                .contains("\"pending_challenge_reward_count\":2")
        );
        assert!(
            summary
                .to_json()
                .contains("\"pending_credit_reward_count\":1")
        );
        assert!(summary.to_json().contains("\"model_count\":1"));
        assert!(summary.to_json().contains("\"attestation_count\":30"));
        let receipts = receipts_json(&[ExplorerReceipt {
            receipt_id: "receipt".to_owned(),
            job_id: "job".to_owned(),
            primitive_type: "tensor_op".to_owned(),
            miner: "miner".to_owned(),
            tensor_work_units: 5,
            attestation_count: 2,
            validator_attestations: vec!["validator-a".to_owned(), "validator-b".to_owned()],
            settled: true,
        }]);
        assert!(receipts.contains("\"attestation_count\":2"));
        assert!(receipts.contains("\"validator_attestations\":[\"validator-a\",\"validator-b\"]"));

        let html = explorer_shell_html("ws://127.0.0.1:8545/explorer/ws?token=secret");
        assert!(html.contains("TensorVM Explorer"));
        assert!(html.contains("data-ui=\"ratzilla-tui\""));
        assert!(html.contains("new WebSocket"));
        assert!(html.contains("\"overview\""));
        assert!(html.contains("ws://127.0.0.1:8545/explorer/ws?token=secret"));

        let health = explorer_health_json("ws://node/explorer/ws?token=\"local\"");
        assert!(health.contains("\\\"local\\\""));
        assert_eq!(escape_json("\"\\\n\r\t\u{7}"), "\\\"\\\\\\n\\r\\t\\u0007");
        let escaped_html = explorer_shell_html("ws://node/<explorer>&\"ws\"");
        assert!(escaped_html.contains("ws://node/&lt;explorer&gt;&amp;&quot;ws&quot;"));
    }
}
