//! Whole selected personal Noodles route; global escape calendars are not seeded.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
fn p(x: u16, y: u16) -> Position {
    Position { x, y, floor: -6 }
}
#[test]
fn questpc5_noodles_king_collar_explore_live_dog_use_and_report_once() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-noodles")
}
async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    pos: Position,
) -> TestResult {
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        pos,
    )
    .await
}
async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/noodles/stage' AND character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_optional(&h.pool).await?.unwrap_or(-1))
}
async fn count(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(item).fetch_one(&h.pool).await?)
}
async fn xp(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    suffix: &str,
) -> TestResult<NpcIntentDisposition> {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-reward/noodles-{suffix}"))
        .ok_or("Noodles dialogue")?;
    let (result, _, _) =
        super::qengine_rewards::wire_grant(owner, admitted, npc, &callback).await?;
    Ok(result)
}
async fn collar(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: EntityRef,
) -> TestResult<UseDisposition> {
    let _reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i14156", "definition-r1"))
        .ok_or("Dog Collar reference")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("collar handle")?),
        use_with: Some(target),
        use_with_map_item: None,
        use_with_position: None,
    };
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    assert_eq!(
        callbacks
            .iter()
            .filter(|c| c.quest == "oteryn:quest.authored.noodles_is_gone_mini_world_change")
            .count(),
        3,
        "King collar, literal dog USE and report callbacks must exist"
    );
    assert_eq!(stage(h).await?, -1, "no quest progress is seeded");
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
    let [king, dog] = super::classic_quests::install_source(
        owner,
        admitted,
        ["oteryn:npc.king_tibianus", "oteryn:npc.noodles"],
    )
    .await?;
    let before = xp(h).await?;
    assert_eq!(count(h, "oteryn:item.tibia.i14156").await?, 0);
    assert_eq!(count(h, "oteryn:item.tibia.i3035").await?, 0);
    at(owner, admitted, p(32310, 32172)).await?;
    let _ = say(owner, admitted, king, "report").await?;
    assert_eq!(stage(h).await?, -1);
    assert_eq!(
        say(owner, admitted, king, "collar").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h).await?, 0);
    assert_eq!(count(h, "oteryn:item.tibia.i14156").await?, 1);
    let _ = say(owner, admitted, king, "collar").await?;
    assert_eq!(
        count(h, "oteryn:item.tibia.i14156").await?,
        1,
        "collar once"
    );
    assert_ne!(
        collar(owner, admitted, king).await?,
        UseDisposition::Committed,
        "same live collar cannot target King instead of Noodles"
    );
    assert_eq!(stage(h).await?, 0);
    assert_eq!(count(h, "oteryn:item.tibia.i14156").await?, 1);
    at(owner, admitted, p(32314, 32178)).await?;
    assert_ne!(
        collar(owner, admitted, dog).await?,
        UseDisposition::Committed,
        "finding step precedes collar use"
    );
    assert_eq!(count(h, "oteryn:item.tibia.i14156").await?, 1);
    super::questpc5_legion_route::step(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::West,
        p(32313, 32178),
    )
    .await?;
    assert_eq!(stage(h).await?, 1);
    super::questpc5_legion_route::step(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        p(32314, 32178),
    )
    .await?;
    at(owner, admitted, p(32310, 32172)).await?;
    assert_ne!(
        collar(owner, admitted, king).await?,
        UseDisposition::Committed,
        "wrong canonical NPC after exploration"
    );
    assert_ne!(
        collar(owner, admitted, dog).await?,
        UseDisposition::Committed,
        "correct dog out of reach"
    );
    assert_eq!(stage(h).await?, 1);
    assert_eq!(count(h, "oteryn:item.tibia.i14156").await?, 1);
    at(owner, admitted, p(32314, 32178)).await?;
    let use_command = admitted.continuity.next_command_id;
    assert_eq!(
        collar(owner, admitted, dog).await?,
        UseDisposition::Committed
    );
    let replay = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("collar handle")?),
        use_with: Some(dog),
        use_with_map_item: None,
        use_with_position: None,
    };
    let body =
        oteryn_protocol_oteryn::world_object::encode_item_use_intent(&replay).map_err(debug)?;
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(use_command).map_err(debug)?,
    );
    assert_eq!(
        owner
            .root
            .reconcile_quest_carried_use(owner.character, fence.character_id, command, body)
            .await
            .map_err(debug)?,
        Some(true),
        "exact frozen actor USE after input consumption"
    );
    assert_eq!(
        owner
            .root
            .reconcile_quest_carried_use(owner.character, fence.character_id, command, vec![255])
            .await
            .map_err(debug)?,
        Some(false),
        "changed actor USE cannot rebind historical consumption"
    );

    assert_eq!(stage(h).await?, 2);
    assert_eq!(
        count(h, "oteryn:item.tibia.i14156").await?,
        0,
        "exact selected collar consumed"
    );
    let _ = collar(owner, admitted, dog).await?;
    assert_eq!(stage(h).await?, 2);
    assert_eq!(xp(h).await?, before);
    at(owner, admitted, p(32310, 32172)).await?;
    assert_eq!(
        say(owner, admitted, king, "report").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h).await?, 3);
    assert_eq!(xp(h).await?, before + 1000);
    assert_eq!(count(h, "oteryn:item.tibia.i3035").await?, 5);
    let _ = say(owner, admitted, king, "report").await?;
    assert_eq!(xp(h).await?, before + 1000);
    assert_eq!(count(h, "oteryn:item.tibia.i3035").await?, 5);
    let achievements:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/dog_sitter'").fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);
    let completed:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.authored.noodles_is_gone_mini_world_change' AND completed_character_revision IS NOT NULL").fetch_one(&h.pool).await?;
    assert_eq!(completed, 1);
    let consumed:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-reward/noodles-recovered' AND quantity=1 AND before_item->>'definition_production_key'='oteryn:item.tibia.i14156'").fetch_one(&h.pool).await?;
    assert_eq!(consumed, 1);
    Ok(())
}
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette = vec![bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i464".into(),
        family: bundle::Family::Terrain,
        id: 0,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(160),
        }),
    }];
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for y in 32170..=32180 {
        for x in 32309..=32317 {
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
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: -6,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    data.manifest.world = bundle::Extent {
        min_x: 32309,
        min_y: 32170,
        max_x: 32318,
        max_y: 32181,
        floors: vec![-6],
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
                x: 32310,
                y: 32172,
                floor: -6,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
