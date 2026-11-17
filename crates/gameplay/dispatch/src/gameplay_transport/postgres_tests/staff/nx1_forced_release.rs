//! A signed kick is a saved terminal release, never ordinary reconnect grace.
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use super::*;
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::fresh_admission::FreshAdmissionStore;
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::foundation::GameSessionState;
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::gameplay_transport::connection::GraceExpiryResult;

#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn clock() -> Option<(u64, u64)> {
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    Some((now, now))
}

#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
async fn authorized_kick(
    signing: &Signing,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    operation: u8,
) -> TestResult {
    let mut inspect = signing.request(
        2,
        &signing.config.newest_activation_sha256,
        94,
        clock().ok_or("clock")?.0,
        rules::Action::Inspect {
            character_id: hex(&id(101)),
        },
    );
    inspect.issuer_character_id = Some(hex(&id(101)));
    let signed = signing.sign(&canonical(&inspect)?, false)?;
    let reply = h
        .root
        .submit_staff_tools(authority, &h.node, fence, signed.clone(), false, clock)
        .await?
        .ok_or("self inspect")?;
    assert_eq!(wire::decode_result(&reply)?.code, wire::StaffCode::Ok);
    let mut kick = signing.request(
        3,
        &sha(&signed.canonical),
        operation,
        clock().ok_or("clock")?.0,
        rules::Action::Kick {
            character_id: hex(&id(41)),
        },
    );
    kick.issuer_character_id = Some(hex(&id(101)));
    let reply = h
        .root
        .submit_staff_tools(
            authority,
            &h.node,
            fence,
            signing.sign(&canonical(&kick)?, false)?,
            false,
            clock,
        )
        .await?
        .ok_or("kick")?;
    assert_eq!(wire::decode_result(&reply)?.code, wire::StaffCode::Ok);
    Ok(())
}

include!("nx1_forced_release/active.rs");
include!("nx1_forced_release/activity.rs");
include!("nx1_forced_release/lost.rs");
include!("nx1_forced_release/lost_activity.rs");
