//! Candidate profile3 artifact pair. Server provenance and client map geometry
//! are independently bounded and hashed; neither carrier grants active status.
use super::digest::sha256;
use super::project::native_spell_world::client_projection;
use super::{CompiledFirstProductionContent, ContentError};
const SERVER_MAGIC: &[u8; 8] = b"OTNSW03\0";
const CLIENT_MAGIC: &[u8; 8] = b"OTNCW03\0";
pub(crate) const MAX_SERVER_BYTES: usize = 96 * 1024 * 1024;
pub(crate) const MAX_CLIENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeSourceWorldArtifactPair {
    pub(crate) server: Vec<u8>,
    pub(crate) client: Vec<u8>,
}
pub(crate) struct DecodedNativeSourceWorld<'a> {
    pub(crate) baseline_server: &'a [u8],
    pub(crate) baseline_client: &'a [u8],
    pub(crate) source_world: &'a [u8],
}
pub(crate) fn is_envelope(bytes: &[u8]) -> bool {
    bytes.starts_with(SERVER_MAGIC)
}
fn invalid() -> ContentError {
    ContentError::InvalidArtifact("native source world artifact pair")
}
fn section(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ContentError> {
    let len = u32::try_from(bytes.len()).map_err(|_| invalid())?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&sha256(bytes));
    out.extend_from_slice(bytes);
    Ok(())
}
pub(crate) fn compile(
    base: &CompiledFirstProductionContent,
    source: &[u8],
) -> Result<NativeSourceWorldArtifactPair, ContentError> {
    if is_envelope(&base.server_artifact) || source.len() > MAX_SOURCE_BYTES {
        return Err(invalid());
    }
    super::production::StagedGeneration::stage(
        &base.server_artifact,
        &base.client_artifact,
        base.expectation(),
    )?;
    let geometry = client_projection(source).map_err(|_| invalid())?;
    let mut server = SERVER_MAGIC.to_vec();
    section(&mut server, &base.server_artifact)?;
    section(&mut server, source)?;
    let mut client = CLIENT_MAGIC.to_vec();
    section(&mut client, &base.client_artifact)?;
    section(&mut client, &geometry)?;
    // Cross-pin the client projection in the actual server envelope, before computing outer pin.
    server.extend_from_slice(&sha256(&client));
    if server.len() > MAX_SERVER_BYTES || client.len() > MAX_CLIENT_BYTES {
        return Err(invalid());
    }
    decode(&server, &client)?;
    Ok(NativeSourceWorldArtifactPair { server, client })
}
struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], ContentError> {
        let end = self.offset.checked_add(len).ok_or_else(invalid)?;
        let value = self.bytes.get(self.offset..end).ok_or_else(invalid)?;
        self.offset = end;
        Ok(value)
    }
    fn section(&mut self, max: usize) -> Result<&'a [u8], ContentError> {
        let len = u32::from_be_bytes(self.take(4)?.try_into().map_err(|_| invalid())?) as usize;
        if len == 0 || len > max {
            return Err(invalid());
        }
        let digest = self.take(32)?;
        let bytes = self.take(len)?;
        if digest != sha256(bytes) {
            return Err(invalid());
        }
        Ok(bytes)
    }
}
pub(crate) fn decode<'a>(
    server: &'a [u8],
    client: &'a [u8],
) -> Result<DecodedNativeSourceWorld<'a>, ContentError> {
    if server.len() > MAX_SERVER_BYTES || client.len() > MAX_CLIENT_BYTES {
        return Err(invalid());
    }
    let mut s = Cursor {
        bytes: server,
        offset: 0,
    };
    let mut c = Cursor {
        bytes: client,
        offset: 0,
    };
    if s.take(8)? != SERVER_MAGIC || c.take(8)? != CLIENT_MAGIC {
        return Err(invalid());
    }
    let baseline_server = s.section(super::native_gameplay::MAX_ARTIFACT_BYTES)?;
    let source_world = s.section(MAX_SOURCE_BYTES)?;
    let baseline_client =
        c.section(super::production::FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES)?;
    let geometry = c.section(MAX_CLIENT_BYTES)?;
    if s.take(32)? != sha256(client)
        || s.offset != server.len()
        || c.offset != client.len()
        || is_envelope(baseline_server)
        || client_projection(source_world).map_err(|_| invalid())? != geometry
    {
        return Err(invalid());
    }
    Ok(DecodedNativeSourceWorld {
        baseline_server,
        baseline_client,
        source_world,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn actual_map_pair_has_independent_client_pin_and_strict_corruption_refusal() {
        let world = crate::foundation::WorldId::decode(&[
            0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])
        .unwrap();
        let base = crate::content::qualify_native_entry_room(world).unwrap();
        let source = include_bytes!(
            "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        );
        let pair = compile(base.compiled(), source).unwrap();
        let decoded = decode(&pair.server, &pair.client).unwrap();
        assert_eq!(decoded.baseline_server, base.compiled().server_artifact);
        assert_eq!(decoded.baseline_client, base.compiled().client_artifact);
        assert_eq!(decoded.source_world, source);
        assert_ne!(sha256(&pair.client), base.compiled().client_digest());
        let compiled = base.compiled().with_native_artifact_pair(
            pair.server.clone(), pair.client.clone());
        let staged = super::super::production::StagedGeneration::stage(
            &pair.server, &pair.client, compiled.expectation()).unwrap();
        assert_eq!(staged.runtime_state().native_source_world().unwrap(), source);
        assert!(staged.runtime_state().native_gameplay().is_none());
        // Outer issuance compares the complete pair before any inner decoding.
        let mut different_server = pair.server.clone();
        different_server[0] ^= 1;
        let wrong_server = base.compiled().with_native_artifact_pair(
            different_server, pair.client.clone());
        assert!(matches!(super::super::production::StagedGeneration::stage(
            &pair.server, &pair.client, wrong_server.expectation()),
            Err(ContentError::RevisionMismatch("native source world outer issuance pins"))));
        let mut different_client = pair.client.clone();
        different_client[0] ^= 1;
        let wrong_client = base.compiled().with_native_artifact_pair(
            pair.server.clone(), different_client);
        assert!(matches!(super::super::production::StagedGeneration::stage(
            &pair.server, &pair.client, wrong_client.expectation()),
            Err(ContentError::RevisionMismatch("native source world outer issuance pins"))));
        let mut server = pair.server.clone();
        server.push(0);
        assert!(decode(&server, &pair.client).is_err());
        let mut client = pair.client.clone();
        let last = client.len() - 1;
        client[last] ^= 1;
        assert!(decode(&pair.server, &client).is_err());
        assert!(decode(&pair.server, &pair.client[..pair.client.len() - 1]).is_err());
    }
}
