//! Stock acquisition proof; source choices are interpreted by the production item owner.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_items::item_cosmetics::{self, GrantKind};
use std::num::NonZeroU32;

pub(super) async fn random_outfit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    reference: impl Fn(&str) -> Option<NonZeroU32>,
) -> TestResult {
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (mut admitted, result, actual) = super::super::rust_lottery::exchange(
        owner,
        admitted,
        1,
        &payload,
        reference("oteryn:item.tibia.i6579"),
        None,
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "stock addon-doll source grants must be admitted"
    );
    let grants: Vec<(String, String)> = sqlx::query_as(
        "SELECT unlock_key,content_revision FROM game_account_appearance_unlocks ORDER BY unlock_key",
    ).fetch_all(&h.pool).await?;
    let catalogue = item_cosmetics::catalogue().ok_or("item catalogue")?;
    let rule = item_cosmetics::rule("oteryn:item.tibia.i6579").ok_or("doll rule")?;
    let keys: Vec<_> = grants.iter().map(|(key, _)| key.clone()).collect();
    let choices: Vec<_> = rule
        .choices
        .iter()
        .filter(|choice| {
            let mut wanted = choice.grants.clone();
            wanted.sort();
            wanted == keys
        })
        .collect();
    assert_eq!(
        choices.len(),
        1,
        "exactly one whole source choice, without mixing outcomes"
    );
    assert!(
        grants
            .iter()
            .all(|(_, provenance)| provenance == item_cosmetics::PROVENANCE)
    );
    let addon = catalogue
        .grants
        .iter()
        .find(|g| keys.contains(&g.key) && g.kind == GrantKind::Addon)
        .ok_or("source addon")?;
    let observation = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("appearance observation")?;
    let option = observation
        .catalogue
        .outfits
        .iter()
        .find(|o| addon.look_types.contains(&o.look_type))
        .ok_or("earned source look offered")?;
    let bit = 1 << (addon.addon.ok_or("addon ordinal")? - 1);
    assert_eq!(option.permitted_addons, bit);
    let selection = oteryn_protocol_oteryn::appearance::Selection {
        outfit_key: option.key.clone(),
        colours: [0; 4],
        addons: bit,
        mount_key: None,
    };
    let (next, replayed, _) =
        super::super::rust_lottery::exchange(owner, admitted, 1, &actual, None, None).await?;
    admitted = next;
    assert_eq!(replayed, UseDisposition::Committed);
    let consumed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(consumed, 1);
    let remaining:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i6579'").fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0);
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: observation.catalogue.context,
        expected_root_revision: observation.catalogue.root_revision,
        prior: observation.catalogue.selected,
        selection,
        content_revision: owner
            .appearance_context()
            .ok_or("content")?
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
