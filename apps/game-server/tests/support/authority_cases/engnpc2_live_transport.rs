//! Genuine composed quest caller: typed fresh admission, native actor and PG.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId,
    MovementLocalPosition, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand, fresh_fixture,
};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, decode_use_intent_target, encode_item_use_intent,
};
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;

#[test]
fn engnpc2_live_transport_rotworm_hotkey_and_replay() -> TestResult {
    run(false)
}

#[test]
fn engnpc2_live_transport_rotworm_handle_and_replay() -> TestResult {
    run(true)
}

#[test]
fn engnpc2_live_transport_slug_corpse_hotkey_and_replay() -> TestResult {
    run_case(false, true)
}

#[test]
fn engnpc2_live_transport_slug_corpse_handle_and_replay() -> TestResult {
    run_case(true, true)
}

fn run(handle: bool) -> TestResult {
    run_case(handle, false)
}

fn run_case(handle: bool, corpse: bool) -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create_with_admission_seed(
                        admin,
                        if corpse && handle {
                            "engnpc2_corpse_handle"
                        } else if corpse {
                            "engnpc2_corpse_caller"
                        } else if handle {
                            "engnpc2_live_handle"
                        } else {
                            "engnpc2_live_transport"
                        },
                        true,
                        false,
                    )
                    .await?;
                    let result = exercise(&h, handle, corpse).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "PG_EXECUTED {} duration={:?}",
                        if corpse && handle {
                            "engnpc2_live_transport_slug_corpse_handle_and_replay"
                        } else if corpse {
                            "engnpc2_live_transport_slug_corpse_hotkey_and_replay"
                        } else if handle {
                            "engnpc2_live_transport_rotworm_handle_and_replay"
                        } else {
                            "engnpc2_live_transport_rotworm_hotkey_and_replay"
                        },
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error: Box<dyn std::error::Error>| error.to_string())
        })?
        .join()
        .map_err(|_| "live transport PG thread failed")?
        .map_err(Into::into)
}

pub(super) async fn fresh(h: &fixture::Harness) -> TestResult {
    use crate::durability::{
        admission_authority_guards::AdmissionGuardStore, fresh_admission::FreshAdmissionStore,
    };
    use crate::foundation::admission_authority_publication::*;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut runtime = fresh_fixture::fresh::Source::new(now)?;
    runtime
        .rows
        .retain(|r| matches!(r.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?;
    h.root
        .publish_runtime_readiness(&h.node, &publication)
        .await
        .map_err(debug)?;
    let mut admission = fresh_fixture::fresh::Source::new(now)?;
    admission
        .rows
        .retain(|r| !matches!(r.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&admission, now).map_err(debug)?;
    AdmissionGuardStore::from_root(h.root.clone())
        .publish(&publication)
        .await
        .map_err(debug)?;
    let request = fresh_fixture::fresh::Source::new(now)?.request()?;
    let result = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&request)
        .await
        .map_err(debug)?;
    assert!(matches!(result,crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)),"{result:?}");
    Ok(())
}

async fn exercise(h: &fixture::Harness, handle: bool, corpse: bool) -> TestResult {
    fresh(h).await?;
    let scope = fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?;
    let start = pin.qualified_room().entry_start();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        12,
        channel_pin,
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(&id(41)).map_err(debug)?,
                current.current_character_lease().generation(),
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let creature = runtime
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
            "oteryn:creature.rotworm",
            100,
        )
        .map_err(debug)?;
    let generation_id = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .find(|c| c.actor == creature)
        .ok_or("visible Rotworm")?
        .generation;
    let other = runtime
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x + 1,
                y: at.y + 1,
                floor: at.floor,
            },
            "oteryn:creature.slug",
            100,
        )
        .map_err(debug)?;
    let other_generation = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .find(|c| c.actor == other)
        .ok_or("visible wrong-species target")?
        .generation;
    let runtime = Mutex::new(runtime);
    let room = oteryn_gameplay_content::content::qualify_native_entry_room(world).map_err(debug)?;
    let content =
        oteryn_gameplay_world::world_runtime::ReferenceContentGeneration::from_content(room.door())
            .map_err(debug)?;
    let fence = oteryn_gameplay_world::world_runtime::ScopeContentGenerationFence::for_test(
        scope,
        fixture::fence(1)?.scope_ownership_generation,
        content,
    );
    let door = Mutex::new(
        oteryn_gameplay_world::world_runtime::bind_native_entry_door(
            room.door(),
            &fence,
            scope,
            fixture::fence(1)?.scope_ownership_generation,
        )
        .map_err(debug)?,
    );
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(),
        ("127.0.0.1".into(), 1),
        "source.test".into(),
        "source.test".into(),
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/ca.der").to_vec(),
        )],
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/client.der").to_vec(),
        )],
        tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(
            include_bytes!("../spell-save-tls-fixture/client-key.der").to_vec(),
        )?,
    )
    .map_err(debug)?;
    let evidence = FreshEvidenceSource::new(descriptor);
    let spells = oteryn_gameplay_spell::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        oteryn_gameplay_social::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(debug)?;
    let charms =
        oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue::embedded()
            .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner: ComposedFreshAdmission<'_, '_, '_> = ComposedTestOwner {
        root: &h.root,
        character: &authority,
        holder: &h.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        required_capabilities: composed_test_owner::empty_capabilities(world, channel)?,
        runtime: &runtime,
        room: &room,
        door: &door,
        spells: &spells,
        achievements: &achievements,
        imported_charms: &charms,
    }
    .build();
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    let key = if corpse {
        "oteryn:item.tibia.i17670"
    } else {
        "oteryn:item.tibia.i17673"
    };
    seed_tool(h, native, key).await?;
    let corpse_item = if corpse {
        Some(corpse::mint(h, &authority, native, at).await?)
    } else {
        None
    };
    let index = native.item_index().ok_or("native item index")?;
    let definition = index
        .definition_ref("Item", key, index.revision(key).ok_or("gatherer revision")?)
        .ok_or("gatherer reference")?;
    let intent = UseIntent {
        target: if handle {
            UseTarget::Item(std::num::NonZeroU64::new(7).ok_or("tool handle")?)
        } else {
            UseTarget::ItemByDefinition(definition)
        },
        use_with: (!corpse).then_some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: creature.placement_identity(),
            generation: generation_id,
        }),
        use_with_map_item: if corpse {
            Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(
                std::num::NonZeroU64::new(77).ok_or("corpse handle")?,
            ))
        } else {
            None
        },
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let intent = decode_use_intent_target(&[4, 6, 11, 15], &payload).map_err(debug)?;
    let command = UseCommand {
        game_session_id: session,
        command_id: 13101,
        item_fence: Some(fence.item_fence()),
    };
    if !corpse {
        capture::assert_target(&runtime, actor, Some(creature), fence, command, &intent).await?;
    }
    let mut wrong = intent.clone();
    wrong.use_with_map_item = None;
    wrong.use_with = Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: other.placement_identity(),
        generation: other_generation,
    });
    let refusal = call(
        &owner,
        actor,
        UseCommand {
            command_id: 13100,
            ..command
        },
        wrong,
        handle,
        (true, true, None),
    )
    .await?
    .ok_or("invalid-target caller fell through")?;
    assert_eq!(refusal, UseDisposition::Rejected);
    let refused: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_character_quest_receipts),(SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(id(92).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        refused,
        (0, 0, 1),
        "invalid target must not consume or credit the tool"
    );
    let outcome = call(
        &owner,
        actor,
        command,
        intent.clone(),
        handle,
        (true, true, corpse_item),
    )
    .await?
    .ok_or("composed quest caller fell through")?;
    assert_eq!(outcome, UseDisposition::Committed);
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 1));
    let output = if corpse {
        "oteryn:item.tibia.i17672"
    } else {
        "oteryn:item.tibia.i17674"
    };
    let values:(i64,i64)=sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1)").bind(id(92).as_slice()).bind(output).fetch_one(&h.pool).await?;
    assert_eq!(values, (0, 1));
    runtime
        .lock()
        .await
        .remove_mount_taming_subject(creature, "oteryn:creature.rotworm")
        .map_err(debug)?;
    if !corpse {
        capture::assert_target(&runtime, actor, None, fence, command, &intent).await?;
    }
    // Spent source and target handles no longer resolve; the live actor is removed.
    // The historical command must still reach the existing receipt owner.
    let replay = call(&owner, actor, command, intent, handle, (false, false, None))
        .await?
        .ok_or("composed replay fell through")?;
    assert_eq!(replay, UseDisposition::Committed);
    let counts_after:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(counts_after, counts);
    Ok(())
}

async fn call(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    intent: UseIntent,
    handle: bool,
    observation: (bool, bool, Option<[u8; 16]>),
) -> TestResult<Option<UseDisposition>> {
    let (source_present, visible, target) = observation;
    let source =
        (handle && source_present).then_some(composed_test_owner::ItemKey::Instance(id(92)));
    if handle && intent.use_with_map_item.is_none() {
        // Match the connection caller: an unrelated event-item owner may decline
        // before the closed quest target owner rejects or commits this intent.
        if let Some(outcome) = owner
            .use_event_item(actor, command, intent.clone(), source, visible)
            .await
            .map_err(|_| "composed quest caller unavailable")?
        {
            return Ok(Some(outcome.disposition));
        }
    }
    owner
        .use_quest_tool(
            actor,
            command,
            intent,
            source,
            target.map(composed_test_owner::ItemKey::Instance),
        )
        .await
        .map(|outcome| outcome.map(|outcome| outcome.disposition))
        .map_err(|_| "composed quest caller unavailable".into())
}

pub(super) async fn seed_tool(
    h: &fixture::Harness,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    tool: &str,
) -> TestResult {
    let index = native.item_index().ok_or("native index")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key) in [(91, "oteryn:item.tibia.i2854"), (92, tool)] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)").bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).ok_or("stock revision")?).bind(id(seed+10).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(91).as_slice()).bind(id(42).as_slice()).bind(id(111).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(91).as_slice()).bind(id(92).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

#[path = "engnpc2_live_transport/corpse.rs"]
mod corpse;

#[path = "engnpc2_live_transport/capture.rs"]
mod capture;
