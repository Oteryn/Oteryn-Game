//! EQMOD2: content -> fenced equipment -> framed attack intent -> real combat owners.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id};
use crate::foundation::{ExactActorRef, GameSessionId, WorldId};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::*};
use oteryn_protocol_oteryn::attack::{ActorCombatState, AttackIntentDisposition};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn eqmod2_postgres_content_cleave_reflection_perfect_shot_and_amplification() -> TestResult {
    party_xp::run(Case::Eqmod2)
}

pub(super) fn activate(
    world: WorldId,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use crate::content::*;
    let mut input = native_gameplay::NativeGameplayInput::from_manifest(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json"),
    )?;
    let world_bytes = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    );
    input.source_world = Some(native_gameplay::PinnedGameplayBytes {
        bytes: world_bytes.to_vec(),
        sha256: hex(&Sha256::digest(world_bytes)),
    });
    let pin = input.item_profiles.as_mut().ok_or("item profiles")?;
    let mut document: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let boots = document["records"]
        .as_array_mut()
        .ok_or("records")?
        .iter_mut()
        .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i6529")
        .ok_or("boots")?;
    let modifier = |kind: &str, shape: &str, value: serde_json::Value| {
        serde_json::json!({
            "kind": kind, "parameter": {"state":"KNOWN", "value":{"kind":shape,"value":value}},
            "target_domain":{"state":"UNKNOWN"}, "evaluation_phase":{"state":"UNKNOWN"}, "priority":{"state":"UNKNOWN"}
        })
    };
    let percent = |kind, n| {
        modifier(
            kind,
            "RATIONAL_PERCENT",
            serde_json::json!({"numerator":n,"denominator":1}),
        )
    };
    boots["semantics"]["skill_modifiers"] = serde_json::json!({"state":"KNOWN","value":{"modifiers":{"state":"KNOWN","value":[
        percent("CLEAVE_PERCENT", 100),
        percent("CRITICAL_HIT_CHANCE", 100), percent("CRITICAL_HIT_DAMAGE", 100),
        percent("LIFE_LEECH_AMOUNT", 50), percent("MANA_LEECH_AMOUNT", 50),
        modifier("PERFECT_SHOT_DAMAGE", "SIGNED_POINTS", serde_json::json!(42)),
        modifier("PERFECT_SHOT_RANGE", "CELLS", serde_json::json!(1)),
        modifier("REFLECT_DAMAGE", "SIGNED_POINTS", serde_json::json!(99)),
        percent("REFLECT_PERCENT_ALL", 10)
    ]}}});
    for record in document["records"].as_array_mut().ok_or("records")? {
        if ["oteryn:item.tibia.i3264", "oteryn:item.tibia.i6529"]
            .iter()
            .any(|key| record["production_definition"]["production_key"] == *key)
        {
            record["authoring"]["forge"] = serde_json::json!({"classification":4,"max_tier":10});
        }
    }
    pin.bytes = serde_json::to_vec(&document)?;
    pin.sha256 = hex(&Sha256::digest(&pin.bytes));
    // An unfleeing source lets the last phase reach a lethal reflected hit rather
    // than changing this mechanic's test into a fleeing/pathfinding qualification.
    let pin = &mut input.creature_profiles;
    let mut creatures: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    for record in creatures["records"].as_array_mut().ok_or("creatures")? {
        if record["profile"]["target"]["key"] == "oteryn:creature.cave_rat" {
            record["behavior"]["data"]["profile"]["targeting"]["flee_health"] =
                serde_json::json!(0);
        }
    }
    pin.bytes = serde_json::to_vec(&creatures)?;
    pin.sha256 = hex(&Sha256::digest(&pin.bytes));
    let room = qualify_selected_native_gameplay_room(world, &input)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    let pin = activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &input,
    )
    .map_err(debug)?;
    Ok((controller, pin))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    let character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?
        .character_id();
    // Test custody provisioning only. All tested reads, installation and damage use real owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, slot, tier) in [
        (112_u8, "oteryn:item.tibia.i3264", 6_i16, 10_i16),
        (114, "oteryn:item.tibia.i6529", 4, 10),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(character.as_bytes().as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
        if tier > 0 {
            sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,$2)").bind(id(seed).as_slice()).bind(tier).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    let effects = owner
        .read_admission_effects(session)
        .await
        .map_err(|_| "effects")?
        .ok_or("effects")?;
    assert_eq!(
        (
            effects.4.cleave,
            effects.4.reflect_percent,
            effects.4.reflect_flat
        ),
        (10_000, 1_000, 99)
    );
    assert_eq!((effects.4.weapon_tier, effects.4.boots_tier), (10, 10));
    assert_eq!(effects.4.perfect_damage(1, 0), 42);
    let runtime = owner.runtime.lock().await;
    owner
        .install_admission_suppressions(&runtime, actor, session, Some(effects))
        .await
        .ok_or("install")?;
    let creatures = runtime.visible_entities().creatures;
    let primary = creatures
        .iter()
        .find(|row| {
            runtime
                .read_actor_position(row.actor)
                .is_ok_and(|p| p.position().y == 11)
        })
        .ok_or("primary")?;
    let flank = creatures
        .iter()
        .find(|row| {
            runtime
                .read_actor_position(row.actor)
                .is_ok_and(|p| p.position().y == 12)
        })
        .ok_or("flank")?
        .actor;
    let target = EntityRef {
        identity: primary.actor.placement_identity(),
        generation: primary.generation,
    };
    let primary = primary.actor;
    drop(runtime);
    framed_target(owner, actor, session, target).await?;
    let mut cleaves = 0;
    let mut hits = 0;
    for _ in 0..32 {
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .set_test_creature_health(primary, 10_000)
                .map_err(debug)?;
            runtime
                .set_test_creature_health(flank, 10_000)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
            state.restore_saved_vitals(10, 0);
        }
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        let runtime = owner.runtime.lock().await;
        let real = 10_000
            - runtime
                .live_creature_health(primary)
                .ok_or("primary health")?;
        let cleave = 10_000 - runtime.live_creature_health(flank).ok_or("flank health")?;
        cleaves += u32::from(cleave > 0);
        hits += u32::from(real > 0);
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, session).ok_or("state")?;
        let leech = u32::try_from(real.div_ceil(2))?;
        assert_eq!(
            state.vitals().health,
            (10 + leech).min(state.facts.max_health),
            "only the primary leeches; cleave={cleave}"
        );
        assert_eq!(state.vitals().mana, leech.min(state.facts.max_mana));
        drop(states);
        drop(runtime);
        owner.clock_origin -= Duration::from_secs(3);
    }
    assert!(
        cleaves > 2 && hits > 2,
        "independent real cleave/primary hits: {cleaves}/{hits}"
    );

    // The same source owner that executes real rat attacks must reflect, including while
    // mana shield absorbs the player's damage. No player auto-attacks run in this phase.
    let (primary_maximum, flank_maximum) = {
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        // The shared fixture supplies two players. Retire the unused player's vitals
        // through the real owner so monster target selection has one eligible player.
        crate::ability::creature_bite::CreatureBiteVitals::apply_creature_damage(
            &mut *states,
            &mut runtime,
            players[1].0,
            players[1].1,
            u32::MAX,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                owner.owner_now().get(),
            ),
        )
        .ok_or("unused player death")?;
        (
            runtime
                .companion_snapshot(primary)
                .map_err(debug)?
                .maximum_health,
            runtime
                .companion_snapshot(flank)
                .map_err(debug)?
                .maximum_health,
        )
    };
    let mut reflected = 0;
    let mut shielded = false;
    for _ in 0..32 {
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .set_test_creature_health(primary, primary_maximum)
                .map_err(debug)?;
            runtime
                .set_test_creature_health(flank, flank_maximum)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
            state.restore_saved_vitals(state.facts.max_health, state.facts.max_mana);
            state.install_equipped_mana_shield(true);
        }
        owner.drain_monster_melee().await;
        let runtime = owner.runtime.lock().await;
        reflected += u64::try_from(primary_maximum)?
            - runtime
                .live_creature_health(primary)
                .ok_or("reflected health")?
            + u64::try_from(flank_maximum)?
            - runtime
                .live_creature_health(flank)
                .ok_or("reflected flank health")?;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, session).ok_or("shield state")?;
        assert_eq!(
            state.vitals().health,
            state.facts.max_health,
            "reflection precedes mana shield"
        );
        shielded |= state.vitals().mana < state.facts.max_mana;
        drop(states);
        drop(runtime);
        owner.clock_origin -= Duration::from_secs(3);
    }
    assert!(reflected > 0, "real monster dispatch reflects damage");
    assert!(shielded, "the same real hits also spend mana shield mana");

    // More than the carrier's 16 retained receipts must continue to reflect. Exact
    // replays do not credit HP twice; an evicted old source clock remains fenced.
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .set_test_creature_health(primary, 10_000)
            .map_err(debug)?;
        let clock = owner.owner_now().get();
        let before = runtime
            .borrow_combat_death()
            .reward_damage_shares(primary)
            .map_err(debug)?
            .into_iter()
            .find(|share| share.character == character)
            .ok_or("contributor")?
            .total;
        for sequence in clock..clock + 24 {
            let hit = runtime
                .commit_equipment_reflection(primary, actor, session, b"eqmod2-replay", sequence, 1)
                .map_err(debug)?;
            assert!(hit.applied);
            let replay = runtime
                .commit_equipment_reflection(primary, actor, session, b"eqmod2-replay", sequence, 1)
                .map_err(debug)?;
            assert!(!replay.applied);
            assert_eq!(replay.health_after, hit.health_after);
            assert!(
                runtime
                    .commit_equipment_reflection(
                        primary,
                        actor,
                        session,
                        b"eqmod2-replay",
                        sequence,
                        2
                    )
                    .is_err()
            );
        }
        assert!(
            runtime
                .commit_equipment_reflection(primary, actor, session, b"eqmod2-replay", clock, 1)
                .is_err()
        );
        let shares = runtime
            .borrow_combat_death()
            .reward_damage_shares(primary)
            .map_err(debug)?;
        let share = shares
            .iter()
            .find(|share| share.character == character)
            .ok_or("contributor")?;
        assert_eq!(share.total - before, 24);
        assert_eq!(share.session, Some(session));
    }

    // Switch the actual equipped item to a throwing weapon: exact range forces a hit and
    // adds 42 after blocking. The native ranged owner pays the normal projectile cost.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i3277',quantity=100 WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(112).as_slice()).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_item_tiers WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(112).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let effects = owner
        .read_admission_effects(session)
        .await
        .map_err(|_| "ranged effects")?;
    let runtime = owner.runtime.lock().await;
    owner
        .install_admission_suppressions(&runtime, actor, session, effects)
        .await
        .ok_or("ranged install")?;
    drop(runtime);
    for _ in 0..8 {
        owner
            .runtime
            .lock()
            .await
            .set_test_creature_health(primary, 10_000)
            .map_err(debug)?;
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        let health = owner
            .runtime
            .lock()
            .await
            .live_creature_health(primary)
            .ok_or("perfect health")?;
        assert!(
            10_000 - health >= 42,
            "perfect shot forces hit and adds flat after blocking: {health}"
        );
        owner.clock_origin -= Duration::from_secs(3);
    }
    // A reflected lethal hit must use the existing durable XP queue once. The
    // primary target remains alive and no player swings run during this phase.
    owner
        .runtime
        .lock()
        .await
        .set_test_creature_health(flank, 1)
        .map_err(debug)?;
    let receipts_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&h.pool)
            .await?;
    for _ in 0..16 {
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("lethal state")?;
            state.restore_saved_vitals(state.facts.max_health, state.facts.max_mana);
            state.install_equipped_mana_shield(true);
        }
        owner.drain_monster_melee().await;
        if owner
            .runtime
            .lock()
            .await
            .live_creature_health(flank)
            .is_none()
        {
            break;
        }
        owner.clock_origin -= Duration::from_secs(3);
    }
    assert!(
        owner
            .runtime
            .lock()
            .await
            .live_creature_health(flank)
            .is_none(),
        "reflection kills the live source"
    );
    assert!(
        owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session),
        "reflected death is queued for the defender"
    );
    for _ in 0..8 {
        if owner.drain_kill_rewards(session).await
            == crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        {
            break;
        }
    }
    assert!(
        !owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session),
        "reflected kill settles durably"
    );
    let receipts_after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert!(
        receipts_after > receipts_before,
        "reflection credits the defender's current lease/session"
    );
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    let replayed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        replayed, receipts_after,
        "a reward drain cannot award the reflected kill twice"
    );
    Ok(())
}

struct WireOwner<'a, 'b, 'c, 'd>(&'a ComposedFreshAdmission<'b, 'c, 'd>);
impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.0.observe(actor).await
    }
    async fn observe_vitals(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, oteryn_protocol_oteryn::actor_spell::ActorVitals)> {
        self.0.observe_vitals(actor, session).await
    }
    async fn observe_combat_state(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<ActorCombatState> {
        self.0.observe_combat_state(actor, session).await
    }
    async fn attack_target(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command: u64,
        target: Option<EntityRef>,
    ) -> AttackIntentDisposition {
        self.0.attack_target(actor, session, command, target).await
    }
}

async fn framed_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    target: EntityRef,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{
        PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    let binding = owner.runtime.lock().await.binding();
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: binding.world_id(),
        channel_id: binding.channel_id(),
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        item_fence: None,
        continuity: SessionContinuity {
            next_command_id: 30,
            selected_capabilities: SelectedCapabilities::select(
                PRODUCTION_OFFERED_CAPABILITIES,
                &[6, 17],
            )
            .ok_or("capabilities")?,
            ..SessionContinuity::FRESH
        },
    };
    let payload = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&target))
        .map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: 30,
            command_type: 11,
            payload: &payload,
        },
    )?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, &WireOwner(owner), IDLE_LIVENESS).await;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut found = false;
    let mut cursor = 0;
    while cursor < bytes.len() {
        let len = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
        let frame =
            oteryn_protocol_oteryn::decode_wire_envelope(&bytes[cursor + 4..cursor + 4 + len])?;
        if frame.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            assert_eq!(
                result.payload,
                oteryn_protocol_oteryn::attack::encode_attack_intent_result(
                    AttackIntentDisposition::Ok
                )
            );
            found = true;
        }
        cursor += 4 + len;
    }
    assert!(found, "framed attack intent dispatched");
    Ok(())
}

#[test]
fn eqmod2_content_forge_tiers_and_modifiers_are_projected() -> TestResult {
    use crate::durability::character_equipment::{EquipmentSnapshot, EquippedItem};
    use crate::durability::item_mint::TypedDefinitionRef;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = activate(world)?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let mut equipment = EquipmentSnapshot {
        character: id(41),
        content_digest: native.source_digest(),
        character_revision: 1,
        revision: 1,
        combat_mode: None,
        items: [(6, "3264", 10), (4, "6529", 10)]
            .into_iter()
            .map(|(slot, key, tier)| EquippedItem {
                slot,
                tier,
                item_instance_id: id(slot),
                quantity: 1,
                state_revision: 1,
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: format!("oteryn:item.tibia.i{key}"),
                    revision_ref: "definition-r1".into(),
                },
            })
            .collect(),
    };
    let offence =
        crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_offence(
            &equipment, native,
        )
        .ok_or("offence")?;
    assert_eq!(
        (offence.weapon_tier, offence.boots_tier, offence.cleave),
        (10, 10, 10_000)
    );
    assert_eq!(
        (
            offence.critical_chance,
            offence.life_leech,
            offence.reflect_percent,
            offence.reflect_flat
        ),
        (10_000, 5_000, 1_000, 99)
    );
    assert_eq!(offence.perfect_damage(1, 0), 42);
    equipment.items[1].tier = 11;
    assert!(
        crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_offence(
            &equipment, native
        )
        .is_none()
    );
    Ok(())
}
