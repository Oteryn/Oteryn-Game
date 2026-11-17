//! NPC-MARRIAGE-1: two genuine Fresh participants, production talk and native value owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn srv9b_marriage_postgres_native_lynda_service() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "marriage PG thread panicked")??;
    assert!(started.elapsed() > std::time::Duration::from_secs(1));
    eprintln!(
        "POSTGRES_CASE_COMPLETED SRV9B marriage duration={:?}",
        started.elapsed()
    );
    Ok(())
}
fn scenario() -> TestResult {
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "srv9b_marriage", true, false).await?;
        prepare_current_session(&h).await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let partner_current =
            crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
                &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                    root: h.root.clone(),
                    node: h.node.clone(),
                    recovery: &h.recovery,
                },
                60,
                63,
                171,
                now,
            )
            .await?;
        let partner_item = crate::gameplay_transport::item_fence_of(partner_current)
            .ok_or("partner item fence")?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let partner_fence =
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id: partner_item.character_id,
                game_session_id: partner_item.game_session_id,
                connection_generation: partner_item.connection_generation,
                character_lease_generation: partner_item.character_lease_generation,
                runtime_scope: partner_item.runtime_scope,
                scope_ownership_generation: partner_item.scope_ownership_generation,
                expected_character_revision: crate::domain::CharacterRevision::new(1)
                    .map_err(debug)?,
            };
        let context = crate::bestiary_postgres_harness::context();
        h.root
            .initialize_character_progression(
                &authority,
                &h.node,
                partner_fence,
                crate::durability::character_progression::ProgressionInitializationRequest {
                    context: context.clone(),
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    policy: crate::domain::progression::FiniteProgressionPolicy {
                        context,
                        policy_revision: "policy-1".into(),
                        reward_revision: "reward-1".into(),
                        death_policy_revision: "death-1".into(),
                        declared_difference_revision: "declaration-1".into(),
                        thresholds: [
                            crate::domain::progression::LevelThreshold {
                                level: 1,
                                minimum_experience: oteryn_simulation_determinism::ExactI64::new(0),
                            },
                            crate::domain::progression::LevelThreshold {
                                level: 2,
                                minimum_experience: oteryn_simulation_determinism::ExactI64::new(
                                    100,
                                ),
                            },
                        ],
                        terminal_exclusive_experience: oteryn_simulation_determinism::ExactI64::new(
                            200,
                        ),
                        death_loss_numerator: 1,
                        death_loss_denominator: 10,
                        death_loss_rounding: oteryn_simulation_determinism::RoundingMode::Floor,
                    },
                },
            )
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let entry = (2, 0, 7);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            entry,
        );
        // One player plus Alice and Rashid each occupy a carrier slot.
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        let npc_key = "oteryn:npc.lynda";
        owner.states.chat.install_npc_talk(Arc::new(book));
        let partner_actor = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(partner_item.game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    partner_item.game_session_id,
                    partner_current.current_character_lease(),
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            actor
        };
        deps.initialize_player(&owner, partner_actor, partner_item.game_session_id)
            .await;
        let partner_name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(partner_item.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        owner.states.chat.admit(
            partner_item.game_session_id,
            &partner_name,
            owner.owner_now(),
        );
        owner
            .states
            .chat
            .set_npc_service(partner_item.game_session_id);
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let actors = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: npc_key.into(),
                            placements: vec![NpcPlacement {
                                x: (at.x + 1) as u16,
                                y: at.y as u16,
                                floor: -7,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?
                .actors;
            let actor = actors[0];
            EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            }
        };
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i3004", 1),
                ("oteryn:item.tibia.i9586", 1),
            ],
        )
        .await?;
        seed_partner_items(&h, partner_item.character_id).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let actor_name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        let before:i64=sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        let partner_command = |command_id| UseCommand {
            game_session_id: partner_item.game_session_id,
            command_id,
            item_fence: Some(partner_item),
        };
        let greeting = speak(&mut owner, deps.actor, command(1), npc, "hi").await?;
        assert!(has_line(&greeting, "pilgrim Fixture Hero!"));
        assert!(!has_line(&greeting, "|PLAYERNAME|"));
        let prompt = speak(&mut owner, deps.actor, command(2), npc, "marry").await?;
        assert!(
            has_line(&prompt, "Do you wish to marry?"),
            "Lynda must reach the native marriage service"
        );
        speak(&mut owner, deps.actor, command(3), npc, "yes").await?;
        let refused = speak(&mut owner, deps.actor, command(4), npc, &actor_name).await?;
        assert!(has_line(&refused, "No eligible partner"));
        assert_eq!(count(&h, "game_character_marriage_receipts").await?, 0);
        for (id, text) in [(5, "marry"), (6, "yes"), (7, partner_name.as_str())] {
            let reply = speak(&mut owner, deps.actor, command(id), npc, text).await?;
            if id == 7 {
                assert!(
                    has_line(&reply, "proposal has been recorded"),
                    "proposal reply: {:?}",
                    reply.conversation
                );
            }
        }
        assert_eq!(relation_status(&h, fence.character_id).await?, Some(1));
        assert_eq!(
            relation_status(&h, partner_item.character_id).await?,
            None,
            "an incoming proposal does not reserve its target"
        );
        assert_eq!(count(&h, "game_item_marriage_records").await?, 1);
        let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 0, "both plain wedding inputs were retired");
        // Exactly one fresh authority fact differs: the native storage boundary must refuse.
        let mut stale = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "current actor fence")?
            .ok_or("current fence")?;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(stale.connection_generation.get() + 1)
                .map_err(debug)?;
        let native = active.native_gameplay().ok_or("native")?;
        let definitions = marriage_definitions(native)?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let denied = slot
            .commit_npc_marriage(
                &h.root,
                &authority,
                &h.node,
                stale,
                crate::durability::npc_marriage::MarriageRequest {
                    command: crate::foundation::CommandRef::new(
                        session,
                        crate::foundation::CommandId::new(900).map_err(debug)?,
                    ),
                    occurrence: id(190),
                    event: id(191),
                    npc_identity: npc.identity,
                    npc_generation: npc.generation,
                    wire_binding: crate::durability::npc_marriage::wire_binding(
                        npc.identity,
                        npc.generation,
                        "yes",
                    ),
                    action: crate::durability::npc_marriage::MarriageAction::Cancel,
                    definitions,
                },
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                    .with_wheel(owner.premium_coordinator),
            )
            .await;
        assert!(matches!(denied,Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
        drop(slot);
        assert_eq!(count(&h, "game_character_marriage_receipts").await?, 1);
        assert!(sqlx::query("UPDATE game_character_marriages SET version=version+1 WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await.is_err(),"no unreceipted relation successor");
        for (id, text) in [(8, "remove"), (9, "yes")] {
            speak(&mut owner, deps.actor, command(id), npc, text).await?;
        }
        assert_eq!(relation_status(&h, fence.character_id).await?, Some(0));
        let refunded:Vec<String>=sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid ORDER BY i.definition_production_key").bind(fence.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            refunded,
            vec!["oteryn:item.tibia.i3004", "oteryn:item.tibia.i9586"]
        );
        speak(&mut owner, deps.actor, command(10), npc, "bye").await?;
        let mut old = fence;
        old.connection_generation = stale.connection_generation;
        let replay = owner
            .npc_talk(
                deps.actor,
                UseCommand {
                    item_fence: Some(old),
                    ..command(7)
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: partner_name.clone(),
                },
            )
            .await;
        assert!(has_line(&replay, "proposal has been recorded"));
        assert!(!replay.unknown);
        let conflict = owner
            .npc_talk(
                deps.actor,
                command(7),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: actor_name.clone(),
                },
            )
            .await;
        assert_eq!(conflict.disposition, NpcIntentDisposition::Rejected);
        assert_eq!(count(&h, "game_character_marriage_receipts").await?, 2);
        // The second participant independently reaches the same production NPC with its own Fresh fence.
        for (id, text) in [
            (1, "hi"),
            (2, "marry"),
            (3, "yes"),
            (4, actor_name.as_str()),
        ] {
            speak(&mut owner, partner_actor, partner_command(id), npc, text).await?;
        }
        assert_eq!(
            relation_status(&h, partner_item.character_id).await?,
            Some(1)
        );
        for (id, text) in [
            (11, "hi"),
            (12, "marry"),
            (13, "yes"),
            (14, partner_name.as_str()),
        ] {
            speak(&mut owner, deps.actor, command(id), npc, text).await?;
        }
        assert_eq!(relation_status(&h, fence.character_id).await?, Some(3));
        assert_eq!(
            relation_status(&h, partner_item.character_id).await?,
            Some(3)
        );
        assert_eq!(
            count(&h, "game_account_appearance_unlocks").await?,
            2,
            "both accounts receive the permanent Newly Wed base grant"
        );
        for (id, text) in [(15, "celebration"), (16, "yes"), (17, "begin")] {
            speak(&mut owner, deps.actor, command(id), npc, text).await?;
        }
        assert_eq!(relation_status(&h, fence.character_id).await?, Some(2));
        assert_eq!(
            relation_status(&h, partner_item.character_id).await?,
            Some(2)
        );
        let engraved:Vec<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i9585' AND i.lifecycle=1").bind(fence.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            engraved.len(),
            1,
            "matching executable donors mint one ring, not the speech's two"
        );
        let item: [u8; 16] = engraved[0].as_slice().try_into()?;
        let inscription = h
            .root
            .read_marriage_inscription(&authority, *world.as_bytes(), item)
            .await?
            .ok_or("native engraving")?;
        assert!(inscription.starts_with(&format!(
            "{actor_name} & {partner_name} forever - married on "
        )));
        assert!(inscription.ends_with('.'));
        let backpack = h
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?;
        let index = native.item_index().ok_or("native Item index")?;
        let inventory = crate::gameplay_transport::item_view::inventory_from_backpack(
            backpack.as_ref(),
            |definition| {
                index.definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
            },
        )
        .ok_or("native inventory projection")?;
        let mut views = crate::gameplay_transport::item_view::SessionItemView::resume(
            crate::gameplay_transport::item_view::ItemViewContinuity::default(),
        );
        views.snapshot(inventory, None).map_err(debug)?;
        let handle = views
            .table()
            .handle(&crate::gameplay_transport::item_view::ItemKey::Instance(
                item,
            ))
            .ok_or("engraved ring handle")?;
        let looked = crate::gameplay_transport::item_look::look_item(
            &owner,
            deps.actor,
            session,
            Some(fence),
            &views,
            handle,
        )
        .await;
        let oteryn_protocol_oteryn::item_look::ItemLookResult::Text(looked) = looked else {
            return Err("production LOOK refused the visible engraved ring".into());
        };
        assert!(
            looked.contains(&inscription),
            "production LOOK must expose the committed engraving"
        );
        assert!(sqlx::query("UPDATE game_item_texts SET text='forged' WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&h.pool).await.is_err());
        let envelope:Vec<u8>=sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_marriage_records r USING(transaction_id) WHERE r.anchor_item_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
        for name in [&actor_name, &partner_name] {
            assert!(
                !envelope
                    .windows(name.len())
                    .any(|bytes| bytes == name.as_bytes()),
                "no names or raw inscription in value audit"
            );
        }
        let replay = speak(&mut owner, deps.actor, command(17), npc, "begin").await?;
        assert!(has_line(&replay, "now married"));
        assert_eq!(count(&h, "game_item_marriage_records").await?, 5);
        for (id, text) in [(18, "divorce"), (19, "yes")] {
            speak(&mut owner, deps.actor, command(id), npc, text).await?;
        }
        assert_eq!(relation_status(&h, fence.character_id).await?, Some(0));
        assert_eq!(
            relation_status(&h, partner_item.character_id).await?,
            Some(0)
        );
        assert_eq!(
            count(&h, "game_account_appearance_unlocks").await?,
            2,
            "divorce never revokes the permanent grant"
        );
        assert_eq!(count(&h, "game_character_marriage_receipts").await?, 6);
        assert_eq!(count(&h, "game_item_marriage_records").await?, 5);
        let revision:i64=sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            revision,
            before + 5,
            "only the acting participant's five committed stages advance its Character revision"
        );
        assert_eq!(
            h.root
                .read_marriage_inscription(&authority, *world.as_bytes(), item)
                .await?
                .as_deref(),
            Some(inscription.as_str())
        );
        drop(owner);
        let restored = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        drop(restored);
        // One lineage mutation under the administrative corruption fixture must stop recovery.
        let mut tamper = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tamper)
            .await?;
        sqlx::query("UPDATE game_character_marriages SET version=version+1 WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tamper).await?;
        tamper.commit().await?;
        assert!(matches!(
            h.root.open_character_authority(&seal).await,
            Err(
                crate::durability::character_authority::CharacterAuthorityError::Unavailable(
                    crate::durability::DurabilityError::InvalidStoredState
                )
            )
        ));
        Ok(())
    })
}

fn has_line(outcome: &crate::gameplay_transport::npc_command::NpcTalkOutcome, part: &str) -> bool {
    outcome
        .conversation
        .as_ref()
        .is_some_and(|c| c.lines.iter().any(|line| line.contains(part)))
}
async fn speak(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    npc: EntityRef,
    text: &str,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    // Advance the owner clock between talks, retaining all normal rate/expiry enforcement.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("owner clock range")?;
    let outcome = owner
        .npc_talk(
            actor,
            command,
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown, "unproven marriage stage {text}");
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "talk {text}");
    Ok(outcome)
}
async fn count(h: &Harness, table: &str) -> TestResult<i64> {
    let sql = match table {
        "game_character_marriage_receipts" => {
            "SELECT count(*) FROM game_character_marriage_receipts"
        }
        "game_item_marriage_records" => "SELECT count(*) FROM game_item_marriage_records",
        "game_account_appearance_unlocks" => {
            "SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:appearance-grant/npc/lynda/newly-wed-base'"
        }
        _ => return Err("unexpected receipt counter".into()),
    };
    Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
}

async fn relation_status(
    h: &Harness,
    character: crate::domain::CharacterId,
) -> TestResult<Option<i16>> {
    Ok(sqlx::query_scalar(
        "SELECT status FROM game_character_marriages WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&h.pool)
    .await?)
}
fn marriage_definitions(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<[crate::durability::item_mint::TypedDefinitionRef; 3]> {
    let index = native.item_index().ok_or("item index")?;
    let definition = |key: &str| -> TestResult<_> {
        Ok(crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: index.revision(key).ok_or("definition")?.into(),
        })
    };
    Ok([
        definition("oteryn:item.tibia.i3004")?,
        definition("oteryn:item.tibia.i9586")?,
        definition("oteryn:item.tibia.i9585")?,
    ])
}
async fn seed_partner_items(h: &Harness, character: crate::domain::CharacterId) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key) in [
        (160, "oteryn:item.tibia.i2854"),
        (161, "oteryn:item.tibia.i3004"),
        (162, "oteryn:item.tibia.i9586"),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)").bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(id(164).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(character.as_bytes().as_slice()).bind(id(160).as_slice()).bind(id(42).as_slice()).bind(id(164).as_slice()).execute(&mut *tx).await?;
    for (seed, ordinal) in [(161, 1_i64), (162, 2)] {
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)").bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(character.as_bytes().as_slice()).bind(id(160).as_slice()).bind(ordinal).bind(id(164).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
