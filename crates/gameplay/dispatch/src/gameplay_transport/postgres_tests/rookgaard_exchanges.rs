//! R–S repeatable NPC exchanges through production NPC_TALK and the runtime database role.
use super::fixture::{Harness, debug, id, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct Case {
    npc: &'static str,
    keyword: &'static str,
    input: &'static str,
    output: &'static str,
    confirmation: bool,
    answer: &'static str,
    success: &'static str,
}

fn execute(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Rookgaard NPC exchange scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED rookgaard npc={} duration_ms={}",
        case.npc,
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn rookgaard_short_sword_amber_exchange_atomic_repeatable_replay() -> TestResult {
    execute(Case {
        npc: "oteryn:npc.amber",
        keyword: "notebook",
        answer: "yes, notebook",
        input: "oteryn:item.tibia.i2821",
        output: "oteryn:item.tibia.i3294",
        confirmation: true,
        success: "Excellent. Here, take this short sword as a reward.",
    })
}
#[test]
fn rookgaard_small_health_potion_billy_exchange_atomic_repeatable_replay() -> TestResult {
    execute(Case {
        npc: "oteryn:npc.billy",
        keyword: "pan",
        answer: "yes, pan",
        input: "oteryn:item.tibia.i3466",
        output: "oteryn:item.tibia.i7876",
        confirmation: true,
        success: "A pan! At last! Take this in case you eat something my cousin has cooked.",
    })
}
#[test]
fn rookgaard_studded_shield_willie_exchange_atomic_repeatable_replay() -> TestResult {
    execute(Case {
        npc: "oteryn:npc.willie",
        keyword: "banana",
        answer: "yes, banana",
        input: "oteryn:item.tibia.i3587",
        output: "oteryn:item.tibia.i3426",
        confirmation: true,
        success: "A banana! Great. Here, take this shield, I don't need it anyway.",
    })
}
#[test]
fn rookgaard_studded_legs_lee_delle_exchange_atomic_repeatable_replay() -> TestResult {
    execute(Case {
        npc: "oteryn:npc.lee_delle",
        keyword: "honey scented flowers",
        answer: "",
        input: "oteryn:item.tibia.i2984",
        output: "oteryn:item.tibia.i3362",
        confirmation: false,
        success: "Oh, thank you so much! Please take this piece of armor as reward.",
    })
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "rookgaard_exchange", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let dependencies = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, _activation, _proof) = purchase_fixture::current(id(40));
        let coordinator =
            SpellPremiumCoordinator::new(consumer.clone(), Some(clock), None, Some(h.root.clone()));
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
        dependencies.initialize(&owner).await;
        owner.premium_coordinator = Some(&coordinator);
        owner.admit_premium(id(40), dependencies.admitted.game_session_id);
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = dependencies.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: case.npc.into(),
                            placements: vec![NpcPlacement {
                                floor: -i8::try_from(spot.floor)?,
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };

        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        dependencies
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                dependencies.actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        seed_main_backpack(&h, &[(case.input, 1), (case.input, 1)]).await?;
        // The second source is in a closed nested bag. Amber's first source
        // retains notebook text, which is not a plain craft input.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2853','definition-r1',1,1,encode($3,'hex')::uuid)")
            .bind(id(73).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,3,encode($5,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(73).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(72).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
            .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(73).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        if case.input == "oteryn:item.tibia.i2821" {
            sqlx::query("INSERT INTO game_item_text_revisions(item_instance_id,last_revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(71).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_texts(item_instance_id,text,written_at,text_revision) VALUES(encode($1,'hex')::uuid,'Hardek *',clock_timestamp(),1)")
                .bind(id(71).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let mut next = 1;
        // No confirmation exists before the actual source keyword.
        let greeting = format!("hi {}", case.keyword);
        let intercepted = format!("job {}", case.keyword);
        for text in [
            greeting.as_str(),
            "hi",
            "yes",
            &intercepted,
            "yes",
            "flowers with honey",
        ] {
            // Advance the existing fixture clock so every guard probe reaches
            // dialogue selection through the production chat limiter.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let outcome = owner
                .npc_talk(
                    dependencies.actor,
                    command(1000 + next),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
            assert!(!outcome.inventory_changed, "{} guard: {text}", case.npc);
            if text == greeting {
                assert!(
                    outcome.conversation.as_ref().is_some_and(|reply| reply
                        .lines
                        .iter()
                        .any(|line| line.contains("Fixture Hero"))),
                    "a mixed greeting must retain the source greeting"
                );
            }
            next += 1;
        }
        assert_eq!(quantity(&h, case.input).await?, 2);
        for round in 0..2 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let (commit_id, text, outcome) = if round == 0 {
                let mut admitted = dependencies.admitted;
                admitted.first_entry =
                    crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
                admitted.item_fence = Some(fence);
                let commit_id = wire_exchange(&owner, admitted, npc, case).await?;
                next = commit_id + 1;
                let outcome = owner
                    .resume_npc_bank(dependencies.actor, command(commit_id))
                    .await
                    .ok_or("wire exchange receipt")?;
                (
                    commit_id,
                    if case.confirmation {
                        case.answer
                    } else {
                        case.keyword
                    },
                    outcome,
                )
            } else {
                let offer = owner
                    .npc_talk(
                        dependencies.actor,
                        command(next),
                        NpcTalkIntent {
                            npc_actor: npc,
                            text: case.keyword.into(),
                        },
                    )
                    .await;
                assert!(!offer.unknown, "keyword storage failure");
                assert_eq!(offer.disposition, NpcIntentDisposition::Ok);
                next += 1;
                if case.confirmation {
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(2))
                        .ok_or("clock")?;
                    let commit_id = next;
                    next += 1;
                    (
                        commit_id,
                        case.answer,
                        owner
                            .npc_talk(
                                dependencies.actor,
                                command(commit_id),
                                NpcTalkIntent {
                                    npc_actor: npc,
                                    text: case.answer.into(),
                                },
                            )
                            .await,
                    )
                } else {
                    (next - 1, case.keyword, offer)
                }
            };
            assert!(!outcome.unknown, "{} commit unknown", case.npc);
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
            assert_eq!(
                outcome.conversation.ok_or("reward reply")?.lines,
                [case.success]
            );
            assert_eq!(quantity(&h, case.input).await?, 1 - round);
            assert_eq!(quantity(&h, case.output).await?, 1 + round);
            let original = footprint(&h).await?;
            let replay = owner
                .npc_talk(
                    dependencies.actor,
                    command(commit_id),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert!(!replay.unknown);
            assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
            assert_eq!(
                footprint(&h).await?,
                original,
                "exact replay cannot mint twice"
            );
            let conflict = owner
                .npc_talk(
                    dependencies.actor,
                    command(commit_id),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: "no".into(),
                    },
                )
                .await;
            assert_eq!(conflict.disposition, NpcIntentDisposition::Rejected);
            assert_eq!(footprint(&h).await?, original, "changed payload conflicts");
            owner.states.npc_conversations.forget(session);
            owner.states.chat.forget(session);
            let cold = owner
                .resume_npc_bank(dependencies.actor, command(commit_id))
                .await
                .ok_or("cold exchange receipt")?;
            assert!(!cold.unknown);
            assert_eq!(cold.disposition, NpcIntentDisposition::Ok);
            let conversation = cold.conversation.ok_or("retained source reply")?;
            assert_eq!(conversation.lines, [case.success]);
            assert_eq!(
                conversation.open, None,
                "replay cannot reopen retired context"
            );
            assert_eq!(
                footprint(&h).await?,
                original,
                "cold replay preserves original instances"
            );
            owner
                .states
                .chat
                .admit(session, "Fixture Hero", owner.owner_now());
            owner.states.chat.set_npc_service(session);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let hi = owner
                .npc_talk(
                    dependencies.actor,
                    command(next),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: "hi".into(),
                    },
                )
                .await;
            assert_eq!(hi.disposition, NpcIntentDisposition::Ok);
            next += 1;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE world_id=encode($1,'hex')::uuid AND definition_production_key=$2 AND lifecycle=1")
        .bind(id(42).as_slice()).bind(key).fetch_one(&h.pool).await?)
}

async fn wire_exchange(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    npc: EntityRef,
    case: Case,
) -> TestResult<u64> {
    let mut texts = vec!["hi", case.keyword];
    if case.confirmation {
        texts.push(case.answer);
    }
    wire_talk(owner, admitted, npc, &texts).await
}

pub(super) async fn wire_talk(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    npc: EntityRef,
    texts: &[&str],
) -> TestResult<u64> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::npc_service::{
        CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, decode_npc_intent_result,
        encode_npc_talk_intent,
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    };
    use tokio::io::AsyncWriteExt;

    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[CAPABILITY_NPC_SERVICE_V1],
    )
    .ok_or("NPC service selection")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let ended = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut command_id = admitted.continuity.next_command_id;
        for text in texts {
            let payload = encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: npc,
                text: (*text).into(),
            })
            .map_err(debug)?;
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_NPC_TALK_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, command_id, "{text}");
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        decode_npc_intent_result(result.payload).map_err(debug)?,
                        NpcIntentDisposition::Ok,
                        "{text}"
                    );
                    break;
                }
            }
            command_id += 1;
        }
        client.shutdown().await?;
        Ok(command_id - 1)
    };
    let (ended, result): (_, TestResult<u64>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    result.map_err(|error| format!("wire ended {ended:?}: {error}").into())
}

async fn footprint(h: &Harness) -> TestResult<Vec<(String, String, i64, i16)>> {
    Ok(sqlx::query_as("SELECT item_instance_id::text,definition_production_key,quantity,lifecycle FROM game_item_instances ORDER BY item_instance_id LIMIT 32").fetch_all(&h.pool).await?)
}
