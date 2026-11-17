use super::*;

#[test]
fn player_context_revalidates_exact_entity_generation_floor_and_party_snapshot() -> Result {
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityDisplayName, EntityKind, EntityRef, StepDirection,
        WorldSpatialEntity,
    };
    let (mut owner, mut commands, outbox) = host()?;
    owner.play.as_ref().ok_or("play")?.link.open_party();
    let _ = command(&mut commands)?;
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    let tile = TileCoord::new(101, 100);
    let entity = EntityRef {
        identity: [4; 16],
        generation: 8,
    };
    let player = WorldSpatialEntity {
        display_name: Some(EntityDisplayName::new("Visible player").map_err(|_| "name")?),
        kind: EntityKind::Player,
        entity,
        position: ActorPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let view = || PlayView::new(TileCoord::new(100, 100), 7, &[]);
    owner.play.as_mut().ok_or("play")?.view = view()?.with_entities(vec![player])?;
    let target = context(&owner, tile).ok_or("target")?;
    assert_eq!(target.name(), Some("Visible player"));
    assert!(target.invite_enabled());
    owner.open_graphics();
    assert!(!invite(&owner, &target));
    owner.cancel();
    owner.play.as_mut().ok_or("play")?.view = view()?.with_entities(vec![WorldSpatialEntity {
        entity: EntityRef {
            generation: 9,
            ..entity
        },
        ..player
    }])?;
    assert!(!invite(&owner, &target));
    assert!(commands.try_recv().is_err());
    owner.play.as_mut().ok_or("play")?.view = view()?.with_entities(vec![WorldSpatialEntity {
        position: ActorPosition {
            floor: 6,
            ..player.position
        },
        ..player
    }])?;
    assert!(context(&owner, tile).is_none());
    assert!(!invite(&owner, &target));
    owner.play.as_mut().ok_or("play")?.view = view()?.with_entities(vec![player])?;
    assert!(invite(&owner, &target));
    let sent = command(&mut commands)?;
    assert_eq!(
        party::decode_party_intent(&sent.payload).map_err(|_| "invite")?,
        party::PartyIntent::Invite(entity)
    );
    assert!(!invite(&owner, &target));
    assert!(commands.try_recv().is_err());
    publish(&outbox, &state(), party::PartyDisposition::NotVisible)?;
    assert_eq!(
        owner.play.as_ref().ok_or("play")?.link.party_panel().state,
        Some(state())
    );
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .party_request(Intent::Refresh);
    let _ = command(&mut commands)?;
    let mut updated = state();
    updated.members[0].leader = false;
    updated.members[1].leader = true;
    publish(&outbox, &updated, party::PartyDisposition::Ok)?;
    assert!(!invite(&owner, &target), "stale captured leader state");
    assert!(
        !context(&owner, tile)
            .ok_or("nonleader target")?
            .invite_enabled()
    );
    assert!(commands.try_recv().is_err());
    outbox.bevy_party_selected(false, true, "Leader");
    assert!(context(&owner, tile).is_none());
    assert!(!invite(&owner, &target));
    owner.close();
    assert!(!invite(&owner, &target));
    assert!(commands.try_recv().is_err());
    Ok(())
}
