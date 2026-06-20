use super::*;
use crate::chain::{
    Chain, ChainParams, HardwareClass, JobState, PendingReceiptReward, ReceiptRewardKind,
};
use crate::faucet::Faucet;
use crate::hash::hex;
use crate::jobs::{
    LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob, PrimitiveType, TensorOpReceipt,
};
use crate::profile::ChainProfile;
use crate::tensor::{DType, Tensor};
use crate::types::{address, hash_bytes};
use crate::verify::FreivaldsParams;
use std::net::SocketAddr;

use super::explorer::{hardware_class_label, primitive_label};
use super::http::{
    ParsedHttpRequest, read_http_request_from, split_path_and_auth_token, try_parse_http_request,
};
use super::websocket::{
    base64_encode, read_websocket_text_frame, websocket_accept_key, write_websocket_frame,
};

fn response_json(response: &RpcResponse) -> serde_json::Value {
    json_text(&response.body)
}

fn json_text(body: &str) -> serde_json::Value {
    serde_json::from_str(body).expect("RPC response body must be JSON")
}

fn json_hex_field<'a>(json: &'a serde_json::Value, field: &str) -> &'a str {
    let value = json[field]
        .as_str()
        .expect("RPC JSON field must be a string");
    assert_eq!(value.len(), 64);
    assert!(value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    value
}

fn html_tag_text<'a>(html: &'a str, tag: &str) -> &'a str {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    html.split_once(&open)
        .and_then(|(_, tail)| tail.split_once(&close))
        .map(|(value, _)| value)
        .unwrap_or_else(|| panic!("HTML document must contain <{tag}> text"))
}

fn html_definition_value<'a>(html: &'a str, label: &str) -> &'a str {
    let open = format!("<dt>{label}</dt><dd>");
    html.split_once(&open)
        .and_then(|(_, tail)| tail.split_once("</dd>"))
        .map(|(value, _)| value)
        .unwrap_or_else(|| panic!("HTML document must contain definition row {label:?}"))
}

fn assert_html_line(html: &str, expected: &str) {
    assert!(
        html.lines().map(str::trim).any(|line| line == expected),
        "HTML document must contain exact line {expected:?}"
    );
}

mod http;
mod routes;
mod tensors;
mod websocket;
