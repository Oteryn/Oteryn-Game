use super::super::weapon_proficiency::{KillDispatch, ProductionProficiencyCatalogue};
use super::*;
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::{
    ChannelId, ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};

fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn fence() -> CurrentCharacterGameplayFence {
    CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(1)).expect("character"),
        game_session_id: GameSessionId::decode(&id(2)).expect("session"),
        connection_generation: ConnectionGeneration::new(1).expect("connection"),
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(3)).expect("world"),
            ChannelId::decode(&id(4)).expect("channel"),
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("scope"),
        expected_character_revision: CharacterRevision::new(1).expect("revision"),
    }
}

fn credit(owner: &mut WeaponProficiencyDispatch, key: &str, points: u32, tag: u8) {
    assert!(
        owner
            .dispatch_kill(KillDispatch {
                death_occurrence: id(tag),
                death_at_ms: 400_000,
                creature_is_summon: false,
                points: Some(points),
                contributor: crate::domain::weapon_proficiency::ProficiencyKillContributor {
                    character_in_channel: true,
                    last_damage_at_ms: 399_999,
                    equipped_weapon_item_key: Some(key),
                    weapon_has_promoted_proficiency: true,
                },
            })
            .is_some()
    );
}

#[test]
fn proficiency_checkpoint_one_receipt_all_tracks_retains_successor_and_rebases_losing_delta() {
    let catalogue = ProductionProficiencyCatalogue::embedded().expect("catalogue");
    let definitions = catalogue.for_content("content-1");
    let keys = ["oteryn:item.tibia.i3264", "oteryn:item.tibia.i3295"];
    let mut owner = WeaponProficiencyDispatch::default();
    for key in keys {
        owner.install_track(key, catalogue.empty_track(key).expect("track"));
    }
    credit(&mut owner, keys[0], 100, 10);
    credit(&mut owner, keys[1], 200, 11);
    let retained = owner
        .prepare_checkpoint(fence(), &[], definitions.as_ref())
        .expect("checkpoint");
    assert_eq!(retained.request.lines().len(), 2);
    owner.checkpoint = Some(retained.clone());
    // Equipping a new weapon while an older receipt is unknown must still admit its track.
    let newly_equipped = "oteryn:item.tibia.i3276";
    owner.install_track(
        newly_equipped,
        catalogue.empty_track(newly_equipped).expect("new track"),
    );
    credit(&mut owner, newly_equipped, 25, 13);
    credit(&mut owner, keys[0], 50, 12);
    // A read sees our receipt before its acknowledgement. It must not add the delta twice.
    let stored: Vec<_> = retained
        .request
        .lines()
        .iter()
        .map(|line| StoredProficiencyTrack {
            state: line.after().clone(),
            committed_character_revision: CharacterRevision::new(2).expect("revision"),
            last_proficiency_occurrence_id: *retained.request.occurrence().as_bytes(),
        })
        .collect();
    for track in &stored {
        owner.install_track(
            track.state.item_key(),
            catalogue.live_track(track).expect("live"),
        );
    }
    assert_eq!(owner.track(keys[0]).expect("track").progress, 150);
    let receipt = CommittedProficiencyChange {
        character_id: fence().character_id,
        occurrence_id: *retained.request.occurrence().as_bytes(),
        original_character_revision: fence().expected_character_revision,
        committed_character_revision: CharacterRevision::new(2).expect("revision"),
        cause: ProficiencyCause::Training,
        command_binding: Vec::new(),
        policy_digest: definitions.digest(),
        lines: retained.request.lines().to_vec(),
    };
    assert!(owner.acknowledge_checkpoint(retained.clone(), receipt.clone(), &catalogue));
    assert!(!owner.acknowledge_checkpoint(retained, receipt, &catalogue));
    assert_eq!(owner.pending_progress.get(keys[0]), Some(&50));
    assert_eq!(owner.pending_progress.get(newly_equipped), Some(&25));
    assert!(!owner.pending_progress.contains_key(keys[1]));
    assert_eq!(owner.track(keys[0]).expect("track").progress, 150);
    // A losing writer keeps 50 points and rebuilds its candidate over a concurrent 25 points.
    let first = owner
        .prepare_checkpoint(fence(), &stored, definitions.as_ref())
        .expect("first");
    let mut concurrent = stored;
    let state = &concurrent[0].state;
    concurrent[0].state = DurableProficiencyState::new(
        state.item_key(),
        state.definition_key(),
        state.definition_revision(),
        state.progress() + 25,
        state.selections().to_vec(),
    )
    .expect("concurrent progress");
    let mut next_fence = fence();
    next_fence.expected_character_revision = CharacterRevision::new(3).expect("revision");
    let next = owner
        .prepare_checkpoint(next_fence, &concurrent, definitions.as_ref())
        .expect("next");
    assert_ne!(first.request.occurrence(), next.request.occurrence());
    assert_eq!(next.request.lines()[0].before().progress(), 125);
    assert_eq!(next.request.lines()[0].after().progress(), 175);
    assert_eq!(owner.pending_progress.get(keys[0]), Some(&50));
}
