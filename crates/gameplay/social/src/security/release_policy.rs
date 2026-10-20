//! SEC_CLIENT01 §3 role and release-key membership policy only.
//!
//! Raw public key bytes are local comparison facts, not a proposed signed key-id
//! schema. The future cryptographic owner must verify detached signatures over
//! the accepted canonical bytes, resolve current authenticated key state and
//! keep the offline keys distinct. These checks do none of those operations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningRole {
    Release,
    TrustRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedObject {
    Manifest,
    TrustRecord,
    Checkpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    WrongSignerRole,
    RootUsedAsReleaseKey,
    RevokedReleaseKey,
    UnknownReleaseKey,
}

/// A matching role is necessary but proves neither a signature nor authority.
pub fn check_signer_role(
    signer: SigningRole,
    object: SignedObject,
) -> Result<(), PolicyError> {
    match (signer, object) {
        (SigningRole::Release, SignedObject::Manifest)
        | (SigningRole::TrustRoot, SignedObject::TrustRecord | SignedObject::Checkpoint) => Ok(()),
        _ => Err(PolicyError::WrongSignerRole),
    }
}

/// Check a release signer's public bytes against supplied key-set facts. A
/// revocation wins even if malformed input also lists the signer as active.
/// Inputs are not a current-state wrapper: callers must authenticate and resolve
/// the key sets again at their actual consuming boundary. No key-id format,
/// bootstrap state, embedded pin or release artifact schema is defined here.
pub fn check_release_key_membership(
    signer: &[u8; 32],
    pinned_root: &[u8; 32],
    active_release_keys: &[[u8; 32]],
    revoked_release_keys: &[[u8; 32]],
) -> Result<(), PolicyError> {
    if signer == pinned_root {
        return Err(PolicyError::RootUsedAsReleaseKey);
    }
    if revoked_release_keys.contains(signer) {
        return Err(PolicyError::RevokedReleaseKey);
    }
    if !active_release_keys.contains(signer) {
        return Err(PolicyError::UnknownReleaseKey);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
