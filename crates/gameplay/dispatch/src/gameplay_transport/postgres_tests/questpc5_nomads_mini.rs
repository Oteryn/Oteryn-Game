//! Selected personal three-cache route; no daily world-change state is seeded.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
const CAMPS: [(u16, u16); 3] = [(33220, 32580), (33264, 32818), (33207, 32610)];
const TRACK: &str = "oteryn:quest-progress/nomads_mini/stage";
fn p(x: u16, y: u16) -> Position {
    Position { x, y, floor: -7 }
}
#[test]
fn questpc5_nomads_three_camps_exact_independent_rewards_and_chest_robber_once() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-nomads-mini")
}
async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(TRACK).fetch_optional(&h.pool).await?.unwrap_or(-1))
}
async fn items(h: &Harness) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=ANY($1) GROUP BY definition_production_key ORDER BY definition_production_key").bind(vec!["oteryn:item.tibia.i3582","oteryn:item.tibia.i3578","oteryn:item.tibia.i7343","oteryn:item.tibia.i3032","oteryn:item.tibia.i3027","oteryn:item.tibia.i9057","oteryn:item.tibia.i3054"]).fetch_all(&h.pool).await?)
}
async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    position: Position,
) -> TestResult {
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        position,
    )
    .await
}
async fn open(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    x: u16,
    y: u16,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-7, x, y, 1).ok_or("cache key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.pinned().object_revision(key),
    };
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    assert_eq!(
        callbacks
            .iter()
            .filter(|c| c.quest == "oteryn:quest.authored.nomads_mini_world_change")
            .count(),
        6,
        "six selected exploration/cache bindings must exist"
    );
    assert_eq!(stage(h).await?, -1, "no quest progress is seeded");
    assert!(items(h).await?.is_empty());
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    for (i, (x, y)) in CAMPS.into_iter().enumerate() {
        at(owner, admitted, p(x, y - 1)).await?;
        let before = items(h).await?;
        let _ = open(owner, admitted, x, y).await?;
        assert_eq!(
            stage(h).await?,
            2 * i as i64 - 1,
            "cache cannot bypass exploration"
        );
        assert_eq!(items(h).await?, before);
        at(owner, admitted, p(x, y - 2)).await?;
        if i == 0 {
            let premium = owner.premium_coordinator.take();
            super::questpc5_legion_route::step(
                owner,
                admitted,
                oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                p(x, y - 1),
            )
            .await?;
            assert_eq!(
                stage(h).await?,
                -1,
                "missing current Premium cannot start route"
            );
            owner.premium_coordinator = premium;
            at(owner, admitted, p(x, y - 2)).await?;
        }
        super::questpc5_legion_route::step(
            owner,
            admitted,
            oteryn_protocol_oteryn::world_spatial::StepDirection::South,
            p(x, y - 1),
        )
        .await?;
        assert_eq!(stage(h).await?, 2 * i as i64);
        if i == 2 {
            let premium = owner.premium_coordinator.take();
            assert_ne!(
                open(owner, admitted, x, y).await?,
                UseDisposition::Committed
            );
            assert_eq!(
                items(h).await?,
                before,
                "premium refusal grants no partial terminal output"
            );
            assert_eq!(stage(h).await?, 4);
            owner.premium_coordinator = premium;
        }
        assert_eq!(
            open(owner, admitted, x, y).await?,
            UseDisposition::Committed
        );
        assert_eq!(stage(h).await?, 2 * i as i64 + 1);
        let after = items(h).await?;
        let _ = open(owner, admitted, x, y).await?;
        assert_eq!(items(h).await?, after, "each cache grants once");
    }
    assert_eq!(
        items(h).await?,
        vec![
            ("oteryn:item.tibia.i3027".into(), 1),
            ("oteryn:item.tibia.i3032".into(), 3),
            ("oteryn:item.tibia.i3054".into(), 1),
            ("oteryn:item.tibia.i3578".into(), 50),
            ("oteryn:item.tibia.i3582".into(), 10),
            ("oteryn:item.tibia.i7343".into(), 1),
            ("oteryn:item.tibia.i9057".into(), 1)
        ]
    );
    let achievement:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/chest_robber'").fetch_one(&h.pool).await?;
    assert_eq!(achievement, 1);
    let completed:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.authored.nomads_mini_world_change' AND completed_character_revision IS NOT NULL").fetch_one(&h.pool).await?;
    assert_eq!(completed, 1);
    let nested:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i7343'").fetch_one(&h.pool).await?;
    assert_eq!(nested, 0, "independent gems are not inside Fur Bag");
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-callback/authored-nomads-%'").fetch_one(&h.pool).await?;
    assert_eq!(receipts, 6, "exactly six real route commands commit");
    Ok(())
}
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette = vec![
        bundle::PaletteEntry {
            key: "oteryn:terrain.tibia.i231".into(),
            family: bundle::Family::Terrain,
            id: 0,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(160),
            }),
        },
        bundle::PaletteEntry {
            key: "oteryn:item.tibia.i2472".into(),
            family: bundle::Family::Item,
            id: 1,
            terrain: None,
        },
    ];
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for (cx, cy) in CAMPS {
        for y in cy - 2..=cy + 1 {
            for x in cx - 1..=cx + 1 {
                let mut entries = vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: Default::default(),
                }];
                if (x, y) == (cx, cy) {
                    entries.push(sector::Item {
                        palette: 1,
                        depth: 0,
                        attrs: sector::Attrs {
                            description: Some("A fixed nomad supply cache.".into()),
                            ..Default::default()
                        },
                    });
                }
                grouped
                    .entry((y / 32, x / 32))
                    .or_default()
                    .push(sector::Tile {
                        x,
                        y,
                        flags: 0,
                        house: 0,
                        zones: vec![],
                        items: entries,
                    });
            }
        }
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: -7,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    data.manifest.world = bundle::Extent {
        min_x: 33206,
        min_y: 32578,
        max_x: 33266,
        max_y: 32820,
        floors: vec![-7],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 33220,
                y: 32578,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            (key == "oteryn:item.tibia.i2472").then_some(crate::map::facts::ItemDefinition {
                reference: std::num::NonZeroU32::new(2)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: true,
                door: false,
            })
        },
    )
    .map_err(debug)
}
