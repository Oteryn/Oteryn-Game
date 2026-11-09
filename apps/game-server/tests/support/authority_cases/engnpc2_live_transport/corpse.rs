//! Real native Slug corpse, minted at the current actor's cell by the death owner.
use super::{TestResult, debug, fixture, id};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;

pub(super) async fn mint(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    native: &NativeGameplayState,
    at: crate::foundation::MovementLocalPosition,
) -> TestResult<[u8; 16]> {
    let reward = native
        .reward_table()
        .row("oteryn:creature.slug")
        .map_err(debug)?;
    let definition = TypedDefinitionRef {
        family: reward.corpse_item.family.clone(),
        production_key: reward.corpse_item.production_key.clone(),
        revision_ref: reward.corpse_item.revision_ref.clone(),
    };
    let race = reward.race.clone().ok_or("native Slug race")?;
    let expected = fixture::fence(1)?;
    let mut body = oteryn_gameplay_entities::carrier::CombatDeathFixture::new_for_scope_at(
        expected.runtime_scope,
        expected.scope_ownership_generation,
        crate::foundation::MovementLocalPosition {
            x: at.x,
            y: at.y,
            floor: at.floor.checked_neg().ok_or("stored corpse floor")?,
        },
    )
    .map_err(debug)?;
    body.strike(
        &definition.production_key,
        oteryn_gameplay_entities::carrier::CombatDeathFixture::HEALTH,
    )
    .map_err(debug)?;
    let projection = body.project_death_projection().map_err(debug)?;
    let death = projection.occurrence().death_key().map_err(debug)?;
    let position = projection.position();
    let (frame, map, content) = projection.context_markers();
    let spatial = [
        position.x.to_be_bytes().as_slice(),
        position.y.to_be_bytes().as_slice(),
        position.floor.to_be_bytes().as_slice(),
    ]
    .concat();
    let cause = ItemMintCause::from_creature_death(
        death,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(debug)?
    .with_corpse_race(race)
    .map_err(debug)?;
    let mut candidate = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            ItemMintRequest {
                cause,
                item: definition,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position: spatial,
                    corpse_ref: death.native_bytes(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: [
                        frame.to_be_bytes(),
                        map.to_be_bytes(),
                        content.to_be_bytes(),
                    ]
                    .concat(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let item = *candidate.item_instance_id();
    let permit = candidate.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(&permit, authority, &h.node, &mut candidate, id(41))
            .await
            .map_err(debug)?,
        ItemMintOutcome::Committed(_)
    ));
    Ok(item)
}
