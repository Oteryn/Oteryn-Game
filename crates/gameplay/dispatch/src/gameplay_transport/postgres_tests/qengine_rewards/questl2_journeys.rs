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
    let botany = name.ends_with("botany");
    let bone = name.ends_with("bone");
    let sam = name.ends_with("sam");
    let (x, y, floor, door, action, chest, unique) = if botany {
        (33007, 31536, -10, 7721, 42965, 2469, None)
    } else if name.ends_with("vampire") {
        (32953, 31460, -9, 8259, 41731, 2469, None)
    } else if bone {
        (32408, 32107, -10, 103, 4205, 3133, None)
    } else if sam {
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
            id: if bone {
                data.manifest.palette.len() as u32
            } else {
                reference(&key).ok_or("native door definition")?.get() - 1
            },
            key: if bone {
                format!("oteryn:terrain.tibia.i{id}")
            } else {
                key
            },
            family: if bone {
                bundle::Family::Terrain
            } else {
                bundle::Family::Item
            },
            terrain: bone.then_some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
            }),
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
    if botany {
        for (id, appearance) in [(4, 10735), (5, 10697)] {
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
    let min_x = if botany { x - 6 } else { x - 3 };
    let min_y = if botany { y - 8 } else { y - 1 };
    data.manifest.world = bundle::Extent {
        min_x,
        min_y,
        max_x: x + 7,
        max_y: y + 3,
        floors: vec![floor],
    };
    let mut grouped: BTreeMap<(u16, u16), Vec<sector::Tile>> = BTreeMap::new();
    for ty in min_y..=y + 2 {
        for tx in min_x..=x + 6 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (tx, ty) == (x, y) {
                if bone {
                    items.clear();
                }
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: sector::Attrs {
                        action: if bone { None } else { Some(action) },
                        ..Default::default()
                    },
                });
            }
            let chest_at = if botany { (x - 3, y - 6) } else { (x, y + 1) };
            if !bone && (tx, ty) == chest_at {
                items.push(sector::Item {
                    palette: 3,
                    depth: 0,
                    attrs: sector::Attrs {
                        unique,
                        ..Default::default()
                    },
                });
            }
            if botany {
                let plant = if (tx, ty) == (x - 4, y - 7) {
                    Some(4)
                } else if (tx, ty) == (x - 5, y - 7) {
                    Some(5)
                } else {
                    None
                };
                if let Some(palette) = plant {
                    items.push(sector::Item {
                        palette,
                        depth: 0,
                        attrs: Default::default(),
                    });
                }
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
pub(super) async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
pub(super) async fn use_at(
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
    let ordinal = u8::from((p.x, p.y, p.floor) != (32408, 32107, -10));
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, ordinal)
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
pub(super) async fn walk(
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
    authority: &ReconciledCharacterAuthority<'_, '_>,
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
    if name.ends_with("bone") {
        return bone_journey(owner, admitted, h, authority).await;
    }
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

async fn bone_journey(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [muriel] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.muriel"])
            .await?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let start = callbacks
        .iter()
        .find(|c| c.key.ends_with("/l2-bone-start"))
        .ok_or("Muriel issuance")?;
    let finish = callbacks
        .iter()
        .find(|c| c.key.ends_with("/l2-bone-return"))
        .ok_or("Muriel return")?;
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("BONE_PIT_STARTING_LEVEL {level}");
    if level < 35 {
        assert_eq!(
            wire_grant(owner, admitted, muriel, start).await?.0,
            NpcIntentDisposition::Rejected,
            "Muriel requires source level 35"
        );
        assert_eq!(quantity(h, 4852).await?, 0);
        let binding = owner
            .progression_binding(admitted.game_session_id)
            .ok_or("progression")?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        // Level only the actor through the existing XP owner; no quest track is seeded.
        let amount: i64 = sqlx::query_scalar(
            "SELECT 601800-total_experience FROM game_character_progression_state",
        )
        .fetch_one(&h.pool)
        .await?;
        let outcome=h.root.commit_character_experience(authority,&h.node,fence,
            crate::durability::character_progression::ExperienceAwardRequest {
                occurrence:crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(super::super::fixture::id(92)).map_err(debug)?,
                amount:oteryn_simulation_determinism::ExactI64::new(amount),context:binding.context.clone(),
                policy_revision:binding.policy_revision.clone(),reward_revision:binding.reward_revision.clone(),policy:binding.policy.clone(),
            }).await.map_err(debug)?;
        assert!(matches!(
            outcome,
            crate::durability::character_progression::ExperienceCommitOutcome::Committed(_)
        ));
    }
    assert_eq!(
        wire_grant(owner, admitted, muriel, start).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 4852).await?, 1);
    assert_eq!(quantity(h, 131).await?, 0);
    let pit = Position {
        x: 32408,
        y: 32107,
        floor: -10,
    };
    use_at(owner, admitted, pit, UseDisposition::Rejected).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: pit,
            item: "oteryn:item.tibia.i3457".into(),
            count: 1,
        },
        5200,
        true,
    )
    .await?;
    use_at(owner, admitted, pit, UseDisposition::Committed).await?;
    assert_eq!(quantity(h, 3457).await?, 1, "shovel is not consumed");
    let remains = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<u64> {
        let map = owner.bundle_map().ok_or("map")?;
        let entries = map.facts.movement_items(crate::map::overlay::TilePos {
            x: pit.x,
            y: pit.y,
            floor: pit.floor,
        });
        let matching: Vec<_> = entries
            .iter()
            .filter(|e| e.facts.appearance_id == 3133)
            .collect();
        assert_eq!(matching.len(), 1);
        let crate::map::view::EntrySource::Added(id) = matching[0].source else {
            return Err("remains identity".into());
        };
        Ok(id.get())
    };
    let first = remains(owner)?;
    let map = owner.bundle_map().ok_or("map")?;
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("remains deadline")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert!(map.facts.movement_item(first).is_none());
    assert_eq!(
        quantity(h, 4852).await?,
        1,
        "expiry preserves empty container"
    );
    use_at(owner, admitted, pit, UseDisposition::Committed).await?;
    let second = remains(owner)?;
    assert_ne!(first, second, "expired object identity is never reused");
    let container:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i4852'").fetch_one(&h.pool).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&second.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(second),
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    let result =
        super::super::quest_world_callbacks::wire(owner, admitted, id, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::super::quest_world_callbacks::wire(owner, admitted, id, 2, payload).await?,
        result
    );
    assert!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_item(second)
            .is_none()
    );
    assert_eq!(quantity(h, 4852).await?, 0);
    assert_eq!(quantity(h, 131).await?, 1);
    let transformed:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i131'").fetch_one(&h.pool).await?;
    assert_eq!(container, transformed, "container transforms in place");
    assert_eq!(
        owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest copy")?
            .get(&admitted.game_session_id)
            .and_then(|s| s.copy.as_ref())
            .and_then(|s| s
                .tracks()
                .get("oteryn:quest-progress/quest/u8_1/tibia_tales/into_the_bone_pit"))
            .copied(),
        Some(2),
        "the next NPC command sees the committed world stage immediately"
    );
    let before: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        wire_grant(owner, admitted, muriel, finish).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 131).await?, 0);
    assert_eq!(quantity(h, 6299).await?, 1);
    let after: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after - before, 1000);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_1/tibia_tales/into_the_bone_pit'").fetch_one(&h.pool).await?,3);
    use_at(owner, admitted, pit, UseDisposition::Rejected).await?;
    assert_eq!(quantity(h, 6299).await?, 1);
    Ok(())
}
