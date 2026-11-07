//! C16c client capture journey.
//!
//! The immediately preceding E2EF5 C16c journey drives the real purchases, refusals,
//! Convince Creature conversion, corpse creation, Animate Dead replacement, follow/attack,
//! logout, and death paths. This phase records the capability-6 wire shapes consumed by
//! `Session::apply_state_delta` (`crates/session/src/domain_dispatch.rs`) and installed by
//! `WorldEntities::after` (`crates/session/src/entities.rs`).

use super::*;
use oteryn_protocol_oteryn::entity_names::EntityDisplayName;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesDelta, WorldSpatialEntity,
    encode_world_spatial_entities_delta,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    // Run the real row on a fresh admission. Its own receipts fail the journey if any server
    // action or reply is absent; capture failures remain evidence-only as required.
    full_e2ef5::run(clients, world).await;
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = write_capture(Path::new(&root)) {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C16c.client status=CAPTURE-WRITE-ERROR evidence={error}"
        ));
    }
}

fn write_capture(root: &Path) -> TestResult {
    let generation = [0x11; 32];
    let at = ActorPosition { x: 1, y: 0, floor: 0 };
    let actor = |marker: u8, name: &str, position: ActorPosition| -> TestResult<_> {
        Ok(WorldSpatialEntity {
            display_name: Some(
                EntityDisplayName::new(name).map_err(|error| format!("{error:?}"))?,
            ),
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [marker; 16],
                generation: 1,
            },
            position,
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        })
    };
    let convince = WorldSpatialEntitiesDelta {
        content_generation: generation,
        actor_position: ActorPosition { x: 0, y: 0, floor: 0 },
        enter: vec![],
        update: vec![actor(2, "Rat", at)?],
        leave: vec![],
    };
    let animate = WorldSpatialEntitiesDelta {
        content_generation: generation,
        actor_position: ActorPosition { x: 0, y: 0, floor: 0 },
        enter: vec![actor(4, "Skeleton", ActorPosition { x: 2, ..at })?],
        update: vec![],
        leave: vec![EntityRef {
            identity: [3; 16],
            generation: 0,
        }],
    };
    std::fs::write(
        root.join("c16c-convince.bin"),
        encode_world_spatial_entities_delta(&convince)
            .map_err(|error| format!("{error:?}"))?,
    )?;
    std::fs::write(
        root.join("c16c-animate.bin"),
        encode_world_spatial_entities_delta(&animate).map_err(|error| format!("{error:?}"))?,
    )?;
    std::fs::write(
        root.join("c16c-expected.txt"),
        "Rat 02020202020202020202020202020202 1\nSkeleton 04040404040404040404040404040404 1\n",
    )?;
    Ok(())
}
