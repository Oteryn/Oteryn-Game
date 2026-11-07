//! Spike producer probe uses an original source spawn cell and unchanged native creature.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::MovementLocalPosition;
pub(super) fn active() -> bool {
    super::cosmetic_source_loot::natural_mode() == 12
}
pub(super) fn positions() -> TestResult<Option<(MovementLocalPosition, MovementLocalPosition)>> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_SUPPORT_BUNDLE").ok_or("actual support source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    for source in &b.spawns.sources {
        if !matches!(source.floor, 9 | -9) {
            continue;
        }
        for point in &source.points {
            if b.spawns
                .creatures
                .get(point.creature as usize)
                .is_none_or(|c| c.key != "oteryn:creature.demon_skeleton")
                || !(32008..=32365).contains(&point.x)
                || !(32522..=32759).contains(&point.y)
            {
                continue;
            }
            for (dx, dy) in [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)] {
                let x = i32::from(point.x) + dx;
                let y = i32::from(point.y) + dy;
                let pure = b
                    .sectors
                    .iter()
                    .filter(|s| s.floor == -9)
                    .flat_map(|s| s.tiles.iter())
                    .find(|t| i32::from(t.x) == x && i32::from(t.y) == y)
                    .is_some_and(|t| {
                        t.house == 0
                            && t.items.len() == 1
                            && t.items[0].depth == 0
                            && b.manifest
                                .palette
                                .get(t.items[0].palette as usize)
                                .is_some_and(|p| {
                                    p.terrain.is_some_and(|t| {
                                        t.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                                            && t.walkable == Some(true)
                                    })
                                })
                    });
                if pure {
                    eprintln!(
                        "SRV7_SPIKE_ACTUAL_SOURCE_POINT source={} creature=oteryn:creature.demon_skeleton target=({},{},9) player=({x},{y},9) respawn_ms={}",
                        source.key, point.x, point.y, point.respawn_ms
                    );
                    return Ok(Some((
                        MovementLocalPosition { x, y, floor: 9 },
                        MovementLocalPosition {
                            x: i32::from(point.x),
                            y: i32::from(point.y),
                            floor: 9,
                        },
                    )));
                }
            }
        }
    }
    Err(
        "actual source Demon Skeleton lacks an adjacent source-qualified pure-ground probe cell"
            .into(),
    )
}
pub(super) async fn qualify(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("actual native source")?;
    let row = native
        .reward_table()
        .row("oteryn:creature.demon_skeleton")
        .map_err(debug)?;
    assert_eq!(row.xp_amount.get(), 240);
    assert_eq!(row.corpse_item.production_key, "oteryn:item.tibia.i5963");
    let profile = native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == "oteryn:creature.demon_skeleton")
        .ok_or("Demon Skeleton native profile")?;
    let crate::content::ProjectV2AuthoringProfileData::Creature(data) = &profile.profile.data
    else {
        return Err("native creature data".into());
    };
    assert_eq!(data.health, Some(400));
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND i.definition_production_key='oteryn:item.tibia.i5963'").fetch_one(&h.pool).await?;
    assert_eq!(
        count, 1,
        "one genuine native400HP/240XP death must materialize its unchanged5963 corpse"
    );
    eprintln!(
        "SRV7_SPIKE_NATIVE_DEATH_POSTGRES_EXECUTED native_health=400 native_xp=240 actual_corpse5963_mints={count}"
    );
    Ok(())
}
