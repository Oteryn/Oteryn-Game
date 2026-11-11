//! Seven source construction pieces use the ordinary item-action owner.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use std::num::NonZeroU32;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    reference: impl Fn(&str) -> Option<NonZeroU32>,
    missing: bool,
) -> TestResult {
    let before: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i",
    )
    .fetch_one(&h.pool)
    .await?;
    let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&h.pool)
        .await?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) = super::super::rust_lottery::exchange(
        owner,
        admitted,
        command,
        &payload,
        reference("oteryn:item.tibia.i21897"),
        None,
    )
    .await?;
    admitted = next;
    if missing {
        assert_eq!(result, UseDisposition::RequirementNotMet);
        let after: serde_json::Value = sqlx::query_scalar(
            "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            before, after,
            "missing one piece must preserve every source item"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_account_appearance_unlocks")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        return Ok(());
    }
    assert_eq!(
        result,
        UseDisposition::Committed,
        "all seven source pieces construct the Glooth Glider"
    );
    let consumed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=2")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        consumed, 7,
        "construction consumes exactly seven carried pieces"
    );
    let after_ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after_ground, ground,
        "a Ground trigger is not a carried ingredient"
    );
    let (next, replayed, _) =
        super::super::rust_lottery::exchange(owner, admitted, command, &actual, None, None).await?;
    admitted = next;
    assert_eq!(replayed, UseDisposition::Committed);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_consume_transform_records")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN('oteryn:item.tibia.i21897','oteryn:item.tibia.i21898','oteryn:item.tibia.i21899','oteryn:item.tibia.i21901','oteryn:item.tibia.i21902','oteryn:item.tibia.i21905','oteryn:item.tibia.i21906')")
        .fetch_one(&h.pool).await?;
    assert_eq!(remaining, 7 + ground, "one complete carried recipe remains");
    let next_command = admitted.continuity.next_command_id;
    let (next, duplicate, _) = super::super::rust_lottery::exchange(
        owner,
        admitted,
        next_command,
        &payload,
        reference("oteryn:item.tibia.i21897"),
        None,
    )
    .await?;
    admitted = next;
    assert_eq!(duplicate, UseDisposition::Committed);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances WHERE lifecycle=2")
            .fetch_one(&h.pool)
            .await?,
        7,
        "already-owned construction preserves the second complete recipe"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_account_appearance_unlocks")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    let seen = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("appearance observation")?;
    assert!(
        seen.catalogue
            .mounts
            .iter()
            .any(|key| key == "oteryn:content.mount.glooth_glider"),
        "the constructed source mount must be normally offered"
    );
    let outfit = seen.catalogue.outfits.first().ok_or("starter outfit")?;
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: seen.catalogue.context,
        expected_root_revision: seen.catalogue.root_revision,
        prior: seen.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: outfit.key.clone(),
            colours: [0; 4],
            addons: 0,
            mount_key: Some("oteryn:content.mount.glooth_glider".into()),
        },
        content_revision: owner
            .appearance_context()
            .ok_or("appearance context")?
            .1
            .content_revision()
            .into(),
    };
    let command = admitted.continuity.next_command_id;
    let result = super::super::quest_world_callbacks::wire(
        owner,
        &mut admitted,
        command,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&result).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    Ok(())
}
