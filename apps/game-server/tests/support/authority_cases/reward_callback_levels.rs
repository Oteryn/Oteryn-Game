//! Level gates and level-scaled rewards through the real NPC dialogue owner.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;

const PREMIUM_T0: i64 = 1_790_812_800_000_000;
struct PremiumClock;
impl crate::gameplay_transport::spell_premium_coordinator::clock_registration::Registered
    for PremiumClock
{
}
impl crate::gameplay_transport::spell_premium_coordinator::RegisteredPremiumClock for PremiumClock {
    fn read_current_window(&self) -> Option<production_server::premium::TrustedNow> {
        production_server::premium::TrustedNow::new(PREMIUM_T0 + 60_000_000, 0)
    }
}

async fn premium(
    h: &fixture::Harness,
) -> TestResult<crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator> {
    use crate::durability::premium_fence::{EntitlementState, PremiumEvidence};
    use crate::gameplay_transport::spell_premium_coordinator::{
        PremiumActivation, SpellPremiumCoordinator,
    };
    let consumer = Arc::new(production_server::premium::PremiumConsumer::default());
    consumer.prove_for_test(PremiumEvidence {
        account_id: id(40),
        producer_revision: "1".repeat(40),
        producer_profile: production_server::premium::PRODUCER_PROFILE.into(),
        product_id: production_server::premium::PRODUCT_ID.into(),
        product_version: production_server::premium::PRODUCT_VERSION,
        entitlement_id: Some("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70".into()),
        state: EntitlementState::Active,
        lifecycle_revision: 1,
        authority_revision: 7,
        effective_from_us: PREMIUM_T0 - 60_000_000,
        effective_until_us: PREMIUM_T0 + 3_600_000_000,
        authority_issued_at_us: PREMIUM_T0,
        authority_valid_until_us: PREMIUM_T0 + 3_600_000_000,
        refresh_after_us: PREMIUM_T0 + 1_800_000_000,
    });
    assert!(consumer.premium_current(
        id(40),
        production_server::premium::TrustedNow::new(PREMIUM_T0 + 60_000_000, 0)
    ));
    h.root
        .latch_premium_activation("reward-level-test", PREMIUM_T0, PREMIUM_T0 + 60_000_000)
        .await
        .map_err(debug)?;
    let coordinator = SpellPremiumCoordinator::new(
        consumer,
        Some(Arc::new(PremiumClock)),
        Some(PremiumActivation {
            activation_id: "reward-level-test".into(),
            switch_over_us: PREMIUM_T0,
        }),
        Some(h.root.clone()),
    );
    let mut tx = h.pool.begin().await?;
    assert!(
        crate::durability::quest_state::grizzly::GrizzlyPremiumGate::task_permitted(
            &coordinator,
            &mut tx,
            id(40)
        )
        .await
        .map_err(debug)?,
        "fixture Premium gate must be current"
    );
    tx.rollback().await?;
    Ok(coordinator)
}

#[test]
fn reward_callback_maximum_level_refuses_above_bound() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, false, 25)
}
#[test]
fn reward_callback_maximum_level_pays_at_or_below_bound() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin.clone(), false, 24)?;
    run(admin, false, 20)
}
#[test]
fn reward_callback_level_scaled_experience_pays_factor_times_level() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin.clone(), true, 20)?;
    run(admin, true, 40)
}

fn run(admin: String, spirit: bool, level: i64) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = fixture::Harness::create_with_admission_seed(
                        admin,
                        &format!("reward_levels_{spirit}_{level}"),
                        true,
                        false,
                    )
                    .await?;
                    let result = exercise(&h, spirit, level).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    eprintln!("PG_EXECUTED reward_levels spirit={spirit} level={level}");
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "reward level PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness, spirit: bool, level: i64) -> TestResult {
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
    // Compile immutable content before publishing time-bounded admission evidence.
    let npcs = oteryn_gameplay_content::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")),
        std::ffi::OsStr::new("world"),
        oteryn_gameplay_content::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = npcs.service_model_for_project().map_err(debug)?;
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
    let spells = oteryn_gameplay_spell::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        oteryn_gameplay_social::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(debug)?;
    let charms =
        oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue::embedded()
            .map_err(debug)?;
    let quest_catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(native.source_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    super::engnpc2_live_transport::fresh(h).await?;
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
    let runtime = Mutex::new(runtime);
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
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let premium = premium(h).await?;
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
    owner.premium_coordinator = Some(&premium);
    owner.quest_catalogue = Some(quest_catalogue);

    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let key = if spirit {
        "oteryn:quest-reward/l6-spirit-confession"
    } else {
        "oteryn:quest-reward/m01c-pilgrimage-s8"
    };
    let callback = reward_callbacks::load_for(catalogue)?
        .into_iter()
        .find(|c| c.key == key)
        .ok_or("production callback")?;
    let terms = serde_json::to_value(&callback)?;
    if spirit {
        assert_eq!(terms["experience_per_level"], 50);
    } else {
        assert_eq!(terms["maximum_level"], 24);
    }
    // Establish only this terminal dialogue's prerequisites; production callback,
    // catalogue, fresh session, premium and all durable writers remain real.
    super::engnpc2_live_transport::seed_tool(h, native, "oteryn:item.tibia.i11544").await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$1")
        .bind(level)
        .execute(&mut *tx)
        .await?;
    if spirit {
        // Faded wills are non-stackable: carry three distinct legal instances.
        for (seed, ordinal) in [(93, 2), (94, 3)] {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,1,lifecycle,encode($2,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($3,'hex')::uuid")
                .bind(id(seed).as_slice()).bind(id(seed + 10).as_slice()).bind(id(92).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(91).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(ordinal).bind(id(seed + 20).as_slice()).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    // Establish a real receipt chain: revisions cannot be imported independently
    // of the progression owner's complete history.
    let setup_fence = composed_test_owner::load_quests(&owner, session).await?;
    let sequencer =
        crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut setup_slot = sequencer.acquire(setup_fence.character_id).await;
    let weights = super::engitem_reward_text::Weights(native);
    let predecessors: Vec<String> = if spirit {
        vec!["oteryn:quest-reward/l6-spirit-accept".into()]
    } else {
        (1..8)
            .map(|stage| format!("oteryn:quest-reward/m01c-pilgrimage-s{stage}"))
            .collect()
    };
    for (command, transition_key) in (16900..).zip(predecessors) {
        let previous = setup_slot
            .commit_quest_reward(
                &h.root,
                &authority,
                &h.node,
                setup_fence,
                crate::durability::quest_state::QuestTransitionRequest {
                    transition_key,
                    cause: crate::durability::quest_state::QuestCause::Command(
                        super::engitem_reward_text::command(command)?,
                    ),
                },
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                crate::durability::reward_claim_mint::QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: None,
                    peer_use: None,
                },
                &owner.premium_coordinator,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                previous,
                crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
            ),
            "prerequisite transition: {previous:?}"
        );
    }
    drop(setup_slot);
    if spirit {
        // The three previously collected wills use the existing imported flags;
        // the acceptance receipt above owns their quest's current revision/pin.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for storage in callback.storage.iter().skip(1) {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,value,quest_key,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)")
                .bind(id(41).as_slice()).bind(&storage.track).bind(storage.minimum).bind(&callback.quest).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    owner.states.chat.install_npc_talk(Arc::new(
        oteryn_gameplay_npc::npc_talk::NpcTalkBook::from_model_with_native_quests(
            &model,
            &[&callback.npc],
        ),
    ));
    owner.states.chat.admit_as(
        session,
        "Reward witness",
        Some(id(41)),
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1),
    );
    owner.states.chat.set_npc_service(session);
    let npc = {
        let mut r = runtime.lock().await;
        r.place_npcs(
            &crate::foundation::NpcPlacementTable {
                npcs: vec![crate::foundation::NpcPlacements {
                    key: callback.npc.clone(),
                    placements: vec![crate::foundation::NpcPlacement {
                        x: u16::try_from(at.x + 1)?,
                        y: u16::try_from(at.y)?,
                        floor: -i8::try_from(at.floor)?,
                        direction: crate::foundation::MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
        let n = r
            .visible_entities()
            .npcs
            .into_iter()
            .next()
            .ok_or("placed NPC")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: n.actor.placement_identity(),
            generation: n.generation,
        }
    };
    let say = |command_id, text: String| {
        owner.npc_talk(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text,
            },
        )
    };
    let hello = say(17000, "hi".into()).await;
    assert_eq!(hello.disposition, NpcIntentDisposition::Ok);
    {
        let r = runtime.lock().await;
        r.assert_actor_spell_unreserved(actor).map_err(debug)?;
        assert!(
            r.content_pin().server_artifact_digest()
                == generation.identity().server_artifact_digest()
        );
        let lease = r.bound_attacker_lease(actor, session).map_err(debug)?;
        assert!(lease.character_id().as_bytes() == fence.character_id.as_bytes());
        assert_eq!(lease.generation(), fence.character_lease_generation);
    }
    assert!(
        fence.item_fence()
            == composed_test_owner::load_quests(&owner, session)
                .await?
                .item_fence()
    );
    let outcome = say(17001, callback.keywords[0].clone()).await;
    let gold: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity * 100),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1").fetch_one(&h.pool).await?;
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?;
    if !spirit && level > 24 {
        assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
        assert_eq!(gold, 0);
        assert_eq!(receipts, 0);
        assert_eq!(
            outcome.conversation.ok_or("refusal conversation")?.lines,
            callback.failure_reply
        );
    } else {
        assert_eq!(
            outcome.disposition,
            NpcIntentDisposition::Ok,
            "unknown={}, gold={gold}, receipts={receipts}, conversation: {:?}",
            outcome.unknown,
            outcome
                .conversation
                .as_ref()
                .map(|conversation| &conversation.lines)
        );
        assert_eq!(receipts, 1);
        if spirit {
            let amount: i64 = sqlx::query_scalar("SELECT coalesce((SELECT sum(amount) FROM game_character_quest_xp_obligations),(SELECT sum(experience_awarded) FROM game_character_xp_receipts),0)::bigint").fetch_one(&h.pool).await?;
            assert_eq!(amount, 50 * level);
        } else {
            assert_eq!(gold, 2000);
        }
    }
    Ok(())
}

#[test]
fn reward_callback_level_scaled_experience_overflow_refuses() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "reward_level_overflow", true).await?;
                    let result = overflow(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    eprintln!("PG_EXECUTED reward_level_overflow");
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "reward overflow PG thread failed")?
        .map_err(Into::into)
}

async fn overflow(h: &fixture::Harness) -> TestResult {
    // The forward migration can be reapplied without changing its proof again.
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/2891_reward_level_scaled_experience.sql"
    )))
    .execute(&h.pool)
    .await?;
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::durability::reward_claim_mint::QuestRewardAdmission;
    use oteryn_gameplay_quests::quest::QuestStateCatalogue;
    use sha2::Digest;
    let mut callback: reward_callbacks::RewardCallback = serde_json::from_value(
        serde_json::json!({
            "key":"oteryn:quest-reward/level-overflow", "quest":"oteryn:quest.level_overflow",
            "npc":"oteryn:npc.fixture", "keywords":["reward"], "topic":0, "next_topic":0,
            "storage":[{"track":"oteryn:quest-progress/level-overflow","minimum":0,"maximum":0,"set":0,"preserve":true}],
            "repeatable":true, "take":[{"item":"oteryn:item.tibia.i3031","count":1}], "give":null, "outfit":null, "achievement":null,
            "experience_per_level":i64::MAX, "reply":["Reward."], "sources":["overflow boundary fixture"]
        }),
    )?;
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            fixture::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    super::engnpc2_live_transport::seed_tool(h, native, "oteryn:item.tibia.i3031").await?;
    let weights = super::engitem_reward_text::Weights(native);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    for (factor, command, commits) in [(i64::MAX, 17010, false), (50, 17011, true)] {
        callback.experience_per_level = Some(factor);
        let bindings = oteryn_gameplay_quests::quest::authored_bindings::Bindings::parse(
            &serde_json::to_vec(&serde_json::json!({
                "schema": 1, "quests": [{
                    "quest": callback.quest,
                    "recipe_sha256": sha2::Sha256::digest(b"engine reward scaling fixture").iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
                    "sources": ["engine reward scaling fixture"],
                    "tracks": [{"key": callback.storage[0].track, "initial":0,"minimum":0,"maximum":1}],
                    "stages": [{"key":"reward","completes":false,"trigger":{"kind":"npc_dialogue","callback": &callback}}]
                }]
            }))?,
        )?;
        let catalogue =
            Arc::new(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("base")?)?);
        let outcome = h
            .root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                fixture::fence(1)?,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Command(super::engitem_reward_text::command(command)?),
                },
                catalogue,
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: None,
                    peer_use: None,
                },
                callback.clone(),
            )
            .await
            .map_err(debug)?;
        if commits {
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            let amount: i64 =
                sqlx::query_scalar("SELECT amount FROM game_character_quest_xp_obligations")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(amount, 2500);
        } else {
            assert!(matches!(outcome, QuestTransitionOutcome::Refused(_)));
            assert_eq!(h.root_revision().await?, "1");
            assert_eq!(h.count("game_character_quest_receipts").await?, 0);
            assert_eq!(h.count("game_character_quest_xp_obligations").await?, 0);
            assert_eq!(h.count("game_quest_reward_sources").await?, 0);
        }
    }
    Ok(())
}
