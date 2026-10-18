//! MAP-WIRE-1 §3 (ARCH-MAP-WIRE-1 §1.5) on the production authority: the composed owner of a
//! bundle World offers and requires capability 18 and holds no domain 2 `WORLD_OBJECT_OVERLAY`
//! state, and a committed door `USE` there carries no domain 2 entry (its object state is a
//! domain 17 tile change). The fixture World's owner, built from the same dependencies, keeps
//! domain 2 and requires nothing.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.

use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::world_object::{UseDisposition, WorldObjectTarget};
use crate::map::boot::tests::{DOOR_AT, door_key, door_world};
use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;

fn door_target(digest: [u8; 32], expected_revision: u64) -> WorldObjectTarget {
    let mut placement = digest.to_vec();
    placement.extend_from_slice(&door_key().to_be_bytes());
    WorldObjectTarget {
        placement,
        expected_revision,
    }
}

#[test]
fn a_bundle_world_owner_requires_18_and_never_holds_domain_2_state() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "bundle_no_domain_2", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let offers_18 = |owner: &super::ComposedFreshAdmission<'_, '_, '_>| {
            owner
                .offered_capabilities()
                .iter()
                .any(|offer| offer.id == CAPABILITY_WORLD_MAP_VIEW_V1)
        };

        // Control: the fixture World's owner serves domain 2 and neither offers nor requires 18.
        let fixture = dependencies.owner(&harness, &authority);
        assert!(fixture.observe_world_object_overlay().await.is_some());
        assert!(!offers_18(&fixture));
        assert_eq!(fixture.required_capability(), None);
        drop(fixture);

        // The same owner on a bundle World's movement cells.
        let world = door_world()?;
        let cells = world
            .movement_cells(dependencies.room.movement_cells())
            .map_err(debug)?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.movement_cells = &cells;
        assert!(offers_18(&owner));
        assert_eq!(
            owner.required_capability(),
            Some(CAPABILITY_WORLD_MAP_VIEW_V1)
        );
        assert!(owner.observe_world_object_overlay().await.is_none());

        // The actor stands beside the door (native floor -7 is movement floor 7).
        owner
            .runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                dependencies.actor,
                MovementLocalPosition {
                    x: i32::from(DOOR_AT.x) - 1,
                    y: i32::from(DOOR_AT.y),
                    floor: -i16::from(DOOR_AT.floor),
                },
            )
            .map_err(debug)?;
        // One committed door USE (the world gate admits one act per owner turn): the door's
        // toggle is a domain 17 tile change, never a domain 2 entry.
        let outcome = owner
            .use_object(
                dependencies.actor,
                UseCommand {
                    game_session_id: dependencies.admitted.game_session_id,
                    command_id: 1,
                    item_fence: None,
                },
                door_target(world.base().digest(), 0),
            )
            .await;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert_eq!(
            outcome.committed, None,
            "no domain 2 entry for a bundle door"
        );
        assert!(owner.observe_world_object_overlay().await.is_none());
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
