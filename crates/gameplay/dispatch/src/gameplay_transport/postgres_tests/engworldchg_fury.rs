//! Fury admission, source stacks, actual native deaths and safe personal returns.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const PHASE: &str = "oteryn:quest-progress/world-change-journal/fury_gates/phase";
const KILLS: &str = "oteryn:quest-progress/world-change-journal/fury_gates/kills";

async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: &Callback,
    command: u64,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    super::engworldchg_journals::use_marker_at_depth(owner, admitted, callback, command, ordinal)
        .await
}

async fn assert_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    expected: Position,
) -> TestResult {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(at, super::quest_world_callbacks::local(expected));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    entry: &Callback,
    seal: &Callback,
) -> TestResult<u64> {
    use super::engworldchg_journals::{step, value};
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    let exit = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/worldchg-m31/fury_gates/egress")
        .ok_or("egress")?;
    let premium = owner.premium_coordinator;
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.fury", 28999).await?;
    assert_eq!(
        value(h, KILLS).await?,
        0,
        "a native death before admission cannot count"
    );
    assert_eq!(
        use_at(owner, admitted, entry, 1, 0).await?,
        UseDisposition::Rejected,
        "an under-level player cannot open the level-sixty passage"
    );
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 60).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, admitted, entry, 2, 0).await?,
        UseDisposition::Rejected
    );
    assert_eq!(value(h, PHASE).await?, 0);
    owner.premium_coordinator = premium;
    assert_eq!(
        use_at(owner, admitted, entry, 3, 0).await?,
        UseDisposition::Committed
    );
    assert_position(owner, admitted, entry.teleport.ok_or("landing")?).await?;
    assert_eq!(value(h, PHASE).await?, 1);
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, admitted, exit, 4, 0).await?,
        UseDisposition::Committed
    );
    assert_position(owner, admitted, entry.selector.position.ok_or("Thais")?).await?;
    assert_eq!(
        value(h, PHASE).await?,
        1,
        "early exit preserves earned access"
    );
    owner.premium_coordinator = premium;
    assert_eq!(
        use_at(owner, admitted, entry, 5, 0).await?,
        UseDisposition::Committed
    );
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.fury", 29000).await?;
    assert_eq!(
        value(h, KILLS).await?,
        0,
        "landing alone cannot bypass the native dungeon STEP"
    );
    step(owner, admitted, 6, StepDirection::South).await?;
    step(owner, admitted, 7, StepDirection::North).await?;
    assert_eq!(value(h, PHASE).await?, 2);
    assert_eq!(
        use_at(owner, admitted, seal, 8, 2).await?,
        UseDisposition::Rejected
    );
    for n in 0..10 {
        super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.fury", 29001 + n)
            .await?;
    }
    assert_eq!(value(h, KILLS).await?, 10);
    assert_eq!(
        use_at(owner, admitted, seal, 9, 2).await?,
        UseDisposition::Committed
    );
    assert_position(owner, admitted, entry.selector.position.ok_or("Thais")?).await?;
    assert_eq!(value(h, PHASE).await?, 5);
    assert_eq!(
        use_at(owner, admitted, entry, 10, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(value(h, PHASE).await?, 5, "reentry preserves completion");
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, admitted, exit, 11, 0).await?,
        UseDisposition::Committed,
        "earned egress remains available after Premium authority expires"
    );
    assert_position(owner, admitted, entry.selector.position.ok_or("Thais")?).await?;
    assert_eq!(value(h, PHASE).await?, 5);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(&entry.quest)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let loaded = h
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(loaded.tracks().get(PHASE), Some(&5));
    assert_eq!(loaded.tracks().get(KILLS), Some(&10));
    Ok(12)
}

pub(super) fn scene(
    entry: &Callback,
    seal: &Callback,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    use std::collections::BTreeMap;
    let at = entry.selector.position.ok_or("entry")?;
    let landing = entry.teleport.ok_or("landing")?;
    let seal = seal.selector.position.ok_or("seal")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let appearances = [7066, 10549, 14113, 1866, 776, 5068, 5066];
    for appearance in appearances {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: if [7066, 10549].contains(&appearance) {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: [7066, 10549].contains(&appearance).then_some(true),
                ground_speed: [7066, 10549].contains(&appearance).then_some(150),
                blocks_projectile: None,
            }),
        });
    }
    let mut sectors: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for centre in [at, landing] {
        for y in centre.y - 2..=centre.y + 2 {
            for x in centre.x - 2..=centre.x + 2 {
                // Source B3 stacks: Thais ground; dungeon landing; northern completion arch.
                let stack: &[u32] = if centre.floor == at.floor && (x, y) == (at.x, at.y) {
                    &[7066]
                } else if centre.floor == landing.floor && (x, y) == (landing.x, landing.y) {
                    &[10549, 14113, 1866]
                } else if centre.floor == seal.floor && (x, y) == (seal.x, seal.y) {
                    &[10549, 14113, 776, 5068, 5066]
                } else {
                    &[]
                };
                let items = if stack.is_empty() {
                    vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: Default::default(),
                    }]
                } else {
                    stack
                        .iter()
                        .map(|appearance| {
                            Ok(sector::Item {
                                palette: u32::try_from(
                                    appearances
                                        .iter()
                                        .position(|v| v == appearance)
                                        .ok_or("palette")?
                                        + 1,
                                )?,
                                depth: 0,
                                attrs: Default::default(),
                            })
                        })
                        .collect::<TestResult<Vec<_>>>()?
                };
                sectors
                    .entry((centre.floor, x / 32, y / 32))
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
        }
    }
    data.manifest.world = bundle::Extent {
        min_x: at.x.min(landing.x) - 2,
        min_y: at.y.min(landing.y) - 2,
        max_x: at.x.max(landing.x) + 3,
        max_y: at.y.max(landing.y) + 3,
        floors: vec![landing.floor, at.floor],
    };
    let mut sectors: Vec<_> = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
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
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
        },
        crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
