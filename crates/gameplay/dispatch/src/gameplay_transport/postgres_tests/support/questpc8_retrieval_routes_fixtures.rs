use super::fixture::{Harness, debug, id};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};

use super::TestResult;

use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

use crate::map::overlay::TilePos;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

use oteryn_protocol_oteryn::world_object::UseDisposition;

use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[derive(Clone, Copy)]
pub(super) enum Route {
    Voodoo,
    VoodooSouthFirst,
    Mastermind,
    Kormarak,
}

impl Route {
    pub(super) fn slug(self) -> &'static str {
        match self {
            Self::Voodoo | Self::VoodooSouthFirst => "voodoo",
            Self::Mastermind => "mastermind",
            Self::Kormarak => "kormarak",
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Voodoo => "authored-voodoo",
            Self::VoodooSouthFirst => "authored-voodoo-south-first",
            Self::Mastermind => "authored-mastermind",
            Self::Kormarak => "authored-kormarak",
        }
    }
    pub(super) fn anchor(self) -> Position {
        match self {
            Self::Voodoo | Self::VoodooSouthFirst => Position {
                x: 32743,
                y: 31950,
                floor: -9,
            },
            Self::Mastermind => Position {
                x: 32216,
                y: 31170,
                floor: -11,
            },
            Self::Kormarak => Position {
                x: 32357,
                y: 32171,
                floor: -7,
            },
        }
    }
    fn quest(self) -> &'static str {
        match self {
            Self::Voodoo | Self::VoodooSouthFirst => "oteryn:quest.authored.voodoo_doll_quest",
            Self::Mastermind => "oteryn:quest.authored.mastermind_potion_quest",
            Self::Kormarak => "oteryn:quest.authored.kingdom_of_kormarak_quest",
        }
    }
}

pub(super) fn route(name: &str) -> Option<Route> {
    [
        Route::Voodoo,
        Route::VoodooSouthFirst,
        Route::Mastermind,
        Route::Kormarak,
    ]
    .into_iter()
    .find(|route| route.name() == name)
}

pub(super) async fn exercise(
    route: Route,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    match route {
        Route::Mastermind => {
            return super::questpc8_mastermind_fixtures::exercise(owner, admitted, h).await;
        }
        Route::Kormarak => {
            return super::questpc8_kormarak_fixtures::exercise(owner, admitted, h).await;
        }
        Route::Voodoo | Route::VoodooSouthFirst => {}
    }
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    step(owner, admitted, 1, StepDirection::East).await?;
    let mut command = 2;
    assert_eq!(progress(h, route, "stage").await?, 1);
    let north = TilePos {
        x: 32757,
        y: 31953,
        floor: -9,
    };
    let south = TilePos {
        x: 32757,
        y: 31957,
        floor: -9,
    };
    let (first, second, first_item, first_stage) = if matches!(route, Route::Voodoo) {
        walk(owner, admitted, &mut command, 32755, 31954).await?;
        hunt(owner, admitted, "oteryn:creature.witch", 3, command).await?;
        assert_eq!(progress(h, route, "kills").await?, 3);
        (north, south, "oteryn:item.tibia.i3209", 2)
    } else {
        assert_eq!(progress(h, route, "kills").await?, 0);
        (south, north, "oteryn:item.tibia.i3046", 3)
    };
    walk(
        owner,
        admitted,
        &mut command,
        i32::from(first.x) - 1,
        i32::from(first.y),
    )
    .await?;
    assert_eq!(
        claim(owner, admitted, &mut command, first, 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h, route, "stage").await?, first_stage);
    assert_eq!(quantity(h, first_item).await?, 1);
    assert_eq!(
        claim(owner, admitted, &mut command, first, 1).await?,
        UseDisposition::Rejected
    );
    let completed: bool = sqlx::query_scalar(
            "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
        )
        .bind(id(41).as_slice())
        .bind(route.quest())
        .fetch_one(&h.pool)
        .await?;
    assert!(!completed, "one box does not finish the quest");
    walk(
        owner,
        admitted,
        &mut command,
        i32::from(second.x) - 1,
        i32::from(second.y),
    )
    .await?;
    finish(owner, admitted, h, &mut command, second, 1).await?;
    assert_eq!(quantity(h, "oteryn:item.tibia.i3209").await?, 1);
    assert_eq!(quantity(h, "oteryn:item.tibia.i3046").await?, 1);
    assert_eq!(progress(h, route, "stage").await?, 4);
    let complete:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(route.quest()).fetch_one(&h.pool).await?;
    assert!(complete);
    Ok(())
}

async fn hunt(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    creature: &str,
    count: u32,
    command: u64,
) -> TestResult {
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
    super::grizzly_npc_fixtures::questpc8_campaign::hunt(owner, admitted, creature, count, command)
        .await
}

async fn progress(h: &Harness, route: Route, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/{}/{suffix}",route.slug())).fetch_one(&h.pool).await?)
}

async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(item).fetch_one(&h.pool).await?)
}

async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    pos: TilePos,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    let target = object_target(owner, pos, ordinal)?;
    let result = use_target(owner, admitted, *command, &target).await?;
    *command += 1;
    Ok(result)
}

async fn finish(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    command: &mut u64,
    pos: TilePos,
    ordinal: u8,
) -> TestResult {
    let target = object_target(owner, pos, ordinal)?;
    assert_eq!(
        use_target(owner, admitted, *command, &target).await?,
        UseDisposition::Committed
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        use_target(owner, admitted, *command, &target).await?,
        UseDisposition::Committed
    );
    *command += 1;
    assert_eq!(
        claim(owner, admitted, command, pos, ordinal).await?,
        UseDisposition::Rejected
    );
    {
        let mut sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest cache")?;
        sessions.remove(&admitted.game_session_id);
    }
    owner.admit_quest_session(admitted).await;
    assert_eq!(
        claim(owner, admitted, command, pos, ordinal).await?,
        UseDisposition::Rejected
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, before);
    Ok(())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..512 {
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        let direction = if at.x < x {
            StepDirection::East
        } else if at.x > x {
            StepDirection::West
        } else if at.y < y {
            StepDirection::South
        } else if at.y > y {
            StepDirection::North
        } else {
            return Ok(());
        };
        step(owner, admitted, *command, direction).await?;
        *command += 1;
    }
    Err("bounded route did not arrive".into())
}

// Neutral transport fixture retaining each approved native selector and ordinal.
use oteryn_world_bundle::{bundle, sector};

type FixtureObjects = Vec<(u16, u16, Vec<u16>)>;

pub(super) fn world(route: Route) -> TestResult<crate::map::boot::BundleWorld> {
    let a = route.anchor();
    if !matches!(route, Route::Voodoo | Route::VoodooSouthFirst) {
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world_id,
                "content-1",
            )?;
        let native = controller
            .active()
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let (patches, start): (&[_], _) = match route {
            Route::Mastermind => (
                &[
                    (32214, 31167, 32218, 31173, -11),
                    (32082, 31061, 32086, 31065, -10),
                    (32145, 31119, 32147, 31121, -9),
                ],
                TilePos {
                    x: 32215,
                    y: 31170,
                    floor: -11,
                },
            ),
            Route::Kormarak => (
                &[
                    (32354, 32168, 32358, 32173, -7),
                    (32354, 32168, 32358, 32173, -8),
                    (32523, 32152, 32528, 32158, -12),
                    (32494, 32215, 32498, 32216, -12),
                    (32548, 32197, 32557, 32214, -11),
                ],
                TilePos {
                    x: 32356,
                    y: 32171,
                    floor: -7,
                },
            ),
            Route::Voodoo | Route::VoodooSouthFirst => {
                return Err("expected a native retrieval route".into());
            }
        };
        return super::questpc8_native_slice::world(native, patches, &[], start);
    }
    let (min_x, max_x, min_y, max_y, objects): (u16, u16, u16, u16, FixtureObjects) = match route {
        Route::Voodoo | Route::VoodooSouthFirst => (
            32741,
            32759,
            31948,
            31959,
            vec![(32757, 31953, vec![2473]), (32757, 31957, vec![2473])],
        ),
        Route::Mastermind | Route::Kormarak => return Err("expected the Voodoo fixture".into()),
    };
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut palette = std::collections::BTreeMap::new();
    for (_, _, items) in &objects {
        for &appearance in items {
            let id = u32::try_from(palette.len() + 1)?;
            if let std::collections::btree_map::Entry::Vacant(entry) = palette.entry(appearance) {
                entry.insert(id);
                data.manifest.palette.push(bundle::PaletteEntry {
                    key: format!("oteryn:terrain.tibia.i{appearance}"),
                    family: bundle::Family::Terrain,
                    id,
                    terrain: Some(bundle::Terrain {
                        blocks_projectile: None,
                        kind: bundle::TerrainKind::Common,
                        walkable: None,
                        ground_speed: None,
                    }),
                });
            }
        }
    }
    data.manifest.world = bundle::Extent {
        min_x,
        min_y,
        max_x: max_x + 1,
        max_y: max_y + 1,
        floors: vec![a.floor],
    };
    let mut grouped = std::collections::BTreeMap::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if let Some((_, _, appearances)) =
                objects.iter().find(|(px, py, _)| (*px, *py) == (x, y))
            {
                for appearance in appearances {
                    items.push(sector::Item {
                        palette: *palette.get(appearance).ok_or("palette")?,
                        depth: 0,
                        attrs: Default::default(),
                    });
                }
            }
            grouped
                .entry((y / 32, x / 32))
                .or_insert_with(Vec::new)
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
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: a.floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: a.x - 1,
                y: a.y,
                floor: a.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
