use super::{RpcNode, RpcRequest, RpcResponse, parse_address};
use crate::chain::{ChainCommand, ChainEngine, ChainEvent};
use crate::hash::hex;
use crate::txpool::parse_transaction_envelope;
use serde_json::json;

impl RpcNode {
    pub(super) fn submit_faucet_claim(&mut self, request: &RpcRequest) -> RpcResponse {
        let Some(address) = request.path.strip_prefix("/faucet/claim/") else {
            return self.not_found("route not found");
        };
        let Ok(address) = parse_address(address) else {
            return self.bad_request("invalid faucet address");
        };
        let Some(faucet) = self.faucet.as_mut() else {
            return self.not_found("faucet not configured");
        };
        match faucet.claim(address, self.chain.state().epoch()) {
            Ok(amount) => match self
                .chain
                .apply_command(ChainCommand::CreditReward { address, amount })
            {
                Ok(events) => {
                    let pending = events.iter().find_map(|event| match event {
                        ChainEvent::CreditRewardPending {
                            claim_id,
                            claimable_at_height,
                            ..
                        } => Some((*claim_id, *claimable_at_height)),
                        _ => None,
                    });
                    let balance = faucet.balance();
                    let mut body = json!({
                        "claimed": amount,
                        "address": hex(&address),
                        "faucet_balance": balance,
                    });
                    if let Some((claim_id, claimable_at_height)) = pending {
                        body["claim_id"] = json!(hex(&claim_id));
                        body["claimable_at_height"] = json!(claimable_at_height);
                    }
                    self.ok(body.to_string())
                }
                Err(error) => self.bad_request(&error.to_string()),
            },
            Err(error) => self.bad_request(&error.to_string()),
        }
    }

    pub(super) fn submit_transaction(&mut self, request: &RpcRequest) -> RpcResponse {
        let envelope = match parse_transaction_envelope(&request.body) {
            Ok(envelope) => envelope,
            Err(error) => return self.bad_request(&error.to_string()),
        };
        if envelope.transaction.is_reference_submission() {
            return if self.txpool.submit_envelope(&envelope) {
                self.accepted()
            } else {
                self.conflict("duplicate transaction")
            };
        }
        match self
            .chain
            .apply_transaction(envelope.sender, envelope.transaction.clone())
        {
            Ok(_) => {
                self.txpool.submit_envelope(&envelope);
                self.accepted()
            }
            Err(error) => self.bad_request(&error.to_string()),
        }
    }

    pub(super) fn submit_receipt_reference(&mut self, request: &RpcRequest) -> RpcResponse {
        let mut body = b"submit_tensor_receipt ".to_vec();
        body.extend_from_slice(&request.body);
        self.submit_reference_payload(&body)
    }

    pub(super) fn submit_attestation_reference(&mut self, request: &RpcRequest) -> RpcResponse {
        let mut body = b"submit_attestation ".to_vec();
        body.extend_from_slice(&request.body);
        self.submit_reference_payload(&body)
    }

    fn submit_reference_payload(&mut self, body: &[u8]) -> RpcResponse {
        let envelope = match parse_transaction_envelope(body) {
            Ok(envelope) => envelope,
            Err(error) => return self.bad_request(&error.to_string()),
        };
        if self.txpool.submit_envelope(&envelope) {
            self.accepted()
        } else {
            self.conflict("duplicate transaction")
        }
    }
}
