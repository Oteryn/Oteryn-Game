use super::*;

#[test]
fn only_the_release_role_signs_manifests() {
    assert_eq!(
        check_signer_role(SigningRole::Release, SignedObject::Manifest),
        Ok(())
    );
    assert_eq!(
        check_signer_role(SigningRole::TrustRoot, SignedObject::Manifest),
        Err(PolicyError::WrongSignerRole)
    );
}

#[test]
fn only_the_root_role_signs_records_and_checkpoints() {
    for object in [SignedObject::TrustRecord, SignedObject::Checkpoint] {
        assert_eq!(check_signer_role(SigningRole::TrustRoot, object), Ok(()));
        assert_eq!(
            check_signer_role(SigningRole::Release, object),
            Err(PolicyError::WrongSignerRole)
        );
    }
}

#[test]
fn a_known_separate_active_release_key_passes_membership_policy() {
    assert_eq!(
        check_release_key_membership(&[2; 32], &[1; 32], &[[2; 32]], &[]),
        Ok(())
    );
}

#[test]
fn a_root_key_cannot_gain_release_role_through_active_membership() {
    assert_eq!(
        check_release_key_membership(&[1; 32], &[1; 32], &[[1; 32]], &[]),
        Err(PolicyError::RootUsedAsReleaseKey)
    );
}

#[test]
fn revocation_wins_over_conflicting_active_membership() {
    assert_eq!(
        check_release_key_membership(&[2; 32], &[1; 32], &[[2; 32]], &[[2; 32]]),
        Err(PolicyError::RevokedReleaseKey)
    );
    assert_eq!(
        check_release_key_membership(&[2; 32], &[1; 32], &[], &[[2; 32]]),
        Err(PolicyError::RevokedReleaseKey)
    );
}

#[test]
fn an_unknown_release_key_is_not_accepted() {
    assert_eq!(
        check_release_key_membership(&[3; 32], &[1; 32], &[[2; 32]], &[]),
        Err(PolicyError::UnknownReleaseKey)
    );
}
