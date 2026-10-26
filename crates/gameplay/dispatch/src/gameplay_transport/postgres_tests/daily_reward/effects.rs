//! Continue the actual daily claim with a new composed owner and real monster kills.
use super::*;
use crate::foundation::CharacterLease;
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) async fn kills_before_and_at_expiry(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (generation, pin) =
        super::super::death_ml_loss_composed::production_generation_with_source(
            world,
            "content-1",
            Some(include_bytes!(
                "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        )?;
    let active = generation.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native")?;
    let room = pin.qualified_room().clone();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
    );
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.activation_sequence().to_string())
        .bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    // Each pass reconstructs runtime/session owners from durable authority. The second
    // pass observes the exact expiry instant on the same server clock, without waiting.
    for expired in [false, true] {
        if expired {
            sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS timestamptz LANGUAGE sql VOLATILE AS $$ SELECT max(effect_expires_at) FROM public.game_character_daily_reward_lines WHERE kind='XP_BOOST' $$")
                .execute(&h.pool).await?;
        }
        let mut deps = Dependencies::with_content(
            channel_pin.clone(),
            room.clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.combat_room = Some(room.clone());
        {
            let mut runtime = deps.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let mut sources = Vec::new();
            for (key, y) in [
                ("oteryn:creature.cave_rat", 12),
                ("oteryn:creature.rat", 11),
            ] {
                if y == 12 && !expired {
                    continue;
                }
                let policy = runtime.companion_policy(key).map_err(debug)?;
                sources.push((
                    vec![(1, y, -7)],
                    60_000,
                    5_000,
                    key.to_owned(),
                    u64::try_from(policy.maximum_health)?,
                ));
            }
            runtime.realize_activation_spawns(sources).map_err(debug)?;
        }
        let mut owner = deps.owner(h, authority);
        owner.active_generation = Some(active);
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .ok_or("clock")?;
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let actor = deps.actor;
        let session = admitted.game_session_id;
        let rat = {
            let runtime = owner.runtime.lock().await;
            let visible = runtime.visible_entities();
            let creature = visible
                .creatures
                .iter()
                .find(|c| {
                    runtime
                        .companion_snapshot(c.actor)
                        .is_ok_and(|s| s.state.policy.definition_key == "oteryn:creature.rat")
                })
                .ok_or("rat")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    owner.rooms.gate_auto_attack(),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(rat),
                    30
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        for pass in 0..256 {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
            assert!(pass < 255, "real rat kill");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .ok_or("swing clock")?;
        }
        for pass in 0..8 {
            if owner.drain_kill_rewards(session).await == DrainOutcome::Drained {
                break;
            }
            assert!(pass < 7, "kill settles");
        }
        let xp: i64 =
            sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            xp,
            if expired { 12 } else { 7 },
            "daily boost changes real rat XP from 5 to 7, then expires"
        );
    }
    Ok(())
}
