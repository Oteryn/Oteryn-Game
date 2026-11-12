//! S3: lethal PvE damage while grouped, carrying a container and retaining Prey/conditions.
use super::*;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, ExactI64, GameplayDecisionRoot};

#[test]
fn s3_death_items_blessings_build_party_prey_respawn_and_relogin() -> TestResult {
    timed("s3-death", scenario)
}

fn scenario() -> TestResult {
    world_scenario!("xmech_s3", |h,
                                 request,
                                 seal,
                                 authority,
                                 world_id,
                                 native,
                                 world,
                                 cells,
                                 room,
                                 pin,
                                 deps,
                                 actor,
                                 session,
                                 owner,
                                 fence,
                                 admitted| {
        let other = peer(&h, &mut owner, admitted).await?;
        join_party(&h, &owner, admitted, other).await?;
        let formula = native.training_formula().ok_or("training formula")?;
        let before = h
            .root
            .read_character_build_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let druid = convert_vocation(formula, &before, "druid").map_err(debug)?;
        let mut skills = druid.skills();
        skills[2] = (14, 1);
        let trained = DurableBuildState::new("druid", (3, 1), skills).map_err(debug)?;
        for (seed, cause, before, after) in [
            (130, BuildCause::VocationChoice, before, druid.clone()),
            (131, BuildCause::Training, druid, trained.clone()),
        ] {
            let current = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            owner
                .revision_sequencer
                .acquire(fence.character_id)
                .await
                .commit_build(
                    owner.root,
                    owner.character,
                    owner.holder,
                    current,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(id(seed)).map_err(debug)?,
                        cause,
                        before,
                        after,
                        pruned_stance: None,
                    },
                    formula,
                    None,
                )
                .await
                .map_err(debug)?;
        }
        assert_eq!(
            h.root
                .read_character_build_state(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            trained
        );
        let binding = owner.progression_binding(session).ok_or("progression")?;
        let current = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                current,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(132)).map_err(debug)?,
                    amount: ExactI64::new(98_800),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        owner.revision_sequencer =
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new()
                .into();
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let mut state = crate::spell::cast::PlayerSpellState::new(
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Druid,
                    level: 20,
                    magic_level: 3,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                0,
                0,
            )
            .ok_or("druid state")?;
            state.make_playable(owner.owner_now()).map_err(debug)?;
            *states.get_mut(&runtime, actor, session).ok_or("state")? = state;
        }
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'spiritual_shielding','fixture')").bind(id(41).as_slice()).execute(&mut *seed).await?;
        seed.commit().await?;
        // Red skull makes container loss deterministic; this remains a PvE lethal hit.
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,clock_timestamp()+interval '1 day')").bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,creature_list,hunting_s_left,slot_epoch) VALUES(encode($1,'hex')::uuid,1,'SELECTION',ARRAY['oteryn:creature.rat'],0,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &crate::gameplay_transport::prey::offered(owner.offered_capabilities(), true),
            &[6, 140],
        )
        .ok_or("prey capability")?;
        let select = oteryn_protocol_oteryn::prey::encode_request(
            &oteryn_protocol_oteryn::prey::Request::Select { slot: 1, index: 1 },
        )
        .map_err(debug)?;
        let (status, bytes) = exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::prey::COMMAND_TYPE_PREY,
            &select,
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert_eq!(
            oteryn_protocol_oteryn::prey::decode_reply(&bytes)
                .map_err(debug)?
                .slots[0]
                .state,
            2
        );
        let prey_before:String=sqlx::query_scalar("SELECT to_jsonb(p)::text FROM game_character_prey_slots p WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let items_before = count(
            &h,
            "SELECT count(*) FROM game_item_instances WHERE lifecycle=1",
        )
        .await?;
        assert_eq!(items_before, 2);
        let pack:Vec<u8>=sqlx::query_scalar("SELECT decode(replace(item_instance_id::text,'-',''),'hex') FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let child_before = backpack_contents_snapshot(&h, &pack).await?;
        assert_eq!(child_before.len(), 1);
        step_west(&mut owner, actor, session).await?;
        let temple = owner.home_temple(session).await.ok_or("temple")?;
        assert_ne!(position(&owner, actor).await?, temple);
        let death = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(states.load_owned_build(&runtime, actor, session, &trained));
            let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
            state
                .enable_owned_training(&trained, formula)
                .map_err(debug)?;
            let definition = crate::foundation::ConditionDefinition::new_damage_schedule(
                "fixture.xmech.poison",
                1,
                crate::foundation::DotElement::Poison,
                crate::foundation::DamageSchedule::Fixed {
                    delayed: true,
                    segments: vec![crate::foundation::DamageSegment {
                        count: 3,
                        interval_ms: 1000,
                        amount: 1,
                    }],
                },
            )
            .ok_or("poison")?;
            let root = GameplayDecisionRoot::from_bytes([111; 32]);
            let facts = crate::foundation::ApplicationFacts {
                now: owner.owner_now().get(),
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes(id(133)),
            };
            let next = state
                .stage_owned_test_conditions(&[definition], &facts)
                .ok_or("condition")?;
            assert!(states.commit(&runtime, actor, session, next));
            assert!(
                !states
                    .get(&runtime, actor, session)
                    .ok_or("state")?
                    .owned_conditions()
                    .instances()
                    .is_empty()
            );
            let hit = states
                .apply_creature_damage_native(
                    &runtime,
                    actor,
                    session,
                    100_000,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        owner.owner_now().get(),
                    ),
                )
                .ok_or("lethal PvE hit")?;
            assert_eq!(hit.damage.health_after, 0);
            states
                .player_death(&runtime, actor, session)
                .ok_or("death")?
        };
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_character_death_receipts").await?,
            0
        );
        let (_, vitals) = owner
            .respawn_after_death(actor, session, death)
            .await
            .ok_or("durable death and temple placement")?;
        assert_eq!(vitals.health, vitals.max_health);
        let receipt = h
            .root
            .reconcile_character_death(&authority, death.occurrence)
            .await
            .map_err(debug)?
            .ok_or("receipt")?;
        assert_eq!(receipt.experience_before.get(), 98_800);
        assert_eq!(
            receipt.experience_after.get(),
            98_800 - receipt.experience_lost.get()
        );
        assert_eq!(receipt.experience_lost.get(), 15_824);
        assert!(
            receipt.build_loss.is_none(),
            "current rule preserves trained skills on death"
        );
        assert!(receipt.blessings_after.is_empty());
        assert_eq!(
            h.root
                .read_character_build_state(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            trained
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_character_blessings").await?,
            0
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_character_pending_respawns").await?,
            0
        );
        let lost = receipt.items.as_ref().ok_or("loss selection")?;
        assert_eq!(lost.lost, vec![<[u8; 16]>::try_from(pack.as_slice())?]);
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_player_death_item_records").await?,
            1
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_item_ground_locations").await?,
            1
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_item_container_slots").await?,
            0
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_item_container_entries").await?,
            0,
            "death normalizes the selected backpack's retained custody"
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_item_nested_entries").await?,
            1,
            "child follows the dropped backpack by identity"
        );
        assert_eq!(
            backpack_contents_snapshot(&h, &pack).await?,
            child_before,
            "death preserves the child ItemInstance and exact logical placement"
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_item_instances WHERE lifecycle=1"
            )
            .await?,
            items_before
        );
        assert_eq!(position(&owner, actor).await?, temple);
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("respawned state")?
                    .owned_conditions()
                    .instances()
                    .is_empty()
            );
        }
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_party_members").await?,
            2,
            "PvE death retains party membership"
        );
        assert_eq!(sqlx::query_scalar::<_,String>("SELECT to_jsonb(p)::text FROM game_character_prey_slots p WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?,prey_before,"death does not reroll Prey");
        let after = snapshot(&h).await?;
        owner.respawn_after_death(actor, session, death).await;
        assert_eq!(
            snapshot(&h).await?,
            after,
            "replayed death never drops or charges twice"
        );
        assert_eq!(
            backpack_contents_snapshot(&h, &pack).await?,
            child_before,
            "death replay preserves the same child and placement provenance"
        );
        logout_relogin(&h, &mut owner, admitted).await?;
        assert_eq!(
            h.root
                .read_character_build_state(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            trained
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_party_members").await?,
            0
        );
        assert_eq!(
            count(
                &h,
                "SELECT chosen_kills FROM game_character_prey_slots WHERE slot=1"
            )
            .await?,
            0
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_character_prey_slots WHERE state='ACTIVE'"
            )
            .await?,
            1
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_character_death_receipts").await?,
            1
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_player_death_item_records").await?,
            1
        );
        drop(owner);
        drop(authority);
        drop(seal);
        drop(request);
        h.cleanup().await
    })
}

async fn backpack_contents_snapshot(
    h: &Harness,
    parent: &[u8],
) -> TestResult<Vec<(String, String)>> {
    Ok(sqlx::query_as("SELECT to_jsonb(i)::text,e.data::text FROM game_item_instances i JOIN (SELECT item_instance_id,to_jsonb(e)-'character_id' AS data FROM game_item_container_entries e WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT item_instance_id,to_jsonb(n) FROM game_item_nested_entries n WHERE parent_item_instance_id=encode($1,'hex')::uuid) e USING(item_instance_id) ORDER BY e.data::text LIMIT 2")
        .bind(parent).fetch_all(&h.pool).await?)
}
