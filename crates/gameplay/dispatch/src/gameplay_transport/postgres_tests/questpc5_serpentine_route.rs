//! Selected finite trial on retained tower geometry; no quest or reward state is seeded.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
const QUEST: &str = "oteryn:quest.serpentine_tower_quest";
const TRACK: &str = "oteryn:quest-progress/serpentine-trial/stage";
fn p(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}
#[test]
fn questpc5_serpentine_whole_pot_torch_lever_observation_and_existing_pearl_once() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-serpentine")
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
async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(TRACK).fetch_optional(&h.pool).await?.unwrap_or(-1))
}
async fn completed(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL")
        .bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?)
}
async fn pearls(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3026'").fetch_one(&h.pool).await?)
}
async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pos: Position,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)
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
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED serpentine_whole server_version={version}");
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
    assert_eq!(pearls(h).await?, 0);
    assert_eq!(completed(h).await?, 0, "no seeded quest completion");
    at(owner, admitted, p(33150, 32863, -7)).await?;
    let _ = use_at(owner, admitted, p(33150, 32862, -7), 1).await?;
    assert_eq!(
        pearls(h).await?,
        0,
        "the source chest cannot bypass the trial"
    );
    at(owner, admitted, p(33146, 32863, -7)).await?;
    let premium = owner.premium_coordinator.take();
    super::questpc5_legion_route::step(owner, admitted, StepDirection::East, p(33147, 32863, -7))
        .await?;
    assert_eq!(stage(h).await?, -1, "current Premium is required to start");
    owner.premium_coordinator = premium;
    at(owner, admitted, p(33146, 32863, -7)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::East, p(33147, 32863, -7))
        .await?;
    assert_eq!(
        stage(h).await?,
        0,
        "actual exploration starts the selected trial"
    );
    at(owner, admitted, p(33146, 32866, -7)).await?;
    let _ = use_at(owner, admitted, p(33145, 32865, -7), 3).await?;
    assert_eq!(
        stage(h).await?,
        0,
        "observation cannot bypass either mechanism"
    );
    at(owner, admitted, p(33147, 32864, -7)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::East, p(33145, 32863, -7))
        .await?;
    assert_eq!(
        stage(h).await?,
        0,
        "an empty fire takes the source default forcefield branch"
    );
    at(owner, admitted, p(33146, 32862, -7)).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    // Baseline ingredient only; the existing custody owner transfers and drops it.
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: p(33145, 32862, -7),
            item: "oteryn:item.tibia.i3465".into(),
            count: 1,
        },
        77,
        false,
    )
    .await?;
    owner.states.loop_ground.invalidate();
    at(owner, admitted, p(33147, 32864, -7)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::East, p(33151, 32864, -7))
        .await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::North, p(33151, 32863, -7))
        .await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::North, p(33151, 32862, -7))
        .await?;
    let premium = owner.premium_coordinator.take();
    let _ = use_at(owner, admitted, p(33151, 32861, -7), 2).await?;
    assert_eq!(
        stage(h).await?,
        0,
        "current Premium is required for the torch"
    );
    owner.premium_coordinator = premium;
    assert_eq!(
        use_at(owner, admitted, p(33151, 32861, -7), 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 1);
    at(owner, admitted, p(33150, 32866, -8)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::East, p(33151, 32866, -8))
        .await?;
    assert_eq!(
        use_at(owner, admitted, p(33152, 32866, -8), 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 2);
    at(owner, admitted, p(33148, 32866, -9)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::South, p(33148, 32867, -9))
        .await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::South, p(33148, 32868, -9))
        .await?;
    assert_eq!(
        pearls(h).await?,
        0,
        "lever projection never mints a parallel pearl"
    );
    at(owner, admitted, p(33146, 32866, -7)).await?;
    assert_eq!(
        use_at(owner, admitted, p(33145, 32865, -7), 3).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 3);
    assert_eq!(completed(h).await?, 1);
    at(owner, admitted, p(33147, 32864, -7)).await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::East, p(33151, 32864, -7))
        .await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::North, p(33151, 32863, -7))
        .await?;
    super::questpc5_legion_route::step(owner, admitted, StepDirection::West, p(33150, 32863, -7))
        .await?;
    assert_eq!(
        use_at(owner, admitted, p(33150, 32862, -7), 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(pearls(h).await?, 1);
    let _ = use_at(owner, admitted, p(33150, 32862, -7), 1).await?;
    assert_eq!(pearls(h).await?, 1, "the existing claim remains once-only");
    at(owner, admitted, p(33146, 32866, -7)).await?;
    let _ = use_at(owner, admitted, p(33145, 32865, -7), 3).await?;
    assert_eq!(stage(h).await?, 3);
    assert_eq!(pearls(h).await?, 1);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key LIKE 'oteryn:quest-callback/authored-serpentine-%'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, 4,
        "only the four progress steps write quest receipts"
    );
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts WHERE character_id=encode($1,'hex')::uuid AND claim_production_key='oteryn:reward-claim.quest.u7_3.serpentine_tower.white_pearl'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(claims, 1, "one existing source claim owns the pearl mint");
    eprintln!(
        "POSTGRES_CASE_COMPLETED serpentine_whole duration={:?}",
        started.elapsed()
    );
    Ok(())
}
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    source_world(&[(129, 128)], p(33146, 32863, -7))
}

// The retained source map and production Item facts are shared by whole-route tests.
pub(super) fn source_world(regions: &[(u16, u16)], start: Position) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (template, mut pins) = crate::map::boot::tests::bundle();
    let identity = bundle::read(&template)?.manifest.identity;
    let project = source::project_regions(&root, regions)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: vec![],
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = bundle::read(&compiled.bytes)?.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    // Use the production item generation: real Item palette entries cannot boot
    // through the terrain-only fixture lookup.
    let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        "content-1",
    )?;
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let index = native.item_index().ok_or("native item index")?;
    let item = |key: &str| {
        use crate::content::{ReferenceItemField, ReferenceItemType};
        let revision = index.revision(key)?;
        let profile = native
            .item_policy(key, revision)
            .map(|policy| policy.record());
        Some(crate::map::facts::ItemDefinition {
            reference: index.definition_ref("Item", key, revision)?,
            solid: profile.and_then(|profile| profile.attributes.blocks_movement),
            blocks_projectile: profile.and_then(|profile| profile.attributes.blocks_projectile).unwrap_or(true),
            pickupable: profile.is_some_and(|profile| matches!(&profile.semantics.physical,
                ReferenceItemField::Known(physical) if physical.pickupable == ReferenceItemField::Known(true))),
            door: profile.is_some_and(|profile| matches!(&profile.semantics.classification,
                ReferenceItemField::Known(classification) if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door))),
        })
    };
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        item,
    )
    .map_err(debug)
}
