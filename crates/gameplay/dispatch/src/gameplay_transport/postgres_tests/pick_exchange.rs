//! Al Dee's fixed item exchange through the real NPC command and NPC trade transaction.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MessageType, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
    decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

const AL_DEE: &str = "oteryn:npc.al_dee";
const AXE: &str = "oteryn:item.tibia.i3462";
const PICK: &str = "oteryn:item.tibia.i3456";

type Replies = Vec<(CommandStatus, Vec<u8>)>;

struct ExchangeCase {
    npc: &'static str,
    keyword: &'static str,
    output: &'static str,
    inputs: &'static [(&'static str, i64)],
    before: i64,
    after: i64,
    expected_inputs: &'static [(&'static str, i64)],
    tag: &'static str,
}

const PICK_CASE: ExchangeCase = ExchangeCase {
    npc: AL_DEE,
    keyword: "pick",
    output: PICK,
    inputs: &[(AXE, 1)],
    before: 1,
    after: 0,
    expected_inputs: &[],
    tag: "pick_exchange",
};
const KNIFE_CASE: ExchangeCase = ExchangeCase {
    npc: "oteryn:npc.uzgod",
    keyword: "draconian steel",
    output: "oteryn:item.tibia.i5908",
    inputs: &[
        ("oteryn:item.tibia.i5889", 2),
        ("oteryn:item.tibia.i3313", 1),
        ("oteryn:item.tibia.i3313", 1),
    ],
    before: 4,
    after: 2,
    expected_inputs: &[
        ("oteryn:item.tibia.i3313", 1),
        ("oteryn:item.tibia.i5889", 1),
    ],
    tag: "obsidian_knife",
};

// TIBIAWIKI_STRUCTURED: .questpc2/wiki/Present Quest_Spoiler.wiki:1-21;
// Item joins: captured data/items/items.xml:5953 (present), :9797 (helmet).
const PRESENT_CASE: ExchangeCase = ExchangeCase {
    npc: "oteryn:npc.seymour",
    keyword: "present box",
    output: "oteryn:item.tibia.i3374",
    inputs: &[("oteryn:item.tibia.i2856", 1)],
    before: 1,
    after: 0,
    expected_inputs: &[],
    tag: "present_exchange",
};

#[test]
fn present_exchange_seymour_atomic_confirmation_refusal_and_replay() -> TestResult {
    execute(&PRESENT_CASE, false)
}

#[test]
fn pick_exchange_atomic_confirmation_refusal_and_replay() -> TestResult {
    execute(&PICK_CASE, false)
}

#[test]
fn obsidian_knife_uzgod_atomic_exchange_cooldown_and_replay() -> TestResult {
    execute(&KNIFE_CASE, false)
}

#[test]
fn present_exchange_seymour_filled_box_is_preserved() -> TestResult {
    execute(&PRESENT_CASE, true)
}

fn execute(case: &'static ExchangeCase, filled: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case, filled).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC item exchange thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {} atomic confirmation/refusal/replay elapsed_ms={}",
        if filled {
            "present_exchange_filled"
        } else {
            case.tag
        },
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u64, u32, Vec<u8>)],
) -> TestResult<Replies> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        loop {
            let bytes = read_frame(&mut client).await?;
            if decode_wire_envelope(&bytes).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut results = Vec::new();
        for (command_id, command_type, payload) in commands {
            let bytes = encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: *command_id,
                    command_type: *command_type,
                    payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &bytes).await?;
            loop {
                let bytes = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, *command_id);
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(results)
    };
    let (ended, results): (_, TestResult<Replies>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    match ended {
        ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenClosed(current, _) => *admitted = current,
        other => return Err(format!("pick exchange wire: {other:?}").into()),
    }
    results
}

fn scenario(case: &ExchangeCase, filled: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, case.tag, true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let _index = native.item_index().ok_or("item index")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            2,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        let _entry = book
            .get(case.npc)
            .ok_or("committed exchange NPC dialogue is not admitted")?;
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: case.npc.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                floor: -i8::try_from(spot.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        super::item_look::seed_main_backpack(&h, case.inputs).await?;
        if filled {
            let mut tx = h.pool.begin().await?;
            // Lane-private initial custody only; no production guard is changed.
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3462','definition-r1',1,1,encode($3,'hex')::uuid)")
                .bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice())
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                .bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(71).as_slice()).bind(id(72).as_slice())
                .execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[3],
        )
        .ok_or("NPC capabilities")?;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let talk = |text: &str| {
            encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            })
            .map_err(debug)
        };
        let replies = wire(
            &owner,
            &mut admitted,
            &[
                (1, 7, talk("hi")?),
                (2, 7, talk(case.keyword)?),
                (3, 7, talk("no")?),
            ],
        )
        .await?;
        assert!(replies.iter().all(|r| r.0 == CommandStatus::Accepted));
        assert_eq!(counts(&h, case).await?, (case.before, 0, 0, 0));
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let replies = wire(
            &owner,
            &mut admitted,
            &[(4, 7, talk("hi")?), (5, 7, talk("yes")?)],
        )
        .await?;
        assert!(replies.iter().all(|r| r.0 == CommandStatus::Accepted));
        assert_eq!(counts(&h, case).await?, (case.before, 0, 0, 0));
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let replies = wire(
            &owner,
            &mut admitted,
            &[
                (6, 7, talk("hi")?),
                (7, 7, talk(case.keyword)?),
                (8, 7, talk("yes")?),
                (8, 7, talk("yes")?),
            ],
        )
        .await?;
        if filled {
            assert_eq!(replies[2].0, CommandStatus::Rejected);
            assert_eq!(
                decode_npc_intent_result(&replies[2].1).map_err(debug)?,
                NpcIntentDisposition::Rejected
            );
            assert_eq!(counts(&h, case).await?, (case.before, 0, 0, 0));
            let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1")
                .bind(id(90).as_slice()).bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(retained, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        assert_eq!(replies[2].0, CommandStatus::Accepted);
        assert_eq!(
            decode_npc_intent_result(&replies[2].1).map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            replies[3],
            (CommandStatus::DuplicateReplay, replies[2].1.clone())
        );
        assert_eq!(counts(&h, case).await?, (case.after, 1, 1, 1));
        assert_eq!(
            remaining_inputs(&h, case).await?,
            case.expected_inputs
                .iter()
                .map(|(key, n)| ((*key).to_owned(), *n))
                .collect::<Vec<_>>()
        );
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let replies = wire(
            &owner,
            &mut admitted,
            &[
                (9, 7, talk("hi")?),
                (10, 7, talk(case.keyword)?),
                (11, 7, talk("yes")?),
            ],
        )
        .await?;
        assert_eq!(
            decode_npc_intent_result(&replies[2].1).map_err(debug)?,
            NpcIntentDisposition::Rejected
        );
        assert_eq!(counts(&h, case).await?, (case.after, 1, 1, 1));
        assert_eq!(
            remaining_inputs(&h, case).await?,
            case.expected_inputs
                .iter()
                .map(|(key, n)| ((*key).to_owned(), *n))
                .collect::<Vec<_>>()
        );
        drop(owner);
        // Lose every volatile conversation and command cache. The production command
        // handler still reconciles the committed occurrence before requiring live dialogue.
        let cold = deps.owner(&h, &authority);
        let replay = crate::gameplay_transport::npc_command::npc_bank_talk(
            &cold,
            deps.actor,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: session,
                command_id: 8,
                item_fence: admitted.item_fence,
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "yes".into(),
            },
        )
        .await;
        assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
        assert!(replay.inventory_changed);
        assert!(replay.conversation.is_none());
        assert_eq!(counts(&h, case).await?, (case.after, 1, 1, 1));
        assert_eq!(
            remaining_inputs(&h, case).await?,
            case.expected_inputs
                .iter()
                .map(|(key, n)| ((*key).to_owned(), *n))
                .collect::<Vec<_>>()
        );
        drop(cold);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn counts(h: &Harness, case: &ExchangeCase) -> TestResult<(i64, i64, i64, i64)> {
    let keys: Vec<&str> = case.inputs.iter().map(|(key, _)| *key).collect();
    Ok(sqlx::query_as("SELECT (SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=ANY($1) AND lifecycle=1), (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1), (SELECT count(*) FROM game_npc_trades WHERE side=3 AND npc_key=$3), (SELECT count(*) FROM game_item_audit_outbox o JOIN game_npc_trades r USING(transaction_id) WHERE r.side=3 AND r.npc_key=$3)")
        .bind(keys).bind(case.output).bind(case.npc).fetch_one(&h.pool).await?)
}

async fn remaining_inputs(h: &Harness, case: &ExchangeCase) -> TestResult<Vec<(String, i64)>> {
    let keys: Vec<&str> = case.inputs.iter().map(|(key, _)| *key).collect();
    Ok(sqlx::query_as("SELECT i.definition_production_key,sum(i.quantity)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=ANY($2) AND i.lifecycle=1 GROUP BY i.definition_production_key ORDER BY i.definition_production_key")
        .bind(id(41).as_slice()).bind(keys).fetch_all(&h.pool).await?)
}
