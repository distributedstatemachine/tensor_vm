use super::public_evidence_crypto::{
    PublicNetworkRuntimeObservationDetails, public_network_runtime_observation_root,
    public_network_runtime_observation_signature,
};
use super::public_urls::public_network_runtime_multiaddr_is_external;
use crate::types::{Hash, Signature};
use libp2p::{Multiaddr, PeerId};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublicNetworkRuntimeEvidence {
    pub libp2p_runtime_used: bool,
    pub peer_discovery_observed: bool,
    pub gossip_propagation_observed: bool,
    pub request_response_observed: bool,
    pub dos_controls_enabled: bool,
}

impl PublicNetworkRuntimeEvidence {
    pub fn has_production_libp2p_runtime(&self) -> bool {
        self.libp2p_runtime_used
            && self.peer_discovery_observed
            && self.gossip_propagation_observed
            && self.request_response_observed
            && self.dos_controls_enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicNetworkRuntimeObservation {
    pub operator_id: Hash,
    pub peer_id: String,
    pub listen_address: String,
    pub observed_at_unix_seconds: u64,
    pub gossip_topic_count: u64,
    pub request_response_protocol_count: u64,
    pub bootstrap_peer_count: u64,
    pub max_transmit_bytes: u64,
    pub request_timeout_seconds: u64,
    pub max_concurrent_streams: u64,
    pub idle_connection_timeout_seconds: u64,
    pub record_root: Hash,
    pub observation_signature: Signature,
}

impl PublicNetworkRuntimeObservation {
    pub(super) fn new(details: PublicNetworkRuntimeObservationDetails) -> Self {
        let record_root = public_network_runtime_observation_root(&details);
        let observation_signature =
            public_network_runtime_observation_signature(&details.operator_id, &record_root);
        Self {
            operator_id: details.operator_id,
            peer_id: details.peer_id,
            listen_address: details.listen_address,
            observed_at_unix_seconds: details.observed_at_unix_seconds,
            gossip_topic_count: details.gossip_topic_count,
            request_response_protocol_count: details.request_response_protocol_count,
            bootstrap_peer_count: details.bootstrap_peer_count,
            max_transmit_bytes: details.max_transmit_bytes,
            request_timeout_seconds: details.request_timeout_seconds,
            max_concurrent_streams: details.max_concurrent_streams,
            idle_connection_timeout_seconds: details.idle_connection_timeout_seconds,
            record_root,
            observation_signature,
        }
    }

    fn details(&self) -> PublicNetworkRuntimeObservationDetails {
        PublicNetworkRuntimeObservationDetails {
            operator_id: self.operator_id,
            peer_id: self.peer_id.clone(),
            listen_address: self.listen_address.clone(),
            observed_at_unix_seconds: self.observed_at_unix_seconds,
            gossip_topic_count: self.gossip_topic_count,
            request_response_protocol_count: self.request_response_protocol_count,
            bootstrap_peer_count: self.bootstrap_peer_count,
            max_transmit_bytes: self.max_transmit_bytes,
            request_timeout_seconds: self.request_timeout_seconds,
            max_concurrent_streams: self.max_concurrent_streams,
            idle_connection_timeout_seconds: self.idle_connection_timeout_seconds,
        }
    }

    #[cfg(test)]
    pub(super) fn with_listen_address_for_testing(&self, listen_address: String) -> Self {
        let mut details = self.details();
        details.listen_address = listen_address;
        Self::new(details)
    }

    #[cfg(test)]
    pub(super) fn with_peer_id_for_testing(&self, peer_id: String) -> Self {
        let mut details = self.details();
        details.peer_id = peer_id;
        Self::new(details)
    }

    pub(super) fn has_public_network_observation_proof(&self) -> bool {
        let details = self.details();
        self.operator_id != [0; 32]
            && self.record_root != [0; 32]
            && self.observed_at_unix_seconds > 0
            && self.gossip_topic_count > 0
            && self.request_response_protocol_count > 0
            && self.bootstrap_peer_count > 0
            && self.max_transmit_bytes > 0
            && self.request_timeout_seconds > 0
            && self.max_concurrent_streams > 0
            && self.idle_connection_timeout_seconds > 0
            && self.peer_id.parse::<PeerId>().is_ok()
            && self
                .listen_address
                .parse::<Multiaddr>()
                .is_ok_and(|address| public_network_runtime_multiaddr_is_external(&address))
            && self.record_root == public_network_runtime_observation_root(&details)
            && self.observation_signature
                == public_network_runtime_observation_signature(
                    &self.operator_id,
                    &self.record_root,
                )
    }
}
