//! PREM-5 promotion end to end on PostgreSQL: one admitted Character talks to a placed NPC
//! through the real command-7 dispatch (`FreshAdmissionAuthority::npc_talk`). `promotion` opens
//! the offer, `yes` buys it through the native owner under the pre-delivery promotion gate, and
//! the coins-only fee burn, the promotion receipt and the promotion row commit together. A retry
//! of the same command replays that receipt and burns nothing twice; a new command is refused.
//!
//! The only fabricated inputs are the session fixture, the clock, the coins in the backpack and
//! the Premium NPC service catalogue: no qualified promotion NPC source is pinned in production
//! yet, so the test installs one through the same seam a pinned source would use.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::content::{
    CanonicalProjectDocuments, NpcDataCatalogue, ProjectDraft, ProjectV2CandidateField,
    ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Draft, ProjectV2Family,
    ProjectV2Identity, ProjectV2State, npc_catalogue_preproduction_limits,
};
use crate::domain::currency::Coin;
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementFacing,
    MovementLocalPosition, NodeId, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::NpcTalkBook;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_promotion::{
    BlessingNpcOffer, PremiumBlessingService, PremiumNpcServiceCatalogue, PromotionNpcOffer,
};
use crate::gameplay_transport::npc_talk::NpcTalkEntry;
use crate::gameplay_transport::premium_promotion::PROMOTION_PRICE;
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// A Thalom cell outside the protection zone, and the open cell west of it for the NPC.
const PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const NPC_KEY: &str = "oteryn:npc.promotion-test";
const SERVICE_KEY: &str = "oteryn:service.promotion-test";
/// The fixture Character root's Content revision (`seed_character`).
const CONTENT_REVISION: &str = "content-1";
/// One intent per step stays well inside the `NPC0-RL-05` window of four per second.
const STEP: Duration = Duration::from_millis(400);
const SCENARIO_STACK_BYTES: usize = 64 << 20;
const BACKPACK: u8 = 60;
const CRYSTAL: u8 = 63;

/// The test catalogue: one NPC whose one Service is the promotion. No blessing is offered:
/// no blessing price table is sourced, so the seam answers none (fail closed).
struct PromotionCatalogue {
    npcs: NpcDataCatalogue,
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

impl PremiumNpcServiceCatalogue for PromotionCatalogue {
    fn content_revision(&self) -> &str {
        CONTENT_REVISION
    }
    fn npcs(&self) -> &NpcDataCatalogue {
        &self.npcs
    }
    fn promotion(&self, native_npc_key: &str) -> Option<PromotionNpcOffer> {
        (native_npc_key == NPC_KEY).then(|| PromotionNpcOffer {
            npc: reference(ProjectV2Family::Npc, NPC_KEY),
            service: reference(ProjectV2Family::Service, SERVICE_KEY),
        })
    }
    fn blessing(&self, _: &str, _: PremiumBlessingService) -> Option<BlessingNpcOffer> {
        None
    }
}

fn npc_catalogue() -> NpcDataCatalogue {
    let identity = |key: &str| ProjectV2Identity {
        key: key.into(),
        revision: "definition-r1".into(),
    };
    let draft = ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "promotion-r1".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "license:project-owned-v1".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: vec![],
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            declarations: vec![
                ProjectV2Declaration::Npc {
                    identity: identity(NPC_KEY),
                    presentation: None,
                    behavior: None,
                    dialogue: None,
                    services: vec![reference(ProjectV2Family::Service, SERVICE_KEY)],
                    fields: vec![],
                },
                ProjectV2Declaration::Service {
                    bank: false,
                    identity: identity(SERVICE_KEY),
                    offers: vec![],
                    recipes: vec![],
                    routes: vec![],
                    fields: Vec::<ProjectV2CandidateField>::new(),
                },
            ],
            ..ProjectV2State::default()
        },
    };
    let limits = npc_catalogue_preproduction_limits().project;
    let project = CanonicalProjectDocuments::from_v2_draft(draft, limits)
        .expect("native documents")
        .into_snapshot(limits)
        .expect("snapshot")
        .parse(limits)
        .expect("native project");
    NpcDataCatalogue::from_project(project, limits).expect("native NPC catalogue")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

/// The independently declared vocation choice of the Character (revision 1 to 2), with its
/// real build receipt: lifecycle reads need a classed build, and the progression guard binds the
/// build row to its receipt chain. Level and experience stay the fixture's 50 and 1000.
fn vocation_choice() -> String {
    let columns = "magic_level{s}, mana_spent{s}, fist_level{s}, fist_tries{s}, club_level{s}, \
                   club_tries{s}, sword_level{s}, sword_tries{s}, axe_level{s}, axe_tries{s}, \
                   distance_level{s}, distance_tries{s}, shielding_level{s}, shielding_tries{s}, \
                   fishing_level{s}, fishing_tries{s}";
    let seed = "0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0";
    let character = uuid(41);
    let occurrence = uuid(91);
    format!(
        "UPDATE game_character_roots SET character_revision = 2 \
          WHERE character_id = {character} AND character_revision = 1; \
         UPDATE game_character_progression_state SET character_revision = 2 \
          WHERE character_id = {character} AND character_revision = 1; \
         INSERT INTO game_character_build_receipts(\
           build_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, cause, level_before, \
           level_after, experience_before, experience_after, vocation_before, {before_cols}, \
           vocation_after, {after_cols}, stance_before, stance_after, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           1, 2, 'vocation_choice', 50, 50, 1000, 1000, 'none', {seed}, 'sorcerer', {seed}, \
           NULL, NULL, 'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
           'declaration-1', 'policy-1', 'reward-1', 2); \
         INSERT INTO game_character_build_state(character_id, vocation, {plain}, \
           committed_character_revision, last_build_occurrence_id) \
         VALUES ({character}, 'sorcerer', {seed}, 2, {occurrence});",
        before_cols = columns.replace("{s}", "_before"),
        after_cols = columns.replace("{s}", "_after"),
        plain = columns.replace("{s}", ""),
        binding = hex(&[91; 33]),
        digest = hex(&[91; 32]),
    )
}

/// Fixture item rows as the migration owner with every trigger off, as the item cases do.
async fn seed_items(harness: &Harness, script: &str) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn item(seed: u8, key: &str, revision: &str, quantity: u32) -> String {
    format!(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES ({}, {}, 'Item', '{key}', '{revision}', {quantity}, 1, {});",
        uuid(seed),
        uuid(42),
        uuid(seed + 1),
    )
}

#[test]
fn promotion_talk_through_dispatch_burns_fee_and_promotes() -> TestResult {
    std::thread::Builder::new()
        .name("promotion-npc-e2e".into())
        .stack_size(SCENARIO_STACK_BYTES)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "promotion NPC e2e thread panicked")?
        .map_err(Into::into)
}

#[allow(clippy::too_many_lines)]
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "promotion_npc_e2e", true, false).await?;
        prepare_current_session(&harness).await?;
        {
            let mut tx = harness.pool.begin().await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(vocation_choice()))
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let (controller, pin) = crate::content::native_gameplay::tests::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let index = native
            .item_index()
            .expect("the generation pins an Item index");
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (PLAYER.x, PLAYER.y, PLAYER.floor),
        );

        // The equipment owner's empty seed (as fresh admission leaves it), the Channel's durably
        // active Content and the control grant the lifecycle read checks.
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice())
            .execute(&harness.pool)
            .await?;

        // The main backpack (the generation's own definition) holding three crystal coins.
        let backpack = native
            .item_policy_for_source_id(2854)
            .expect("the backpack policy")
            .record()
            .production_definition
            .clone();
        let crystal = Coin::Crystal.production_key();
        let crystal_revision = index.revision(crystal).expect("the crystal coin revision");
        seed_items(
            &harness,
            &format!(
                "{backpack_item} \
                 INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
                   placed_transaction_id) VALUES ({character}, {backpack_id}, {world}, {placed}); \
                 {crystal_item} \
                 INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
                   parent_item_instance_id, placement_ordinal, placed_transaction_id) \
                 VALUES ({crystal_id}, {world}, {character}, {backpack_id}, 1, {crystal_placed});",
                backpack_item = item(
                    BACKPACK,
                    &backpack.production_key,
                    &backpack.revision_ref,
                    1
                ),
                character = uuid(41),
                backpack_id = uuid(BACKPACK),
                world = uuid(42),
                placed = uuid(BACKPACK + 2),
                crystal_item = item(CRYSTAL, crystal, crystal_revision, 3),
                crystal_id = uuid(CRYSTAL),
                crystal_placed = uuid(CRYSTAL + 2),
            ),
        )
        .await?;

        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let spot = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let table = NpcPlacementTable {
            npcs: vec![NpcPlacements {
                key: NPC_KEY.to_owned(),
                placements: vec![NpcPlacement {
                    floor: -i8::try_from(spot.floor)?,
                    y: u16::try_from(spot.y)?,
                    x: u16::try_from(spot.x - 1)?,
                    direction: MovementFacing::East,
                }],
            }],
        };
        runtime.place_npcs(&table, |_| true, true).map_err(debug)?;
        let npc = {
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };

        let door_content =
            crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                .map_err(debug)?;
        let fence = super::fixture::fence(1)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            door_content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence =
            crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        // Before Premium delivery: the WHEEL-0 §6.2 promotion exception through the real latch.
        let premium_gate =
            SpellPremiumCoordinator::new(Arc::default(), None, None, Some(harness.root.clone()));
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let mut owner = ComposedFreshAdmission {
            root: &harness.root,
            character: &authority,
            holder: &harness.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            runtime: &runtime,
            movement_cells: room.movement_cells(),
            door: &door,
            chest: room.door(),
            spells: &spells,
            active_generation: Some(active),
            premium_coordinator: Some(&premium_gate),
            rooms: crate::gameplay_transport::SeamRooms::new(None, Some(&room)),
            achievements: &achievements,
            imported_charms: &charms,
            spell_states: Mutex::default(),
            world_gate: std::sync::Mutex::default(),
            attack: Mutex::default(),
            clock_origin: std::time::Instant::now()
                .checked_sub(Duration::from_secs(60))
                .expect("a monotonic clock older than a minute"),
            lost: std::sync::Mutex::default(),
            revision_sequencer:
                crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
            quest_catalogue: None,
            quest_sessions: std::sync::Mutex::default(),
            chat: crate::gameplay_transport::chat_intent::ChatRuntime::default(),
            npc_conversations: Default::default(),
            wheel_ruleset: None,
            wheel_sessions: std::sync::Mutex::default(),
            progression_sessions: std::sync::Mutex::default(),
            last_positions: crate::gameplay_transport::last_position::LastPositions::default(),
            input_gate: Default::default(),
            premium: crate::premium::refresh::PremiumRefresher::new(
                Default::default(),
                harness.root.clone(),
                None,
            ),
            premium_sessions: std::sync::Mutex::default(),
            fence_holders: Default::default(),
            spell_lane: crate::durability::spell_owner_commit::SpellLane::new(world, channel),
        };
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::gameplay_transport::actor_spell::tests::FACTS,
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }
        // The session's chat state serving command 7, with the NPC's talk entry, and the
        // Premium service seam a pinned NPC source would fill.
        owner.chat.admit(session, "Fixture Hero", owner.owner_now());
        owner.chat.set_npc_service(session);
        owner.chat.install_npc_talk(Arc::new(
            NpcTalkBook::default().with(NPC_KEY, NpcTalkEntry::new("Aldric", None)),
        ));
        assert!(
            owner
                .npc_conversations
                .install_premium_services(Arc::new(PromotionCatalogue {
                    npcs: npc_catalogue(),
                }))
        );

        let before = revision(&harness).await?;
        assert_eq!(before, 2);
        assert_eq!(crystals(&harness).await?, 3);

        // The greeting opens the conversation; the topic prompts the offer and writes nothing.
        talk(&mut owner, (actor, session, npc), 1, "hello").await;
        let (prompt, _) = talk(&mut owner, (actor, session, npc), 2, "promotion").await;
        assert_eq!(
            prompt,
            vec![format!(
                "Do you want to be promoted in your vocation for {PROMOTION_PRICE} gold?"
            )]
        );
        assert_eq!(promotion_receipts(&harness).await?, 0);

        // `yes` buys it: one receipt, one promotion row, a 20000 coins-only fee burn and one
        // committed revision, all together.
        let (bought, changed) = talk(&mut owner, (actor, session, npc), 3, "yes").await;
        assert_eq!(
            bought,
            vec!["Congratulations! You are now promoted.".to_owned()]
        );
        assert!(changed);
        assert_eq!(promotion_receipts(&harness).await?, 1);
        let premium_admission: i16 =
            sqlx::query_scalar("SELECT premium_admission FROM game_character_promotion_receipts")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(premium_admission, 2, "admitted before Premium delivery");
        let promoted: bool = sqlx::query_scalar(
            "SELECT promoted FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert!(promoted);
        assert_eq!(fee_burns(&harness).await?, vec![(2, 20_000)]);
        assert_eq!(revision(&harness).await?, before + 1);
        assert_eq!(crystals(&harness).await?, 1);

        // The same command again replays its receipt: no second burn or revision.
        talk(&mut owner, (actor, session, npc), 4, "promotion").await;
        let (replayed, _) = talk(&mut owner, (actor, session, npc), 3, "yes").await;
        assert_eq!(
            replayed,
            vec!["Congratulations! You are now promoted.".to_owned()]
        );
        assert_eq!(promotion_receipts(&harness).await?, 1);
        assert_eq!(fee_burns(&harness).await?, vec![(2, 20_000)]);
        assert_eq!(revision(&harness).await?, before + 1);

        // A new command is refused by the durable promotion and writes nothing.
        talk(&mut owner, (actor, session, npc), 5, "promotion").await;
        let (refused, changed) = talk(&mut owner, (actor, session, npc), 6, "yes").await;
        assert_eq!(refused, vec!["You are already promoted!".to_owned()]);
        assert!(!changed);
        assert_eq!(promotion_receipts(&harness).await?, 1);
        assert_eq!(fee_burns(&harness).await?, vec![(2, 20_000)]);
        assert_eq!(revision(&harness).await?, before + 1);
        assert_eq!(crystals(&harness).await?, 1);

        // No blessing source is sourced: the topic fails closed with no offer and no purchase.
        let (blessing, _) = talk(&mut owner, (actor, session, npc), 7, "wisdom of solitude").await;
        assert!(
            !blessing
                .iter()
                .any(|line| line.contains("Wisdom of Solitude")),
            "{blessing:?}"
        );

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// One command-7 intent through the real dispatch, one step later on the owner clock.
async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    (actor, session, npc): (ExactActorRef, GameSessionId, EntityRef),
    command_id: u64,
    text: &str,
) -> (Vec<String>, bool) {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(STEP)
        .expect("a monotonic clock older than the steps");
    let outcome = owner
        .npc_talk(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: None,
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.to_owned(),
            },
        )
        .await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
    assert!(!outcome.unknown, "{text}");
    let lines = outcome
        .conversation
        .map(|conversation| conversation.lines)
        .unwrap_or_default();
    (lines, outcome.inventory_changed)
}

async fn revision(harness: &Harness) -> TestResult<u64> {
    let value: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    Ok(value.parse()?)
}

async fn promotion_receipts(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_promotion_receipts")
            .fetch_one(&harness.pool)
            .await?,
    )
}

async fn fee_burns(harness: &Harness) -> TestResult<Vec<(i16, i64)>> {
    Ok(sqlx::query_as(
        "SELECT cause_kind, fee_gold_units::bigint FROM game_item_fee_burns ORDER BY transaction_id",
    )
    .fetch_all(&harness.pool)
    .await?)
}

async fn crystals(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(i.quantity), 0)::bigint FROM game_item_container_entries e \
         JOIN game_item_instances i USING (item_instance_id) \
         WHERE i.definition_production_key = $1 AND i.lifecycle = 1",
    )
    .bind(Coin::Crystal.production_key())
    .fetch_one(&harness.pool)
    .await?)
}
