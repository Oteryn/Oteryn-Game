use super::fixture::{Harness, debug};
use super::l6_paradox_native_fixtures::at;
use super::l6_world_pickups_fixtures::{
    ExpectedTeleport, NativeCrop, native_scene_with_appearances,
};
use super::quest_world_callbacks_fixtures::{callback, offering, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::content::native_gameplay::NativeGameplayState;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, compose_tile, placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    CAPABILITY_DIAGONAL_MOVEMENT_V1, CAPABILITY_PACED_MOVEMENT_V1, StepDirection, StepDisposition,
    decode_step_result_profile, encode_step_intent,
};

const fn p(x: u16, y: u16, z: i8) -> Position {
    Position { x, y, floor: -z }
}

const fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

fn start(route: &str) -> TestResult<Position> {
    Ok(match route {
        "portals" => p(32670, 31653, 10),
        "keys" => p(32793, 31572, 7),
        "puzzle" => p(32804, 31587, 1),
        "devil" => p(32575, 32216, 15),
        "corpse" | "corpse-predecessor" | "corpse-shovel-rollback" | "corpse-scythe-rollback" => {
            p(32148, 32207, 7)
        }
        "bags" | "bags-reverse" => p(32804, 31584, 2),
        "tower" | "tower-reverse" => p(33073, 32170, 2),
        _ => return Err("classic quest route".into()),
    })
}

pub(super) fn run(route: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(route, move || {
        super::quest_world_callbacks_fixtures::scenario(
            &format!("classic-native-{route}"),
            start(route)?,
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED classic-native-{route} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

const PORTALS: [(Position, StepDirection, Position); 4] = [
    (
        p(32670, 31653, 10),
        StepDirection::West,
        p(32679, 31673, 10),
    ),
    (
        p(32676, 31646, 10),
        StepDirection::West,
        p(32725, 31589, 12),
    ),
    (p(32795, 31576, 5), StepDirection::West, p(32812, 31577, 5)),
    (p(32812, 31577, 5), StepDirection::North, p(32794, 31577, 5)),
];

const KEY_SOURCES: [(Position, u16, u8); 8] = [
    (p(32794, 31572, 7), 3001, 2),
    (p(32802, 31576, 7), 3002, 1),
    (p(32790, 31593, 7), 3003, 1),
    (p(32807, 31576, 6), 3004, 1),
    (p(32792, 31590, 6), 3005, 1),
    (p(32801, 31579, 6), 3006, 1),
    (p(32813, 31576, 5), 3007, 1),
    (p(32800, 31582, 2), 3008, 1),
];

const KEY_DOORS: [Position; 8] = [
    p(32814, 31597, 7),
    p(32796, 31597, 7),
    p(32811, 31597, 7),
    p(32809, 31593, 6),
    p(32794, 31582, 6),
    p(32814, 31595, 6),
    p(32797, 31592, 4),
    p(32804, 31585, 1),
];

fn crop(z: u8, bounds: (u16, u16, u16, u16), start: Position) -> TestResult<NativeCrop> {
    macro_rules! region {
        ($z:literal, $x:literal, $y:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z",
                $z,
                "-x",
                $x,
                "-y",
                $y,
                ".b3"
            ))
        };
    }
    let rx = bounds.0 / 256;
    let ry = bounds.1 / 256;
    let bytes: &'static [u8] = match (z, rx, ry) {
        (1, 128, 123) => region!("01", "128", "123"),
        (2, 128, 123) => region!("02", "128", "123"),
        (2, 129, 125) => region!("02", "129", "125"),
        (4, 128, 123) => region!("04", "128", "123"),
        (5, 128, 123) => region!("05", "128", "123"),
        (6, 127, 123) => region!("06", "127", "123"),
        (6, 128, 123) => region!("06", "128", "123"),
        (7, 125, 125) => region!("07", "125", "125"),
        (7, 128, 123) => region!("07", "128", "123"),
        (8, 125, 125) => region!("08", "125", "125"),
        (9, 125, 125) => region!("09", "125", "125"),
        (9, 128, 123) => region!("09", "128", "123"),
        (10, 127, 123) => region!("10", "127", "123"),
        (12, 127, 123) => region!("12", "127", "123"),
        (14, 126, 125) => region!("14", "126", "125"),
        (14, 127, 125) => region!("14", "127", "125"),
        (15, 126, 125) => region!("15", "126", "125"),
        (15, 127, 125) => region!("15", "127", "125"),
        _ => return Err("classic quest crop region".into()),
    };
    Ok(NativeCrop {
        bytes,
        region: (z, rx, ry),
        bounds,
        start: (start.x, start.y),
        expected: vec![],
    })
}

pub(super) fn world(
    native: &NativeGameplayState,
    route: &str,
    start: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    let crop = |z, bounds| crop(z, bounds, start);
    let mut teleports = Vec::new();
    let mut extras: &[u16] = &[];
    let specs = match route {
        "portals" => {
            for ((x, y, z), (dx, dy, dz)) in [
                ((32669, 31653, 10), (32679, 31673, 10)),
                ((32675, 31646, 10), (32725, 31589, 12)),
                ((32794, 31576, 5), (32812, 31577, 5)),
                ((32812, 31576, 5), (32794, 31577, 5)),
            ] {
                teleports.push(ExpectedTeleport {
                    position: (x, y, z),
                    ordinal: 1,
                    destination: (dx, dy, dz),
                });
            }
            vec![
                crop(10, (32669, 31653, 32672, 31655))?,
                crop(10, (32675, 31646, 32677, 31647))?,
                crop(10, (32679, 31673, 32680, 31674))?,
                crop(12, (32725, 31589, 32726, 31590))?,
                crop(5, (32794, 31576, 32796, 31578))?,
                crop(5, (32812, 31576, 32814, 31578))?,
            ]
        }
        "keys" => {
            extras = &[2773, 1282, 1772];
            let around = |point: Position| {
                crop(
                    point.floor.unsigned_abs(),
                    (point.x - 1, point.y - 1, point.x + 2, point.y + 2),
                )
            };
            // Key 3007 is beside the eastern portal.
            teleports.push(ExpectedTeleport {
                position: (32812, 31576, 5),
                ordinal: 1,
                destination: (32794, 31577, 5),
            });
            let mut specs: Vec<_> = KEY_SOURCES
                .iter()
                .filter(|(point, _, _)| point.floor != -7)
                .map(|(point, _, _)| around(*point))
                .collect::<Result<_, _>>()?;
            specs.insert(0, crop(7, (32789, 31571, 32816, 31599))?);
            specs.extend(
                KEY_DOORS
                    .into_iter()
                    .filter(|point| point.floor != -7)
                    .map(around)
                    .collect::<Result<Vec<_>, _>>()?,
            );
            specs
        }
        "puzzle" => {
            extras = &[2773, 1282, 1772];
            vec![
                crop(1, (32801, 31582, 32807, 31588))?,
                crop(2, (32800, 31581, 32806, 31585))?,
                crop(7, (32789, 31578, 32794, 31597))?,
                crop(9, (32815, 31598, 32820, 31602))?,
                crop(6, (32701, 31639, 32702, 31640))?,
            ]
        }
        "devil" => {
            extras = &[435];
            vec![
                crop(15, (32574, 32215, 32578, 32218))?,
                crop(14, (32587, 32098, 32594, 32104))?,
                crop(14, (32467, 32117, 32470, 32120))?,
                crop(14, (32450, 32141, 32483, 32177))?,
                crop(15, (32454, 32142, 32485, 32178))?,
            ]
        }
        "corpse" | "corpse-predecessor" | "corpse-shovel-rollback" | "corpse-scythe-rollback" => {
            extras = &[594, 3651, 3652, 3605];
            vec![
                crop(7, (32146, 32207, 32149, 32208))?,
                crop(8, (32142, 32205, 32171, 32222))?,
                crop(9, (32163, 32204, 32192, 32227))?,
            ]
        }
        "bags" | "bags-reverse" => vec![crop(2, (32800, 31582, 32806, 31585))?],
        "tower" | "tower-reverse" => vec![crop(2, (33071, 32169, 33080, 32172))?],
        _ => return Err("classic quest crop".into()),
    };
    native_scene_with_appearances(native, specs, &teleports, extras)
}

async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
) -> TestResult {
    super::l6_native_signs_fixtures::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        at,
    )
    .await
    .map_err(|error| format!("classic quest fixture landing at {at:?}: {error}").into())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let before = at(owner, admitted).await?;
    let reply = wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        1,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result_profile(&reply, true, true)
            .map_err(debug)?
            .disposition,
        StepDisposition::Moved,
        "STEP {direction:?} from {before:?}"
    );
    Ok(())
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("classic quest map")?;
    let key = placement_key(tile(position), ordinal).ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    position: Position,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    owner.clock_origin -= std::time::Duration::from_millis(500);
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target(owner, position, ordinal)?)
            .map_err(debug)?;
    let reply = wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    decode_use_result(&reply).map_err(debug)
}

async fn use_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    tool: u16,
    position: Position,
    ordinal: u8,
    offset: usize,
) -> TestResult<UseDisposition> {
    owner.clock_origin -= std::time::Duration::from_millis(500);
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(owner, position, ordinal)?)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native items")?;
    let reference = native
        .item_index()
        .ok_or("item index")?
        .definition_ref(
            "Item",
            &format!("oteryn:item.tibia.i{tool}"),
            "definition-r1",
        )
        .ok_or("tool definition")?;
    let (next, disposition, _) = super::item_craft_fixtures::exchange_selected(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
        offset,
    )
    .await?;
    *admitted = next;
    Ok(disposition)
}

async fn approach(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    object: Position,
) -> TestResult {
    for (dx, dy) in [
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
        (0, 0),
    ] {
        let next = Position {
            x: u16::try_from(i32::from(object.x) + dx)?,
            y: u16::try_from(i32::from(object.y) + dy)?,
            ..object
        };
        if walkable(owner, next) {
            return relocate(owner, admitted, next).await;
        }
    }
    Err(format!("no usable approach to {object:?}").into())
}

fn has(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: Position,
    appearance: u16,
) -> TestResult<bool> {
    let map = owner.bundle_map().ok_or("scene map")?;
    Ok(compose_tile(&map.overlay, &map.facts, tile(position))
        .map_err(debug)?
        .ok_or("scene tile")?
        .entries
        .iter()
        .any(|entry| entry.facts.appearance_id == appearance))
}

async fn keyed_door(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    position: Position,
    number: u16,
) -> TestResult {
    approach(owner, admitted, position).await?;
    use_carried_key(owner, admitted, h, position, number).await
}

async fn use_carried_key(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    position: Position,
    number: u16,
) -> TestResult {
    use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(k.item_instance_id) FROM game_item_keys k
         JOIN game_item_container_entries e USING (item_instance_id) WHERE k.key_number=$1",
    )
    .bind(i32::from(number))
    .fetch_one(&h.pool)
    .await?;
    owner.clock_origin -= std::time::Duration::from_millis(500);
    let outcome = owner
        .use_bundle_key(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: placement_key(tile(position), 1).ok_or("keyed door")?,
                item_fence: admitted.item_fence,
            },
            ItemKey::Instance(bytes.try_into().map_err(|_| "carried key identity")?),
            &target(owner, position, 1)?,
        )
        .await
        .ok_or("numbered door use")?;
    assert_eq!(
        outcome.disposition,
        UseDisposition::Committed,
        "source door {position:?} number {number}"
    );
    assert!(walkable(owner, position), "opened source door {position:?}");
    Ok(())
}

async fn loose_key(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    position: Position,
    ordinal: u8,
    number: u16,
) -> TestResult {
    use crate::gameplay_transport::{
        connection::UseCommand,
        item_move::{LoopMove, LoopMoveResult, LoopTarget},
        item_view::ItemKey,
    };
    approach(owner, admitted, position).await?;
    let map = owner.bundle_map().ok_or("loose key map")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 8_000 + u64::from(number),
        item_fence: admitted.item_fence,
    };
    let source = placement_key(tile(position), ordinal).ok_or("loose key")?;
    let visible = compose_tile(&map.overlay, &map.facts, tile(position))
        .map_err(debug)?
        .ok_or("loose key tile")?;
    assert!(
        visible.entries.iter().any(|entry| matches!(
            entry.source,
            crate::map::view::EntrySource::Base { placement_key, movable: true, .. }
                if placement_key == source
        )),
        "the map view must publish a handle for key {number}"
    );
    let intent = LoopMove {
        item: ItemKey::MapBase {
            bundle_digest: map.overlay.base().digest(),
            placement_key: source,
            reset_epoch: owner.reset_epoch,
        },
        count: None,
        target: LoopTarget::MainBackpack,
        slot: None,
    };
    assert_eq!(
        owner
            .move_loop_item(admitted.runtime_actor.ok_or("actor")?, command, intent)
            .await,
        LoopMoveResult::Committed,
        "loose source key {number}"
    );
    Ok(())
}

async fn completed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    quest: &str,
) -> TestResult<bool> {
    let fence = admitted.item_fence.ok_or("claim fence")?;
    let snapshot = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    Ok(snapshot
        .states()
        .get(quest)
        .is_some_and(|state| state.completed))
}

async fn reward(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    position: Position,
    ordinal: u8,
) -> TestResult {
    assert_eq!(
        use_object(owner, admitted, position, ordinal).await?,
        UseDisposition::Committed,
        "native claim at {position:?}"
    );
    assert_eq!(
        use_object(owner, admitted, position, ordinal).await?,
        UseDisposition::NothingToUse,
        "native claim at {position:?} is once per character"
    );
    Ok(())
}

async fn reward_pair(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    tower: bool,
    reverse: bool,
) -> TestResult {
    let (quest, positions, root, children): (_, _, _, [&[(u16, i64)]; 2]) = if tower {
        (
            "oteryn:quest.dragon_tower_quest",
            [p(33072, 32169, 2), p(33078, 32169, 2)],
            2854,
            [
                &[(266, 1), (268, 1), (3448, 60), (3449, 30)],
                &[(3029, 2), (3350, 1)],
            ],
        )
    } else {
        (
            "oteryn:quest.draconia_quest",
            [p(32803, 31582, 2), p(32804, 31582, 2)],
            2853,
            [&[(3284, 1), (3297, 1)], &[(3051, 1), (3081, 1)]],
        )
    };
    if !tower {
        // The level-25 door occupies the only approach inside this room.
        let gate = p(32804, 31583, 2);
        relocate(owner, admitted, p(32804, 31584, 2)).await?;
        super::questpc6_fibula_route_fixtures::level(owner, admitted, 25).await?;
        assert_eq!(
            use_object(owner, admitted, gate, 1).await?,
            UseDisposition::Committed
        );
        step(owner, admitted, StepDirection::North).await?;
    }
    for (index, selected) in if reverse { [1, 0] } else { [0, 1] }
        .into_iter()
        .enumerate()
    {
        let pos = positions[selected];
        if tower {
            use super::l6_paradox_native_fixtures::walk;
            walk(owner, admitted, p(pos.x, 32170, 2)).await?;
        }
        reward(owner, admitted, pos, 1).await?;
        assert_eq!(completed(owner, admitted, quest).await?, index == 1);
    }
    let roots: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r
         JOIN game_item_instances i USING (item_instance_id) LIMIT 3",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(roots, vec![(format!("oteryn:item.tibia.i{root}"), 1); 2]);
    let actual: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r
         JOIN game_item_nested_entries e ON e.parent_item_instance_id=r.item_instance_id
         JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
         ORDER BY i.definition_production_key LIMIT 7",
    )
    .fetch_all(&h.pool)
    .await?;
    let mut expected = children
        .into_iter()
        .flatten()
        .map(|(i, count)| (format!("oteryn:item.tibia.i{i}"), *count))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(actual, expected);
    if !tower {
        use crate::content::ReferenceItemField::Known;
        let key = "oteryn:item.tibia.i3081";
        let profile = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|native| native.item_policy(key, "definition-r1"))
            .ok_or("stone skin amulet definition")?;
        assert!(
            matches!(&profile.record().semantics.charges, Known(charges) if charges.count == Known(5))
        );
        let override_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING (item_instance_id)
             WHERE i.definition_production_key=$1",
        ).bind(key).fetch_one(&h.pool).await?;
        assert_eq!(
            override_count, 0,
            "the Crystal reward uses the full five-charge default"
        );
    }
    Ok(())
}

async fn draconia_puzzle(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let landing = p(32701, 31639, 6);
    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(at(owner, admitted).await?, p(32803, 31587, 1));
    let (cache, _, ordinal) = KEY_SOURCES[7];
    approach(owner, admitted, cache).await?;
    reward(owner, admitted, cache, ordinal).await?;
    keyed_door(owner, admitted, h, KEY_DOORS[7], 3008).await?;
    walk_to(owner, admitted, KEY_DOORS[7], 0).await?;
    for x in [32803, 32805] {
        let lever = p(x, 31584, 1);
        assert_eq!(
            use_object(owner, admitted, lever, 1).await?,
            UseDisposition::Committed
        );
        assert!(has(owner, lever, 2773)?);
    }
    walk_to(owner, admitted, p(32804, 31587, 1), 0).await?;
    step(owner, admitted, StepDirection::East).await?;
    assert_eq!(at(owner, admitted).await?, landing);
    for (lever, obstacle, appearance) in [
        (p(32792, 31595, 7), p(32792, 31581, 7), 1282),
        (p(32792, 31579, 7), p(32790, 31594, 7), 1772),
    ] {
        approach(owner, admitted, lever).await?;
        assert!(has(owner, obstacle, appearance)?);
        assert_eq!(
            use_object(owner, admitted, lever, 1).await?,
            UseDisposition::Committed
        );
        assert!(!has(owner, obstacle, appearance)?);
        assert_eq!(
            use_object(owner, admitted, lever, 1).await?,
            UseDisposition::Committed
        );
        assert!(has(owner, obstacle, appearance)?);
    }
    let escape = callback("draconia-escape-35012")?;
    let fence = admitted.item_fence.ok_or("escape fence")?;
    for paid in [false, true] {
        if paid {
            offering(owner, fence, &escape.offerings[0], 900).await?;
        }
        relocate(owner, admitted, p(32816, 31599, 9)).await?;
        step(owner, admitted, StepDirection::West).await?;
        assert_eq!(
            at(owner, admitted).await?,
            if paid { landing } else { p(32818, 31599, 9) }
        );
        let burned: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(burned, i64::from(paid));
    }
    let gate = p(32804, 31583, 2);
    relocate(owner, admitted, p(32804, 31584, 2)).await?;
    for (level, expected) in [
        (24, UseDisposition::Locked),
        (25, UseDisposition::Committed),
    ] {
        super::questpc6_fibula_route_fixtures::level(owner, admitted, level).await?;
        assert_eq!(use_object(owner, admitted, gate, 1).await?, expected);
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    route: &str,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                11,
                CAPABILITY_PACED_MOVEMENT_V1,
                15,
                18,
                CAPABILITY_DIAGONAL_MOVEMENT_V1,
            ],
        )
        .ok_or("classic quest capabilities")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_DIAGONAL_MOVEMENT_V1)
    );
    match route {
        "portals" => {
            for (from, direction, destination) in PORTALS {
                relocate(owner, admitted, from).await?;
                step(owner, admitted, direction).await?;
                assert_eq!(at(owner, admitted).await?, destination);
            }
        }
        "keys" => {
            for ((position, number, ordinal), door) in KEY_SOURCES.into_iter().zip(KEY_DOORS) {
                if number == 3003 {
                    for lever in [p(32792, 31595, 7), p(32792, 31579, 7)] {
                        approach(owner, admitted, lever).await?;
                        assert_eq!(
                            use_object(owner, admitted, lever, 1).await?,
                            UseDisposition::Committed
                        );
                    }
                }
                if matches!(number, 3002 | 3008) {
                    approach(owner, admitted, position).await?;
                    reward(owner, admitted, position, ordinal).await?;
                } else {
                    loose_key(owner, admitted, position, ordinal, number).await?;
                }
                keyed_door(owner, admitted, h, door, number).await?;
            }
            let numbers: Vec<i32> = sqlx::query_scalar(
                "SELECT key_number FROM game_item_keys ORDER BY key_number LIMIT 9",
            )
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(numbers, (3001..=3008).collect::<Vec<_>>());
        }
        "puzzle" => draconia_puzzle(owner, admitted, h).await?,
        "devil" => devil(owner, admitted, h).await?,
        "corpse" | "corpse-predecessor" | "corpse-shovel-rollback" | "corpse-scythe-rollback" => {
            corpse(owner, admitted, h, route).await?
        }
        "bags" | "bags-reverse" | "tower" | "tower-reverse" => {
            reward_pair(
                owner,
                admitted,
                h,
                route.starts_with("tower"),
                route.ends_with("reverse"),
            )
            .await?;
        }
        _ => return Err("classic quest dispatch route".into()),
    }
    Ok(())
}

fn walkable(owner: &ComposedFreshAdmission<'_, '_, '_>, at: Position) -> bool {
    use crate::content::{CollisionClass, LogicalCell};
    owner.movement_cells.index().lookup(
        owner.movement_cells.scope(),
        LogicalCell {
            x: i32::from(at.x),
            y: i32::from(at.y),
            z: -i32::from(at.floor),
        },
    ) == Ok(CollisionClass::Walkable)
}

async fn walk_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    reach: u16,
) -> TestResult<usize> {
    use crate::map::floor::{Ground, resolve_landing};
    use crate::movement::CardinalStep;
    use std::collections::{BTreeMap, VecDeque};
    let origin = at(owner, admitted).await?;
    let map = owner.bundle_map().ok_or("route map")?;
    let directions = [
        (CardinalStep::North, StepDirection::North),
        (CardinalStep::East, StepDirection::East),
        (CardinalStep::South, StepDirection::South),
        (CardinalStep::West, StepDirection::West),
        (CardinalStep::NorthEast, StepDirection::NorthEast),
        (CardinalStep::SouthEast, StepDirection::SouthEast),
        (CardinalStep::SouthWest, StepDirection::SouthWest),
        (CardinalStep::NorthWest, StepDirection::NorthWest),
    ];
    let mut queue = VecDeque::from([origin]);
    let mut seen = BTreeMap::from([(origin, None)]);
    let goal = loop {
        let point = queue
            .pop_front()
            .ok_or_else(|| format!("no source route from {origin:?} to {target:?}"))?;
        if point.floor == target.floor
            && point.x.abs_diff(target.x).max(point.y.abs_diff(target.y)) <= reach
        {
            break point;
        }
        let cell = (
            i32::from(point.x),
            i32::from(point.y),
            -i32::from(point.floor),
        );
        for (cardinal, direction) in directions {
            let Ok((x, y, z)) = resolve_landing(
                &map.floors,
                |(x, y, z)| {
                    let Some((x, y, z)) = u16::try_from(x)
                        .ok()
                        .zip(u16::try_from(y).ok())
                        .zip(i8::try_from(z).ok())
                        .map(|((x, y), z)| (x, y, z))
                    else {
                        return Ground::Absent;
                    };
                    let at = p(x, y, z);
                    if walkable(owner, at) {
                        Ground::Walkable
                    } else if map.overlay.base().tile(x, y, -z).is_some() {
                        Ground::Blocked
                    } else {
                        Ground::Absent
                    }
                },
                |_| true,
                cell,
                cardinal,
            ) else {
                continue;
            };
            let next = p(x.try_into()?, y.try_into()?, z.try_into()?);
            if let std::collections::btree_map::Entry::Vacant(entry) = seen.entry(next) {
                entry.insert(Some((point, direction)));
                queue.push_back(next);
            }
        }
    };
    let mut path = Vec::new();
    let mut cursor = goal;
    while let Some((previous, direction)) = seen[&cursor] {
        path.push((cursor, direction));
        cursor = previous;
    }
    let mut fire_fields = 0;
    for (position, direction) in path.into_iter().rev() {
        step(owner, admitted, direction).await?;
        assert_eq!(at(owner, admitted).await?, position);
        if has(owner, position, 2123)? || has(owner, position, 2124)? || has(owner, position, 2125)?
        {
            fire_fields += 1;
        }
    }
    Ok(fire_fields)
}

async fn corpse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    route: &str,
) -> TestResult {
    const QUEST: &str = "oteryn:quest.dragon_corpse_quest";
    let fence = admitted.item_fence.ok_or("corpse tool fence")?;
    for (n, item) in [(901, 3457), (902, 3453)] {
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence,
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: start("corpse")?,
                item: format!("oteryn:item.tibia.i{item}"),
                count: 1,
            },
            n,
            true,
        )
        .await?;
    }
    step(owner, admitted, StepDirection::West).await?;
    step(owner, admitted, StepDirection::West).await?;
    assert_eq!(at(owner, admitted).await?, p(32145, 32207, 8));
    if route == "corpse-predecessor" {
        let hash = oteryn_gameplay_quests::quest::npc_rules::fixture_dragon_corpse_predecessor();
        let mut save = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *save)
            .await?;
        let character = fence.character_id.as_bytes().as_slice();
        for statement in [
            "UPDATE game_character_quest_states SET pinned_content_revision='content-main-era', definition_hash=$1 WHERE quest_key=$2 AND character_id=encode($3,'hex')::uuid",
            "UPDATE game_character_quest_receipts SET pinned_content_revision='content-main-era', definition_hash=$1 WHERE quest_key=$2 AND character_id=encode($3,'hex')::uuid",
        ] {
            let updated = sqlx::query(statement)
                .bind(hash.as_slice())
                .bind(QUEST)
                .bind(character)
                .execute(&mut *save)
                .await?;
            assert_eq!(updated.rows_affected(), 1);
        }
        save.commit().await?;
    }
    let hole = p(32166, 32216, 8);
    walk_to(owner, admitted, hole, 0).await?;
    assert_eq!(at(owner, admitted).await?, hole);
    if route == "corpse-shovel-rollback" {
        return corpse_tool_rollback(owner, admitted, h, 3457, hole, 0).await;
    }
    assert_eq!(
        use_tool(owner, admitted, 3457, hole, 0, 0).await?,
        UseDisposition::Committed
    );
    assert!(has(owner, hole, 594)?);
    let shovel_credit: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2 AND character_id=encode($3,'hex')::uuid",
    )
    .bind(QUEST)
    .bind("oteryn:quest-progress/chosen-source/dragon_corpse_quest/s2/shoveled")
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(shovel_credit, Some(1));
    walk_to(owner, admitted, p(32167, 32216, 8), 0).await?;
    step(owner, admitted, StepDirection::West).await?;
    assert_eq!(at(owner, admitted).await?, p(32166, 32216, 9));
    for x in 32170..=32172 {
        let wheat = p(x, 32214, 9);
        walk_to(owner, admitted, wheat, 1).await?;
        if route == "corpse-scythe-rollback" {
            return corpse_tool_rollback(owner, admitted, h, 3453, wheat, 1).await;
        }
        assert_eq!(
            use_tool(owner, admitted, 3453, wheat, 1, 0).await?,
            UseDisposition::Committed
        );
        assert!(has(owner, wheat, 3651)?);
    }
    let fire_fields = walk_to(owner, admitted, p(32178, 32224, 9), 0).await?;
    assert!(
        fire_fields > 0,
        "the route must cross the native fire fields"
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    for stage in ["s1", "s2/shoveled", "s2", "s3"] {
        assert_eq!(
            state.tracks().get(&format!(
                "oteryn:quest-progress/chosen-source/dragon_corpse_quest/{stage}"
            )),
            Some(&1),
            "stage {stage}"
        );
    }
    assert!(!completed(owner, admitted, QUEST).await?);
    reward(owner, admitted, p(32179, 32224, 9), 1).await?;
    assert!(completed(owner, admitted, QUEST).await?);
    let children: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r
         JOIN game_item_nested_entries e ON e.parent_item_instance_id=r.item_instance_id
         JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
         ORDER BY i.definition_production_key LIMIT 3",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        children,
        [
            ("oteryn:item.tibia.i3374".into(), 1),
            ("oteryn:item.tibia.i3430".into(), 1)
        ]
    );
    Ok(())
}

async fn corpse_tool_rollback(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    tool: u16,
    position: Position,
    ordinal: u8,
) -> TestResult {
    sqlx::raw_sql(
        "CREATE SEQUENCE refused_terrain_credit;
         CREATE FUNCTION refuse_terrain_credit() RETURNS trigger LANGUAGE plpgsql
         SECURITY DEFINER SET search_path=public,pg_temp AS $$
         BEGIN
             PERFORM nextval('refused_terrain_credit');
             RAISE EXCEPTION 'injected Dragon Corpse credit write failure';
         END $$;
         CREATE TRIGGER refuse_terrain_credit BEFORE INSERT ON game_character_quest_receipts
         FOR EACH ROW WHEN (NEW.transition_key LIKE 'oteryn:quest-terrain-use/dragon-corpse-%')
         EXECUTE FUNCTION refuse_terrain_credit();",
    )
    .execute(&h.pool)
    .await?;
    let command = admitted.continuity.next_command_id.to_string();
    let session = admitted.game_session_id;
    let result = use_tool(owner, admitted, tool, position, ordinal, 0).await;
    let attempted: bool = sqlx::query_scalar("SELECT is_called FROM refused_terrain_credit")
        .fetch_one(&h.pool)
        .await?;
    assert!(
        attempted,
        "the real USE must attempt to persist its quest credit"
    );
    let (terrain, craft): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_terrain_tool_receipts
                   WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric),
                (SELECT count(*) FROM game_item_craft_records
                   WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric)",
    )
    .bind(session.as_bytes().as_slice())
    .bind(command)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        (terrain, craft),
        (0, 0),
        "a failed quest write must roll back the native tool USE"
    );
    assert!(!matches!(result, Ok(UseDisposition::Committed)));
    Ok(())
}

async fn devil(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    const QUEST: &str = "oteryn:quest.devil_helmet_quest";
    let first_key = p(32576, 32216, 15);
    reward(owner, admitted, first_key, 1).await?;
    keyed_door(owner, admitted, h, p(32592, 32102, 14), 3667).await?;
    let cache = p(32589, 32100, 14);
    walk_to(owner, admitted, cache, 1).await?;
    reward(owner, admitted, cache, 0).await?;

    // The plate and grate are on different branches. Admit the player holding
    // the plate at that branch, while the same claimant continues via the grate.
    let fence = admitted.item_fence.ok_or("sewer fence")?;
    let peer = super::private_channel_peers::active_peer(
        h,
        owner,
        fence,
        350,
        *owner.world_id.as_bytes(),
        "Plate Holder",
    )
    .await?;
    let plate_start = p(32468, 32118, 14);
    let holder = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(peer.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                super::quest_world_callbacks_fixtures::local(plate_start),
            )
            .map_err(debug)?;
        actor
    };
    let mut plate_holder = *admitted;
    plate_holder.game_session_id = peer.game_session_id;
    plate_holder.item_fence = Some(peer);
    plate_holder.runtime_actor = Some(holder);
    owner.admit_progression_session(&mut plate_holder).await;
    {
        let runtime = owner.runtime.lock().await;
        let mut actors = owner.states.spell_states.lock().await;
        let claimant = admitted.runtime_actor.ok_or("sewer claimant")?;
        let facts = actors
            .get(&runtime, claimant, admitted.game_session_id)
            .ok_or("sewer claimant vitals")?
            .character_facts();
        actors
            .initialize(
                &runtime,
                holder,
                peer.game_session_id,
                facts,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("plate holder vitals")?;
    }
    let sewer = p(32482, 32170, 14);
    assert!(!has(owner, sewer, 435)?);
    let (moved, _) = owner
        .paced_step(holder, peer.game_session_id, StepDirection::South)
        .await;
    assert_eq!(moved.disposition, StepDisposition::Moved);
    assert_eq!(at(owner, &plate_holder).await?, p(32468, 32119, 14));
    assert!(has(owner, sewer, 435)?);
    approach(owner, admitted, sewer).await?;
    let (reply, _) = super::pressure_plate_sewer_fixtures::use_sewer(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        tile(sewer),
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(at(owner, admitted).await?, p(32482, 32170, 15));
    assert!(has(owner, sewer, 435)?);
    owner.clock_origin -= std::time::Duration::from_millis(500);
    let (moved, _) = owner
        .paced_step(holder, peer.game_session_id, StepDirection::North)
        .await;
    assert_eq!(moved.disposition, StepDisposition::Moved);
    assert_eq!(at(owner, &plate_holder).await?, plate_start);
    assert!(!has(owner, sewer, 435)?);

    let gate = p(32479, 32174, 15);
    walk_to(owner, admitted, gate, 1).await?;
    for (level, expected) in [
        (29, UseDisposition::Locked),
        (30, UseDisposition::Committed),
    ] {
        super::questpc6_fibula_route_fixtures::level(owner, admitted, level).await?;
        assert_eq!(use_object(owner, admitted, gate, 1).await?, expected);
    }
    walk_to(owner, admitted, gate, 0).await?;
    let ladder = p(32469, 32169, 15);
    walk_to(owner, admitted, ladder, 0).await?;
    assert_eq!(
        use_object(owner, admitted, ladder, 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(at(owner, admitted).await?.floor, -14);
    let upper = p(32462, 32153, 14);
    walk_to(owner, admitted, upper, 1).await?;
    use_carried_key(owner, admitted, h, upper, 3610).await?;
    // The north-facing stairs send a descent one tile south of the trapdoor.
    walk_to(owner, admitted, p(32461, 32156, 15), 0).await?;
    for (index, carrier) in [
        p(32459, 32144, 15),
        p(32465, 32148, 15),
        p(32466, 32148, 15),
    ]
    .into_iter()
    .enumerate()
    {
        walk_to(owner, admitted, carrier, 1).await?;
        reward(owner, admitted, carrier, 1).await?;
        assert_eq!(completed(owner, admitted, QUEST).await?, index == 2);
    }
    keyed_door(owner, admitted, h, p(32462, 32151, 15), 3610).await?;
    let rewards: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r
         JOIN game_item_instances i USING (item_instance_id)
         WHERE i.definition_production_key <> 'oteryn:item.tibia.i2969'
         ORDER BY i.definition_production_key LIMIT 4",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        rewards,
        [
            ("oteryn:item.tibia.i3029".into(), 4),
            ("oteryn:item.tibia.i3269".into(), 1),
            ("oteryn:item.tibia.i3356".into(), 1)
        ]
    );
    Ok(())
}
