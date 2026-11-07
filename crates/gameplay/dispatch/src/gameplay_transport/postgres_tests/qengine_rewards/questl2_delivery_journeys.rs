//! L2 delivery, travel and survey source bindings enter through real NPC/world commands.
use super::*;

fn probe(case: &'static str, reward: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_delivery_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = callbacks.iter().any(|c| c.key.ends_with(reward));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE {case} duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "complete source hand-in binding is absent: {reward}"
        );
        Ok(())
    })?;
    scenario(case)
}

#[test]
fn questl2_delivery_batch_steal_from_thieves_full_journey() -> TestResult {
    probe("qengine-rewards-l2-steal", "/l2-steal-return")
}
#[test]
fn questl2_delivery_batch_troll_sabotage_full_journey() -> TestResult {
    probe("qengine-rewards-l2-troll", "/l2-troll-return")
}
#[test]
fn questl2_delivery_batch_cartography_full_journey() -> TestResult {
    probe("qengine-rewards-l2-cartography", "/l2-cartography-return")
}

pub(crate) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    use std::collections::BTreeMap;
    let index = native.item_index().ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        id: 1,
        key: "oteryn:terrain.tibia.i2473".into(),
        family: bundle::Family::Terrain,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    data.manifest.palette.push(bundle::PaletteEntry {
        id: reference("oteryn:item.tibia.i1949")
            .ok_or("native vortex")?
            .get()
            - 1,
        key: "oteryn:item.tibia.i1949".into(),
        family: bundle::Family::Item,
        terrain: None,
    });
    if name.ends_with("alliance") {
        // Combat tile qualification requires a real source ground identity.
        data.manifest.palette[0].key = "oteryn:terrain.tibia.i100".into();
        for (id, appearance) in [(3, 2472), (4, 10186), (5, 10187)] {
            data.manifest.palette.push(bundle::PaletteEntry {
                id,
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                }),
            });
        }
    }
    let books = if name.ends_with("tomes") {
        let mut rows = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .filter(|c| c.key.starts_with("oteryn:quest-callback/l2-tome-"))
            .map(|c| {
                Ok((
                    c.selector.position.ok_or("book position")?,
                    c.selector.appearance.ok_or("book appearance")?,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
        rows.sort_by_key(|(p, _)| (p.floor, p.x, p.y));
        for (n, appearance) in [2472, 2480].into_iter().enumerate() {
            data.manifest.palette.push(bundle::PaletteEntry {
                id: 100 + u32::try_from(n)?,
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                }),
            });
        }
        for appearance in [9874, 9875] {
            data.manifest.palette.push(bundle::PaletteEntry {
                id: reference(&format!("oteryn:item.tibia.i{appearance}"))
                    .ok_or("native palace door")?
                    .get()
                    - 1,
                key: format!("oteryn:item.tibia.i{appearance}"),
                family: bundle::Family::Item,
                terrain: None,
            });
        }
        rows
    } else {
        Vec::new()
    };
    let mut cells = BTreeMap::<(i8, u16, u16), Vec<sector::Item>>::new();
    let mut patch = |floor: i8, x0: u16, x1: u16, y0: u16, y1: u16| {
        for x in x0..=x1 {
            for y in y0..=y1 {
                cells.entry((floor, x, y)).or_insert_with(|| {
                    vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: Default::default(),
                    }]
                });
            }
        }
    };
    type FixtureCell = (u16, u16, i8);
    let mut portals: Vec<(FixtureCell, FixtureCell)> = Vec::new();
    let start = if name.ends_with("tomes") {
        for (z, x0, x1, y0, y1) in [
            (0, 33002, 33006, 31539, 31542),
            (-6, 33196, 33198, 31346, 31349),
            (-7, 33343, 33346, 31115, 31118),
            (-10, 33159, 33162, 31264, 31268),
        ] {
            patch(z, x0, x1, y0, y1);
        }
        portals.extend([
            ((33155, 31229, -15), (33003, 31540, 0)),
            ((33003, 31539, 0), (33344, 31116, -7)),
            ((33197, 31348, -6), (33344, 31116, -7)),
            ((33344, 31115, -7), (33160, 31265, -10)),
            ((33345, 31115, -7), (33160, 31265, -10)),
            ((33160, 31264, -10), (33156, 31229, -15)),
        ]);
        for (p, _) in &books {
            patch(p.floor, p.x - 2, p.x + 1, p.y - 1, p.y + 2);
        }
        let p = books.first().ok_or("source books")?.0;
        Position {
            x: p.x - 1,
            y: p.y,
            floor: p.floor,
        }
    } else if name.ends_with("alliance") {
        patch(-3, 33148, 33160, 31294, 31300);
        Position {
            x: 33153,
            y: 31297,
            floor: -3,
        }
    } else if name.ends_with("steal") {
        patch(-7, 33125, 33134, 32658, 32665);
        Position {
            x: 33130,
            y: 32661,
            floor: -7,
        }
    } else if name.ends_with("troll") {
        for (z, x0, x1, y0, y1) in [
            (-7, 33225, 33252, 31745, 31777),
            (-8, 33238, 33250, 31769, 31775),
            (-9, 33225, 33233, 31768, 31774),
            (-2, 33229, 33234, 31754, 31768),
            (-1, 33229, 33234, 31752, 31757),
        ] {
            patch(z, x0, x1, y0, y1);
        }
        // Ordinary fixture connectors shorten the unrelated climb. The six
        // quest gates and the imported vortex retain their actual source cells.
        portals.extend([
            ((33235, 31749, -7), (33231, 31766, -2)),
            ((33232, 31766, -2), (33233, 31748, -7)),
            ((33231, 31755, -2), (33231, 31755, -1)),
            ((33232, 31755, -1), (33231, 31756, -2)),
            ((33229, 31770, -9), (33235, 31748, -7)),
        ]);
        Position {
            x: 33240,
            y: 31758,
            floor: -7,
        }
    } else {
        patch(-7, 32204, 32212, 32294, 32301);
        for (x, y) in [(32953, 32685), (33264, 32014), (33109, 31722)] {
            patch(-7, x - 2, x + 2, y - 2, y + 2);
        }
        // Only travel between independent landmarks is compressed. Survey USE
        // still verifies each exact native source coordinate and carried map.
        for (from, to) in [
            ((32210, 32297, -7), (32952, 32685, -7)),
            ((32208, 32299, -7), (33263, 32014, -7)),
            ((32206, 32297, -7), (33108, 31722, -7)),
            ((32952, 32686, -7), (32208, 32297, -7)),
            ((33263, 32015, -7), (32208, 32297, -7)),
            ((33108, 31723, -7), (32208, 32297, -7)),
        ] {
            portals.push((from, to));
        }
        Position {
            x: 32208,
            y: 32297,
            floor: -7,
        }
    };
    if name.ends_with("steal") {
        cells
            .get_mut(&(-7, 33131, 32661))
            .ok_or("chest tile")?
            .push(sector::Item {
                palette: 1,
                depth: 0,
                attrs: Default::default(),
            });
    }
    if name.ends_with("alliance") {
        for (x, y, palette, action) in [(33154, 31297, 3, None), (33154, 31298, 4, Some(40030))] {
            cells
                .get_mut(&(-3, x, y))
                .ok_or("alliance tile")?
                .push(sector::Item {
                    palette,
                    depth: 0,
                    attrs: sector::Attrs {
                        action,
                        ..Default::default()
                    },
                });
        }
    }
    for (n, (p, _)) in books.iter().enumerate() {
        cells
            .get_mut(&(p.floor, p.x, p.y))
            .ok_or("source book tile")?
            .push(sector::Item {
                palette: if books[n].1 == 2472 { 3 } else { 4 },
                depth: 0,
                attrs: Default::default(),
            });
        let next = books[(n + 1) % books.len()].0;
        portals.push((
            (p.x - 1, p.y + 1, p.floor),
            (next.x - 1, next.y, next.floor),
        ));
    }
    if name.ends_with("tomes") {
        cells
            .get_mut(&(-10, 33161, 31265))
            .ok_or("palace door tile")?
            .push(sector::Item {
                palette: 5,
                depth: 0,
                attrs: sector::Attrs {
                    action: Some(42730),
                    ..Default::default()
                },
            });
    }
    for ((x, y, z), destination) in portals {
        cells
            .get_mut(&(z, x, y))
            .ok_or("portal tile")?
            .push(sector::Item {
                palette: 2,
                depth: 0,
                attrs: sector::Attrs {
                    teleport: Some((destination.0, destination.1, destination.2.to_le_bytes()[0])),
                    ..Default::default()
                },
            });
    }
    data.manifest.world = bundle::Extent {
        min_x: cells
            .keys()
            .map(|(_, x, _)| *x)
            .min()
            .ok_or("empty world")?,
        min_y: cells
            .keys()
            .map(|(_, _, y)| *y)
            .min()
            .ok_or("empty world")?,
        max_x: 1 + cells
            .keys()
            .map(|(_, x, _)| *x)
            .max()
            .ok_or("empty world")?,
        max_y: 1 + cells
            .keys()
            .map(|(_, _, y)| *y)
            .max()
            .ok_or("empty world")?,
        floors: cells
            .keys()
            .map(|(z, _, _)| *z)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    let mut grouped = BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for ((z, x, y), items) in cells {
        grouped
            .entry((z, x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let mut sectors = Vec::new();
    for ((floor, sx, sy), mut tiles) in grouped {
        tiles.sort_by_key(|t| (t.y, t.x));
        sectors.push(bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        });
    }
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{map_revision:crate::map::overlay::map_revision(&base),bundle:pins,start:crate::map::overlay::TilePos{x:start.x,y:start.y,floor:start.floor}},
      crate::foundation::WorldId::decode(&super::super::fixture::id(42)).map_err(debug)?,crate::foundation::ChannelId::decode(&super::super::fixture::id(43)).map_err(debug)?,|key|{
        use crate::content::{ReferenceItemField,ReferenceItemType};
        let policy=native.item_policy(key,"definition-r1")?.record();Some(crate::map::facts::ItemDefinition{reference:reference(key)?,solid:policy.attributes.blocks_movement,blocks_projectile:policy.attributes.blocks_projectile.unwrap_or(true),pickupable:matches!(&policy.semantics.physical,ReferenceItemField::Known(p) if p.pickupable==ReferenceItemField::Known(true)),door:matches!(&policy.semantics.classification,ReferenceItemField::Known(c) if c.item_type==ReferenceItemField::Known(ReferenceItemType::Door))})
      }).map_err(debug)
}

async fn stage(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    count: u16,
    n: u64,
    position: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "item fence")?
        .ok_or("item fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position,
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        n,
        true,
    )
    .await
}
pub(super) async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Position> {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok(Position {
        x: u16::try_from(at.x)?,
        y: u16::try_from(at.y)?,
        floor: -i8::try_from(at.floor)?,
    })
}
pub(super) async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    d: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(600))
        .ok_or("paced clock")?;
    super::questl2_journeys::walk(owner, admitted, d).await
}
pub(super) async fn refused_step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    d: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    let before = at(owner, admitted).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(600))
        .ok_or("paced clock")?;
    let id = admitted.continuity.next_command_id;
    let _ = super::super::quest_world_callbacks::wire(
        owner,
        admitted,
        id,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(d),
    )
    .await?;
    assert_eq!(
        at(owner, admitted).await?,
        before,
        "failed equipment/time gate returns to its actual origin"
    );
    Ok(())
}
async fn equip(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    item: u32,
    slot: i16,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::hotkey_equip as wire;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| {
            i.definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{item}"),
                "definition-r1",
            )
        })
        .ok_or("equipment reference")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("equipment clock")?;
    let id = admitted.continuity.next_command_id;
    let result = super::super::quest_world_callbacks::wire(
        owner,
        admitted,
        id,
        wire::COMMAND_TYPE_HOTKEY_EQUIP,
        wire::Request {
            definition: reference,
        }
        .encode()
        .to_vec(),
    )
    .await?;
    assert_eq!(
        wire::decode_reply(&result).map_err(debug)?,
        oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_equipment_slots e JOIN game_item_instances i USING(item_instance_id) WHERE e.slot=$1 AND i.definition_production_key=$2").bind(slot).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?,1);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([
        3,
        4,
        6,
        11,
        12,
        14,
        15,
        18,
        25,
        oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_V1,
    ]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("journey capabilities")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .command_selected(oteryn_protocol_oteryn::hotkey_equip::COMMAND_TYPE_HOTKEY_EQUIP),
        "native hotkey equipment dependencies are negotiated"
    );
    let expected_start = if name.ends_with("steal") {
        Position {
            x: 33130,
            y: 32661,
            floor: -7,
        }
    } else if name.ends_with("troll") {
        Position {
            x: 33240,
            y: 31758,
            floor: -7,
        }
    } else {
        Position {
            x: 32208,
            y: 32297,
            floor: -7,
        }
    };
    assert_eq!(
        at(owner, admitted).await?,
        expected_start,
        "native journey begins at its declared player cell"
    );
    if name.ends_with("steal") {
        steal(owner, admitted, h).await
    } else if name.ends_with("troll") {
        troll(owner, admitted, h).await
    } else {
        cartography(owner, admitted, h).await
    }
}

async fn steal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::{quantity, use_at};
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    const TRACK: &str =
        "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/steal_from_thieves";
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let start = all
        .iter()
        .find(|c| c.key.ends_with("/l2-steal-start"))
        .ok_or("Ahmet mission")?;
    let finish = all
        .iter()
        .find(|c| c.key.ends_with("/l2-steal-return"))
        .ok_or("Ahmet hand-in")?;
    let [npc] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.ahmet"]).await?;
    assert_eq!(
        wire_grant(owner, admitted, npc, start).await?.0,
        NpcIntentDisposition::Ok
    );
    let position = Position {
        x: 33130,
        y: 32661,
        floor: -7,
    };
    let chest = Position {
        x: 33131,
        y: 32661,
        floor: -7,
    };
    supply(owner, admitted, 2854, 1, 14000, position).await?;
    let nested:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2854' LIMIT 1").fetch_one(&h.pool).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
        native,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
    )
    .map_err(debug)?;
    let capacity = facts.container_capacity.ok_or("backpack capacity")?;
    for n in 1..capacity {
        supply(owner, admitted, 2815, 1, 14000 + u64::from(n), position).await?;
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_container_entries")
            .fetch_one(&h.pool)
            .await?,
        i64::from(capacity)
    );
    use_at(owner, admitted, chest, UseDisposition::Rejected).await?;
    assert_eq!(stage(h, TRACK).await?, 1);
    assert_eq!(quantity(h, 235).await?, 0);
    let filler:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_container_slots s ON e.parent_item_instance_id=s.item_instance_id AND e.character_id=s.character_id WHERE i.definition_production_key='oteryn:item.tibia.i2815' LIMIT 1").fetch_one(&h.pool).await?;
    into_bag(
        owner,
        admitted,
        *filler.as_bytes(),
        *nested.as_bytes(),
        14500,
    )
    .await?;
    use_at(owner, admitted, chest, UseDisposition::Committed).await?;
    assert_eq!(stage(h, TRACK).await?, 2);
    assert_eq!(quantity(h, 235).await?, 1);
    let before = counts(h).await?;
    assert_eq!(
        wire_reward(owner, admitted, npc, finish, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(
        counts(h).await?,
        before,
        "one free input slot cannot partially grant two reward outputs"
    );
    assert_eq!(quantity(h, 235).await?, 1);
    assert_eq!(quantity(h, 3725).await?, 0);
    assert_eq!(stage(h, TRACK).await?, 2);
    let filler:sqlx::types::Uuid=sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_container_slots s ON e.parent_item_instance_id=s.item_instance_id AND e.character_id=s.character_id WHERE i.definition_production_key='oteryn:item.tibia.i2815' LIMIT 1").fetch_one(&h.pool).await?;
    into_bag(
        owner,
        admitted,
        *filler.as_bytes(),
        *nested.as_bytes(),
        14501,
    )
    .await?;
    assert_eq!(
        wire_grant(owner, admitted, npc, finish).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h, TRACK).await?, 3);
    assert_eq!(quantity(h, 235).await?, 0);
    assert_eq!(quantity(h, 3725).await?, 100);
    assert_eq!(quantity(h, 3031).await?, 100);
    assert_eq!(quantity(h, 2815).await?, i64::from(capacity - 1));
    use_at(owner, admitted, chest, UseDisposition::Rejected).await?;
    Ok(())
}

async fn cartography(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::{quantity, use_at_ordinal};
    use oteryn_protocol_oteryn::{
        world_object::UseDisposition,
        world_spatial::StepDirection::{East, South, West},
    };
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let start = all
        .iter()
        .find(|c| c.key.ends_with("/l2-cartography-start"))
        .ok_or("Rotem issue")?;
    let finish = all
        .iter()
        .find(|c| c.key.ends_with("/l2-cartography-return"))
        .ok_or("Rotem return")?;
    let [npc] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.rotem_valos"])
            .await?;
    assert_eq!(
        wire_grant(owner, admitted, npc, start).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 22107).await?, 1);
    assert_ne!(
        wire_reward(owner, admitted, npc, finish, false).await?.0,
        NpcIntentDisposition::Ok,
        "an incomplete survey cannot finish"
    );
    for (direction, landmark) in [
        (
            East,
            Position {
                x: 32953,
                y: 32685,
                floor: -7,
            },
        ),
        (
            South,
            Position {
                x: 33264,
                y: 32014,
                floor: -7,
            },
        ),
        (
            West,
            Position {
                x: 33109,
                y: 31722,
                floor: -7,
            },
        ),
    ] {
        step(owner, admitted, direction).await?;
        step(owner, admitted, direction).await?;
        assert_eq!(
            at(owner, admitted).await?,
            Position {
                x: landmark.x - 1,
                ..landmark
            }
        );
        use_at_ordinal(owner, admitted, landmark, 0, UseDisposition::Committed).await?;
        assert_eq!(
            quantity(h, 22107).await?,
            1,
            "survey marks retain the issued map"
        );
        use_at_ordinal(owner, admitted, landmark, 0, UseDisposition::Rejected).await?;
        step(owner, admitted, South).await?;
        assert_eq!(
            at(owner, admitted).await?,
            Position {
                x: 32208,
                y: 32297,
                floor: -7
            }
        );
    }
    assert_eq!(
        wire_grant(owner, admitted, npc, finish).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 22107).await?, 0);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/cartography_101'").fetch_one(&h.pool).await?,1);
    Ok(())
}

async fn troll(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::quantity;
    use oteryn_protocol_oteryn::world_spatial::StepDirection::{East, North, South, West};
    const TRACK: &str = "oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline";
    // The source jump requires real movement speed as well as worn boots.
    // Earn an ordinary character level; never seed quest progress or its timer.
    let authority=owner.character;
    super::super::qengine_outfit_stories::earn_level(owner,admitted,authority,100).await?;
    owner.refresh_quest_session(admitted.game_session_id,false).await;

    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let start = all
        .iter()
        .find(|c| c.key.ends_with("/l2-troll-start"))
        .ok_or("Jerom mission")?;
    let necklace = all
        .iter()
        .find(|c| c.key.ends_with("/l2-troll-necklace"))
        .ok_or("chief necklace")?;
    let finish = all
        .iter()
        .find(|c| c.key.ends_with("/l2-troll-return"))
        .ok_or("Jerom hand-in")?;
    let [jerom, chief] = super::super::classic_quests::install_at(
        owner,
        admitted,
        ["oteryn:npc.jerom", "oteryn:npc.chief_grarkharok"],
        [
            Position {
                x: 33240,
                y: 31757,
                floor: -7,
            },
            Position {
                x: 33231,
                y: 31754,
                floor: -1,
            },
        ],
    )
    .await?;
    assert_eq!(
        wire_grant(owner, admitted, jerom, start).await?.0,
        NpcIntentDisposition::Ok
    );
    let source = Position {
        x: 33240,
        y: 31758,
        floor: -7,
    };
    supply(owner, admitted, 5460, 1, 15000, source).await?;
    supply(owner, admitted, 3079, 1, 15001, source).await?;
    // Empty equipment initialization is fixture admission state, not worn custody.
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING").bind(super::super::fixture::id(41).as_slice()).execute(&h.pool).await?;
    for _ in 0..7 {
        step(owner, admitted, West).await?;
    }
    for _ in 0..10 {
        step(owner, admitted, North).await?;
    }
    refused_step(owner, admitted, East).await?;
    assert_eq!(stage(h, TRACK).await?, 1);
    equip(owner, admitted, 5460, 1, h).await?;
    step(owner, admitted, East).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33247,
            y: 31771,
            floor: -8
        }
    );
    for _ in 0..6 {
        step(owner, admitted, West).await?;
    }
    step(owner, admitted, South).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33228,
            y: 31771,
            floor: -9
        }
    );
    step(owner, admitted, East).await?;
    step(owner, admitted, North).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33235,
            y: 31748,
            floor: -7
        },
        "existing native vortex returns to the surface"
    );
    step(owner, admitted, South).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33231,
            y: 31766,
            floor: -2
        }
    );
    step(owner, admitted, North).await?;
    step(owner, admitted, North).await?;
    step(owner, admitted, North).await?;
    refused_step(owner, admitted, North).await?;
    equip(owner, admitted, 3079, 4, h).await?;
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    refused_step(owner, admitted, North).await?;
    step(owner, admitted, South).await?;
    step(owner, admitted, South).await?;
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_millis(600)).ok_or("paced clock")?;
    let first=admitted.continuity.next_command_id;
    let commands=(0..3).map(|n|(first+n,1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(North))).collect();
    let results=super::super::quest_world_callbacks::wire_sequence(owner,admitted,commands).await?;
    assert_eq!(results.len(),3);
    for result in results {
        assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&result).map_err(debug)?,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved);
    }
    let jump_clock: Option<(i64,i64)> = sqlx::query_as(
        "SELECT value,floor(extract(epoch FROM now()))::bigint FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/jump_timer'"
    ).fetch_optional(&h.pool).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position { x: 33231, y: 31759, floor: -2 },
        "native fresh jump; timer/current PostgreSQL seconds={jump_clock:?}"
    );
    for _ in 0..4 {
        step(owner, admitted, North).await?;
    }
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33231,
            y: 31755,
            floor: -1
        }
    );

    assert_eq!(
        wire_grant(owner, admitted, chief, necklace).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 7754).await?, 1);
    assert_eq!(stage(h, TRACK).await?, 2);
    step(owner, admitted, East).await?;
    for _ in 0..4 {
        step(owner, admitted, South).await?;
    }
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33231,
            y: 31763,
            floor: -2
        }
    );
    for _ in 0..3 {
        step(owner, admitted, South).await?;
    }
    step(owner, admitted, East).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33233,
            y: 31748,
            floor: -7
        }
    );
    step(owner, admitted, East).await?;
    for _ in 0..6 {
        step(owner, admitted, West).await?;
    }
    step(owner, admitted, South).await?;
    step(owner, admitted, East).await?;
    step(owner, admitted, North).await?;
    assert_eq!(
        at(owner, admitted).await?,
        Position {
            x: 33235,
            y: 31748,
            floor: -7
        }
    );
    for _ in 0..4 {
        step(owner, admitted, East).await?;
    }
    for _ in 0..10 {
        step(owner, admitted, South).await?;
    }
    step(owner, admitted, East).await?;

    assert_eq!(
        wire_grant(owner, admitted, jerom, finish).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 7754).await?, 0);
    assert_eq!(quantity(h, 646).await?, 1);
    assert_eq!(stage(h, TRACK).await?, 3);
    Ok(())
}
