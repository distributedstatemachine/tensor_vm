use crate::error::{Result as TvmResult, TvmError};
use crate::types::hash_bytes;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use std::fs;
use std::path::{Path, PathBuf};

const PEER_BOOK_MAGIC: &[u8] = b"TENSORVM_LIBP2P_PEER_BOOK_V1\n";
const PEER_BOOK_DIGEST_LEN: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerRecord {
    pub peer_id: String,
    pub address: String,
}

impl PeerRecord {
    pub fn new(peer_id: PeerId, address: Multiaddr) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            address: address.to_string(),
        }
    }

    pub fn from_strings(peer_id: &str, address: &str) -> TvmResult<Self> {
        let record = Self {
            peer_id: peer_id.to_owned(),
            address: address.to_owned(),
        };
        record.bootstrap_multiaddr()?;
        Ok(record)
    }

    pub fn peer_id(&self) -> TvmResult<PeerId> {
        self.peer_id
            .parse()
            .map_err(|_| TvmError::Storage("invalid peer id"))
    }

    pub fn multiaddr(&self) -> TvmResult<Multiaddr> {
        parse_multiaddr(&self.address)
    }

    pub fn bootstrap_multiaddr(&self) -> TvmResult<Multiaddr> {
        let peer_id = self.peer_id()?;
        let mut address = self.multiaddr()?;
        if !multiaddr_has_nonzero_tcp(&address) {
            return Err(TvmError::Storage("peer book address missing tcp port"));
        }
        if let Some((embedded_peer_id, _)) = bootstrap_peer_address(&address) {
            if embedded_peer_id != peer_id {
                return Err(TvmError::Storage("peer book address peer id mismatch"));
            }
            return Ok(address);
        }
        address.push(Protocol::P2p(peer_id));
        Ok(address)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerBookStore {
    path: PathBuf,
}

impl PeerBookStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_records(&self, records: &[PeerRecord]) -> TvmResult<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create peer book directory"))?;
        }
        fs::write(&self.path, encode_peer_records(records))
            .map_err(|_| TvmError::Storage("failed to write peer book"))?;
        Ok(())
    }

    pub fn upsert_record(&self, record: PeerRecord) -> TvmResult<Vec<PeerRecord>> {
        let mut records = if self.path.exists() {
            self.load_records()?
        } else {
            Vec::new()
        };
        if let Some(existing) = records
            .iter_mut()
            .find(|existing| existing.peer_id == record.peer_id)
        {
            *existing = record;
        } else {
            records.push(record);
        }
        self.save_records(&records)?;
        Ok(records)
    }

    pub fn load_records(&self) -> TvmResult<Vec<PeerRecord>> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read peer book"))?;
        decode_peer_records(&bytes)
    }

    pub fn load_bootstrap_addresses(&self) -> TvmResult<Vec<String>> {
        self.load_records()?
            .into_iter()
            .map(|record| Ok(record.bootstrap_multiaddr()?.to_string()))
            .collect()
    }
}

pub fn normalize_bootstrap_multiaddr(address: &str) -> TvmResult<String> {
    let address = parse_multiaddr(address)?;
    if !multiaddr_has_nonzero_tcp(&address) {
        return Err(TvmError::Storage("bootstrap address missing tcp port"));
    }
    if bootstrap_peer_address(&address).is_none() {
        return Err(TvmError::Storage("bootstrap address missing peer id"));
    }
    Ok(address.to_string())
}

pub(super) fn parse_multiaddr(address: &str) -> TvmResult<Multiaddr> {
    address
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))
}

pub(super) fn bootstrap_peer_address(address: &Multiaddr) -> Option<(PeerId, Multiaddr)> {
    let mut peer_address = address.clone();
    match peer_address.pop() {
        Some(Protocol::P2p(peer_id)) => Some((peer_id, peer_address)),
        _ => None,
    }
}

fn multiaddr_has_nonzero_tcp(address: &Multiaddr) -> bool {
    address
        .iter()
        .any(|protocol| matches!(protocol, Protocol::Tcp(port) if port != 0))
}

fn encode_peer_records(records: &[PeerRecord]) -> Vec<u8> {
    let mut payload = Vec::new();
    write_u64(&mut payload, records.len() as u64);
    for record in records {
        write_string(&mut payload, &record.peer_id);
        write_string(&mut payload, &record.address);
    }
    let digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&payload]);
    let mut encoded =
        Vec::with_capacity(PEER_BOOK_MAGIC.len() + payload.len() + PEER_BOOK_DIGEST_LEN);
    encoded.extend_from_slice(PEER_BOOK_MAGIC);
    encoded.extend_from_slice(&payload);
    encoded.extend_from_slice(&digest);
    encoded
}

fn decode_peer_records(bytes: &[u8]) -> TvmResult<Vec<PeerRecord>> {
    if !bytes.starts_with(PEER_BOOK_MAGIC) {
        return Err(TvmError::Storage("invalid peer book magic"));
    }
    let minimum_len = PEER_BOOK_MAGIC.len() + 8 + PEER_BOOK_DIGEST_LEN;
    if bytes.len() < minimum_len {
        return Err(TvmError::Storage("invalid peer book length"));
    }
    let payload_start = PEER_BOOK_MAGIC.len();
    let payload_end = bytes.len() - PEER_BOOK_DIGEST_LEN;
    let payload = &bytes[payload_start..payload_end];
    let expected_digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[payload]);
    if bytes[payload_end..] != expected_digest {
        return Err(TvmError::Storage("peer book checksum mismatch"));
    }

    let mut offset = 0;
    let record_count = read_peer_u64(payload, &mut offset)? as usize;
    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        let peer_id = read_peer_string(payload, &mut offset)?;
        let address = read_peer_string(payload, &mut offset)?;
        let record = PeerRecord { peer_id, address };
        record.peer_id()?;
        record.multiaddr()?;
        records.push(record);
    }
    if offset != payload.len() {
        return Err(TvmError::Storage("trailing peer book bytes"));
    }
    Ok(records)
}

fn read_peer_u64(bytes: &[u8], offset: &mut usize) -> TvmResult<u64> {
    if bytes.len().saturating_sub(*offset) < 8 {
        return Err(TvmError::Storage("truncated peer book u64"));
    }
    let mut out = [0_u8; 8];
    out.copy_from_slice(&bytes[*offset..*offset + 8]);
    *offset += 8;
    Ok(u64::from_le_bytes(out))
}

fn read_peer_string(bytes: &[u8], offset: &mut usize) -> TvmResult<String> {
    let len = read_peer_u64(bytes, offset)? as usize;
    let end = offset
        .checked_add(len)
        .ok_or(TvmError::Storage("peer book string length overflow"))?;
    let Some(raw) = bytes.get(*offset..end) else {
        return Err(TvmError::Storage("truncated peer book string"));
    };
    *offset = end;
    String::from_utf8(raw.to_vec()).map_err(|_| TvmError::Storage("invalid peer book utf8"))
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u64(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_book_store_persists_libp2p_bootstrap_records_and_detects_tampering() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let address_a: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let address_b: Multiaddr = "/dns/bootstrap.tensorvm.example/tcp/4001".parse().unwrap();
        let records = vec![
            PeerRecord::new(peer_a, address_a.clone()),
            PeerRecord::new(peer_b, address_b.clone()),
        ];
        let path = std::env::temp_dir().join(format!(
            "tensor-vm-libp2p-peer-book-{}-{}.bin",
            std::process::id(),
            records.len()
        ));
        let store = PeerBookStore::new(path.clone());
        store.save_records(&records).unwrap();
        assert_eq!(store.path(), path.as_path());

        let loaded = store.load_records().unwrap();
        assert_eq!(loaded, records);
        assert_eq!(
            store.load_bootstrap_addresses().unwrap(),
            vec![
                format!("{address_a}/p2p/{peer_a}"),
                format!("{address_b}/p2p/{peer_b}")
            ]
        );
        assert_eq!(loaded[0].peer_id().unwrap(), peer_a);
        assert_eq!(loaded[1].multiaddr().unwrap(), address_b);

        let mut bytes = std::fs::read(&path).unwrap();
        bytes[PEER_BOOK_MAGIC.len() + 4] ^= 1;
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(
            store.load_records(),
            Err(TvmError::Storage("peer book checksum mismatch"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn peer_book_store_upserts_bootstrap_records_with_peer_ids() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let address_a: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let address_a_updated: Multiaddr = "/ip4/127.0.0.1/tcp/4002".parse().unwrap();
        let address_b = format!("/ip4/127.0.0.1/tcp/4003/p2p/{peer_b}");
        let path = std::env::temp_dir().join(format!(
            "tensor-vm-libp2p-peer-book-upsert-{}-{}.bin",
            std::process::id(),
            peer_a
        ));
        let store = PeerBookStore::new(path.clone());

        let records = store
            .upsert_record(PeerRecord::new(peer_a, address_a))
            .unwrap();
        assert_eq!(records.len(), 1);
        let records = store
            .upsert_record(PeerRecord::from_strings(&peer_b.to_string(), &address_b).unwrap())
            .unwrap();
        assert_eq!(records.len(), 2);
        let records = store
            .upsert_record(PeerRecord::new(peer_a, address_a_updated.clone()))
            .unwrap();
        assert_eq!(records.len(), 2);

        assert_eq!(
            store.load_bootstrap_addresses().unwrap(),
            vec![
                format!("{address_a_updated}/p2p/{peer_a}"),
                address_b.clone()
            ]
        );

        let mismatched_peer = PeerId::random();
        let mismatch = PeerRecord::from_strings(
            &mismatched_peer.to_string(),
            &format!("/ip4/127.0.0.1/tcp/4004/p2p/{peer_a}"),
        );
        assert_eq!(
            mismatch,
            Err(TvmError::Storage("peer book address peer id mismatch"))
        );
        let missing_tcp = PeerRecord::from_strings(&peer_a.to_string(), &format!("/p2p/{peer_a}"));
        assert_eq!(
            missing_tcp,
            Err(TvmError::Storage("peer book address missing tcp port"))
        );
        let zero_tcp = PeerRecord::from_strings(&peer_a.to_string(), "/ip4/127.0.0.1/tcp/0");
        assert_eq!(
            zero_tcp,
            Err(TvmError::Storage("peer book address missing tcp port"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bootstrap_multiaddr_normalization_requires_tcp_and_peer_id() {
        let peer = PeerId::random();
        let address = format!("/dns/bootstrap.tensorvm.example/tcp/4001/p2p/{peer}");

        assert_eq!(normalize_bootstrap_multiaddr(&address).unwrap(), address);
        assert_eq!(
            normalize_bootstrap_multiaddr("/dns/bootstrap.tensorvm.example/tcp/4001"),
            Err(TvmError::Storage("bootstrap address missing peer id"))
        );
        assert_eq!(
            normalize_bootstrap_multiaddr(&format!(
                "/dns/bootstrap.tensorvm.example/tcp/0/p2p/{peer}"
            )),
            Err(TvmError::Storage("bootstrap address missing tcp port"))
        );
        assert_eq!(
            normalize_bootstrap_multiaddr("not-a-multiaddr"),
            Err(TvmError::InvalidReceipt("invalid libp2p multiaddr"))
        );
    }

    #[test]
    fn peer_book_decode_rejects_malformed_records() {
        assert_eq!(
            decode_peer_records(b"bad-peer-book"),
            Err(TvmError::Storage("invalid peer book magic"))
        );

        let mut short = Vec::from(PEER_BOOK_MAGIC);
        short.extend_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            decode_peer_records(&short),
            Err(TvmError::Storage("invalid peer book length"))
        );

        let mut trailing_payload = Vec::new();
        write_u64(&mut trailing_payload, 0);
        trailing_payload.push(1);
        let trailing_digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&trailing_payload]);
        let mut trailing = Vec::from(PEER_BOOK_MAGIC);
        trailing.extend_from_slice(&trailing_payload);
        trailing.extend_from_slice(&trailing_digest);
        assert_eq!(
            decode_peer_records(&trailing),
            Err(TvmError::Storage("trailing peer book bytes"))
        );

        let mut bad_record_payload = Vec::new();
        write_u64(&mut bad_record_payload, 1);
        write_string(&mut bad_record_payload, "not-a-peer-id");
        write_string(&mut bad_record_payload, "/ip4/127.0.0.1/tcp/4001");
        let bad_record_digest =
            hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&bad_record_payload]);
        let mut bad_record = Vec::from(PEER_BOOK_MAGIC);
        bad_record.extend_from_slice(&bad_record_payload);
        bad_record.extend_from_slice(&bad_record_digest);
        assert_eq!(
            decode_peer_records(&bad_record),
            Err(TvmError::Storage("invalid peer id"))
        );

        let mut truncated_string_payload = Vec::new();
        write_u64(&mut truncated_string_payload, 1);
        write_u64(&mut truncated_string_payload, 10);
        truncated_string_payload.extend_from_slice(b"short");
        let truncated_string_digest = hash_bytes(
            b"tensor-vm-libp2p-peer-book-v1",
            &[&truncated_string_payload],
        );
        let mut truncated_string = Vec::from(PEER_BOOK_MAGIC);
        truncated_string.extend_from_slice(&truncated_string_payload);
        truncated_string.extend_from_slice(&truncated_string_digest);
        assert_eq!(
            decode_peer_records(&truncated_string),
            Err(TvmError::Storage("truncated peer book string"))
        );

        let peer = PeerId::random();
        let mut bad_addr_payload = Vec::new();
        write_u64(&mut bad_addr_payload, 1);
        write_string(&mut bad_addr_payload, &peer.to_string());
        write_string(&mut bad_addr_payload, "not-a-multiaddr");
        let bad_addr_digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&bad_addr_payload]);
        let mut bad_addr = Vec::from(PEER_BOOK_MAGIC);
        bad_addr.extend_from_slice(&bad_addr_payload);
        bad_addr.extend_from_slice(&bad_addr_digest);
        assert_eq!(
            decode_peer_records(&bad_addr),
            Err(TvmError::InvalidReceipt("invalid libp2p multiaddr"))
        );

        assert_eq!(
            read_peer_u64(&[1, 2], &mut 0),
            Err(TvmError::Storage("truncated peer book u64"))
        );
    }
}
