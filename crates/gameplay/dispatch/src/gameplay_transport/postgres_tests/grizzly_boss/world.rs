use super::*;
use std::collections::BTreeMap;

pub(super) fn source_rooms(bosses: Vec<&quest::GrizzlyBoss>) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    type SectorTiles = BTreeMap<(u16, u16), sector::Tile>;
    let first = bosses.first().ok_or("source room")?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut sectors: BTreeMap<(i8, u16, u16), SectorTiles> = BTreeMap::new();
    let mut extent = (u16::MAX, u16::MAX, 0, 0);
    let mut floors = Vec::new();
    for boss in &bosses {
        let min_x = boss.portal.x.min(boss.player_position.x).min(boss.boss_position.x) - 2;
        let max_x = boss.portal.x.max(boss.player_position.x).max(boss.boss_position.x) + 2;
        let min_y = boss.portal.y.min(boss.player_position.y).min(boss.boss_position.y) - 1;
        let max_y = boss.portal.y.max(boss.player_position.y).max(boss.boss_position.y) + 1;
        let floor = -i8::try_from(boss.portal.z)?;
        floors.push(floor);
        extent = (extent.0.min(min_x), extent.1.min(min_y), extent.2.max(max_x), extent.3.max(max_y));
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                sectors.entry((floor, x / 32, y / 32)).or_default().insert((y, x), sector::Tile {
                    x, y, flags: 0, house: 0, zones: vec![],
                    items: vec![sector::Item { palette: 0, depth: 0, attrs: Default::default() }],
                });
            }
        }
    }
    floors.sort(); floors.dedup();
    data.manifest.world = bundle::Extent { min_x: extent.0, min_y: extent.1, max_x: extent.2 + 1,
        max_y: extent.3 + 1, floors };
    let mut sectors = sectors.into_iter().map(|((floor, sx, sy), tiles)|
        bundle::Sector { floor, sx, sy, tiles: tiles.into_values().collect() }).collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&bytes, &crate::map::boot::BootPins {
        map_revision: crate::map::overlay::map_revision(&base), bundle: pins,
        start: crate::map::overlay::TilePos { x: first.portal.x - 1, y: first.portal.y,
            floor: -i8::try_from(first.portal.z)? },
    }, crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?, |_| None).map_err(debug)
}

pub(super) async fn activate(h: &Harness, server: [u8; 32], client: [u8; 32], frame: [u8; 32]) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(server.as_slice()).bind(client.as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    Ok(())
}

pub(super) async fn boss_points(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=1 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(quest::storage_track(quest::BOSS_POINTS_STORAGE)).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn expire(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_grizzly_boss_entries SET entered_at=entered_at-600001,deadline_at=deadline_at-600001 WHERE state='ACTIVE'")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
