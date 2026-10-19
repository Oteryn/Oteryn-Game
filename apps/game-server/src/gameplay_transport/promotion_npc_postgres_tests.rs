//! PREM-5 promotion and Premium blessing end to end on PostgreSQL: one admitted Character talks
//! to a placed NPC through the real command-7 dispatch (`FreshAdmissionAuthority::npc_talk`).
//! `promotion` or a blessing name opens the offer, `yes` buys it through the native owner under
//! one Premium gate, and the coins-only fee burn, the receipt and the promotion row or held
//! blessing commit together. A retry of the same command replays that receipt and burns nothing
//! twice; a resumed connection repeating the CommandId reconciles it from the receipt
//! (`FreshAdmissionAuthority::resume_npc_bank`); a new command is refused.
//!
//! The offers come from the production composition: the node's
//! [`SourcePremiumNpcCatalogue::embedded`] over an NPC catalogue, installed with the NPC talk by
//! [`ComposedFreshAdmission::install_npcs`] as `serve_gameplay` does. The blessing prices (the
//! owner pricing ruling of 2026-10-18), NPC names and storage keys come from the embedded
//! BLESSINGS-CONTENT-1 source. The only fabricated inputs are the session fixture, the clock, the
//! Premium snapshot, the coins in the backpack and the NPC declarations: the pinned World content
//! declares no promotion or blessing Service yet.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::domain::currency::Coin;
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementFacing,
    MovementLocalPosition, NodeId, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_promotion::{
    BLESSING_OCCURRENCE_TAG, PROMOTION_OCCURRENCE_TAG, PremiumReconcile, promotion_command_id,
};
use crate::gameplay_transport::npc_talk::NpcTalkEntry;
use crate::gameplay_transport::premium_blessing_catalogue::SourcePremiumNpcCatalogue;
use crate::gameplay_transport::premium_blessing_catalogue::tests::npc_catalogue;
use crate::gameplay_transport::premium_promotion::PROMOTION_PRICE;
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use crate::gameplay_transport::{ComposedFreshAdmission, NpcTalkBook, NpcTalkOwners};
use crate::premium::{IngestOutcome, PremiumConsumer, TrustedNow};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// The entry start, and the open cell west of it for the NPC. NPC placement works in the
/// frame's legacy `z` (`0..=15`, native floor `-z`), so the start is legacy `z` 7.
const PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: 7,
};
/// The promotion NPC and its Service: `oteryn:service.promotion.` and the NPC's key stem.
const NPC_KEY: &str = "oteryn:npc.king_tibianus";
const SERVICE_KEY: &str = "oteryn:service.promotion.king_tibianus";
/// The source NPC of Blood of the Mountain (BLESSINGS-CONTENT-1: `Kais`).
const BLESSING_NPC_KEY: &str = "oteryn:npc.kais";
/// The Services the blessing NPC declares: its own promotion and the three Premium blessings.
/// Only Blood of the Mountain is offered at Kais: the Heart is Nomad's and the Wisdom Eremo's.
const BLESSING_SERVICE_KEYS: [&str; 4] = [
    "oteryn:service.promotion.kais",
    "oteryn:service.blessing.blood_of_the_mountain",
    "oteryn:service.blessing.heart_of_the_mountain",
    "oteryn:service.blessing.the_wisdom_of_solitude",
];
const BLOOD_KEY: &str = "oteryn:blessing.blood-of-the-mountain";
/// The Premium test clock: 2026-09-30T12:00:00Z, after the activation switch-over.
const PREMIUM_TEST_NOW: i64 = 1_790_769_600_000_000;
/// The fixture Character root's Content revision (`seed_character`).
const CONTENT_REVISION: &str = "content-1";
/// One intent per step stays well inside the `NPC0-RL-05` window of four per second.
const STEP: Duration = Duration::from_millis(400);
const SCENARIO_STACK_BYTES: usize = 64 << 20;
const BACKPACK: u8 = 60;
const CRYSTAL: u8 = 63;

/// Which Premium NPC service the scenario buys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    Promotion,
    Blessing,
}

impl Case {
    const fn npc_key(self) -> &'static str {
        match self {
            Self::Promotion => NPC_KEY,
            Self::Blessing => BLESSING_NPC_KEY,
        }
    }
}

struct PremiumClock(TrustedNow);
impl clock_registration::Registered for PremiumClock {}
impl RegisteredPremiumClock for PremiumClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        Some(self.0)
    }
}

/// A delivered, Current Premium for the fixture account through the real snapshot consumer.
async fn current_premium(harness: &Harness) -> TestResult<SpellPremiumCoordinator> {
    let consumer = Arc::new(PremiumConsumer::default());
    let nonce = "6b1f0c2d9e8a47f3a1b2c3d4e5f60718";
    let response = crate::premium::test_producer::snapshot(
        &crate::premium::test_producer::ProducerRequest {
            account_id: crate::premium::snapshot::canonical_uuid(id(40)),
            nonce: nonce.into(),
        },
        7,
        &[],
    );
    assert!(matches!(
        consumer
            .ingest(&harness.root, id(40), nonce, &response)
            .await,
        IngestOutcome::Accepted
    ));
    Ok(SpellPremiumCoordinator::new(
        consumer,
        Some(Arc::new(PremiumClock(
            TrustedNow::new(PREMIUM_TEST_NOW, 1).ok_or("trusted now")?,
        )) as Arc<dyn RegisteredPremiumClock>),
        Some(PremiumActivation {
            activation_id: "prem-5-blessing-test".into(),
            switch_over_us: PREMIUM_TEST_NOW - 60_000_000,
        }),
        Some(harness.root.clone()),
    ))
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
    on_scenario_stack("promotion-npc-e2e", Case::Promotion)
}

#[test]
fn blessing_talk_through_dispatch_burns_fee_and_blesses() -> TestResult {
    on_scenario_stack("blessing-npc-e2e", Case::Blessing)
}

fn on_scenario_stack(name: &str, case: Case) -> TestResult {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(SCENARIO_STACK_BYTES)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Premium NPC e2e thread panicked")?
        .map_err(Into::into)
}

#[allow(clippy::too_many_lines)]
fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let name = match case {
            Case::Promotion => "promotion_npc_e2e",
            Case::Blessing => "blessing_npc_e2e",
        };
        let harness = Harness::create_with_admission_seed(admin, name, true, false).await?;
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
                key: case.npc_key().to_owned(),
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
        // The promotion: before Premium delivery, the WHEEL-0 §6.2 exception through the real
        // latch. A blessing has no such exception: it needs a Current Premium.
        let premium_gate = match case {
            Case::Promotion => {
                SpellPremiumCoordinator::new(Arc::default(), None, None, Some(harness.root.clone()))
            }
            Case::Blessing => current_premium(&harness).await?,
        };
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let mut owner = ComposedFreshAdmission {
            root: &harness.root,
            character: &authority,
            holder: &harness.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            // Gameplay-only fixture: it never admits through this owner, so the ADMIT0 binding
            // declares nothing and only has to be a valid Channel boot binding.
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .expect("an empty declared set binds"),
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
            loop_ground: Default::default(),
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
                        (0, 0),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }
        // The session's chat state serving command 7, and the NPC's talk and Premium services
        // composed as the node composes them (`node::serve`, `serve_gameplay`).
        owner.chat.admit(session, "Fixture Hero", owner.owner_now());
        owner.chat.set_npc_service(session);
        let (npc_name, declared, offers): (_, &[&str], _) = match case {
            Case::Promotion => ("King Tibianus", &[SERVICE_KEY], (1, 0)),
            Case::Blessing => ("Kais", &BLESSING_SERVICE_KEYS, (1, 1)),
        };
        let services = SourcePremiumNpcCatalogue::embedded(
            Arc::new(npc_catalogue(&[(case.npc_key(), declared)], declared)),
            CONTENT_REVISION.to_owned(),
        )
        .map_err(debug)?;
        assert_eq!(services.offer_counts(), offers);
        assert!(owner.install_npcs(NpcTalkOwners {
            book: Arc::new(
                NpcTalkBook::default().with(case.npc_key(), NpcTalkEntry::new(npc_name, None)),
            ),
            premium_services: Some(Arc::new(services)),
        }));

        let before = revision(&harness).await?;
        assert_eq!(before, 2);
        assert_eq!(crystals(&harness).await?, 3);
        let at = (actor, session, npc);
        match case {
            Case::Promotion => promotion_steps(&harness, &mut owner, at, before).await?,
            Case::Blessing => blessing_steps(&harness, &mut owner, at, before).await?,
        }

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

type At = (ExactActorRef, GameSessionId, EntityRef);

/// The promotion through dispatch, its replay, its reconcile after a lost outcome and its refusal.
async fn promotion_steps(
    harness: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    at: At,
    before: u64,
) -> TestResult {
    // The greeting opens the conversation; the topic prompts the offer and writes nothing.
    talk(owner, at, 1, "hello").await;
    let (prompt, _) = talk(owner, at, 2, "promotion").await;
    assert_eq!(
        prompt,
        vec![format!(
            "Do you want to be promoted in your vocation for {PROMOTION_PRICE} gold?"
        )]
    );
    assert_eq!(promotion_receipts(harness).await?, 0);

    // `yes` buys it: one receipt, one promotion row, a 20000 coins-only fee burn and one
    // committed revision, all together.
    let (bought, changed) = talk(owner, at, 3, "yes").await;
    assert_eq!(
        bought,
        vec!["Congratulations! You are now promoted.".to_owned()]
    );
    assert!(changed);
    assert_eq!(promotion_receipts(harness).await?, 1);
    let premium_admission: i16 =
        sqlx::query_scalar("SELECT premium_admission FROM game_character_promotion_receipts")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(premium_admission, 2, "admitted before Premium delivery");
    receipt_binds_its_command(harness, Case::Promotion, at, 3).await?;
    let promoted: bool = sqlx::query_scalar(
        "SELECT promoted FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert!(promoted);
    assert_eq!(fee_burns(harness).await?, vec![(2, 20_000)]);
    assert_eq!(revision(harness).await?, before + 1);
    assert_eq!(crystals(harness).await?, 1);

    // The same command again replays its receipt: no second burn or revision.
    talk(owner, at, 4, "promotion").await;
    let (replayed, _) = talk(owner, at, 3, "yes").await;
    assert_eq!(
        replayed,
        vec!["Congratulations! You are now promoted.".to_owned()]
    );
    assert_eq!(promotion_receipts(harness).await?, 1);
    assert_eq!(fee_burns(harness).await?, vec![(2, 20_000)]);
    assert_eq!(revision(harness).await?, before + 1);

    // A resumed connection whose first command repeats CommandId 3 after a lost outcome: the
    // conversation (and its pending offer) is gone, and the receipt alone answers it.
    reconciles(owner, at, 3, "Congratulations! You are now promoted.").await;
    assert_eq!(fee_burns(harness).await?, vec![(2, 20_000)]);
    assert_eq!(revision(harness).await?, before + 1);

    // A new command is refused by the durable promotion and writes nothing.
    talk(owner, at, 5, "promotion").await;
    let (refused, changed) = talk(owner, at, 6, "yes").await;
    assert_eq!(refused, vec!["You are already promoted!".to_owned()]);
    assert!(!changed);
    assert_eq!(promotion_receipts(harness).await?, 1);
    assert_eq!(fee_burns(harness).await?, vec![(2, 20_000)]);
    assert_eq!(revision(harness).await?, before + 1);
    assert_eq!(crystals(harness).await?, 1);

    // King Tibianus is no blessing source NPC: the topic fails closed with no offer and no
    // purchase.
    let (blessing, _) = talk(owner, at, 7, "wisdom of solitude").await;
    assert!(
        !blessing
            .iter()
            .any(|line| line.contains("Wisdom of Solitude")),
        "{blessing:?}"
    );
    Ok(())
}

/// A promotion at Kais, then the source-backed Blood of the Mountain through dispatch at its
/// level-50 source price for the promoted buyer (a promotion has no price effect), its replay,
/// its reconcile, the death hook's held set, and the fail-closed offers.
#[allow(clippy::too_many_lines)]
async fn blessing_steps(
    harness: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    at: At,
    before: u64,
) -> TestResult {
    // Owner ruling 2026-10-18 (`enhanced`): 2600 + 260 * (L - 30) gold for 30 < L < 120.
    const BLOOD_PRICE: i64 = 2_600 + 260 * (50 - 30);
    let (actor, session, _) = at;
    talk(owner, at, 1, "hello").await;

    // Kais declares its own promotion Service: two crystal coins burn and the buyer is promoted.
    talk(owner, at, 2, "promotion").await;
    let (promoted, changed) = talk(owner, at, 3, "yes").await;
    assert_eq!(
        promoted,
        vec!["Congratulations! You are now promoted.".to_owned()]
    );
    assert!(changed);
    assert_eq!(fee_burns(harness).await?, vec![(2, 20_000)]);
    assert_eq!(crystals(harness).await?, 1);
    let before = before + 1;
    assert_eq!(revision(harness).await?, before);

    let (prompt, _) = talk(owner, at, 4, "blood of the mountain").await;
    assert_eq!(
        prompt,
        vec!["Do you want to receive the Blood of the Mountain blessing?".to_owned()]
    );
    assert_eq!(fee_burns(harness).await?, vec![(2, 20_000)]);

    // `yes` buys it at the unpromoted price: one receipt with the writer-selected band, one held
    // blessing, a coins-first fee burn with change, and one committed revision, all together.
    let fees = vec![(2, 20_000), (3, BLOOD_PRICE)];
    let (bought, changed) = talk(owner, at, 5, "yes").await;
    assert_eq!(bought, vec!["You have been blessed.".to_owned()]);
    assert!(changed);
    assert_eq!(fee_burns(harness).await?, fees);
    assert_eq!(held_blessings(harness).await?, vec![BLOOD_KEY.to_owned()]);
    let receipt: (String, i64, i64, i64, i16) = sqlx::query_as(
        "SELECT blessing_key, price_gold_units::bigint, price_minimum_level, \
                price_maximum_level, blessing_service \
           FROM game_character_blessing_purchase_receipts",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(receipt, (BLOOD_KEY.to_owned(), BLOOD_PRICE, 31, 119, 2));
    receipt_binds_its_command(harness, Case::Blessing, at, 5).await?;
    assert_eq!(revision(harness).await?, before + 1);
    assert_eq!(crystals(harness).await?, 0);

    // The same command again replays its receipt: no second burn or revision.
    talk(owner, at, 6, "blood of the mountain").await;
    let (replayed, _) = talk(owner, at, 5, "yes").await;
    assert_eq!(replayed, vec!["You have been blessed.".to_owned()]);
    assert_eq!(fee_burns(harness).await?, fees);
    assert_eq!(revision(harness).await?, before + 1);

    // A resumed connection repeating CommandId 5 reconciles it from the receipt alone; an
    // unrelated command has no Premium purchase.
    reconciles(owner, at, 5, "You have been blessed.").await;
    assert_eq!(
        owner
            .reconcile_premium_purchase(CommandRef::new(session, CommandId::new(99)?))
            .await,
        PremiumReconcile::None
    );
    assert_eq!(fee_burns(harness).await?, fees);
    assert_eq!(revision(harness).await?, before + 1);

    // DEATH_COMPLETE0 §4.7: a death consumes exactly the held set.
    let fence = owner
        .promotion_actor_facts(actor, session)
        .await
        .ok_or("current actor facts")?
        .fence;
    let death = crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(77))
        .map_err(debug)?;
    assert_eq!(
        owner.death_held_blessings(fence, death).await,
        Some(vec![BLOOD_KEY.to_owned()])
    );

    // A new command is refused by the held blessing and writes nothing.
    talk(owner, at, 7, "blood of the mountain").await;
    let (refused, changed) = talk(owner, at, 8, "yes").await;
    assert_eq!(
        refused,
        vec!["You already possess this blessing.".to_owned()]
    );
    assert!(!changed);
    assert_eq!(fee_burns(harness).await?, fees);
    assert_eq!(revision(harness).await?, before + 1);

    // Kais declares the Heart and the Wisdom too, but their source NPCs are Nomad and Eremo, so
    // neither is offered here.
    for (command, topic, name) in [
        (9, "heart of the mountain", "Heart of the Mountain"),
        (10, "wisdom of solitude", "Wisdom of Solitude"),
    ] {
        let (lines, _) = talk(owner, at, command, topic).await;
        assert!(!lines.iter().any(|line| line.contains(name)), "{lines:?}");
    }
    assert_eq!(fee_burns(harness).await?, fees);
    assert_eq!(revision(harness).await?, before + 1);
    Ok(())
}

/// The resumed connection's original command through `resume_npc_bank`: the committed Premium
/// purchase answers it with its line and nothing is written. The receipt alone answers it even
/// before the actor's spell state is back, so the answer needs no live actor facts.
async fn reconciles(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    (actor, session, npc): At,
    command_id: u64,
    line: &str,
) {
    let command = CommandRef::new(session, CommandId::new(command_id).expect("a command id"));
    let spells = std::mem::take(&mut *owner.spell_states.lock().await);
    assert!(
        owner.promotion_actor_facts(actor, session).await.is_none(),
        "no live actor facts while the actor re-enters"
    );
    let reconciled = owner.reconcile_premium_purchase(command).await;
    *owner.spell_states.lock().await = spells;
    assert!(
        matches!(
            reconciled,
            PremiumReconcile::Committed { npc: from, line: answer } if from == npc && answer == line
        ),
        "{reconciled:?}"
    );
    let outcome = owner
        .resume_npc_bank(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: None,
            },
        )
        .await
        .expect("the committed purchase answers the original command");
    assert!(!outcome.unknown);
    assert!(outcome.inventory_changed);
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    let conversation = outcome.conversation.expect("the purchase line");
    assert_eq!(conversation.open.map(|(talking, _)| talking), Some(npc));
    assert_eq!(conversation.lines, vec![line.to_owned()]);
}

/// The single receipt of `case` stores the CommandRef it answers, and the schema derives its
/// occurrence from that CommandRef: the SQL derivation is `promotion_command_id` (also above
/// `i64::MAX`), and a copy of the receipt under another CommandId fails its derivation CHECK.
async fn receipt_binds_its_command(
    harness: &Harness,
    case: Case,
    (_, session, npc): At,
    command_id: u64,
) -> TestResult {
    let (tag, stored, copy, insert, constraint) = match case {
        Case::Promotion => (
            PROMOTION_OCCURRENCE_TAG,
            "SELECT game_session_id = encode($1,'hex')::uuid, command_id::text \
               FROM game_character_promotion_receipts",
            "CREATE TEMP TABLE copied ON COMMIT DROP AS \
             SELECT * FROM game_character_promotion_receipts",
            "INSERT INTO game_character_promotion_receipts SELECT * FROM copied",
            "game_character_promotion_receipts_command_occurrence",
        ),
        Case::Blessing => (
            BLESSING_OCCURRENCE_TAG,
            "SELECT game_session_id = encode($1,'hex')::uuid, command_id::text \
               FROM game_character_blessing_purchase_receipts",
            "CREATE TEMP TABLE copied ON COMMIT DROP AS \
             SELECT * FROM game_character_blessing_purchase_receipts",
            "INSERT INTO game_character_blessing_purchase_receipts SELECT * FROM copied",
            "game_character_blessing_purchase_receipts_command_occurrence",
        ),
    };
    let row: (bool, String) = sqlx::query_as(stored)
        .bind(session.as_bytes().as_slice())
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(row, (true, command_id.to_string()));

    for command in [command_id, u64::MAX] {
        let derived: String = sqlx::query_scalar(
            "SELECT replace(game_premium_command_occurrence($1, encode($2,'hex')::uuid, \
                    $3::text::numeric, encode($4,'hex')::uuid, $5::text::numeric)::text, '-', '')",
        )
        .bind(tag)
        .bind(session.as_bytes().as_slice())
        .bind(command.to_string())
        .bind(npc.identity.as_slice())
        .bind(npc.generation.to_string())
        .fetch_one(&harness.pool)
        .await?;
        let expected: String =
            promotion_command_id(tag, CommandRef::new(session, CommandId::new(command)?), npc)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
        assert_eq!(derived, expected, "command {command}");
    }

    let mut tx = harness.pool.begin().await?;
    sqlx::query(copy).execute(&mut *tx).await?;
    sqlx::query("UPDATE copied SET command_id = command_id + 1")
        .execute(&mut *tx)
        .await?;
    let refused = sqlx::query(insert)
        .execute(&mut *tx)
        .await
        .expect_err("an occurrence not derived from its CommandRef is refused");
    assert_eq!(
        refused
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::constraint),
        Some(constraint),
        "{refused}"
    );
    tx.rollback().await?;
    Ok(())
}

async fn held_blessings(harness: &Harness) -> TestResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT blessing_key FROM game_character_blessings \
          WHERE character_id=encode($1,'hex')::uuid ORDER BY 1",
    )
    .bind(id(41).as_slice())
    .fetch_all(&harness.pool)
    .await?)
}

/// One command-7 intent through the real dispatch, one step later on the owner clock.
async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    (actor, session, npc): At,
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
        "SELECT cause_kind, fee_gold_units::bigint FROM game_item_fee_burns \
         ORDER BY cause_kind, fee_gold_units",
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
