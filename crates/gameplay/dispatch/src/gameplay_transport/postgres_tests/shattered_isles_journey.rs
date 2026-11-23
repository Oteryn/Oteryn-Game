//! The chosen Shattered Isles journey on the compiled World and embedded quest catalogue.

use std::{ffi::OsStr, path::Path, sync::Arc, time::Duration};

use super::fixture::{Harness, debug, id};
use super::m39_bone_journey_fixtures::carried;
use super::quest_world_callbacks_fixtures::{input_item, wire};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
    reset,
};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::{
    fresh_admission::FreshAdmissionStore, item_mint::TypedDefinitionRef,
    starter_grant::StarterTemplate,
};
use crate::foundation::{
    ChannelContentPin, MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable,
    NpcPlacements,
};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities,
    connection::{FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, SessionContinuity},
    item_ref_admission::NativeItemWeights,
    npc_talk::NpcTalkBook,
    quest_log::{QuestLogLines, QuestLogObservation},
    spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture},
};
use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state,
    log::loader::load_embedded_quest_log,
    world_callbacks::{Offering, Position},
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::{
    logout::LogoutResult,
    npc_service::{
        COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent,
        decode_npc_intent_result, encode_npc_talk_intent,
    },
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
        encode_use_intent,
    },
    world_spatial::StepDirection,
    world_spatial_entities::EntityRef,
};

const NAME: &str = "shattered_isles_full_journey_earns_four_passages";
const QUEST: &str = "oteryn:quest.the_shattered_isles_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u7_8/the_shattered_isles/";
const CLAIM: &str = "oteryn:reward-claim.quest.u7_8.the_shattered_isles.dragahs_spellbook";
const NPCS: &[&str] = &[
    "captain_waverider",
    "charlotta",
    "eleonore",
    "elvith",
    "jack_fate",
    "jack_fate_goroma",
    "klaus",
    "marina",
    "ocelus",
    "raymond_striker",
    "sebastian",
];

#[test]
fn shattered_isles_full_journey_earns_four_passages() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        scenario(admin).await
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h =
        Harness::create_with_admission_seed(admin, "shattered_isles_journey", false, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = super::fence(1)?.runtime_scope.world_id();
    let (controller, activation_pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let source = load_embedded_quest_state("content-1").map_err(debug)?;
    let journal = load_embedded_quest_log(&source)?;
    let journal_index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|entry| entry.key == QUEST)
            .ok_or("Shattered Isles journal index")?,
    )?;
    let world = super::qc_pinned_world::world(native)?;
    let (bytes, _) = super::qc_pinned_world::artifact()?;
    let npcs = placements(&bytes)?;
    let claims = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|_| true,
    )?;
    let spellbook = claims
        .served
        .iter()
        .find(|row| row.key.as_str() == CLAIM)
        .ok_or("qualified Dragah's Spellbook placement")?;
    assert_eq!(spellbook.placements.len(), 1);
    let cells = world
        .movement_cells(activation_pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (32330, 32763, 5),
    );
    let deps = Dependencies::with_content_and_capacity(
        pin,
        activation_pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        64,
    )?;
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    let mut future_activation = activation.clone();
    future_activation.switch_over_us += 120_000_000;
    let before_activation = SpellPremiumCoordinator::new(
        Arc::clone(&consumer),
        Some(Arc::clone(&clock)),
        Some(future_activation),
        Some(h.root.clone()),
    );
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation.clone()),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.premium_coordinator = Some(&before_activation);
    owner.quest_catalogue = Some(Arc::new(source.catalogue().clone()));
    owner.quest_log_content = Some(QuestLogLines::new(journal.content).ok_or("quest journal")?);
    let npcs_catalogue = NpcDataCatalogue::load(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = npcs_catalogue.service_model_for_project().map_err(debug)?;
    let authored = oteryn_gameplay_quests::quest::authored_bindings::native_dialogue_npcs()?;
    let native_contacts: Vec<_> = authored.iter().map(String::as_str).collect();
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &model,
            &native_contacts,
        )));
    let mut admitted = deps.admitted;
    let session = admitted.game_session_id;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(session)
        .await?;
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &[3, 4, 6, 11, 13, 15, 16, 18])
            .ok_or("journey capabilities")?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .bind_attacker_lease(deps.actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime.place_npcs(&npcs, |_| true, true).map_err(debug)?;
    }
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("current character")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let request = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            request.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: item(2854),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    owner.admit_premium(id(40), session);
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    deps.initialize(&owner).await;
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let mut journey = Journey {
        owner: &mut owner,
        admitted: &mut admitted,
        h: &h,
        npcs: &npcs,
        deps: &deps,
        journal_index,
    };
    let eleonore = journey.contact("eleonore").await?;
    assert_eq!(
        journey.talk(eleonore, "ring").await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(journey.track("the_governor_daughter").await?, None);
    journey.supply(3035, 10, 100).await?;
    journey.confirm_storm_roll(false).await?;
    super::qengine_outfit_stories_fixtures::earn_level(
        journey.owner,
        journey.admitted,
        &authority,
        20,
    )
    .await?;
    Box::pin(journey.owner.refresh_level_up_vitals(deps.actor, session)).await;
    // Legacy Premium travel waives its restriction before activation. A
    // shipwreck still requires a live entitlement to start the repair quest.
    journey.confirm_storm_roll(false).await?;
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    journey.owner.premium_coordinator = Some(&premium);
    journey.run().await?;
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn placements(bytes: &[u8]) -> TestResult<NpcPlacementTable> {
    use oteryn_world_bundle::npc::Direction;
    let mut found = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))
        .map_err(debug)?
        .npcs
        .npcs;
    found.retain(|npc| {
        npc.key
            .strip_prefix("oteryn:npc.")
            .is_some_and(|name| NPCS.contains(&name))
    });
    assert_eq!(
        found.len(),
        NPCS.len(),
        "all journey contacts must be in the compiled World"
    );
    Ok(NpcPlacementTable {
        npcs: found
            .into_iter()
            .map(|entry| NpcPlacements {
                key: entry.key,
                placements: entry
                    .placements
                    .into_iter()
                    .map(|p| NpcPlacement {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                        direction: match p.direction {
                            Direction::North => MovementFacing::North,
                            Direction::South => MovementFacing::South,
                            Direction::East => MovementFacing::East,
                            Direction::West => MovementFacing::West,
                        },
                    })
                    .collect(),
            })
            .collect(),
    })
}

async fn speak(
    connection: &mut super::quest_world_callbacks_fixtures::WireConversation,
    npc: EntityRef,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    let payload = encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(debug)?;
    decode_npc_intent_result(&connection.talk(payload).await?).map_err(debug)
}

struct Journey<'a, 'b, 'f, 's> {
    owner: &'a mut ComposedFreshAdmission<'b, 'f, 's>,
    admitted: &'a mut AdmittedSession,
    h: &'a Harness,
    npcs: &'a NpcPlacementTable,
    deps: &'a Dependencies,
    journal_index: u32,
}

impl Journey<'_, '_, '_, '_> {
    async fn run(&mut self) -> TestResult {
        assert_eq!(self.track("the_governor_daughter").await?, None);
        assert_eq!(self.completions().await?, 0);
        for (index, (id, count)) in [(3452, 1), (5467, 1), (5901, 29), (3035, 10)]
            .into_iter()
            .enumerate()
        {
            self.supply(id, count, u64::try_from(index)? + 1).await?;
        }
        self.repair_ship_before_meriana().await?;
        self.mission("eleonore", "ring", "the_governor_daughter", 1, (6093, 0))
            .await?;
        self.use_at((32421, 32770, 1), 2, Some(3452)).await?;
        self.expect("the_governor_daughter", 2, (6093, 1)).await?;
        self.mission("eleonore", "ring", "the_governor_daughter", 3, (6093, 0))
            .await?;
        self.mission("eleonore", "errand", "the_errand", 1, (6093, 0))
            .await?;
        self.mission("charlotta", "errand", "the_errand", 2, (6093, 0))
            .await?;
        self.mission("eleonore", "errand", "the_errand", 3, (6093, 0))
            .await?;
        self.mission("eleonore", "peg leg", "access_to_meriana", 1, (6093, 0))
            .await?;
        self.passage("captain_waverider", "peg leg", (32346, 32625, 7))
            .await?;

        self.passage("jack_fate", "goroma", (32161, 32558, 6))
            .await?;
        self.use_at((32093, 32574, 8), 1, None).await?;
        self.expect("access_to_goroma", 1, (6120, 1)).await?;
        self.passage("jack_fate_goroma", "liberty bay", (32285, 32892, 6))
            .await?;

        self.mission(
            "raymond_striker",
            "mermaid",
            "apoem_for_the_mermaid",
            1,
            (6119, 0),
        )
        .await?;
        self.mission(
            "marina",
            "raymond striker",
            "apoem_for_the_mermaid",
            2,
            (6119, 0),
        )
        .await?;
        self.mission("ocelus", "date", "adjinn_in_love", 1, (6119, 0))
            .await?;
        self.mission("marina", "date", "adjinn_in_love", 2, (6119, 0))
            .await?;
        self.mission("ocelus", "marina", "adjinn_in_love", 3, (6119, 0))
            .await?;
        self.mission("elvith", "love poem", "poem_purchased", 1, (6119, 1))
            .await?;
        self.mission("elvith", "love poem", "poem_purchased", 1, (6119, 2))
            .await?;
        self.mission("ocelus", "love poem", "adjinn_in_love", 4, (6119, 1))
            .await?;
        self.mission("marina", "date", "access_to_laguna_island", 1, (6119, 1))
            .await?;
        self.seat((32340, 32540, 7)).await?;
        super::annihilator_route_fixtures::step(self.owner, self.admitted, StepDirection::North)
            .await?;
        self.arrived((32359, 32901, 7)).await?;
        self.expect("access_to_laguna_island", 1, (6119, 1)).await?;

        self.mission("raymond_striker", "mission", "rays_mission2", 1, (5467, 1))
            .await?;
        self.passage("sebastian", "nargor", (32024, 32813, 7))
            .await?;
        self.use_at((31978, 32887, 6), 1, Some(5467)).await?;
        self.expect("rays_mission2", 2, (5467, 1)).await?;
        self.mission("raymond_striker", "mission", "rays_mission2", 3, (5467, 1))
            .await?;
        self.mission("raymond_striker", "mission", "rays_mission3", 1, (5467, 1))
            .await?;
        for (i, cell) in [(31978, 32828, 2), (31982, 32844, 2), (31992, 32847, 2)]
            .into_iter()
            .enumerate()
        {
            self.use_at(cell, 2, None).await?;
            self.expect(&format!("tavern_map{}", i + 1), 1, (5467, 1))
                .await?;
        }
        self.mission("raymond_striker", "mission", "rays_mission3", 3, (6105, 0))
            .await?;
        self.mission("klaus", "mission", "rays_mission4", 2, (6105, 0))
            .await?;
        self.mission("raymond_striker", "mission", "rays_mission4", 3, (6105, 1))
            .await?;
        self.mission("klaus", "mission", "rays_mission4", 4, (6105, 0))
            .await?;
        assert_eq!(self.completions().await?, 0);
        let raymond = self.contact("raymond_striker").await?;
        let terminal = self.admitted.continuity.next_command_id.checked_add(1).ok_or("talk command id")?;
        let original = self.talk(raymond, "mission").await?;
        assert_eq!(original, NpcIntentDisposition::Ok);
        self.terminal().await?;
        assert_eq!(self.talk_id(raymond, "mission", terminal).await?, original);
        self.terminal().await?;
        self.reconnect().await?;
        let raymond = self.contact("raymond_striker").await?;
        let receipts = self.receipts().await?;
        self.talk(raymond, "mission").await?;
        assert_eq!(
            self.receipts().await?,
            receipts,
            "fresh command after relogin refuses a second completion"
        );
        self.terminal().await?;
        self.journal().await
    }

    async fn repair_ship_before_meriana(&mut self) -> TestResult {
        assert_eq!(self.track("access_to_meriana").await?, None);
        self.confirm_storm_roll(true).await?;
        self.mission("jack_fate_goroma", "sail", "shipwrecked", 1, (5901, 29))
            .await?;
        assert_eq!(self.track("default_start").await?, Some(1));
        let jack = self.contact("jack_fate_goroma").await?;
        assert_eq!(
            self.talk(jack, "sail").await?,
            NpcIntentDisposition::Rejected
        );
        self.expect("shipwrecked", 1, (5901, 29)).await?;
        self.pick_up_wood().await?;
        self.expect("shipwrecked", 1, (5901, 31)).await?;
        self.mission("jack_fate_goroma", "sail", "access_to_goroma", 1, (5901, 1))
            .await?;
        self.passage("jack_fate_goroma", "liberty bay", (32285, 32892, 6))
            .await?;
        assert_eq!(self.track("access_to_meriana").await?, None);
        Ok(())
    }

    // Place the fixture within reach of widely separated contacts. Passage commands
    // still have to move the actor themselves, and the STEP crosses the actual turtle tile.
    async fn seat(&self, (x, y, z): (i32, i32, i16)) -> TestResult {
        self.owner
            .runtime
            .lock()
            .await
            .place_admitted_respawn(
                self.admitted.runtime_actor.ok_or("actor")?,
                self.admitted.game_session_id,
                Some(MovementLocalPosition { x, y, floor: z }),
                |_| true,
            )
            .map_err(debug)?;
        Ok(())
    }

    async fn contact(&mut self, name: &str) -> TestResult<EntityRef> {
        let key = format!("oteryn:npc.{name}");
        let npc = self
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or("NPC placement")?;
        let p = if name == "captain_waverider" {
            npc.placements
                .iter()
                .find(|p| p.x == 32346 && p.y == 32859)
                .ok_or("Liberty Bay Waverider")?
        } else {
            npc.placements.first().ok_or("NPC cell")?
        };
        self.seat((i32::from(p.x) - 1, i32::from(p.y), -i16::from(p.floor)))
            .await?;
        let actor = {
            let runtime = self.owner.runtime.lock().await;
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|actor| {
                    runtime.npc_key(actor.actor) == Some(&key)
                        && actor.position.x == i32::from(p.x)
                        && actor.position.y == i32::from(p.y)
                })
                .ok_or("live journey contact")?
        };
        let npc = EntityRef {
            identity: actor.actor.placement_identity(),
            generation: actor.generation,
        };
        Ok(npc)
    }

    async fn talk(&mut self, npc: EntityRef, text: &str) -> TestResult<NpcIntentDisposition> {
        self.advance_clock()?;
        super::quest_world_callbacks_fixtures::wire_conversation(
            self.owner,
            self.admitted,
            async |connection| {
                assert_eq!(
                    speak(connection, npc, "hi").await?,
                    NpcIntentDisposition::Ok
                );
                speak(connection, npc, text).await
            },
        ).await
    }

    async fn talk_id(
        &mut self,
        npc: EntityRef,
        text: &str,
        command: u64,
    ) -> TestResult<NpcIntentDisposition> {
        self.advance_clock()?;
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        })
        .map_err(debug)?;
        let reply = wire(
            self.owner,
            self.admitted,
            command,
            COMMAND_TYPE_NPC_TALK_INTENT,
            payload,
        )
        .await?;
        decode_npc_intent_result(&reply).map_err(debug)
    }

    fn advance_clock(&mut self) -> TestResult {
        self.owner.clock_origin = self
            .owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("owner clock")?;
        Ok(())
    }

    async fn mission(
        &mut self,
        npc: &str,
        message: &str,
        track: &str,
        value: i64,
        inventory: (u32, i64),
    ) -> TestResult {
        let npc = self.contact(npc).await?;
        assert_eq!(
            self.talk(npc, message).await?,
            NpcIntentDisposition::Ok,
            "{track}"
        );
        self.expect(track, value, inventory).await
    }

    async fn wait_for_passage(&self) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        loop {
            let ready: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS(SELECT 1 FROM game_newhaven_travel_receipts WHERE arrived_at > clock_timestamp() - interval '3 seconds')",
            ).fetch_one(&self.h.pool).await?;
            if ready {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("travel cooldown did not clear".into());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn confirm_storm_roll(&mut self, expect_wreck: bool) -> TestResult {
        self.wait_for_passage().await?;
        let jack = self.contact("jack_fate").await?;
        assert!(self.owner.states.newhaven_travel.set_jack_fate_test_roll(true));
        self.advance_clock()?;
        let owner = &*self.owner;
        let actor = self.admitted.runtime_actor.ok_or("actor")?;
        super::quest_world_callbacks_fixtures::wire_conversation(
            owner,
            self.admitted,
            async |connection| {
                for word in ["hi", "thais"] {
                    assert_eq!(speak(connection, jack, word).await?, NpcIntentDisposition::Ok);
                }
                let before = super::full_world_support::position(owner, actor).await?;
                assert_eq!(speak(connection, jack, "yes").await?, NpcIntentDisposition::Ok);
                assert_eq!(
                    super::full_world_support::position(owner, actor).await?,
                    before
                );
                assert_eq!(speak(connection, jack, "yes").await?, NpcIntentDisposition::Ok);
                Ok(())
            },
        ).await?;
        if expect_wreck {
            self.arrived_at_goroma().await?;
        } else {
            self.arrived((32310, 32210, 6)).await?;
        }
        assert_eq!(self.track("shipwrecked").await?, None);
        Ok(())
    }

    async fn passage(
        &mut self,
        npc: &str,
        keyword: &str,
        destination: (i32, i32, i16),
    ) -> TestResult {
        self.wait_for_passage().await?;
        let npc = self.contact(npc).await?;
        self.advance_clock()?;
        super::quest_world_callbacks_fixtures::wire_conversation(
            self.owner,
            self.admitted,
            async |connection| {
                for word in ["hi", keyword, "yes"] {
                    assert_eq!(speak(connection, npc, word).await?, NpcIntentDisposition::Ok);
                }
                Ok(())
            },
        ).await?;
        if keyword == "goroma" {
            self.arrived_at_goroma().await
        } else {
            self.arrived(destination).await
        }
    }

    async fn arrived_at_goroma(&self) -> TestResult {
        // The captain stands on the source landing. The native arrival owner
        // can choose a walkable unoccupied cell within its bounded fallback.
        let cell = super::full_world_support::position(
            self.owner,
            self.admitted.runtime_actor.ok_or("actor")?,
        )
        .await?;
        assert_eq!(cell.floor, 6);
        assert!((cell.x - 32161).abs().max((cell.y - 32558).abs()) <= 3);
        Ok(())
    }

    async fn arrived(&self, destination: (i32, i32, i16)) -> TestResult {
        let p = super::full_world_support::position(
            self.owner,
            self.admitted.runtime_actor.ok_or("actor")?,
        )
        .await?;
        assert_eq!((p.x, p.y, p.floor), destination);
        Ok(())
    }

    async fn use_at(
        &mut self,
        (x, y, z): (u16, u16, i8),
        ordinal: u8,
        tool: Option<u32>,
    ) -> TestResult {
        self.seat((i32::from(x) - 1, i32::from(y), i16::from(z)))
            .await?;
        self.advance_clock()?;
        let map = self.owner.bundle_map().ok_or("active World")?;
        let key = oteryn_world_bundle::bundle::placement_key(-z, x, y, ordinal)
            .ok_or("native placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        let disposition = if let Some(tool) = tool {
            let reference = self
                .owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .and_then(|n| n.item_index())
                .and_then(|i| i.definition_ref("Item", &item(tool), "definition-r1"))
                .ok_or("selected native tool")?;
            super::quest_world_callbacks_fixtures::wire_selected_map_tool(
                self.owner, self.admitted, reference, target,
            ).await?
        } else {
            let reply = wire(
                self.owner,
                self.admitted,
                self.admitted.continuity.next_command_id,
                COMMAND_TYPE_USE_INTENT,
                encode_use_intent(&target).map_err(debug)?,
            ).await?;
            decode_use_result(&reply).map_err(debug)?
        };
        assert_eq!(
            disposition,
            UseDisposition::Committed,
            "USE at {x},{y},{z}"
        );
        Ok(())
    }

    async fn pick_up_wood(&mut self) -> TestResult {
        use crate::gameplay_transport::{
            connection::UseCommand,
            item_move::{ItemMoveStep, item_move},
            item_view::{ItemKey, ItemViewContinuity, SessionItemView},
            world_map::SessionMapView,
        };
        use oteryn_protocol_oteryn::item_view::{
            ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
            encode_item_move_intent_for,
        };
        let actor = self.admitted.runtime_actor.ok_or("actor")?;
        let session = self.admitted.game_session_id;
        let cell = super::full_world_support::position(self.owner, actor).await?;
        let fence = self
            .owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("wood fence")?
            .item_fence();
        super::quest_world_callbacks_fixtures::mint_input_on_ground(
            self.owner,
            fence,
            &Offering {
                position: Position {
                    x: u16::try_from(cell.x)?,
                    y: u16::try_from(cell.y)?,
                    floor: -i8::try_from(cell.floor)?,
                },
                item: item(5901),
                count: 2,
            },
            5,
        )
        .await?;
        let wood: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT item_instance_id FROM game_item_instances WHERE definition_production_key=$1 AND quantity=2 AND lifecycle=1 LIMIT 1",
        ).bind(item(5901)).fetch_one(&self.h.pool).await?;
        let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
        view.snapshot(
            self.owner
                .observe_character_inventory(actor, session)
                .await
                .ok_or("inventory")?,
            None,
        )
        .map_err(debug)?;
        self.owner
            .observe_world_map(actor, &mut SessionMapView::default(), &mut view)
            .await
            .ok_or("ground view")?
            .map_err(debug)?;
        let selection = ItemMoveSelection {
            equip_drop: false,
            container_tree: false,
            world_interaction: true,
            manual_sort: false,
        };
        let payload = encode_item_move_intent_for(
            &ItemMoveIntent {
                source: view
                    .table()
                    .handle(&ItemKey::Instance(*wood.as_bytes()))
                    .ok_or("wood handle")?,
                destination: ItemMoveDestination::MainBackpack,
            },
            selection,
        )
        .map_err(debug)?;
        let fence = self
            .owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("current wood fence")?
            .item_fence();
        let command = UseCommand {
            game_session_id: session,
            command_id: 100_000,
            item_fence: Some(fence),
        };
        let moved = item_move(
            self.owner, actor, command, &payload, selection, false, &mut view,
        )
        .await;
        assert!(matches!(
            moved,
            ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
        ));
        Ok(())
    }

    async fn supply(&self, source: u32, count: u16, occurrence: u64) -> TestResult {
        let fence = self
            .owner
            .current_quest_fence(self.admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("fixture item fence")?;
        input_item(
            self.owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 32330,
                    y: 32763,
                    floor: -5,
                },
                item: item(source),
                count,
            },
            occurrence,
            true,
        )
        .await
    }

    async fn track(&self, suffix: &str) -> TestResult<Option<i64>> {
        super::m39_bone_journey_fixtures::stage(self.h, &format!("{TRACK}{suffix}")).await
    }

    async fn expect(&self, track: &str, value: i64, (item, count): (u32, i64)) -> TestResult {
        assert_eq!(self.track(track).await?, Some(value), "{track}");
        assert_eq!(
            carried(self.h, item).await?,
            count,
            "carried {item} after {track}"
        );
        Ok(())
    }

    async fn completions(&self) -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
        )
        .bind(QUEST)
        .fetch_one(&self.h.pool)
        .await?)
    }

    async fn receipts(&self) -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
        )
        .bind(QUEST)
        .fetch_one(&self.h.pool)
        .await?)
    }

    async fn terminal(&self) -> TestResult {
        self.expect("rays_mission4", 5, (6105, 0)).await?;
        assert_eq!(self.completions().await?, 1);
        assert_eq!(
            carried(self.h, 2994).await?,
            0,
            "the chosen reward is passage access"
        );
        assert_eq!(
            carried(self.h, 6120).await?,
            1,
            "the Spellbook is a side claim"
        );
        for island in ["meriana", "laguna_island", "goroma", "nargor"] {
            assert_eq!(self.track(&format!("access_to_{island}")).await?, Some(1));
        }
        let routes: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT route) FROM game_newhaven_travel_receipts WHERE arrived_at IS NOT NULL AND route IN ('oteryn:service.travel.captain_waverider/meriana','oteryn:service.travel.jack_fate/goroma','oteryn:service.travel.sebastian/nargor')"
        ).fetch_one(&self.h.pool).await?;
        assert_eq!(routes, 3);
        let observation = self
            .owner
            .quest_log_observation(
                self.admitted.runtime_actor.ok_or("actor")?,
                self.admitted.game_session_id,
                None,
            )
            .await;
        let QuestLogObservation::Changed(source) = observation else {
            return Err("live quest journal".into());
        };
        assert!(
            source
                .copy
                .quest_state(QUEST)
                .is_some_and(|state| state.completed)
        );
        Ok(())
    }

    async fn journal(&mut self) -> TestResult {
        use oteryn_protocol_oteryn::quest_log::{
            COMMAND_TYPE_QUEST_LOG_QUERY, QuestLogQuery, QuestLogView, decode_quest_log,
            encode_quest_log_query,
        };
        use oteryn_protocol_oteryn::{
            ClientCommandValue, MessageType, decode_state_delta, decode_wire_envelope,
            encode_client_command,
        };
        let payload = encode_quest_log_query(&QuestLogQuery::Quest {
            quest_index: self.journal_index,
        })
        .map_err(debug)?;
        let command = encode_client_command(
            1,
            &ClientCommandValue {
                command_id: self.admitted.continuity.next_command_id,
                command_type: COMMAND_TYPE_QUEST_LOG_QUERY,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        let mut input = u32::try_from(command.len())?.to_be_bytes().to_vec();
        input.extend(command);
        let (_, output) = super::weather_wire_fixtures::fixture_access::serve_input(
            self.owner,
            *self.admitted,
            &input,
        )
        .await?;
        let mut frames = output.as_slice();
        while !frames.is_empty() {
            let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut frames).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == 16
                    && matches!(
                        decode_quest_log(delta.payload).map_err(debug)?.view,
                        Some(QuestLogView::Line(line)) if line.quest_index == self.journal_index && line.completed
                    )
                {
                    return Ok(());
                }
            }
        }
        Err("completed Shattered Isles journal line missing from LOG response".into())
    }

    async fn reconnect(&mut self) -> TestResult {
        self.owner.clock_origin = self
            .owner
            .clock_origin
            .checked_sub(Duration::from_secs(120))
            .ok_or("owner clock")?;
        assert_eq!(
            self.owner.logout_session(*self.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let session = super::full_world_support::relogin(self.h).await?;
        let current = FreshAdmissionStore::from_root(self.h.root.clone())
            .current_session(session)
            .await?;
        let fence = self
            .owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("successor session")?;
        let position = self
            .owner
            .read_admission_position(fence)
            .await
            .map_err(debug)?;
        let actor = {
            let mut runtime = self.owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(actor, session, current.current_character_lease())
                .map_err(debug)?;
            self.owner
                .initialize_admitted_position(&mut runtime, actor, session, &position)
                .await
                .map_err(debug)?;
            actor
        };
        assert!(self.owner.track_restored(session, actor, &position, fence));
        let capabilities = self.admitted.continuity.selected_capabilities;
        *self.admitted = AdmittedSession {
            game_session_id: session,
            runtime_actor: Some(actor),
            item_fence: crate::gameplay_transport::item_fence_of(current),
            continuity: SessionContinuity {
                selected_capabilities: capabilities,
                ..SessionContinuity::FRESH
            },
            ..*self.admitted
        };
        self.owner.admit_premium(id(40), session);
        self.owner.admit_progression_session(self.admitted).await;
        self.owner.admit_quest_session(self.admitted).await;
        self.deps
            .initialize_player(self.owner, actor, session)
            .await;
        self.owner
            .states
            .chat
            .admit(session, "Fixture Hero", self.owner.owner_now());
        self.owner.states.chat.set_npc_service(session);
        Ok(())
    }
}

fn item(id: u32) -> String {
    format!("oteryn:item.tibia.i{id}")
}
