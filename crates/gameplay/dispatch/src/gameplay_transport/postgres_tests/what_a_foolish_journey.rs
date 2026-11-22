//! The chosen five-prank journey, using production quest and Item providers.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use super::questpc7_outfit_items_fixtures::quantity;
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::starter_grant::StarterTemplate;
use crate::foundation::{ChannelRuntimeV1, NodeId, WorldId};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::num::NonZeroU64;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.what_a_foolish_quest";
const PHASE: &str = "oteryn:quest-progress/chosen/what-a-foolish/stage";
const FLOWER: Position = at(32349, 32361, -7);
const BOZO: Position = at(32313, 32183, -7);
const START: Position = at(32312, 32183, -7);
const MIRABELL: Position = at(33172, 31801, -6);
const BAKERY: Position = at(33171, 31801, -6);
const SLIME: Position = at(32405, 32159, -11);
const GUARD: Position = at(33190, 31787, -7);
const CRATE: Position = at(32397, 32216, -7);
const WATCH: Position = at(32563, 32115, -4);
const BED: Position = at(32646, 31903, -3);
const PAST: Position = at(32659, 31853, -13);
const BEARD: Position = at(32659, 31852, -13);
const BAG: Position = at(32661, 31855, -13);

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

#[test]
fn what_a_foolish_full_journey_earns_jester_rewards() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e_fixtures::scenario_thread::run("what-a-foolish", move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(admin, "what_a_foolish", true, false)
                    .await?;
                let result = journey(&h).await;
                h.cleanup().await?;
                result
            })
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED what-a-foolish duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop, native_scene};
    macro_rules! crop {
        ($file:literal, $pos:expr, $x:expr, $y:expr) => {{
            let p = $pos;
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/",
                    $file
                )),
                region: ((-p.floor) as u8, p.x / 256, p.y / 256),
                bounds: (p.x - $x, p.y - $y, p.x + $x, p.y + $y),
                start: (p.x, p.y),
                expected: Vec::new(),
            }
        }};
    }
    let mut first = crop!("region-z07-x126-y125.b3", BOZO, 2, 2);
    first.start = (START.x, START.y);
    native_scene(
        native,
        vec![
            first,
            crop!("region-z07-x126-y126.b3", FLOWER, 2, 2),
            crop!("region-z06-x129-y124.b3", MIRABELL, 2, 2),
            crop!("region-z11-x126-y125.b3", SLIME, 2, 2),
            crop!("region-z07-x129-y124.b3", GUARD, 3, 2),
            crop!("region-z07-x126-y125.b3", CRATE, 2, 2),
            crop!("region-z04-x127-y125.b3", WATCH, 2, 2),
            crop!("region-z03-x127-y124.b3", BED, 2, 2),
            crop!("region-z13-x127-y124.b3", PAST, 4, 3),
        ],
        &[],
    )
}

async fn journey(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("production generation")?;
    let native = generation.native_gameplay().ok_or("native policies")?;
    let map = world(native)?;
    let cells = map
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = map.frame_binding_digest(world_id);
    let room = pin
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(START.x),
                y: i32::from(START.y),
                floor: -i16::from(START.floor),
            },
        )
        .ok_or("journey room")?;
    let mut deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        crate::content::qualify_native_entry_room(world_id).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        8,
    )?;
    let channel_pin = ChannelContentPin::from_activation(
        world_id,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        map.map_revision_digest(),
        (
            i32::from(START.x),
            i32::from(START.y),
            -i16::from(START.floor),
        ),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin,
    )
    .map_err(debug)?;
    runtime.use_classic_creature_deaths().map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, local(START))
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);

    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = Some(&premium);
    owner.active_generation = Some(generation);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    owner.movement_cells = (&cells).into();
    let quests = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(quests.catalogue().clone()));
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    owner.admit_premium(id(40), admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?
        .item_fence();
    admitted.item_fence = Some(fence);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence,
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence,
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
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;
    super::questpc7_native_deaths_fixtures::prepare_native_hunter(&owner, &admitted).await?;
    super::qd_world_fixtures::select_talk_capabilities(
        &owner,
        &mut admitted,
        &[3, 4, 6, 11, 13, 14, 15, 16, 17, 18, 25],
    )?;
    exercise(h, &mut owner, &mut admitted).await
}

async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
) -> TestResult {
    let [bozo, mirabell] = super::classic_quests_fixtures::install_positions(
        owner,
        a,
        ["oteryn:npc.bozo", "oteryn:npc.mirabell"],
        [BOZO, MIRABELL],
    )
    .await?;
    let fence = a.item_fence.ok_or("inventory fence")?;
    let mut input = 100;
    macro_rules! stock {
        ($id:literal, $count:expr) => {{
            input += 1;
            input_item(
                owner,
                fence,
                &Offering {
                    position: START,
                    item: format!("oteryn:item.tibia.i{}", $id),
                    count: $count,
                },
                input,
                true,
            )
            .await?;
        }};
    }
    macro_rules! verify {
        ($phase:expr $(, $id:literal => $qty:expr)* $(,)?) => {{
            assert_eq!(super::questpc7_ferumbras_fixtures::value(owner, a, PHASE).await?.max(0), $phase);
            $(assert_eq!(quantity(h, $id).await?, $qty, "item {} after phase {}", $id, $phase);)*
        }};
    }
    macro_rules! say {
        ($npc:expr, $words:expr) => {{
            let command = a.continuity.next_command_id;
            super::grizzly_npc_fixtures::wire::talk(
                owner,
                a,
                $npc,
                &[(command, "hi"), (command + 1, $words)],
            )
            .await?;
        }};
    }
    macro_rules! move_to {
        ($p:expr) => {{
            // Between-location travel is fixture positioning. Quest movement uses STEP.
            super::m01b_explore_fixtures::koshei::relocate(
                owner,
                a.runtime_actor.ok_or("actor")?,
                a.game_session_id,
                $p,
            )
            .await?;
        }};
    }
    stock!(3469, 1);
    verify!(0, 3469 => 1, 102 => 0, 895 => 0);
    say!(bozo, "join");
    verify!(1, 3469 => 1, 102 => 0);
    move_to!(at(FLOWER.x - 1, FLOWER.y, FLOWER.floor));
    use_selected(owner, a, 3469, Some((FLOWER, 2))).await?;
    verify!(2, 3469 => 0, 102 => 1);
    move_to!(START);
    say!(bozo, "mission");
    verify!(3, 102 => 0);
    say!(bozo, "mission");
    verify!(4, 135 => 1, 107 => 0);
    move_to!(SLIME);
    super::questpc7_goblin_fixtures::stalker_death(owner, a, "oteryn:creature.slime", SLIME, 700)
        .await?;
    verify!(4, 135 => 1, 107 => 0);
    sample_slime(h, owner, a).await?;
    verify!(5, 135 => 0, 107 => 1);
    move_to!(START);
    say!(bozo, "mission");
    verify!(6, 107 => 0);
    say!(bozo, "mission");
    verify!(7, 119 => 0);
    stock!(3603, 24);
    stock!(3035, 6);
    move_to!(BAKERY);
    say!(mirabell, "flour");
    verify!(8, 3603 => 0, 119 => 0);
    say!(mirabell, "12 pies");
    verify!(9, 119 => 1);
    move_to!(at(GUARD.x, GUARD.y + 1, GUARD.floor));
    super::questpc7_ferumbras_fixtures::step(
        owner,
        a,
        oteryn_protocol_oteryn::world_spatial::StepDirection::North,
    )
    .await?;
    verify!(7, 119 => 0, 3135 => 1);
    move_to!(START);
    stock!(3603, 24);
    move_to!(BAKERY);
    say!(mirabell, "flour");
    verify!(8, 3603 => 0);
    say!(mirabell, "12 pies");
    verify!(9, 119 => 1);
    move_to!(START);
    say!(bozo, "mission");
    verify!(10, 119 => 0);
    say!(bozo, "mission");
    verify!(11, 117 => 0);
    for _ in 0..18 {
        stock!(2874, 1);
    }
    fill_wine_stock(h).await?;
    say!(bozo, "wine");
    verify!(12, 2874 => 0, 117 => 1);
    move_to!(at(CRATE.x - 1, CRATE.y, CRATE.floor));
    use_selected(owner, a, 117, Some((CRATE, 2))).await?;
    verify!(13, 117 => 0, 118 => 1);
    move_to!(START);
    say!(bozo, "mission");
    verify!(14, 118 => 0);
    say!(bozo, "mission");
    verify!(15, 112 => 0);
    move_to!(at(WATCH.x - 1, WATCH.y, WATCH.floor));
    claim(owner, a, WATCH).await?;
    verify!(16, 112 => 1, 895 => 0);
    move_to!(START);
    say!(bozo, "mission");
    verify!(17, 112 => 1);
    move_to!(BED);
    use_selected(owner, a, 112, None).await?;
    verify!(18, 112 => 1);
    position(owner, a, PAST).await?;
    move_to!(at(BAG.x - 1, BAG.y, BAG.floor));
    claim(owner, a, BAG).await?;
    verify!(18, 2853 => 1, 2821 => 1, 3469 => 1, 895 => 0);
    move_knife(h, owner, a).await?;
    verify!(18, 3469 => 1);
    move_to!(PAST);
    use_selected(owner, a, 3469, Some((BEARD, 1))).await?;
    verify!(19, 3469 => 1, 113 => 1);
    use_selected(owner, a, 112, None).await?;
    verify!(20, 112 => 1, 113 => 1);
    position(owner, a, BED).await?;
    move_to!(START);
    let terminal = a.continuity.next_command_id + 1;
    say!(bozo, "mission");
    verify!(21, 113 => 0, 895 => 1);
    // A new serving connection replays the command through the retained ingress
    // and then makes a fresh request under the same durable character session.
    super::grizzly_npc_fixtures::wire::talk(owner, a, bozo, &[(terminal, "mission")]).await?;
    verify!(21, 895 => 1);
    let next = a.continuity.next_command_id;
    super::grizzly_npc_fixtures::wire::talk_refused(
        owner,
        a,
        bozo,
        &[(next, "hi"), (next + 1, "mission")],
    )
    .await?;
    verify!(21, 895 => 1);
    journal(owner, a).await?;
    let outfits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:outfit.jester/quest-grant/0'")
        .fetch_one(&h.pool).await?;
    assert_eq!(outfits, 1);
    Ok(())
}

fn object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    p: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    use oteryn_gameplay_world::map::view::MapFacts;
    let map = owner.bundle_map().ok_or("world map")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, ordinal)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend(key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_selected(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    item: u16,
    anchor: Option<(Position, u8)>,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("item catalogue")?;
    let reference = native
        .item_index()
        .and_then(|index| {
            index.definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{item}"),
                "definition-r1",
            )
        })
        .ok_or("native source reference")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: anchor
            .map(|(p, ordinal)| object(owner, p, ordinal))
            .transpose()?
            .map(MapUseTarget::Object),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _) = super::item_craft_fixtures::exchange(
        owner,
        *a,
        a.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *a = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "use item {item} at {anchor:?}"
    );
    Ok(())
}

async fn claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    p: Position,
) -> TestResult {
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&object(owner, p, 1)?)
        .map_err(debug)?;
    let command = a.continuity.next_command_id;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(
            &wire(owner, a, command, 2, payload).await?
        )
        .map_err(debug)?,
        UseDisposition::Committed
    );
    Ok(())
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    expected: Position,
) -> TestResult {
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        local(expected)
    );
    Ok(())
}

async fn sample_slime(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
) -> TestResult {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|corpse| corpse.position == local(SLIME))
        .ok_or("fresh Slime corpse")?;
    let flask = instance(h, 135).await?;
    let outcome = owner
        .use_quest_tool(
            a.runtime_actor.ok_or("actor")?,
            action(a, 701)?,
            UseIntent {
                target: UseTarget::Item(NonZeroU64::MIN),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Item(
                    NonZeroU64::new(2).ok_or("corpse handle")?,
                )),
                use_with_position: None,
            },
            Some(ItemKey::Instance(flask)),
            Some(ItemKey::Instance(corpse.item.item_instance_id)),
        )
        .await
        .map_err(|()| "sample USE")?
        .ok_or("sample dispatch")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    Ok(())
}

async fn instance(h: &Harness, item: u16) -> TestResult<[u8; 16]> {
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1 ORDER BY item_instance_id LIMIT 1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
    bytes
        .try_into()
        .map_err(|_| "item instance identity".into())
}

fn action(a: &AdmittedSession, command_id: u64) -> TestResult<UseCommand> {
    Ok(UseCommand {
        game_session_id: a.game_session_id,
        command_id,
        item_fence: Some(a.item_fence.ok_or("item fence")?),
    })
}

async fn fill_wine_stock(h: &Harness) -> TestResult {
    // Trade stock is a fixture input. Bozo must consume the exact wine subtype.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) SELECT item_instance_id,world_id,2,game_character_uuid_v7() FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i2874'")
        .execute(&mut *tx).await?;
    assert_eq!(rows.rows_affected(), 18);
    tx.commit().await?;
    Ok(())
}

async fn move_knife(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    let item = instance(h, 3469).await?;
    let result = owner
        .move_loop_item(
            a.runtime_actor.ok_or("actor")?,
            action(a, 702)?,
            LoopMove {
                item: ItemKey::Instance(item),
                target: LoopTarget::MainBackpack,
                count: None,
                slot: None,
            },
        )
        .await;
    assert_eq!(result, LoopMoveResult::Committed);
    Ok(())
}

async fn journal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    let quests = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&quests)?;
    let index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|row| row.key == QUEST)
            .ok_or("quest journal index")?,
    )?;
    let payload =
        encode_quest_log_query(&QuestLogQuery::Quest { quest_index: index }).map_err(debug)?;
    let packet = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: a.continuity.next_command_id,
            command_type: 22,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut framed = u32::try_from(packet.len())?.to_be_bytes().to_vec();
    framed.extend(packet);
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(owner, *a, &framed).await?;
    let mut reader = output.as_slice();
    while !reader.is_empty() {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
            let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
            if delta.domain_id == 16
                && let Some(QuestLogView::Line(line)) =
                    decode_quest_log(delta.payload).map_err(debug)?.view
            {
                assert_eq!(line.quest_index, index);
                assert!(line.completed);
                return Ok(());
            }
        }
    }
    Err("terminal journal missing from LOG response".into())
}
