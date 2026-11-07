//! Chosen seven-seal journey; original charged amulet hold is outside its reward set.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const TRACK: &str = "oteryn:quest-progress/banshees-trial/stage";
fn p(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}
#[test]
fn questpc5_banshees_seven_seals_queen_two_existing_claims_and_escape_once() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-banshees")
}
#[test]
fn questpc5_banshees_level_59_cannot_start_or_claim() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-banshees-low-level")
}
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    super::questpc5_serpentine_route::source_world(&[(125, 124), (126, 124)], p(32276, 31903, -13))
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
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(TRACK).fetch_optional(&h.pool).await?.unwrap_or(-1))
}
async fn rewards(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce(sum(quantity) FILTER(WHERE definition_production_key='oteryn:item.tibia.i3079'),0)::bigint,coalesce(sum(quantity) FILTER(WHERE definition_production_key='oteryn:item.tibia.i3035'),0)::bigint FROM game_item_instances WHERE lifecycle=1").fetch_one(&h.pool).await?)
}
async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pos: Position,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, 1)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
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
    low: bool,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_CASE_STARTED banshees_whole server_version={version} low_level={low}");
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(level, if low { 59 } else { 60 }, "verified starting level");
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
        .map_err(|_| "fence")?
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
    assert_eq!(stage(h).await?, -1);
    assert_eq!(rewards(h).await?, (0, 0));
    at(owner, admitted, p(32218, 31911, -15)).await?;
    let _ = use_at(owner, admitted, p(32218, 31912, -15)).await?;
    assert_eq!(
        rewards(h).await?,
        (0, 0),
        "source reward cannot bypass the seven seals"
    );
    let explores = [
        (
            p(32276, 31903, -13),
            StepDirection::East,
            p(32277, 31903, -13),
        ),
        (
            p(32171, 31855, -15),
            StepDirection::North,
            p(32171, 31854, -15),
        ),
        (
            p(32215, 31847, -15),
            StepDirection::South,
            p(32215, 31848, -15),
        ),
    ];
    for (i, (from, dir, to)) in explores.into_iter().enumerate() {
        at(owner, admitted, from).await?;
        super::questpc5_legion_route::step(owner, admitted, dir, to).await?;
        assert_eq!(
            stage(h).await?,
            if low { -1 } else { i as i64 },
            "seal exploration obeys level and order"
        );
        if low {
            assert_eq!(rewards(h).await?, (0, 0));
            eprintln!(
                "POSTGRES_CASE_COMPLETED banshees_low_level duration={:?}",
                started.elapsed()
            );
            return Ok(());
        }
    }
    for (i, (from, target)) in [
        (p(32249, 31892, -14), p(32250, 31892, -14)),
        (p(32191, 31938, -14), p(32192, 31938, -14)),
        (p(32310, 31978, -13), p(32311, 31978, -13)),
    ]
    .into_iter()
    .enumerate()
    {
        at(owner, admitted, from).await?;
        assert_eq!(
            use_at(owner, admitted, target).await?,
            UseDisposition::Committed
        );
        assert_eq!(stage(h).await?, 3 + i as i64);
    }
    let [queen] = super::classic_quests::install_source(
        owner,
        admitted,
        ["oteryn:npc.the_queen_of_the_banshees"],
    )
    .await?;
    at(owner, admitted, p(32260, 31867, -14)).await?;
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/banshees-pact")
        .ok_or("Queen pact binding")?;
    let (disposition, _, _) =
        super::qengine_rewards::wire_grant(owner, admitted, queen, &callback).await?;
    assert_eq!(
        disposition,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h).await?, 6);
    at(owner, admitted, p(32219, 31910, -15)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::South, p(32219, 31911, -15))
        .await?;
    assert_eq!(stage(h).await?, 7);
    for (from, target) in [
        (p(32218, 31911, -15), p(32218, 31912, -15)),
        (p(32220, 31911, -15), p(32220, 31912, -15)),
    ] {
        at(owner, admitted, from).await?;
        assert_eq!(
            use_at(owner, admitted, target).await?,
            UseDisposition::Committed
        );
        let before = rewards(h).await?;
        let _ = use_at(owner, admitted, target).await?;
        assert_eq!(rewards(h).await?, before, "existing claims are once-only");
    }
    assert_eq!(rewards(h).await?, (1, 100));
    at(owner, admitted, p(32219, 31912, -15)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::South, p(32269, 31853, -15))
        .await?;
    assert_eq!(stage(h).await?, 8);
    let done:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(done, 1);
    let amulets:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3081'").fetch_one(&h.pool).await?;
    assert_eq!(amulets, 0, "original partial-charge hold remains untouched");
    eprintln!(
        "POSTGRES_CASE_COMPLETED banshees_whole duration={:?}",
        started.elapsed()
    );
    Ok(())
}
