//! Pinned Crystal9f NPC/startup witnesses are recorded in questl2-status and authoring.
//! Material fixtures issue only the lost backpack input; no quest progress/reward is seeded.
use super::*;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;

#[test]
fn questl2_full_sam_backpack_mine_armor_journey() -> TestResult {
    scenario("qengine-rewards-l2-sam")
}
#[test]
fn questl2_full_outfox_sealed_door_helmet_journey() -> TestResult {
    scenario("qengine-rewards-l2-outfox")
}

pub(crate) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let sam = name.ends_with("sam");
    let (x, y, floor, door, action, chest, unique) = if sam {
        (32455, 31967, -14, 1644, 40603, 2472, None)
    } else {
        (32467, 31969, -5, 6260, 41492, 2469, Some(6257))
    };
    let index = native.item_index().ok_or("native item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for id in [door, door + 1] {
        let key = format!("oteryn:item.tibia.i{id}");
        data.manifest.palette.push(bundle::PaletteEntry {
            id: reference(&key).ok_or("native door definition")?.get() - 1,
            key,
            family: bundle::Family::Item,
            terrain: None,
        });
    }
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{chest}"),
        family: bundle::Family::Terrain,
        id: 3,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    data.manifest.world = bundle::Extent {
        min_x: x - 3,
        min_y: y - 1,
        max_x: x + 7,
        max_y: y + 3,
        floors: vec![floor],
    };
    let mut grouped: BTreeMap<(u16, u16), Vec<sector::Tile>> = BTreeMap::new();
    for ty in y - 1..=y + 2 {
        for tx in x - 3..=x + 6 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (tx, ty) == (x, y) {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: sector::Attrs {
                        action: Some(action),
                        ..Default::default()
                    },
                });
            }
            if (tx, ty) == (x, y + 1) {
                items.push(sector::Item {
                    palette: 3,
                    depth: 0,
                    attrs: sector::Attrs {
                        unique,
                        ..Default::default()
                    },
                });
            }
            grouped
                .entry((tx / 32, ty / 32))
                .or_default()
                .push(sector::Tile {
                    x: tx,
                    y: ty,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items,
                });
        }
    }
    let mut sectors: Vec<_> = grouped
        .into_iter()
        .map(|((sx, sy), mut tiles)| {
            tiles.sort_by_key(|t| (t.y, t.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{map_revision:crate::map::overlay::map_revision(&base),bundle:pins,
        start:crate::map::overlay::TilePos{x:x-1,y,floor}},
        crate::foundation::WorldId::decode(&super::super::fixture::id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&super::super::fixture::id(43)).map_err(debug)?,|key|{
            use crate::content::{ReferenceItemField,ReferenceItemType};
            let policy=native.item_policy(key,"definition-r1")?.record();
            Some(crate::map::facts::ItemDefinition{reference:reference(key)?,solid:policy.attributes.blocks_movement,
                blocks_projectile:policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable:matches!(&policy.semantics.physical,ReferenceItemField::Known(p) if p.pickupable==ReferenceItemField::Known(true)),
                door:matches!(&policy.semantics.classification,ReferenceItemField::Known(c) if c.item_type==ReferenceItemField::Known(ReferenceItemType::Door))})
        }).map_err(debug)
}
async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
    expected: UseDisposition,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1)
        .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    let result =
        super::super::quest_world_callbacks::wire(owner, admitted, id, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        expected
    );
    assert_eq!(
        super::super::quest_world_callbacks::wire(owner, admitted, id, 2, payload).await?,
        result,
        "world command replay"
    );
    Ok(())
}
async fn walk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    d: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    let id = admitted.continuity.next_command_id;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(d);
    let result = super::super::quest_world_callbacks::wire(owner, admitted, id, 1, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&result).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_SOURCE_JOURNEY {name} server_version={version}");
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
    selected.extend([3, 4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let sam = name.ends_with("sam");
    let (x, y, floor) = if sam {
        (32455, 31967, -14)
    } else {
        (32467, 31969, -5)
    };
    let door = Position { x, y, floor };
    use_at(owner, admitted, door, UseDisposition::Locked).await?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut issuer = None;
    if sam {
        let [sam, kroox] = super::super::classic_quests::install_nearby(
            owner,
            admitted,
            ["oteryn:npc.sam", "oteryn:npc.kroox"],
        )
        .await?;
        let handin = callbacks
            .iter()
            .find(|c| c.key.ends_with("/sam-backpack"))
            .ok_or("Sam binding")?;
        assert_eq!(
            wire_grant(owner, admitted, sam, handin).await?.0,
            NpcIntentDisposition::Rejected
        );
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: Position { x: x - 1, y, floor },
                item: "oteryn:item.tibia.i3244".into(),
                count: 1,
            },
            5100,
            true,
        )
        .await?;
        assert_eq!(
            wire_grant(owner, admitted, sam, handin).await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(quantity(h, 3244).await?, 0);
        use_at(owner, admitted, door, UseDisposition::Locked).await?;
        let permission = callbacks
            .iter()
            .find(|c| c.key.ends_with("/sam-kroox"))
            .ok_or("Kroox binding")?;
        assert_eq!(
            wire_grant(owner, admitted, kroox, permission).await?.0,
            NpcIntentDisposition::Ok
        );
    } else {
        let [budrik] =
            super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.budrik"])
                .await?;
        issuer = Some(budrik);
        let start = callbacks
            .iter()
            .find(|c| c.key.ends_with("/outfox-start"))
            .ok_or("Outfox mission issuance absent")?;
        assert_eq!(
            wire_grant(owner, admitted, budrik, start).await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(quantity(h, 139).await?, 0);
    }
    use_at(owner, admitted, door, UseDisposition::Committed).await?;
    walk(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    )
    .await?;
    let chest = Position { x, y: y + 1, floor };
    use_at(owner, admitted, chest, UseDisposition::Committed).await?;
    assert_eq!(quantity(h, if sam { 3397 } else { 139 }).await?, 1);
    if !sam {
        let budrik = issuer.ok_or("same mission issuer")?;
        let finish = callbacks
            .iter()
            .find(|c| c.key.ends_with("/outfox-helmet"))
            .ok_or("Outfox return")?;
        assert_eq!(
            wire_grant(owner, admitted, budrik, finish).await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(quantity(h, 139).await?, 0);
        assert_eq!(quantity(h, 875).await?, 1);
    }
    use_at(
        owner,
        admitted,
        chest,
        if sam {
            UseDisposition::Rejected
        } else {
            UseDisposition::NothingToUse
        },
    )
    .await?;
    assert_eq!(quantity(h, if sam { 3397 } else { 875 }).await?, 1);
    Ok(())
}
