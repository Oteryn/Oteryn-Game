use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

pub(super) const KEEPER: &str = "bigfoot-daily-keeper";
pub(super) const MATCHMAKER: &str = "bigfoot-daily-matchmaker";
pub(super) const EXTERMINATION: &str = "bigfoot-daily-extermination";

pub(super) fn contact(name: &str) -> Position {
    if name == MATCHMAKER {
        Position {
            x: 32963,
            y: 31875,
            floor: -9,
        }
    } else {
        Position {
            x: 32805,
            y: 31798,
            floor: -10,
        }
    }
}

pub(super) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let Position { x, y, floor } = contact(name);
    let start = TilePos { x: x - 1, y, floor };
    // The compiler includes all floors of these source regions. That also
    // exercises ordinals after the pinned World omits provisional entries.
    let maximum = if name == MATCHMAKER {
        TilePos { x: 33041, ..start }
    } else {
        start
    };
    super::source_world_crop::compiled_regions(start, maximum, start, native)
}

fn track(name: &str) -> String {
    format!("oteryn:quest-progress/quest/u9_60/bigfoots_burden/{name}")
}

async fn value(h: &Harness, name: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(track(name)).fetch_one(&h.pool).await?)
}

async fn carried(h: &Harness, appearance: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{appearance}")).fetch_one(&h.pool).await?)
}

async fn use_crystal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command_id: u64,
    appearance: u16,
    callback: Option<&Callback>,
) -> TestResult {
    let target = if let Some(callback) = callback {
        use crate::content::{CollisionClass, LogicalCell};
        let Position { x, y, floor } = callback.selector.position.ok_or("crystal position")?;
        let cells = &owner.movement_cells;
        let (px, py) = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .into_iter()
            .find(|&(x, y)| {
                cells.index().lookup(
                    cells.scope(),
                    LogicalCell {
                        x: i32::from(x),
                        y: i32::from(y),
                        z: -i32::from(floor),
                    },
                ) == Ok(CollisionClass::Walkable)
            })
            .ok_or("walkable crystal approach")?;
        super::questpc8_mystery_geometry_fixtures::arrive(
            owner,
            admitted,
            TilePos {
                x: px,
                y: py,
                floor,
            },
        )
        .await?;
        let map = owner.bundle_map().ok_or("crystal map")?;
        let key = oteryn_world_bundle::bundle::placement_key(
            floor,
            x,
            y,
            u8::try_from(callback.selector.ordinal.ok_or("crystal ordinal")?)?,
        )
        .ok_or("crystal placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        }))
    } else {
        None
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?
        .item_fence();
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .map_err(debug)?;
    let definition = format!("oteryn:item.tibia.i{appearance}");
    let source = inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .find(|entry| entry.item.definition.production_key == definition)
        .map(|entry| ItemKey::Instance(entry.item.item_instance_id))
        .ok_or("selected carried crystal")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let actor = admitted.runtime_actor.ok_or("crystal user")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: target,
        use_with_position: None,
    };
    let outcome = if intent.use_with_map_item.is_some() {
        owner
            .tame_mount(actor, command, intent, Some(source), None, true)
            .await
    } else {
        owner
            .use_event_item(actor, command, intent, Some(source), false)
            .await
    };
    let result = outcome
        .map_err(|()| "crystal USE dispatch")?
        .ok_or("crystal USE handler")?
        .disposition;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source {appearance}, {:?}",
        callback.map(|c| &c.key)
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let (npc, keyword, rank, mission, timeout, acceptance, rank_gain, tokens) = match name {
        KEEPER => (
            "oteryn:npc.commander_stone",
            "keeper",
            25,
            "mission_crystal_keeper",
            "crystal_keeper_timout",
            "harmonic crystal",
            5,
            1,
        ),
        MATCHMAKER => (
            "oteryn:npc.gnomeral",
            "matchmaker",
            120,
            "mission_matchmaker",
            "matchmaker_timeout",
            "lonely crystal",
            10,
            2,
        ),
        EXTERMINATION => (
            "oteryn:npc.commander_stone",
            "extermination",
            30,
            "mission_exterminators",
            "exterminators_timeout",
            "Kill 10 wigglers",
            5,
            1,
        ),
        _ => return Err("Bigfoot daily fixture".into()),
    };
    let Position { x, y, floor } = contact(name);
    let [npc] =
        super::classic_quests_fixtures::install_positions(owner, admitted, [npc], [contact(name)])
            .await?;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[
            (track("rank"), rank),
            (track("quest_line"), if name == KEEPER { 25 } else { 26 }),
        ],
        114,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::classic_quests_fixtures::talk(owner, admitted, npc, 8001, "hi").await?;
    let lines = super::classic_quests_fixtures::talk(owner, admitted, npc, 8002, keyword).await?;
    assert!(
        lines.iter().any(|line| line.contains(acceptance)),
        "{lines:?}"
    );
    assert_eq!(value(h, mission).await?, 1);

    match name {
        KEEPER => {
            assert_eq!(carried(h, 15703).await?, 1);
            let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
            let crystals = callbacks
                .iter()
                .filter(|c| {
                    c.key
                        .starts_with("oteryn:quest-callback/bigfoot-daily/crystal-keeper-")
                })
                .take(5)
                .collect::<Vec<_>>();
            assert_eq!(crystals.len(), 5);
            for (index, crystal) in crystals.into_iter().enumerate() {
                use_crystal(
                    owner,
                    admitted,
                    8050 + u64::try_from(index)?,
                    15703,
                    Some(crystal),
                )
                .await?;
                assert_eq!(
                    value(h, "repaired_crystal_count").await?,
                    i64::try_from(index)? + 1
                );
            }
            assert_eq!(carried(h, 15703).await?, 1);
        }
        MATCHMAKER => {
            assert_eq!(carried(h, 15802).await?, 1);
            let target = value(h, "matchmaker_id_needed").await?;
            assert!((15809..=15815).contains(&target));
            use_crystal(owner, admitted, 8050, 15802, None).await?;
            assert_eq!(carried(h, 15802).await?, 0);
            assert_eq!(carried(h, 15801).await?, 1);
            let key = format!("oteryn:quest-callback/bigfoot-daily/matchmaker-{target}");
            let crystal = oteryn_gameplay_quests::quest::world_callbacks::load()?
                .into_iter()
                .find(|c| c.key == key)
                .ok_or("assigned matchmaker crystal")?;
            use_crystal(owner, admitted, 8051, 15801, Some(&crystal)).await?;
            assert_eq!(value(h, "matchmaker_status").await?, 1);
            assert_eq!(carried(h, 15801).await?, 0);
        }
        EXTERMINATION => {
            owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("native gameplay")?
                .install_companion_policies(&mut *owner.runtime.lock().await)
                .map_err(debug)?;
            for count in 1..=10 {
                super::qengine_kill_stories_fixtures::death(
                    owner,
                    admitted,
                    "oteryn:creature.wiggler",
                    8100 + count,
                )
                .await?;
                assert_eq!(value(h, "exterminated_count").await?, i64::try_from(count)?);
            }
        }
        _ => unreachable!(),
    }

    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos { x: x - 1, y, floor },
    )
    .await?;
    super::classic_quests_fixtures::talk(owner, admitted, npc, 8201, "hi").await?;
    let report = super::classic_quests_fixtures::talk(owner, admitted, npc, 8202, "report").await?;
    assert!(
        report.iter().any(|line| line.contains(keyword)),
        "{report:?}"
    );
    let before_report: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let lines = super::classic_quests_fixtures::talk(owner, admitted, npc, 8203, keyword).await?;
    assert!(
        lines
            .iter()
            .any(|line| line.contains("gnomish supply package")),
        "{lines:?}"
    );
    assert_eq!(value(h, mission).await?, 0);
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    assert!((before_report + 72_000..=now + 72_000).contains(&value(h, timeout).await?));
    assert_eq!(value(h, "rank").await?, rank + rank_gain);
    assert_eq!(carried(h, 16128).await?, tokens);
    assert_eq!(carried(h, 15698).await?, 1);
    let achievement = match name {
        KEEPER => "oteryn:achievement/crystal_keeper",
        MATCHMAKER => "oteryn:achievement/crystals_in_love",
        EXTERMINATION => "oteryn:achievement/one_foot_vs_many",
        _ => unreachable!(),
    };
    let earned: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(achievement)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(earned, 1);

    if name == KEEPER {
        assert_eq!(carried(h, 15703).await?, 0);
        assert_eq!(value(h, "repaired_crystal_count").await?, -1);
    } else if name == MATCHMAKER {
        assert_eq!(value(h, "matchmaker_id_needed").await?, -1);
        assert_eq!(value(h, "matchmaker_status").await?, -1);
        let cooldown =
            super::classic_quests_fixtures::talk(owner, admitted, npc, 8204, keyword).await?;
        assert!(
            cooldown.iter().any(|line| line.contains("have to wait")),
            "{cooldown:?}"
        );
        assert_eq!(carried(h, 15802).await?, 0);

        // Elapse the recorded cooldown in the fixture. The next acceptance
        // selects the identical source rule and must mint a fresh package.
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h,
            owner,
            admitted,
            &[(track(timeout), now - 1)],
            115,
        )
        .await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let next =
            super::classic_quests_fixtures::talk(owner, admitted, npc, 8205, keyword).await?;
        assert!(
            next.iter().any(|line| line.contains(acceptance)),
            "{next:?}"
        );
        assert_eq!(value(h, mission).await?, 1);
        assert_eq!(carried(h, 15802).await?, 1);
    } else {
        assert_eq!(value(h, "exterminated_count").await?, -1);
    }
    Ok(())
}
