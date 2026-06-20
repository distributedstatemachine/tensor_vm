use crate::error::{Result, TvmError};
use crate::types::Hash;

pub(super) const HASH_LEN: usize = 32;
pub(super) const U64_LEN: usize = 8;

pub(super) fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn write_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn write_len(out: &mut Vec<u8>, value: usize) {
    write_u64(out, value as u64);
}

pub(super) fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

pub(super) fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    write_len(out, bytes.len());
    out.extend_from_slice(bytes);
}

pub(super) fn write_option_hash(out: &mut Vec<u8>, value: &Option<Hash>) {
    match value {
        Some(hash) => {
            out.push(1);
            write_hash(out, hash);
        }
        None => out.push(0),
    }
}

pub(super) fn read_u64_at(bytes: &[u8], offset: &mut usize, error: &'static str) -> Result<u64> {
    if bytes.len().saturating_sub(*offset) < U64_LEN {
        return Err(TvmError::Storage(error));
    }
    let mut out = [0_u8; U64_LEN];
    out.copy_from_slice(&bytes[*offset..*offset + U64_LEN]);
    *offset += U64_LEN;
    Ok(u64::from_le_bytes(out))
}

pub(super) fn read_hash_at(bytes: &[u8], offset: &mut usize, error: &'static str) -> Result<Hash> {
    if bytes.len().saturating_sub(*offset) < HASH_LEN {
        return Err(TvmError::Storage(error));
    }
    let mut out = [0_u8; HASH_LEN];
    out.copy_from_slice(&bytes[*offset..*offset + HASH_LEN]);
    *offset += HASH_LEN;
    Ok(out)
}

pub(super) struct StateReader<'a> {
    pub(super) input: &'a [u8],
    pub(super) offset: usize,
}

impl<'a> StateReader<'a> {
    pub(super) fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    pub(super) fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.input.len().saturating_sub(self.offset) < len {
            return Err(TvmError::Storage("truncated chain state"));
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.input[start..self.offset])
    }

    pub(super) fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_exact(1)?[0])
    }

    pub(super) fn read_u64(&mut self) -> Result<u64> {
        let mut out = [0_u8; U64_LEN];
        out.copy_from_slice(self.read_exact(U64_LEN)?);
        Ok(u64::from_le_bytes(out))
    }

    pub(super) fn read_i64(&mut self) -> Result<i64> {
        let mut out = [0_u8; U64_LEN];
        out.copy_from_slice(self.read_exact(U64_LEN)?);
        Ok(i64::from_le_bytes(out))
    }

    pub(super) fn read_len(&mut self) -> Result<usize> {
        Ok(self.read_u64()? as usize)
    }

    pub(super) fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.read_len()?;
        Ok(self.read_exact(len)?.to_vec())
    }

    pub(super) fn read_hash(&mut self) -> Result<Hash> {
        let mut out = [0_u8; HASH_LEN];
        out.copy_from_slice(self.read_exact(HASH_LEN)?);
        Ok(out)
    }

    pub(super) fn read_option_hash(&mut self) -> Result<Option<Hash>> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_hash()?)),
            _ => Err(TvmError::Storage("invalid optional hash")),
        }
    }

    pub(super) fn finish(&self) -> Result<()> {
        if self.offset != self.input.len() {
            return Err(TvmError::Storage("trailing chain state bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_codec_reads_and_rejects_bad_edges() {
        let hash = [7; HASH_LEN];
        let mut encoded = Vec::new();
        write_u64(&mut encoded, 42);
        write_i64(&mut encoded, -7);
        write_len(&mut encoded, 3);
        write_hash(&mut encoded, &hash);
        write_bytes(&mut encoded, b"payload");
        write_option_hash(&mut encoded, &Some(hash));
        write_option_hash(&mut encoded, &None);

        let mut reader = StateReader::new(&encoded);
        assert_eq!(reader.read_u64().unwrap(), 42);
        assert_eq!(reader.read_i64().unwrap(), -7);
        assert_eq!(reader.read_len().unwrap(), 3);
        assert_eq!(reader.read_hash().unwrap(), hash);
        assert_eq!(reader.read_bytes().unwrap(), b"payload");
        assert_eq!(reader.read_option_hash().unwrap(), Some(hash));
        assert_eq!(reader.read_option_hash().unwrap(), None);
        reader.finish().unwrap();

        assert_eq!(
            StateReader::new(&[]).read_u8(),
            Err(TvmError::Storage("truncated chain state"))
        );
        assert_eq!(
            StateReader::new(&[2]).read_option_hash(),
            Err(TvmError::Storage("invalid optional hash"))
        );

        let mut offset = 0;
        assert_eq!(
            read_u64_at(&[1, 2], &mut offset, "truncated test u64"),
            Err(TvmError::Storage("truncated test u64"))
        );
        assert_eq!(offset, 0);
        assert_eq!(
            read_hash_at(&[1, 2], &mut offset, "truncated test hash"),
            Err(TvmError::Storage("truncated test hash"))
        );
    }
}
