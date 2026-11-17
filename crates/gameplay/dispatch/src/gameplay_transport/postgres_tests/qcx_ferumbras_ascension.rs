use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, world};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::starter_grant::StarterTemplate;
use crate::foundation::{ChannelRuntimeV1, CharacterId, CharacterLease};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const CHEST: Position = Position {
    x: 33388,
    y: 31409,
    floor: -14,
};
const QUEST: &str = "oteryn:quest.ferumbras_ascension_quest";

#[test]
fn reward_chest_grants_the_complete_bundle_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx36-ferumbras-reward", chest_case);
    eprintln!(
        "QCX36 reward_chest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn chest_case() -> TestResult {
    let trigger = CHEST;
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("questpc7-ferumbras-reward"))
        .ok_or("Ferumbras reward callback")?;
    let world = world(&callback, trigger)?;
    run(async |admin| {
        let h = Harness::create_quest_reward(admin, "qcx36_chest", true).await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("QCX36 reward_chest PostgreSQL {version}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reserved = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        let premium = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), deps.admitted.game_session_id);
        owner.active_generation = Some(generation);
        owner.movement_cells =
            crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "current quest fence")?
            .ok_or("current quest fence")?;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &NativeItemWeights(native),
                &StarterTemplate::main(),
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        raise_level(&owner, &admitted).await?;
        owner.states.level_refresh.mark([fence.character_id]);
        Box::pin(owner.refresh_level_up_vitals(deps.actor, admitted.game_session_id)).await;
        // Set the earned seal and passage state so this case exercises just the chest.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (suffix, value) in [("programme", 2_i64), ("passages", 8), ("seal_7", 1)] {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$1,$2,$3,character_revision FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
                .bind(format!("oteryn:quest-progress/questpc7/ferumbras/{suffix}"))
                .bind(QUEST).bind(value).bind(id(41).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        owner.admit_quest_session(&admitted).await;

        let use_chest = super::questpc7_ferumbras::use_at;
        assert_eq!(
            use_chest(&owner, &mut admitted, CHEST, 4073).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            use_chest(&owner, &mut admitted, CHEST, 4073).await?,
            UseDisposition::Rejected
        );
        for (item, count) in [(22516, 5), (22731, 4), (22737, 4), (22775, 1), (3043, 10)] {
            let quantity: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(child.quantity),0)::bigint FROM game_item_instances child JOIN game_item_nested_entries e ON e.item_instance_id=child.item_instance_id JOIN game_item_instances bag ON bag.item_instance_id=e.parent_item_instance_id WHERE child.lifecycle=1 AND bag.lifecycle=1 AND child.definition_production_key=$1 AND bag.definition_production_key='oteryn:item.tibia.i2853'"
            ).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
            assert_eq!(quantity, count, "reward bag item {item}");
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn raise_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let xp = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == 150)
        .ok_or("level threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let current = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression state")?;
    let amount = xp.checked_sub(current.total_experience.get()).ok_or("XP")?;
    if amount > 0 {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(201)).map_err(debug)?,
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
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    }
    Ok(())
}
