//! The book chapter and wolf handoff use the ordinary NPC, chest and selected carried USE owners.
use super::classic_quests_fixtures::{install, progress, talk, talk_expecting};
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_quests::quest::{threatened_dreams as quest, world_callbacks::Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use oteryn_gameplay_quests::quest::world_callbacks::Offering;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent, UseTarget};

const CAMP: Position = Position {
    x: 32787,
    y: 31976,
    floor: -11,
};
const TABLE: Position = Position {
    x: 32788,
    y: 31976,
    floor: -11,
};
const TENT: Position = Position {
    x: 32950,
    y: 31811,
    floor: -7,
};
const BONES: Position = Position {
    x: 32951,
    y: 31810,
    floor: -7,
};
const STONE: Position = Position {
    x: 33339,
    y: 32052,
    floor: -7,
};
const NOTES: &str = "oteryn:item.tibia.i25242";
const FUR: &str = "oteryn:item.tibia.i25238";

const BOOK: &str = "oteryn:item.tibia.i25235";
const TALISMAN: &str = "oteryn:item.tibia.i25295";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const JOURNAL_ANIMALS: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01_1";

#[test]
fn threatened_dreams_poacher_book_journey() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("threatened-dreams-book", move || {
        scenario(&admin)
    })
}

fn scenario(configured_admin: &str) -> TestResult {
    let trigger = CAMP;
    let chest = super::quest_catalogue_chests_fixtures::cases()?
        .into_iter()
        .find(|chest| {
            chest
                .claim
                .ends_with("threatened_dreams.mission01.poacher_chest")
        })
        .ok_or("poacher chest")?;
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load_for(loaded.catalogue())?;
    let feathers = source_callbacks(&callbacks, "swan-feather-");
    let shrines = source_callbacks(&callbacks, "feyrist-");
    assert_eq!(feathers.len(), 5);
    assert_eq!(shrines.len(), 4);
    let mut props = vec![
        (TABLE, 12648, Default::default()),
        (BONES, 2516, Default::default()),
        (BONES, 25241, Default::default()),
        (STONE, 25237, Default::default()),
    ];
    let mut tiles = vec![CAMP, TENT, south(TENT), west(STONE)];
    for feather in &feathers {
        let site = feather.selector.position.ok_or("feather position")?;
        tiles.extend([site, south(site)]);
    }
    for shrine in &shrines {
        let site = shrine.selector.position.ok_or("shrine position")?;
        tiles.extend([west(site), shrine.teleport.ok_or("Feyrist destination")?]);
        if shrine.key.ends_with("-ice") {
            props.push((site, 7145, Default::default()));
        }
        props.push((
            site,
            shrine.selector.appearance.ok_or("shrine appearance")?,
            Default::default(),
        ));
    }
    let world = super::quest_catalogue_chests_fixtures::catalogue_source_worlds(
        std::slice::from_ref(&chest),
        &props,
        &tiles,
        false,
    )?;
    run(async |admin| {
        assert!(
            admin == configured_admin,
            "fixture must use the configured PostgreSQL route"
        );
        let h = Harness::create_with_admission_seed(admin, "threatened_dreams_book", false, false)
            .await?;
        {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("generation")?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                activated.frame_binding().digest(),
                world.map_revision_digest(),
                (
                    i32::from(trigger.x),
                    i32::from(trigger.y),
                    -i16::from(trigger.floor),
                ),
            );
            let room = activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    activated.frame_binding().digest(),
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: i32::from(trigger.x),
                        y: i32::from(trigger.y),
                        floor: -i16::from(trigger.floor),
                    },
                )
                .ok_or("poacher bundle room")?;
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                8,
                pin,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            let origin = local(trigger);
            runtime
                .initialize_pinned_test_position(deps.actor, origin)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
            owner.quest_catalogue = Some(std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[
                    oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
                    oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
                    oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
                    oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
                    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
                ],
            )
            .ok_or("poacher item-use capabilities")?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
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
            let native = generation.native_gameplay().ok_or("native")?;
            let template = crate::durability::starter_grant::StarterTemplate {
                definition: crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..crate::durability::starter_grant::StarterTemplate::main()
            };
            h.root
                .grant_starter_items(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    &template,
                )
                .await
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;

            // Give the fixtures a contact tile in the compact test map. The dialogue,
            // quest and chest bindings themselves are loaded from production content.
            let [alkestios, ahmet, ikassis, wolf, irmana, swan, tereban] = install(
                &owner,
                &admitted,
                [
                    quest::ALKESTIOS,
                    "oteryn:npc.ahmet",
                    "oteryn:npc.ikassis",
                    "oteryn:npc.ghostly_wolf",
                    "oteryn:npc.irmana",
                    "oteryn:npc.a_swan",
                    "oteryn:npc.tereban",
                ],
            )
            .await?;
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                0
            );
            assert_eq!(quantity(&h, BOOK).await?, 0);
            assert_eq!(quantity(&h, TALISMAN).await?, 0);

            talk(&mut owner, &admitted, alkestios, 2200, "hi").await?;
            talk(&mut owner, &admitted, alkestios, 2201, "help").await?;
            talk(&mut owner, &admitted, alkestios, 2202, "yes").await?;
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                1
            );
            assert_eq!(quantity(&h, BOOK).await?, 0);

            let chest_target = target(
                &owner,
                Position {
                    x: chest.position.0,
                    y: chest.position.1,
                    floor: chest.position.2,
                },
                1,
            )?;
            let claim = owner
                .use_world_object(
                    admitted.runtime_actor.ok_or("actor")?,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 2203,
                        item_fence: admitted.item_fence,
                    },
                    chest_target,
                )
                .await;
            assert_eq!(claim.disposition, UseDisposition::Committed);
            assert_eq!(quantity(&h, BOOK).await?, 1);
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                1
            );

            input_item(
                &owner,
                fence.item_fence(),
                &Offering {
                    position: CAMP,
                    item: PLATINUM.into(),
                    count: 50,
                },
                1,
                true,
            )
            .await?;
            assert_eq!(quantity(&h, PLATINUM).await?, 50);
            talk(&mut owner, &admitted, ahmet, 2204, "hi").await?;
            let thieves_offer = talk(&mut owner, &admitted, ahmet, 2205, "mission").await?;
            assert!(
                thieves_offer
                    .iter()
                    .any(|line| line.contains("I was robbed")),
                "{thieves_offer:?}"
            );
            let offer = talk(&mut owner, &admitted, ahmet, 2206, "book").await?;
            assert!(
                offer.iter().any(|line| line.contains("5000 gold")),
                "{offer:?}"
            );
            assert_eq!(quantity(&h, BOOK).await?, 1);
            assert_eq!(quantity(&h, PLATINUM).await?, 50);
            talk(&mut owner, &admitted, ahmet, 2207, "yes").await?;
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                2
            );
            assert_eq!(quantity(&h, BOOK).await?, 1);
            assert_eq!(quantity(&h, PLATINUM).await?, 0);
            assert_eq!(
                progress(&owner, &admitted, &authority, JOURNAL_ANIMALS).await?,
                2
            );

            let book = native
                .item_index()
                .ok_or("native Item index")?
                .definition_ref("Item", BOOK, "definition-r1")
                .ok_or("book definition")?;
            let intent = UseIntent {
                target: UseTarget::ItemByDefinition(book),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Object(target(&owner, TABLE, 1)?)),
                use_with_position: None,
            };
            let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                .map_err(debug)?;
            let command = admitted.continuity.next_command_id;
            let reply = wire(&owner, &mut admitted, command, 2, payload.clone()).await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            );
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                3
            );
            assert_eq!(quantity(&h, BOOK).await?, 0);
            assert_eq!(quantity(&h, PLATINUM).await?, 0);
            assert_eq!(
                progress(&owner, &admitted, &authority, JOURNAL_ANIMALS).await?,
                3
            );
            let replay = wire(&owner, &mut admitted, command, 2, payload.clone()).await?;
            assert_eq!(replay, reply);
            let fresh_command = admitted.continuity.next_command_id;
            let refused = wire(&owner, &mut admitted, fresh_command, 2, payload).await?;
            assert_ne!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
                UseDisposition::Committed,
            );
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                3
            );
            assert_eq!(quantity(&h, BOOK).await?, 0);
            talk(&mut owner, &admitted, ikassis, 2300, "hi").await?;
            let premature = talk(&mut owner, &admitted, ikassis, 2301, "mission").await?;
            assert!(
                !premature
                    .iter()
                    .any(|line| line.contains("Will you help her?"))
            );
            assert_eq!(quantity(&h, TALISMAN).await?, 0);

            talk(&mut owner, &admitted, alkestios, 2303, "hi").await?;
            talk(&mut owner, &admitted, alkestios, 2304, "mission").await?;
            talk(&mut owner, &admitted, alkestios, 2305, "yes").await?;
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                4
            );

            talk(&mut owner, &admitted, ikassis, 2306, "hi").await?;
            talk(&mut owner, &admitted, ikassis, 2307, "yes").await?;
            assert_eq!(quantity(&h, TALISMAN).await?, 0);
            talk(&mut owner, &admitted, ikassis, 2308, "mission").await?;
            talk(&mut owner, &admitted, ikassis, 2309, "no").await?;
            talk(&mut owner, &admitted, ikassis, 2310, "yes").await?;
            assert_eq!(quantity(&h, TALISMAN).await?, 0);
            let offer = talk(&mut owner, &admitted, ikassis, 2311, "mission").await?;
            assert!(offer.iter().any(|line| line.contains("Will you help her?")));
            let accepted = talk(&mut owner, &admitted, ikassis, 2312, "yes").await?;
            assert!(
                accepted
                    .iter()
                    .any(|line| line.contains("Take this talisman")),
                "{accepted:?}"
            );
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                5
            );
            assert_eq!(
                progress(&owner, &admitted, &authority, JOURNAL_ANIMALS).await?,
                5
            );
            assert_eq!(quantity(&h, TALISMAN).await?, 1);
            let carried =
                super::paradox_tower_journey_fixtures::carried(&owner, &admitted, &h).await?;
            assert!(
                carried
                    .iter()
                    .any(|(item, count)| item == TALISMAN && *count == 1)
            );

            talk(&mut owner, &admitted, ikassis, 2313, "mission").await?;
            talk_expecting(
                &mut owner,
                &admitted,
                ikassis,
                2314,
                "yes",
                NpcIntentDisposition::Rejected,
            )
            .await?;
            assert_eq!(quantity(&h, TALISMAN).await?, 1);
            talk(&mut owner, &admitted, wolf, 2315, "hi").await?;
            talk(&mut owner, &admitted, wolf, 2316, "mission").await?;
            talk(&mut owner, &admitted, wolf, 2317, "yes").await?;
            assert_eq!(
                progress(&owner, &admitted, &authority, quest::ANIMALS).await?,
                6
            );
            assert_eq!(
                progress(&owner, &admitted, &authority, JOURNAL_ANIMALS).await?,
                6
            );
            wolf_chapter(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                [irmana, wolf, ikassis],
            )
            .await?;
            swan_chapter(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                [swan, tereban, alkestios],
                &feathers,
                &shrines,
            )
            .await?;
        }
        h.cleanup().await
    })
}

async fn wolf_chapter(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    [irmana, wolf, ikassis]: [oteryn_protocol_oteryn::world_spatial_entities::EntityRef; 3],
) -> TestResult {
    use super::l6_native_signs_fixtures::relocate;
    use super::l6_paradox_native_fixtures::step;
    use oteryn_protocol_oteryn::world_object::{
        decode_use_result, encode_item_use_intent, encode_use_intent,
    };

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    talk(owner, admitted, irmana, 2320, "hi").await?;
    let early = talk(owner, admitted, irmana, 2321, "fur").await?;
    assert!(!early.iter().any(|line| line.contains("1,000 gold")));

    relocate(
        owner,
        actor,
        session,
        Position {
            y: TENT.y + 1,
            ..TENT
        },
    )
    .await?;
    step(owner, admitted, 0, -1).await?;
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        7
    );
    let notes = encode_use_intent(&target(owner, BONES, 2)?).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let first = wire(owner, admitted, command, 2, notes.clone()).await?;
    assert_eq!(
        decode_use_result(&first).map_err(debug)?,
        UseDisposition::Committed
    );
    let command = admitted.continuity.next_command_id;
    let repeat = wire(owner, admitted, command, 2, notes).await?;
    assert_ne!(
        decode_use_result(&repeat).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(quantity(h, NOTES).await?, 1);

    relocate(owner, actor, session, CAMP).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: CAMP,
            item: PLATINUM.into(),
            count: 10,
        },
        2,
        true,
    )
    .await?;
    talk(owner, admitted, irmana, 2322, "hi").await?;
    let offer = talk(owner, admitted, irmana, 2323, "fur").await?;
    assert!(
        offer.iter().any(|line| line.contains("1,000 gold")),
        "{offer:?}"
    );
    talk(owner, admitted, irmana, 2324, "yes").await?;
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        8
    );
    assert_eq!(quantity(h, FUR).await?, 1);
    assert_eq!(quantity(h, PLATINUM).await?, 0);

    relocate(
        owner,
        actor,
        session,
        Position {
            x: STONE.x - 1,
            ..STONE
        },
    )
    .await?;
    let fur = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", FUR, "definition-r1"))
        .ok_or("native fur definition")?;
    let selected = UseIntent {
        target: UseTarget::ItemByDefinition(fur),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(owner, STONE, 1)?)),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&selected).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let early = wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_ne!(
        decode_use_result(&early).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(quantity(h, FUR).await?, 1);
    relocate(owner, actor, session, CAMP).await?;
    talk(owner, admitted, wolf, 2325, "hi").await?;
    let report = talk(owner, admitted, wolf, 2326, "mission").await?;
    assert!(
        report.iter().any(|line| line.contains("weeping stone")),
        "{report:?}"
    );
    relocate(owner, actor, session, west(STONE)).await?;
    let command = admitted.continuity.next_command_id;
    let result = wire(owner, admitted, command, 2, payload).await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(quantity(h, FUR).await?, 0);
    assert_eq!(
        progress(owner, admitted, authority, JOURNAL_ANIMALS).await?,
        9
    );

    relocate(owner, actor, session, CAMP).await?;
    talk(owner, admitted, wolf, 2327, "hi").await?;
    talk(owner, admitted, wolf, 2328, "whelp").await?;
    talk(owner, admitted, ikassis, 2329, "hi").await?;
    talk(owner, admitted, ikassis, 2330, "mission").await?;
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        11
    );
    assert_eq!(
        progress(owner, admitted, authority, JOURNAL_ANIMALS).await?,
        11
    );
    Ok(())
}

fn source_callbacks<'a>(
    callbacks: &'a [oteryn_gameplay_quests::quest::world_callbacks::Callback],
    stem: &str,
) -> Vec<&'a oteryn_gameplay_quests::quest::world_callbacks::Callback> {
    callbacks
        .iter()
        .filter(|callback| {
            callback
                .key
                .strip_prefix("oteryn:quest-callback/authored-threatened-dreams/")
                .is_some_and(|key| key.starts_with(stem))
        })
        .collect()
}

const fn south(site: Position) -> Position {
    Position {
        y: site.y + 1,
        ..site
    }
}

const fn west(site: Position) -> Position {
    Position {
        x: site.x - 1,
        ..site
    }
}

async fn swan_chapter(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    [swan, tereban, alkestios]: [oteryn_protocol_oteryn::world_spatial_entities::EntityRef; 3],
    feathers: &[&oteryn_gameplay_quests::quest::world_callbacks::Callback],
    shrines: &[&oteryn_gameplay_quests::quest::world_callbacks::Callback],
) -> TestResult {
    use super::l6_native_signs_fixtures::relocate;
    use super::l6_paradox_native_fixtures::step;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    talk(owner, admitted, swan, 2341, "hi").await?;
    talk(owner, admitted, swan, 2342, "mission").await?;
    talk(owner, admitted, swan, 2343, "yes").await?;
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        12
    );
    let first = feathers
        .first()
        .ok_or("first feather")?
        .selector
        .position
        .ok_or("feather position")?;
    relocate(owner, actor, session, south(first)).await?;
    step(owner, admitted, 0, -1).await?;
    let feather_item = "oteryn:item.tibia.i25244";
    assert_eq!(quantity(h, feather_item).await?, 0);
    relocate(owner, actor, session, CAMP).await?;
    talk(owner, admitted, tereban, 2344, "hi").await?;
    talk(owner, admitted, tereban, 2345, "cloak").await?;
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        13
    );
    for (index, feather) in feathers.iter().enumerate() {
        let site = feather.selector.position.ok_or("feather position")?;
        relocate(owner, actor, session, south(site)).await?;
        step(owner, admitted, 0, -1).await?;
        assert_eq!(quantity(h, feather_item).await?, i64::try_from(index + 1)?);
        step(owner, admitted, 0, 1).await?;
        step(owner, admitted, 0, -1).await?;
        assert_eq!(quantity(h, feather_item).await?, i64::try_from(index + 1)?);
    }
    relocate(owner, actor, session, CAMP).await?;
    talk(owner, admitted, swan, 2346, "hi").await?;
    talk(owner, admitted, swan, 2347, "mission").await?;
    assert_eq!(quantity(h, feather_item).await?, 0);
    assert_eq!(
        progress(owner, admitted, authority, quest::ANIMALS).await?,
        15
    );

    for (index, shrine) in shrines.iter().enumerate() {
        let site = shrine.selector.position.ok_or("shrine position")?;
        let gem = &shrine.item_on_target.as_ref().ok_or("shrine gem")?.item;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: CAMP,
                item: gem.clone(),
                count: 1,
            },
            u64::try_from(index)? + 3,
            true,
        )
        .await?;
        relocate(owner, actor, session, west(site)).await?;
        if index == 0 {
            assert_ne!(
                use_shrine(owner, admitted, shrine).await?,
                UseDisposition::Committed
            );
            assert_eq!(quantity(h, gem).await?, 1);
            relocate(owner, actor, session, CAMP).await?;
            talk(owner, admitted, alkestios, 2348, "hi").await?;
            talk(owner, admitted, alkestios, 2349, "mission").await?;
            assert_eq!(
                progress(owner, admitted, authority, JOURNAL_ANIMALS).await?,
                16
            );
            relocate(owner, actor, session, west(site)).await?;
        }
        assert_eq!(
            use_shrine(owner, admitted, shrine).await?,
            UseDisposition::Committed
        );
        let destination = shrine.teleport.ok_or("Feyrist destination")?;
        assert_eq!(
            super::l6_paradox_native_fixtures::at(owner, admitted).await?,
            destination
        );
        assert_eq!(quantity(h, gem).await?, 0);
        relocate(owner, actor, session, CAMP).await?;
    }
    Ok(())
}

async fn use_shrine(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    shrine: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let gem = &shrine.item_on_target.as_ref().ok_or("shrine gem")?.item;
    let definition = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", gem, "definition-r1"))
        .ok_or("native gem definition")?;
    let site = shrine.selector.position.ok_or("shrine position")?;
    let ordinal = u8::try_from(shrine.selector.ordinal.ok_or("shrine ordinal")?)?;
    let use_intent = UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(owner, site, ordinal)?)),
        use_with_position: None,
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_item_use_intent(&use_intent).map_err(debug)?;
    let reply = wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("poacher map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, ordinal)
            .ok_or("poacher placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}

async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(quantity), 0)::bigint FROM game_item_instances WHERE definition_production_key = $1 AND lifecycle = 1"
    ).bind(item).fetch_one(&h.pool).await?)
}
