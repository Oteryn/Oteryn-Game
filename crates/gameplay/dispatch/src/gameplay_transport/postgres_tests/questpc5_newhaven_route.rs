//! Whole selected Newhaven route: no quest flags, counters or reward items are seeded.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::{native_kill_goals, world_callbacks::Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
#[test]
fn questpc5_newhaven_complete_tutor_exploration_native_kills_and_book_route() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-newhaven-route")
}
fn p(x: u16, y: u16) -> Position {
    Position { x, y, floor: -7 }
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
async fn track(owner: &ComposedFreshAdmission<'_, '_, '_>, suffix: &str) -> TestResult<i64> {
    let copy = owner
        .root
        .read_character_quest_state(
            owner.character,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    Ok(copy
        .tracks()
        .get(&format!("oteryn:quest-progress/newhaven/{suffix}"))
        .copied()
        .unwrap_or(-1))
}
async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    let mut dialogue = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/newhaven-welcome")
        .ok_or("welcome dialogue")?;
    // This existing wire helper uses only keywords to send a contiguous hi+message
    // conversation; the real dispatcher selects and validates the actual callback.
    dialogue.keywords = vec![text.to_owned()];
    let (result, _, _) =
        super::qengine_rewards::wire_grant(owner, admitted, npc, &dialogue).await?;
    Ok(result)
}
async fn visit(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    let (approach, direction, destination) = match name {
        "school" => (p(32570, 32506), StepDirection::South, p(32570, 32507)),
        "supply" => (p(32568, 32489), StepDirection::West, p(32567, 32489)),
        _ => return Err("POI".into()),
    };
    at(owner, admitted, approach).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let payload = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&payload).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(destination)
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let started = std::time::Instant::now();
    let snapshot = super::questpc5_newhaven_rewards::snapshot;
    for name in ["school", "supply"] {
        assert!(
            oteryn_gameplay_quests::quest::world_callbacks::load()?
                .iter()
                .any(|c| c.key == format!("oteryn:quest-callback/newhaven-{name}")),
            "missing actual exploration binding {name}"
        );
    }
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    // Character setup uses the ordinary progression owner; no quest state is fabricated.
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    h.root.commit_character_experience(authority,&h.node,fence,crate::durability::character_progression::ExperienceAwardRequest {
        occurrence:crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
        amount:oteryn_simulation_determinism::ExactI64::new(100),context:binding.context.clone(),policy_revision:binding.policy_revision.clone(),reward_revision:binding.reward_revision.clone(),policy:binding.policy.clone(),
    }).await.map_err(debug)?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.gustavo_the_guard"]).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    // A real visit before accepting the welcome does not manufacture progress.
    visit(owner, admitted, "school").await?;
    assert_eq!(track(owner, "school").await?, -1);
    at(owner, admitted, p(32559, 32488)).await?;
    assert_eq!(
        say(owner, admitted, npc, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        say(owner, admitted, npc, "welcome").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(track(owner, "stage").await?, 0);
    visit(owner, admitted, "school").await?;
    assert_eq!(track(owner, "school").await?, 1);
    {
        let sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest session cache")?;
        let copy = sessions
            .get(&admitted.game_session_id)
            .and_then(|entry| entry.copy.as_ref())
            .ok_or("committed quest cache")?;
        assert_eq!(
            copy.tracks().get("oteryn:quest-progress/newhaven/school"),
            Some(&1),
            "next command sees the actual committed STEP immediately"
        );
    }
    visit(owner, admitted, "school").await?;
    assert_eq!(
        track(owner, "school").await?,
        1,
        "repeat visit never grants a second objective"
    );
    assert_eq!(track(owner, "supply").await?, -1);
    visit(owner, admitted, "supply").await?;
    assert_eq!(track(owner, "supply").await?, 1);
    at(owner, admitted, p(32559, 32488)).await?;
    assert_eq!(
        say(owner, admitted, npc, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        say(owner, admitted, npc, "mission").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(track(owner, "native_run").await?, 0);
    let _ = say(owner, admitted, npc, "report").await?;
    assert_eq!(
        snapshot(h).await?,
        (100, 0, 0, 0, 0),
        "no defence reward before ten actual native deaths"
    );
    let goal = native_kill_goals::load()?.first().ok_or("goal")?;
    let source: serde_json::Value = serde_json::from_slice(&std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/spawns/spawns-quest-training.json"
    ))?)?;
    let approaches = source["records"][0]["declaration"]["provenance"]["approaches"]
        .as_array()
        .ok_or("source approaches")?;
    let mut expected_hunting_xp = 100_i64;
    for (index, origin) in goal.origins.iter().enumerate() {
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let award = oteryn_gameplay_characters::hunting_experience::hunting_experience(
            20,
            0,
            0,
            2520,
            false,
            u32::try_from(level)?,
        )
        .map_err(debug)?;
        expected_hunting_xp += award;
        let approach = &approaches[index];
        at(
            owner,
            admitted,
            p(
                u16::try_from(approach["x"].as_u64().ok_or("x")?)?,
                u16::try_from(approach["y"].as_u64().ok_or("y")?)?,
            ),
        )
        .await?;
        let target = super::questpc5_newhaven_native_goal::capture(
            owner,
            admitted,
            &origin.creature,
            MovementLocalPosition {
                x: i32::from(origin.x),
                y: i32::from(origin.y),
                floor: i16::from(origin.floor),
            },
            2000 + u16::try_from(index)?,
        )
        .await?;
        super::questpc5_newhaven_native_goal::drain(owner, admitted, target).await?;
        assert_eq!(
            snapshot(h).await?.0,
            expected_hunting_xp,
            "ordinary native XP follows the accepted level-dependent hunting policy"
        );
        assert_eq!(
            track(owner, "troll_kills").await?,
            i64::try_from(index + 1)?
        );
    }
    assert_eq!(
        snapshot(h).await?,
        (expected_hunting_xp, 0, 0, 0, 0),
        "native base20XP plus the accepted diminishing low-level bonus"
    );
    assert_eq!(
        expected_hunting_xp, 493,
        "three40XP kills then seven39XP kills as level increases"
    );
    at(owner, admitted, p(32559, 32488)).await?;
    assert_eq!(
        say(owner, admitted, npc, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        say(owner, admitted, npc, "report").await?,
        NpcIntentDisposition::Ok
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(snapshot(h).await?, (expected_hunting_xp + 800, 1, 0, 0, 0));
    let _ = say(owner, admitted, npc, "book").await?;
    assert_eq!(
        snapshot(h).await?,
        (expected_hunting_xp + 800, 1, 0, 0, 0),
        "no book means no scarf, XP or completion"
    );
    at(owner, admitted, p(32613, 32419)).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-7, 32614, 32419, 2).ok_or("book placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
        placement,
        expected_revision: 0,
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
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    assert_eq!(
        snapshot(h).await?,
        (expected_hunting_xp + 800, 1, 0, 1, 0),
        "Book acquired by the actual source chest USE"
    );
    at(owner, admitted, p(32559, 32488)).await?;
    assert_eq!(
        say(owner, admitted, npc, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        say(owner, admitted, npc, "book").await?,
        NpcIntentDisposition::Ok
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        snapshot(h).await?,
        (expected_hunting_xp + 1600, 1, 1, 0, 1),
        "selected1600 questXP, one ring/scarf and consumed actual Book"
    );
    assert_eq!(track(owner, "stage").await?, 3);
    let done = snapshot(h).await?;
    let _ = say(owner, admitted, npc, "book").await?;
    let _ = say(owner, admitted, npc, "training").await?;
    assert_eq!(
        snapshot(h).await?,
        done,
        "whole selected quest completes once"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED newhaven_whole_route duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn source_world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette = vec![bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i51914".into(),
        family: bundle::Family::Terrain,
        id: 0,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(120),
        }),
    }];
    data.manifest.world = bundle::Extent {
        min_x: 32558,
        min_y: 32418,
        max_x: 32616,
        max_y: 32510,
        floors: vec![-7],
    };
    let mut positions = std::collections::BTreeSet::new();
    for y in 32487..=32489 {
        for x in 32558..=32561 {
            positions.insert((y, x));
        }
    }
    for origin in native_kill_goals::load()?
        .first()
        .ok_or("goal")?
        .origins
        .iter()
    {
        for y in origin.y - 1..=origin.y + 1 {
            for x in origin.x - 1..=origin.x + 1 {
                positions.insert((y, x));
            }
        }
    }
    for (x, y) in [
        (32570, 32506),
        (32570, 32507),
        (32568, 32489),
        (32567, 32489),
        (32613, 32419),
        (32614, 32419),
    ] {
        positions.insert((y, x));
    }
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for (y, x) in positions {
        grouped
            .entry((y / 32, x / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                }],
            });
    }
    for (appearance, speed) in [(408, 100), (4526, 150), (4518, 150), (16485, 110)] {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(speed),
            }),
        });
    }
    for appearance in [26996, 2472] {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
            }),
        });
    }
    for tiles in grouped.values_mut() {
        for tile in tiles {
            let (ground, protected) = match (tile.x, tile.y) {
                (32568 | 32567, 32489) => (1, true),
                (32570, 32506) => (2, true),
                (32570, 32507) => (3, true),
                (32613 | 32614, 32419) => (4, false),
                _ => (0, false),
            };
            tile.flags = u8::from(protected).into();
            tile.items[0].palette = ground;
            if (tile.x, tile.y) == (32614, 32419) {
                for palette in [5, 6] {
                    tile.items.push(sector::Item {
                        palette,
                        depth: 0,
                        attrs: sector::Attrs::default(),
                    });
                }
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
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32559,
                y: 32488,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
