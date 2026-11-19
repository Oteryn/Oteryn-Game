//! Crystal 201189f8 data-global/scripts/quests/blood_brothers_quest/actions_lersatio_mirrors.lua:3-79.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::{Value, json};
use std::time::Duration;

pub(super) const NAME: &str = "lersatio-mirrors";
const QUEST: &str = "oteryn:quest.blood_brothers_quest";
const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission08";
const COUNTER: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mirror_counter";
const LERSATIO: &str = "oteryn:creature.lersatio";
const VAMPIRE: &str = "oteryn:creature.vampire";
const SECOND_TAUNT: &str = "FINE. DIE PRETTY!";
pub(super) const MIRROR: Position = Position {
    x: 32965,
    y: 31457,
    floor: -2,
};
const STATUE: Position = Position {
    x: 32967,
    y: 31457,
    floor: -2,
};
const DESTINATION: Position = Position {
    x: 32967,
    y: 31459,
    floor: -1,
};
const EXIT: Position = Position {
    y: 31463,
    ..DESTINATION
};

fn binding(second: bool) -> Value {
    let count = i64::from(second);
    let mut effects = json!({
        "transform":8721, "reset_after_ms":30000, "source_magic":3, "source_must_match":true,
        "player_speech": if second {
            SECOND_TAUNT
        } else {
            "STOP THAT, YOU WORM, OR I'LL SHOW YOU THE BEAUTY OF DEATH."
        }
    });
    if second {
        effects["encounter_group"] = json!({
            "minimum":{"x":32954,"y":31447,"floor":-2},
            "maximum":{"x":32972,"y":31465,"floor":-2},
            "destination":DESTINATION
        });
        effects["stationary_encounter"] = json!([
            {"creature":LERSATIO,"position":DESTINATION},
            {"creature":VAMPIRE,"position":Position { x: 32966, ..DESTINATION }},
            {"creature":VAMPIRE,"position":Position { x: 32968, ..DESTINATION }}
        ]);
    }
    json!({
        "key":format!("oteryn:quest-callback/fixture-lersatio-mirror-{count}"),
        "quest":QUEST,"trigger":"use",
        "selector":{"position":MIRROR,"appearance":8667,"native":{"position":MIRROR,"ordinal":2,"appearance":8667}},
        "lever":null,"reversible":false,"teleport":null,"offerings":[],
        "passages":[{"position":STATUE,"appearance":8325,"replacement":8326,"if_present":true}],
        "item_on_target":null,"return_on_failure":false,"failure_message":null,
        "requires_world_admission":true,
        "storage":[
            {"track":MISSION,"minimum":-1,"maximum":1,"set":1,"preserve":true},
            {"track":COUNTER,"minimum":count,"maximum":count,"set":1-count}
        ],
        "effects":effects,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/actions_lersatio_mirrors.lua:3-79"]
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(binding(true))?)
}

pub(super) fn world(row: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks_fixtures::world_with_blood_brothers_exit(row, MIRROR, EXIT)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":[binding(false), binding(true)]}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    let peer_start = Position { x: 32966, ..MIRROR };
    let (peer, peer_session) =
        super::arthei_ghost_tear_fixtures::admit_peer(owner, admitted, h, authority, peer_start)
            .await?;
    let sessions = [admitted.game_session_id, peer_session];
    for session in sessions {
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
    }
    let start = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        use_mirror(owner, admitted, 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::m39_bone_journey_fixtures::stage(h, COUNTER).await?,
        Some(1)
    );
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            start
        );
        assert_eq!(
            runtime.read_actor_position(peer).map_err(debug)?.position(),
            super::quest_world_callbacks_fixtures::local(peer_start)
        );
        assert!(runtime.visible_entities().creatures.is_empty());
    }
    let due = {
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(appearance(map, MIRROR)?, 8721);
        assert_eq!(appearance(map, STATUE)?, 8326);
        map.facts
            .next_movement_decay_time()
            .ok_or("mirror repair deadline")?
    };
    assert_ne!(
        use_mirror(owner, admitted, 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::m39_bone_journey_fixtures::stage(h, COUNTER).await?,
        Some(1)
    );
    {
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(31))
        .ok_or("owner clock")?;
    let stop = oteryn_foundation::CancellationToken::new();
    let wait_for_repair = async {
        let map = owner.bundle_map().ok_or("map")?;
        while appearance(map, MIRROR)? != 8667 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        stop.cancel();
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, repaired) = tokio::time::timeout(
        Duration::from_secs(10),
        super::fixture::join_two(owner.run_source_owner_cycles(&stop), wait_for_repair),
    )
    .await?;
    repaired?;

    for session in sessions {
        let _ = owner.states.chat.drain(session);
    }
    let effect_cursor = super::arthei_ghost_tear_fixtures::group_effect_cursor(owner)?;
    assert_eq!(
        use_mirror(owner, admitted, 3).await?,
        UseDisposition::Committed
    );
    super::arthei_ghost_tear_fixtures::assert_group_source_speech(
        owner,
        actor,
        sessions,
        start,
        SECOND_TAUNT,
    );
    super::arthei_ghost_tear_fixtures::assert_group_magic(
        owner,
        effect_cursor,
        MIRROR,
        DESTINATION,
        3,
    )?;
    assert_eq!(
        super::m39_bone_journey_fixtures::stage(h, COUNTER).await?,
        Some(0)
    );
    let destination = super::quest_world_callbacks_fixtures::local(DESTINATION);
    let runtime = owner.runtime.lock().await;
    for member in [actor, peer] {
        assert_eq!(
            runtime
                .read_actor_position(member)
                .map_err(debug)?
                .position(),
            destination
        );
    }
    let creatures = runtime.visible_entities().creatures;
    for (creature, count) in [(LERSATIO, 1), (VAMPIRE, 2)] {
        assert_eq!(
            creatures.iter().filter(|row| runtime.matches_live_creature_identity(row.actor, creature.as_bytes())).count(),
            count
        );
    }
    drop(runtime);
    super::arthei_ghost_tear_fixtures::leave_room(
        owner,
        [(actor, admitted.game_session_id), (peer, peer_session)],
        DESTINATION,
        EXIT,
    )
    .await
}

fn placement(at: Position, ordinal: u8) -> TestResult<u64> {
    oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ordinal)
        .ok_or_else(|| "mirror fixture placement".into())
}

fn appearance(map: &crate::map::boot::BundleMap, at: Position) -> TestResult<u16> {
    use crate::map::{
        overlay::TilePos,
        view::{MapEntryKey, compose_tile},
    };
    let pos = TilePos {
        x: at.x,
        y: at.y,
        floor: at.floor,
    };
    let key = MapEntryKey::Base(placement(at, if at == MIRROR { 2 } else { 1 })?);
    compose_tile(&map.overlay, &map.facts.pinned(), pos)
        .map_err(debug)?
        .and_then(|tile| {
            tile.entries
                .into_iter()
                .find(|entry| entry.source.key() == key)
        })
        .map(|entry| entry.facts.appearance_id)
        .ok_or_else(|| "mirror or statue appearance".into())
}

async fn use_mirror(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command_id: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = placement(MIRROR, 2)?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    Ok(owner
        .use_object(
            admitted.runtime_actor.ok_or("player actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await
        .disposition)
}
