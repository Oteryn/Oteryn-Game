//! Native region 127,127, native Grizzly signup, source-spawn and wire combat.
//! Session, level/vocation and relocation are explicit fixture setup, not a node walk.
use super::TestResult;
use super::fixture::{debug, id};
use crate::foundation::{MovementLocalPosition, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::attack::{
    AttackIntentDisposition, CAPABILITY_ATTACK_V1, decode_attack_intent_result,
    encode_attack_target_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;

const HOME: MovementLocalPosition = MovementLocalPosition {
    x: 32608,
    y: 32749,
    floor: 9,
};
const CREATURE: &str = "oteryn:creature.crocodile";

#[test]
fn qcx5_grizzly_source_crocodile_wire_combat_credit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qcx5-grizzly-source-crocodile", || {
        super::grizzly_npc::scenario(7)
    })
}

pub(super) fn world(
    world: WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle_compiler::{bundle, compile, project, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(127, 127)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (_, mut pins) = crate::map::boot::tests::bundle();
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity: bundle::Identity {
            project_format_version: pins.project_format_version.clone(),
            world_schema_version: pins.world_schema_version.clone(),
            content_revision: pins.content_revision.clone(),
            ..Default::default()
        },
        world: source::world_extent(&root)?,
        build_class: bundle::BuildClass::NonProduction,
        draft_areas: project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/spawns/spawns-28000-29999.json"),
    )?)?;
    let declaration = &raw["records"][1353]["declaration"];
    assert_eq!(
        declaration["identity"]["key"],
        "oteryn:spawn.x32609_y32749_z9"
    );
    let point = &declaration["points"][0];
    assert_eq!(point["creature"], CREATURE);
    assert_eq!(
        point["cell"],
        serde_json::json!({"x":32608,"y":32749,"floor":9})
    );
    assert_eq!(point["respawn_ms"], 90_000);
    let data = bundle::read(&compiled.bytes)?;
    let source_key = declaration["identity"]["key"]
        .as_str()
        .ok_or("source key")?;
    let source = data
        .spawns
        .sources
        .iter()
        .find(|s| s.key == source_key)
        .ok_or("compiled native source")?;
    let points: Vec<_> = source
        .points
        .iter()
        .filter(|p| p.x == 32608 && p.y == 32749)
        .collect();
    assert_eq!(points.len(), 1);
    assert_eq!(source.floor, -9);
    assert_eq!(points[0].respawn_ms, 90_000);
    assert_eq!(
        data.spawns.creatures[usize::try_from(points[0].creature)?].key,
        CREATURE
    );
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&compiled.bytes, &crate::map::boot::BootPins {
        map_revision: crate::map::overlay::map_revision(&base), bundle: pins,
        start: crate::map::overlay::TilePos { x:32692, y:32767, floor:-6 },
    }, world, crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?, |key| {
        use crate::content::{ReferenceItemField, ReferenceItemType};
        let index = native.item_index()?;
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item", key, revision)?;
        let profile = native.item_policy(key, revision).map(|p| p.record());
        let pickupable = profile.is_some_and(|p| matches!(&p.semantics.physical, ReferenceItemField::Known(v) if v.pickupable == ReferenceItemField::Known(true)));
        let door = profile.is_some_and(|p| matches!(&p.semantics.classification, ReferenceItemField::Known(v) if v.item_type == ReferenceItemField::Known(ReferenceItemType::Door)));
        let attributes = profile.map(|p| &p.attributes);
        Some(crate::map::facts::ItemDefinition { reference, solid: attributes.and_then(|a| a.blocks_movement), blocks_projectile: attributes.and_then(|a| a.blocks_projectile).unwrap_or(true), pickupable, door })
    }).map_err(debug)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    bundle: &crate::map::boot::BundleWorld,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use oteryn_gameplay_quests::quest::grizzly_adams as quest;
    // Explicit combat eligibility fixture, after the native low-level task signup.
    // The real XP owner establishes level 130; this is not earned campaign progress.
    combat_level_fixture(owner, admitted).await?;
    // Fresh admission normally creates this empty equipment state. Keep its
    // production initialization and the current post-XP item fence for fists.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "combat equipment fence")?
        .ok_or("combat equipment fence")?;
    let generation = owner.active_generation.ok_or("combat content generation")?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    owner
        .root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(|error| format!("source crocodile admission equipment: {error:?}"))?;
    admitted.item_fence = Some(fence.item_fence());
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    let counter = quest::KILL_COUNTER_BASE + 1;
    assert_eq!(
        super::grizzly_npc::value(owner, authority, counter).await?,
        0
    );
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native gameplay")?;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let mut sources = bundle
            .activation_spawns(|key| {
                let p = runtime.companion_policy(key).ok()?;
                Some((u64::try_from(p.maximum_health).ok()?, false))
            })
            .ok_or("native spawn policies")?;
        for (points, _) in &mut sources {
            points.retain(|p| p.0 == (HOME.x, HOME.y, HOME.floor));
        }
        sources.retain(|(points, _)| !points.is_empty());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0.len(), 1);
        assert_eq!(sources[0].0[0].1, 90_000);
        assert_eq!(sources[0].0[0].2, CREATURE);
        runtime
            .begin_activation_spawn_sources(sources)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .realize_activation_spawn_window(0)
                .map_err(debug)?
                .realized,
            1
        );
        let points = runtime.spawn_point_creatures();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].1, HOME);
        let target = points[0].2.ok_or("source crocodile")?;
        assert_eq!(runtime.creature_spawn_cell(target), Some(HOME));
        assert_eq!(
            runtime
                .companion_snapshot(target)
                .map_err(debug)?
                .state
                .policy
                .definition_key,
            CREATURE
        );
        target
    };
    // Fixture relocation places the hunter beside the native spawn. No traversal claim.
    let map = owner.bundle_map().ok_or("native map")?;
    {
        let _permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            &owner.movement_cells,
            actor,
            admitted.game_session_id,
            before,
            MovementLocalPosition {
                x: HOME.x - 1,
                ..HOME
            },
            false,
            &doors,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert!(
        owner
            .refresh_source_party_presence(actor, admitted.game_session_id)
            .await
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3, 6, 16, CAPABILITY_ATTACK_V1],
        )
        .ok_or("combat capabilities")?;
    owner.clock_origin -= Duration::from_secs(10);
    owner.drain_monster_melee().await;
    {
        let states = owner.states.spell_states.lock().await;
        assert!(
            states
                .monster_melee
                .creature_ai
                .state(target)
                .and_then(|s| s.target())
                .is_some(),
            "real monster thinking acquired the hunter"
        );
    }
    let reference = EntityRef {
        identity: target.placement_identity(),
        generation: target.actor_local_generation(),
    };
    let payload = encode_attack_target_intent(Some(&reference)).map_err(debug)?;
    let result =
        super::quest_world_callbacks::wire(owner, admitted, 6, 11, payload.clone()).await?;
    assert_eq!(
        decode_attack_intent_result(&result).map_err(debug)?,
        AttackIntentDisposition::Ok
    );
    let queue = owner.states.attack.lock().await.kills().clone();
    for _ in 0..256 {
        // Survival is fixture setup: retain normal damage calculations and AI, while
        // restoring hunter vitals so the test can reach its lethal turn.
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let player = states
                .get_mut(&runtime, actor, admitted.game_session_id)
                .ok_or("hunter cast state")?;
            player.restore_saved_vitals(player.facts.max_health, player.facts.max_mana);
        }
        owner
            .refresh_attack_weapon(actor, admitted.game_session_id)
            .await;
        owner.drain_auto_attacks().await;
        if queue.has_session(admitted.game_session_id) {
            break;
        }
        owner.clock_origin -= Duration::from_secs(3);
        owner.drain_monster_melee().await;
    }
    assert!(
        queue.has_session(admitted.game_session_id),
        "real wire attack produced the lethal queue"
    );
    assert_eq!(
        super::grizzly_npc::value(owner, authority, counter).await?,
        0,
        "no credit before durable drain"
    );
    assert_eq!(
        owner.runtime.lock().await.live_creature_health(target),
        None
    );
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        super::grizzly_npc::value(owner, authority, counter).await?,
        1
    );
    let replay = super::quest_world_callbacks::wire(owner, admitted, 6, 11, payload).await?;
    assert_eq!(replay, result);
    for _ in 0..2 {
        owner.drain_auto_attacks().await;
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            super::grizzly_npc::value(owner, authority, counter).await?,
            1
        );
    }
    assert!(!queue.has_session(admitted.game_session_id));
    Ok(())
}

async fn combat_level_fixture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let target = binding
        .policy
        .thresholds
        .iter()
        .find(|row| row.level == 130)
        .ok_or("level 130")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression")?;
    let amount = target
        .checked_sub(before.total_experience.get())
        .filter(|amount| *amount > 0)
        .ok_or("level eligibility XP")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(239)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(amount),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, ExperienceCommitOutcome::Committed(_)),
        "{outcome:?}"
    );
    // The helper called the durable XP owner directly. Notify the normal live
    // refresh owner, which reloads level/vocation facts before NPC admission.
    owner.states.level_refresh.mark([fence.character_id]);
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    owner
        .refresh_level_up_vitals(actor, admitted.game_session_id)
        .await;
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    assert_eq!(
        states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("level fixture cast state")?
            .facts
            .level,
        130
    );
    Ok(())
}
