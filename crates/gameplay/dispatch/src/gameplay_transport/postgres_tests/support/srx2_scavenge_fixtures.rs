use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};

pub(super) async fn mint_corpse(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    source: u32,
    race: Option<crate::domain::bestiary::BestiaryRace>,
    local: u32,
) -> TestResult<[u8; 16]> {
    let key = format!("oteryn:item.tibia.i{source}");
    native
        .item_index()
        .ok_or("item index")?
        .definition_ref("Item", &key, "definition-r1")
        .ok_or("source corpse admitted in current Item index")?;
    super::maintain_ready(owner.root).await?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: key,
        revision_ref: "definition-r1".into(),
    };
    let generation = owner
        .runtime
        .lock()
        .await
        .owner_fence()
        .map_err(debug)?
        .generation();
    if let Some(race) = &race {
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == race.key())
            .ok_or("canonical source creature")?;
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
        else {
            return Err("canonical Creature profile".into());
        };
        assert_eq!(
            profile
                .details
                .as_ref()
                .and_then(|d| d.corpse_item.as_ref())
                .map(|r| r.key.as_str()),
            Some(definition.production_key.as_str()),
            "fixture corpse must be the actual source Creature corpse"
        );
    }
    let mut cause = ItemMintCause::for_test(
        owner.world_id,
        owner.channel_id,
        generation,
        local,
        1,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    );
    if let Some(race) = race {
        cause = cause.with_corpse_race(race).map_err(debug)?;
    }
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause,
                item: definition,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(7, 0, -7),
                    corpse_ref: vec![5; 16],
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: vec![6; 16],
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    match owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            &harness.node,
            &mut candidate,
            id(41),
        )
        .await
        .map_err(debug)?
    {
        ItemMintOutcome::Committed(v) => Ok(v.item_instance_id),
        other => Err(format!("fresh corpse {other:?}").into()),
    }
}
