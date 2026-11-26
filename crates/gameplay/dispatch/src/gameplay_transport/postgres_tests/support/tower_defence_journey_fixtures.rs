use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, WorldObjectTarget};

use super::classic_quests_fixtures::{install_positions, progress, talk, travel};
use super::fixture::{Harness, debug};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_scene_with_appearances};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use crate::map::{overlay::TilePos, view::placement_key};

pub(super) const NAME: &str = "qengine-rewards-tower-native-journey";
pub(super) const START: Position = Position {
    x: 32317,
    y: 31752,
    floor: -5,
};
const BUNNY: Position = Position { y: 31751, ..START };
const THANITA: Position = Position {
    x: 32535,
    y: 31773,
    floor: -1,
};
const THANITA_STAND: Position = Position {
    y: 31774,
    ..THANITA
};
const OUTSIDE: Position = Position {
    x: 32599,
    y: 31758,
    floor: -9,
};
const INSIDE: Position = Position {
    x: 32605,
    y: 31753,
    floor: -9,
};
const DOOR: TilePos = TilePos {
    x: 32600,
    y: 31758,
    floor: -9,
};
const TRACK: &str = "oteryn:quest-progress/quest/u8_1/tower_defence_quest/questline";
const ENTITLEMENT: &str = "oteryn:quest-progress/quest/u8_1/tower_defence_quest/door";
const LEADER: &str = "oteryn:creature.goblin_leader";

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    let crop = |bytes: &'static [u8], region, bounds, start: Position, expected: Position, ids| {
        NativeCrop {
            bytes,
            region,
            bounds,
            start: (start.x, start.y),
            expected: vec![ExpectedTile {
                position: (expected.x, expected.y),
                ids,
                counted_entry: None,
            }],
        }
    };
    native_scene_with_appearances(
        native,
        vec![
            crop(
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z05-x126-y124.b3"
                )),
                (5, 126, 124),
                (32315, 31750, 32319, 31753),
                START,
                BUNNY,
                vec![464],
            ),
            crop(
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z01-x127-y124.b3"
                )),
                (1, 127, 124),
                (32533, 31772, 32537, 31775),
                THANITA_STAND,
                THANITA,
                vec![408],
            ),
            crop(
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z09-x127-y124.b3"
                )),
                (9, 127, 124),
                (32598, 31751, 32608, 31759),
                OUTSIDE,
                Position {
                    x: DOOR.x,
                    y: DOOR.y,
                    floor: DOOR.floor,
                },
                vec![7356, 6258],
            ),
        ],
        &[],
        &[6259],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [bunny, thanita] = install_positions(
        owner,
        admitted,
        ["oteryn:npc.bunny_bonecrusher", "oteryn:npc.thanita"],
        [BUNNY, THANITA],
    )
    .await?;
    travel(owner, admitted, OUTSIDE).await?;
    let map = owner.bundle_map().ok_or("hideout map")?;
    let key = placement_key(DOOR, 1).ok_or("hideout door")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let reply = Box::pin(super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        command,
        2,
        payload.clone(),
    ))
    .await?;
    assert_eq!(
        world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );

    travel(owner, admitted, START).await?;
    for (id, words) in [(602, "hail general"), (603, "mission"), (604, "yes")] {
        talk(owner, admitted, bunny, id, words).await?;
    }
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 1);

    travel(owner, admitted, THANITA_STAND).await?;
    talk(owner, admitted, thanita, 605, "hi").await?;
    talk(owner, admitted, thanita, 606, "mission").await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 2);
    assert_eq!(progress(owner, admitted, authority, ENTITLEMENT).await?, 2);
    let parcel: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i140'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(parcel, 0);

    travel(owner, admitted, OUTSIDE).await?;
    let command = admitted.continuity.next_command_id;
    let reply = Box::pin(super::quest_world_callbacks_fixtures::wire(
        owner, admitted, command, 2, payload,
    ))
    .await?;
    assert_eq!(
        world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    travel(owner, admitted, INSIDE).await?;
    defeat_goblin_leader(owner, admitted).await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 3);

    travel(owner, admitted, THANITA_STAND).await?;
    talk(owner, admitted, thanita, 608, "hi").await?;
    let lines = talk(owner, admitted, thanita, 609, "mission").await?;
    assert!(lines.iter().any(|line| line.contains("password*")));
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 4);

    travel(owner, admitted, START).await?;
    for (id, words) in [(610, "hail general"), (611, "mission"), (612, "password")] {
        talk(owner, admitted, bunny, id, words).await?;
    }
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 5);
    let arrows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT definition_production_key, sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i761','oteryn:item.tibia.i762','oteryn:item.tibia.i763','oteryn:item.tibia.i774') GROUP BY definition_production_key ORDER BY definition_production_key",
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        arrows,
        [761, 762, 763, 774].map(|item| (format!("oteryn:item.tibia.i{item}"), 50))
    );
    Ok(())
}

async fn defeat_goblin_leader(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native creature policies")?;
    let actor = admitted.runtime_actor.ok_or("player")?;
    let session = admitted.game_session_id;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let observed = owner.owner_now();
    let now = observed.get() / 1_000;
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let pin = runtime.content_pin().clone();
    let snapshot = runtime
        .realize_native_qualification_spawn(
            pin,
            LEADER,
            MovementLocalPosition {
                x: 32604,
                y: 31753,
                floor: 9,
            },
        )
        .map_err(debug)?;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            snapshot.actor,
            actor,
            CommandRef::new(session, CommandId::new(608).map_err(debug)?),
            0,
            crate::foundation::OwnerDamageCommand {
                target: LEADER.as_bytes(),
                occurrence: session.as_bytes(),
                binding: b"tower-defence-native-spawn",
                damage: snapshot.health,
            },
        )
        .map_err(debug)?;
    record_creature_kill(
        &mut runtime,
        KillSink {
            prey: None,
            activity: &[],
            queue: &queue,
            rewards: native.reward_table(),
            charms: None,
        },
        snapshot.actor,
        crate::combat::CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor,
        },
        now,
        Some(observed),
    );
    drop(runtime);
    assert_eq!(
        Box::pin(owner.drain_kill_rewards(session)).await,
        DrainOutcome::Drained
    );
    Ok(())
}
