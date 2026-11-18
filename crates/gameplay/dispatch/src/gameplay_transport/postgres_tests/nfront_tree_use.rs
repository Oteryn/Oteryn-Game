use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const NAME: &str = "nfront1-beaver-tree-3";
pub(super) const TARGET: Position = Position {
    x: 32458,
    y: 31997,
    floor: -7,
};
pub(super) const START: Position = Position { x: 32459, ..TARGET };
const BAIT: &str = "oteryn:item.tibia.i9843";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const TREE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission02/beaver3";

#[test]
fn nfront1_postgres_tree_target_steals_bait_and_squirrel_death_recovers_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("PostgreSQL route is empty".into());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

/// Only geometry for the common fixture. The selected USE reads the production registry.
pub(super) fn geometry() -> TestResult<Callback> {
    let mut callback = oteryn_gameplay_quests::quest::targuna_ram::callback()?;
    callback.selector.position = Some(TARGET);
    callback.selector.appearance = Some(3625);
    callback.selector.native = Some(
        oteryn_gameplay_quests::quest::world_callbacks::NativeSelector {
            position: TARGET,
            ordinal: 1,
            appearance: 3625,
        },
    );
    callback.storage.clear();
    callback.item_on_target = None;
    callback.effects = Default::default();
    Ok(callback)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x126-y124.b3"
            )),
            region: (7, 126, 124),
            bounds: (32457, 31996, 32460, 31999),
            start: (START.x, START.y),
            expected: vec![ExpectedTile {
                position: (TARGET.x, TARGET.y),
                ids: vec![4517, 3625],
                counted_entry: None,
            }],
        }],
        &[],
        &[],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(QUESTLINE.into(), 5)],
        0xbc,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "quest fence")?
        .ok_or("quest fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native creatures")?;
    {
        use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        use crate::foundation::{CharacterId, CharacterLease};
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("selected USE capabilities")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: START,
            item: BAIT.into(),
            count: 1,
        },
        981,
        true,
    )
    .await?;
    let reference = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", BAIT, "definition-r1"))
        .ok_or("bait definition")?;
    let map = owner.bundle_map().ok_or("native map")?;
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 1)
        .ok_or("tree key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (updated, disposition, _) =
        super::item_craft::exchange(owner, *admitted, command, &payload, Some(reference)).await?;
    *admitted = updated;
    assert_eq!(disposition, UseDisposition::Committed);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(TREE), Some(&1));
    let live_bait = || async {
        sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(BAIT).fetch_one(&h.pool).await
    };
    assert_eq!(live_bait().await?, 0);
    let queue = owner.states.attack.lock().await.kills().clone();
    let now = owner.owner_now();
    let milliseconds = now.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        let [squirrel] = census.as_slice() else {
            return Err("one live thief expected".into());
        };
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(*squirrel).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(milliseconds)
            .commit_damage_for_bound_attacker(
                *squirrel,
                actor,
                CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command + 1).map_err(debug)?,
                ),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: b"oteryn:creature.thieving_squirrel",
                    occurrence: admitted.game_session_id.as_bytes(),
                    binding: b"nfront-tree-thief-death",
                    damage: health,
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
            *squirrel,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor,
            },
            milliseconds,
            Some(now),
        );
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        live_bait().await?,
        1,
        "native corpse loot restores the stolen bait"
    );
    Ok(())
}
