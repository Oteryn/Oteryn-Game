//! The chosen Ape City recipe, with the committed World geometry and native quest catalogue.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::wire;
use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::{item_mint::TypedDefinitionRef, starter_grant::StarterTemplate};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks::{self, Position}};
use oteryn_protocol_oteryn::npc_service::{
    COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result,
    encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::{num::NonZeroU64, path::Path, sync::Arc};

const QUEST: &str = "oteryn:quest.the_ape_city_quest";
const NPC: &str = "oteryn:npc.hairycles";
const TRACK: &str = "oteryn:quest-progress/chosen/ape-city/";
const CONTACT: TilePos = tile(32826, 32574, -6);
const START: TilePos = tile(32826, 32575, -6);
const BANANA: (TilePos, u8) = (tile(31983, 32193, -5), 2);
const MOSS: (TilePos, u8) = (tile(32782, 32910, -8), 1);
const PARCHMENT: (TilePos, u8) = (tile(32935, 32886, -7), 1);
const SIGNS: TilePos = tile(33305, 32861, -13);
const HAIR: (TilePos, u8) = (tile(32933, 32495, -7), 1);
const ENTRY: TilePos = tile(32839, 32533, -9);
const DEEPEST: TilePos = tile(32854, 32544, -10);
const GATE: (TilePos, u8) = (tile(32862, 32557, -11), 1);
const STATUE: (TilePos, u8) = (tile(32858, 32526, -11), 1);
const AMPHORAS: [(TilePos, u8); 4] = [
    (tile(32792, 32527, -10), 1),
    (tile(32823, 32525, -10), 1),
    (tile(32876, 32584, -10), 1),
    (tile(32744, 32586, -10), 1),
];
const LEVERS: [(TilePos, u8); 6] = [
    (tile(32891, 32590, -11), 1),
    (tile(32843, 32649, -11), 1),
    (tile(32808, 32613, -11), 2),
    (tile(32775, 32583, -11), 1),
    (tile(32756, 32494, -11), 2),
    (tile(32799, 32556, -11), 1),
];

const fn tile(x: u16, y: u16, floor: i8) -> TilePos {
    TilePos { x, y, floor }
}

#[test]
fn ape_city_world_admission_loads_banuta_routes() -> TestResult {
    let state = load_embedded_quest_state("content-1").map_err(debug)?;
    let callbacks = world_callbacks::load_for(state.catalogue())?;
    assert!(callbacks
        .iter()
        .any(|callback| callback.key == "oteryn:quest-callback/authored-ape-city-deeper-banuta"));
    Ok(())
}

#[test]
fn ape_city_full_journey_earns_banuta_and_shaman() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("ape-city-journey", move || {
        super::fixture::run(async |_| scenario(admin).await)
    })
}

fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::bundle;
    use oteryn_world_bundle_compiler::{compile, project, source};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let targets = [
        CONTACT,
        BANANA.0,
        MOSS.0,
        PARCHMENT.0,
        SIGNS,
        HAIR.0,
        ENTRY,
        DEEPEST,
        GATE.0,
        STATUE.0,
    ];
    let regions: Vec<_> = targets
        .into_iter()
        .chain(
            AMPHORAS
                .into_iter()
                .chain(LEVERS)
                .map(|(position, _)| position),
        )
        .map(|p| (p.x / 256, p.y / 256))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let project = source::project_regions(&root, &regions)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let mut pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: START,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "ape_city_journey", false, false).await?;
    let fresh = prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, content_pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("native generation")?;
    let native = generation.native_gameplay().ok_or("native item provider")?;
    let world = world(native)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let frame = content_pin.frame_binding().digest();
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (
            i32::from(START.x),
            i32::from(START.y),
            -i16::from(START.floor),
        ),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, local(START))
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.admitted.runtime_actor = Some(deps.actor);
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice())
        .execute(&h.pool).await?;

    let (consumer, clock, activation, _) =
        crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
    let premium =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.premium_coordinator = Some(&premium);
    let lowered = load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let quest_index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|quest| quest.key == QUEST)
            .ok_or("Ape City journal")?,
    )?;
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(journal.content)
            .ok_or("quest log lines")?,
    );
    owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("current quest fence")?;
    admitted.item_fence = Some(fence.item_fence());
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            fresh.operation(),
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
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    owner.admit_premium(id(40), admitted.game_session_id);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let available = [1, 2, 3, 4, 6, 11, 12, 13, 15, 16, 18];
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &available)
            .ok_or("journey capabilities")?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    let [npc] = super::classic_quests_fixtures::install_positions(
        &owner,
        &admitted,
        [NPC],
        [Position {
            x: CONTACT.x,
            y: CONTACT.y,
            floor: CONTACT.floor,
        }],
    )
    .await?;
    Journey {
        h: &h,
        owner: &mut owner,
        admitted: &mut admitted,
        npc,
        quest_index,
    }
    .exercise()
    .await?;
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn local(position: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: -i16::from(position.floor),
    }
}

struct Journey<'a, 'b, 'c, 'd> {
    h: &'a Harness,
    owner: &'a mut ComposedFreshAdmission<'b, 'c, 'd>,
    admitted: &'a mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    quest_index: u32,
}

impl Journey<'_, '_, '_, '_> {
    fn next(&self) -> u64 {
        self.admitted.continuity.next_command_id
    }

    // Traverse between geographically distant encounter sites using the existing fenced
    // movement owner. The local STEP/USE and every quest/inventory mutation are real ingress.
    async fn relocate(&self, destination: TilePos) -> TestResult {
        let map = self.owner.bundle_map().ok_or("journey World")?;
        let mut runtime = self.owner.runtime.lock().await;
        let actor = self.admitted.runtime_actor.ok_or("actor")?;
        let expected = runtime.read_actor_position(actor).map_err(debug)?;
        if expected.position() == local(destination) {
            return Ok(());
        }
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            &self.owner.movement_cells,
            actor,
            self.admitted.game_session_id,
            expected,
            local(destination),
            false,
            &doors,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
        Ok(())
    }

    async fn approach(&self, destination: TilePos) -> TestResult<StepDirection> {
        for (dx, dy, direction) in [
            (-1, 0, StepDirection::East),
            (0, -1, StepDirection::South),
            (1, 0, StepDirection::West),
            (0, 1, StepDirection::North),
        ] {
            let neighbor = tile(
                destination.x.checked_add_signed(dx).ok_or("neighbor x")?,
                destination.y.checked_add_signed(dy).ok_or("neighbor y")?,
                destination.floor,
            );
            if self.relocate(neighbor).await.is_ok() {
                return Ok(direction);
            }
        }
        Err(format!("no enterable neighbor for {destination:?}").into())
    }

    async fn talk(&mut self, command: u64, text: &str) -> TestResult<NpcIntentDisposition> {
        self.owner.clock_origin = self
            .owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: self.npc,
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

    async fn say(&mut self, text: &str) -> TestResult<u64> {
        self.relocate(START).await?;
        assert_eq!(
            self.talk(self.next(), "hi").await?,
            NpcIntentDisposition::Ok
        );
        let command = self.next();
        assert_eq!(
            self.talk(command, text).await?,
            NpcIntentDisposition::Ok,
            "{text}"
        );
        Ok(command)
    }

    async fn use_object(
        &mut self,
        (position, ordinal): (TilePos, u8),
        expected: UseDisposition,
    ) -> TestResult {
        self.approach(position).await?;
        let target = object_target(self.owner, position, ordinal)?;
        let command = self.next();
        assert_eq!(
            use_target(self.owner, self.admitted, command, &target).await?,
            expected,
            "{position:?}"
        );
        Ok(())
    }

    async fn use_snake_destroyer(
        &mut self,
        (position, ordinal): (TilePos, u8),
        command: u64,
    ) -> TestResult {
        self.approach(position).await?;
        let target = object_target(self.owner, position, ordinal)?;
        let reference = self
            .owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|native| native.item_index())
            .and_then(|items| {
                items.definition_ref("Item", "oteryn:item.tibia.i4835", "definition-r1")
            })
            .ok_or("native Snake Destroyer")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::new(1).ok_or("handle")?),
            use_with: None,
            use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
                target,
            )),
            use_with_position: None,
        })
        .map_err(debug)?;
        let (next, disposition, _) = super::rust_lottery_fixtures::exchange(
            self.owner,
            *self.admitted,
            command,
            &payload,
            Some(reference),
            None,
        )
        .await?;
        *self.admitted = next;
        assert_eq!(disposition, UseDisposition::Committed, "{position:?}");
        Ok(())
    }

    async fn enter(&mut self, target: TilePos, expected: TilePos) -> TestResult {
        let direction = self.approach(target).await?;
        let command = self.next();
        step(self.owner, self.admitted, command, direction).await?;
        let position = self
            .owner
            .runtime
            .lock()
            .await
            .read_actor_position(self.admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        assert_eq!(position, local(expected));
        Ok(())
    }

    async fn track(&self, suffix: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)"
        ).bind(id(41).as_slice()).bind(format!("{TRACK}{suffix}")).fetch_one(&self.h.pool).await?)
    }

    async fn check(&self, stage: i64, items: [i64; 5]) -> TestResult {
        assert_eq!(self.track("stage").await?, stage, "chosen quest mission");
        for (id, expected) in [3587, 4827, 4831, 4832, 4835].into_iter().zip(items) {
            let carried: i64 = sqlx::query_scalar(
                "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.lifecycle=1"
            ).bind(super::fixture::id(41).as_slice()).bind(super::fixture::id(42).as_slice())
                .bind(format!("oteryn:item.tibia.i{id}")).fetch_one(&self.h.pool).await?;
            assert_eq!(carried, expected, "item {id} at mission {stage}");
        }
        Ok(())
    }

    async fn terminal(&self) -> TestResult {
        self.check(15, [0, 1, 0, 0, 0]).await?;
        assert_eq!(self.track("access").await?, 1);
        let (completed, outfit): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completes),
                    (SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE account_id=encode($3,'hex')::uuid AND unlock_key=$4)"
        ).bind(id(41).as_slice()).bind(QUEST).bind(id(40).as_slice())
            .bind("oteryn:outfit.shaman/quest-grant/0").fetch_one(&self.h.pool).await?;
        assert_eq!((completed, outfit), (1, 1));
        Ok(())
    }

    async fn reopen_catacombs_gate_after_timeout(&mut self) -> TestResult {
        self.owner.clock_origin -= std::time::Duration::from_secs(11);
        let map = self.owner.bundle_map().ok_or("journey World")?;
        assert!(map.facts.expire_movement_items(self.owner.owner_now().get()) > 0);
        self.use_object(GATE, UseDisposition::Committed).await?;
        self.owner.clock_origin -= std::time::Duration::from_secs(600);
        let map = self.owner.bundle_map().ok_or("journey World")?;
        assert!(map.facts.expire_movement_items(self.owner.owner_now().get()) > 0);
        for lever in LEVERS {
            self.use_object(lever, UseDisposition::Committed).await?;
        }
        self.use_object(GATE, UseDisposition::Committed).await?;
        self.check(13, [0, 1, 0, 0, 1]).await
    }

    async fn exercise(&mut self) -> TestResult {
        use UseDisposition::{Committed, Rejected};
        self.check(0, [0; 5]).await?;
        self.say("mission").await?;
        self.check(1, [0; 5]).await?;
        self.use_object(MOSS, Committed).await?;
        self.check(1, [0, 1, 0, 0, 0]).await?;
        self.use_object(BANANA, Committed).await?;
        self.check(2, [1, 1, 0, 0, 0]).await?;
        self.say("banana").await?;
        self.check(3, [0, 1, 0, 0, 0]).await?;
        self.say("mission").await?;
        self.check(4, [0, 1, 0, 0, 0]).await?;
        self.use_object(PARCHMENT, Committed).await?;
        self.check(5, [0, 1, 1, 0, 0]).await?;
        self.say("parchment").await?;
        self.check(6, [0, 1, 0, 0, 0]).await?;
        self.enter(SIGNS, SIGNS).await?;
        self.check(7, [0, 1, 0, 0, 0]).await?;
        self.use_object(HAIR, Committed).await?;
        self.check(8, [0, 1, 0, 1, 0]).await?;
        self.say("holy ape hair").await?;
        self.check(9, [0, 1, 0, 0, 0]).await?;
        self.say("mission").await?;
        self.check(10, [0, 1, 0, 0, 1]).await?;
        self.enter(ENTRY, tile(32749, 32536, -10)).await?;
        self.check(11, [0, 1, 0, 0, 1]).await?;
        self.h.root.set_test_quest_clock(1_800_000_000)?;
        for (index, jar) in AMPHORAS.into_iter().enumerate() {
            let command = self.next();
            self.use_snake_destroyer(jar, command).await?;
            assert_eq!(self.track(&format!("amphora-{}", index + 1)).await?, 1);
            self.check(11, [0, 1, 0, 0, 1]).await?;
        }
        let expired = self.h.root.advance_test_quest_clock(301)?;
        self.owner.clock_origin -= std::time::Duration::from_secs(301);
        let map = self.owner.bundle_map().ok_or("journey World")?;
        assert!(map.facts.expire_movement_items(self.owner.owner_now().get()) >= AMPHORAS.len());
        let retry = self.next();
        self.use_snake_destroyer(AMPHORAS[0], retry).await?;
        assert_eq!(self.track("amphora-window").await?, expired);
        self.enter(DEEPEST, tile(32885, 32632, -11)).await?;
        self.check(12, [0, 1, 0, 0, 1]).await?;
        self.enter(DEEPEST, tile(32885, 32632, -11)).await?;
        self.check(12, [0, 1, 0, 0, 1]).await?;
        self.use_object(GATE, Rejected).await?;
        self.check(12, [0, 1, 0, 0, 1]).await?;
        for (index, lever) in LEVERS.into_iter().enumerate() {
            self.use_object(lever, Committed).await?;
            assert_eq!(self.track(&format!("lever-{}", index + 1)).await?, 1);
            self.check(12, [0, 1, 0, 0, 1]).await?;
        }
        self.use_object(GATE, Committed).await?;
        self.check(13, [0, 1, 0, 0, 1]).await?;
        self.reopen_catacombs_gate_after_timeout().await?;
        let passage = tile(32864, 32556, -11);
        self.enter(passage, passage).await?;
        self.check(13, [0, 1, 0, 0, 1]).await?;
        let command = self.next();
        self.use_snake_destroyer(STATUE, command).await?;
        self.check(14, [0, 1, 0, 0, 0]).await?;
        let report = self.say("mission").await?;
        self.terminal().await?;
        self.owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest cache")?
            .remove(&self.admitted.game_session_id);
        self.owner.admit_quest_session(self.admitted).await;
        assert_eq!(
            self.talk(report, "mission").await?,
            NpcIntentDisposition::Ok
        );
        self.terminal().await?;
        self.use_object(PARCHMENT, Rejected).await?;
        self.terminal().await?;
        self.enter(ENTRY, tile(32749, 32536, -10)).await?;
        self.terminal().await?;
        let query = oteryn_protocol_oteryn::quest_log::encode_quest_log_query(
            &oteryn_protocol_oteryn::quest_log::QuestLogQuery::ListOnly,
        )
        .map_err(debug)?;
        let command = self.next();
        let reply = wire(self.owner, self.admitted, command, 22, query).await?;
        use oteryn_protocol_oteryn::quest_log::{QuestLogView, decode_quest_log};
        let log = decode_quest_log(&reply).map_err(debug)?;
        assert!(matches!(log.view, Some(QuestLogView::List(entries))
            if entries.iter().any(|entry| entry.quest_index == self.quest_index && entry.completed)));

        Ok(())
    }
}
