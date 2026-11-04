//! P02: equipped bosstiary slot bonus on the production native-kill corpse path.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::bosstiary_slots::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const KNEEBITER: &str = "oteryn:creature.foreman_kneebiter";

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    request: BossSlotRequest,
) -> TestResult<BossSlotReply> {
    let payload = request.encode().ok_or("slot request")?;
    let command_id = admitted.continuity.next_command_id;
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_BOSSTIARY_SLOTS,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(bytes.len())?.to_be_bytes())
        .await?;
    client.write_all(&bytes).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(next)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(next, _) => {
            *admitted = next;
        }
        _ => return Err("connection".into()),
    }
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])
                .map_err(debug)?;
        cursor += 4 + length;
        if envelope.message_type() == crate::foundation::MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            return BossSlotReply::decode(result.payload).ok_or("slot reply".into());
        }
    }
    Err("no slot result".into())
}

async fn native_kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    source_id: u16,
    position: MovementLocalPosition,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native gameplay")?;
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == KNEEBITER)
        .ok_or("compiled kneebiter companion policy")?;
    native.reward_table().row(KNEEBITER).map_err(debug)?;
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    let now = owner.owner_now();
    let receipt = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        runtime
            .realize_native_qualification_spawn_for_source(source_id, pin, KNEEBITER, position)
            .map_err(debug)?;
        let boss = runtime
            .visible_entities()
            .creatures
            .last()
            .ok_or("source-qualified kneebiter")?
            .actor;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(u64::from(source_id) + 90).map_err(debug)?,
            ),
            occurrence: SpellOccurrenceBinding {
                id: format!("p02:kneebiter:{source_id}"),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: format!("p02-kneebiter-{source_id}").into_bytes(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: boss,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                    magnitude: policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| {
            effect.target == boss && effect.health.is_some_and(|health| health.health_after == 0)
        }));
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        receipt
    };
    assert!(receipt.applied);
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    Ok(())
}

async fn item_count(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances i \
         JOIN game_item_corpse_container_entries e USING(item_instance_id) \
         WHERE i.definition_production_key=$1",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?)
}

async fn seed_exact_1250(h: &Harness) -> TestResult<String> {
    let bosses = crate::gameplay_transport::charm_progression::canonical_bosses()
        .ok_or("canonical bosses")?;
    let mut candidates = bosses
        .iter()
        .filter(|boss| boss.key != KNEEBITER && boss.category == "nemesis");
    let mut equipped_other = None;
    for candidate in candidates.by_ref().take(12) {
        sqlx::query("INSERT INTO game_character_bosstiary_kill_receipts(character_id,occurrence_id,boss_key,category) SELECT encode($1,'hex')::uuid,game_character_uuid_v7(),$2,$3 FROM generate_series(1::bigint,5::bigint)")
            .bind(id(41).as_slice()).bind(&candidate.key).bind(&candidate.category)
            .execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_bosstiary_progress(character_id,boss_key,kill_count,boss_points) VALUES(encode($1,'hex')::uuid,$2,5,100)")
            .bind(id(41).as_slice()).bind(&candidate.key).execute(&h.pool).await?;
        if equipped_other.is_none() {
            equipped_other = Some(candidate.key.clone());
        }
    }
    let expertise = candidates.next().ok_or("thirteenth nemesis boss")?;
    sqlx::query("INSERT INTO game_character_bosstiary_kill_receipts(character_id,occurrence_id,boss_key,category) SELECT encode($1,'hex')::uuid,game_character_uuid_v7(),$2,$3 FROM generate_series(1::bigint,3::bigint)")
        .bind(id(41).as_slice()).bind(&expertise.key).bind(&expertise.category)
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bosstiary_progress(character_id,boss_key,kill_count,boss_points) VALUES(encode($1,'hex')::uuid,$2,3,40)")
        .bind(id(41).as_slice()).bind(&expertise.key).execute(&h.pool).await?;
    let points: i64 = sqlx::query_scalar(
        "SELECT sum(boss_points)::bigint FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(points, 1250);
    equipped_other.ok_or("seeded nemesis boss".into())
}

#[test]
fn bosstiary_slot_loot_bonus_adds_exact_equipment_roll_to_native_kill_postgres() -> TestResult {
    run_case(true)
}

#[test]
fn bosstiary_slot_loot_bonus_unassigned_boss_gets_no_extra_roll_postgres() -> TestResult {
    run_case(false)
}

fn run_case(assigned: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(assigned).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "P02 slot loot scenario panicked")??;
    Ok(())
}

fn scenario(assigned: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "p02_slot_loot", true, false).await?;
        prepare_current_session(&h).await?;
        super::charm_progression::seed(&h, &[]).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, source_pin) =
            crate::content::native_gameplay::test_support::activated_boss_slot_loot_fixture(
                world, None,
            );
        let room = source_pin.qualified_room().clone();
        let pin = source_pin.into_channel_pin();
        let deps = Dependencies::with_content_and_capacity(
            pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("fence")?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[117])
                .ok_or("boss capability")?;

        native_kill(
            &owner,
            &admitted,
            fence.item_fence(),
            1,
            MovementLocalPosition {
                x: 3,
                y: 0,
                floor: 7,
            },
        )
        .await?;
        assert_eq!(item_count(&h, "oteryn:item.tibia.i3051").await?, 1);
        assert_eq!(item_count(&h, "oteryn:item.tibia.i3031").await?, 1);
        let progress: (i64, i64) = sqlx::query_as("SELECT kill_count,boss_points FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2")
            .bind(id(41).as_slice()).bind(KNEEBITER).fetch_one(&h.pool).await?;
        assert_eq!(
            progress,
            (1, 10),
            "production drain credits kneebiter progress"
        );

        let other = seed_exact_1250(&h).await?;
        let equipped = exchange(
            &owner,
            &mut admitted,
            BossSlotRequest::Equip {
                slot: 1,
                boss: if assigned {
                    KNEEBITER.to_owned()
                } else {
                    other
                },
            },
        )
        .await?;
        assert_eq!(equipped.status, 0);

        native_kill(
            &owner,
            &admitted,
            fence.item_fence(),
            2,
            MovementLocalPosition {
                x: 4,
                y: 0,
                floor: 7,
            },
        )
        .await?;
        assert_eq!(
            item_count(&h, "oteryn:item.tibia.i3051").await?,
            if assigned { 3 } else { 2 }
        );
        assert_eq!(
            item_count(&h, "oteryn:item.tibia.i3031").await?,
            2,
            "currency remains one roll per death"
        );
        Ok(())
    })
}
