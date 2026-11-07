//! Stock acquisition proof; source choices are interpreted by the production item owner.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_items::item_cosmetics::{self, GrantKind};
use std::num::NonZeroU32;

pub(super) async fn random_outfit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    reference: impl Fn(&str) -> Option<NonZeroU32>,
    wanted: Option<&str>,
) -> TestResult {
    let catalogue = item_cosmetics::catalogue().ok_or("item catalogue")?;
    let rule = item_cosmetics::rule("oteryn:item.tibia.i6579").ok_or("doll rule")?;
    let mut command = admitted.continuity.next_command_id;
    if let Some(wanted) = wanted {
        let harmless = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(
                reference("oteryn:item.tibia.i3031").ok_or("coin definition")?,
            ),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let mut matched = false;
        for _ in 0..1024 {
            let choice = item_cosmetics::select_for_command(
                rule,
                *admitted.game_session_id.as_bytes(),
                command,
                id(71),
            )
            .ok_or("source choice")?;
            if choice
                .feedback
                .as_deref()
                .is_some_and(|text| text.contains(&format!("the {wanted} outfit")))
            {
                matched = true;
                break;
            }
            // Advance through ordinary refused coin USE commands. No invented
            // continuity or source receipt is used to force the desired draw.
            let (next, result, _) = super::super::rust_lottery::exchange(
                owner, admitted, command, &harmless, None, None,
            )
            .await?;
            assert_ne!(result, UseDisposition::Committed);
            admitted = next;
            command = admitted.continuity.next_command_id;
        }
        assert!(matched, "bounded deterministic source family");
    }
    let selected = item_cosmetics::select_for_command(
        rule,
        *admitted.game_session_id.as_bytes(),
        command,
        id(71),
    )
    .ok_or("selected source choice")?;
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
        command,
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
    let keys: Vec<_> = grants.iter().map(|(key, _)| key.clone()).collect();
    let mut expected = selected.grants.clone();
    expected.sort();
    assert_eq!(
        keys, expected,
        "the exact selected source outcome, without mixing draws"
    );
    assert!(
        grants
            .iter()
            .all(|(_, provenance)| provenance == item_cosmetics::PROVENANCE)
    );
    let addon = catalogue
        .grants
        .iter()
        .find(|g| keys.contains(&g.key) && g.kind == GrantKind::Outfit)
        .ok_or("source base")?;
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
    let bit = catalogue
        .grants
        .iter()
        .find(|g| keys.contains(&g.key) && g.kind == GrantKind::Addon)
        .and_then(|g| g.addon)
        .map_or(0, |ordinal| 1 << (ordinal - 1));
    assert_eq!(option.permitted_addons, bit);
    let selection = oteryn_protocol_oteryn::appearance::Selection {
        outfit_key: option.key.clone(),
        colours: [0; 4],
        addons: bit,
        mount_key: None,
    };
    let (next, replayed, _) =
        super::super::rust_lottery::exchange(owner, admitted, command, &actual, None, None).await?;
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
