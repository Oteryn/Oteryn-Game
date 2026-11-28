use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{appearance, world_object::MapUseTarget};
use std::num::{NonZeroU32, NonZeroU64};

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    mut admitted: AdmittedSession,
    case: positive::Case,
    reference: impl Fn(&str) -> Option<NonZeroU32>,
) -> TestResult {
    let glooth = matches!(case, positive::Case::Gloothomotive);
    let (source, mount, achievement) = if glooth {
        (
            "oteryn:item.tibia.i37397",
            "oteryn:content.mount.wind_up_loco",
            "oteryn:achievement/engine_driver",
        )
    } else {
        (
            "oteryn:item.tibia.i50063",
            "oteryn:content.mount.ancient_demonosaur",
            "oteryn:achievement/alpha_rider",
        )
    };
    let index = reference(source).ok_or("production item definition")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("production Native Gameplay")?;
    assert!(
        native
            .item_policy(source, "definition-r1")
            .ok_or("source custody")?
            .record()
            .admission
            .materializable
    );

    let observe =
        |session: AdmittedSession| async move { positive::appearance(owner, &session).await };
    assert!(
        observe(admitted)
            .await?
            .catalogue
            .mounts
            .iter()
            .all(|key| key != mount)
    );
    let mut intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let exchange = super::super::rust_lottery_fixtures::exchange;
    if glooth {
        let self_use = UseIntent {
            target: UseTarget::ItemByDefinition(index),
            ..intent.clone()
        };
        let (next, refusal, _) = exchange(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            &encode_item_use_intent(&self_use).map_err(debug)?,
            None,
            None,
        )
        .await?;
        assert_ne!(refusal, UseDisposition::Committed);
        assert_eq!(positive::item_quantity(harness, 71).await?, 1);
        assert_eq!(positive::grant_count(harness).await?, 0);
        admitted = next;
        intent.use_with_map_item = Some(MapUseTarget::Item(NonZeroU64::MIN));
        let (next, refusal, _) = exchange(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            &encode_item_use_intent(&intent).map_err(debug)?,
            Some(index),
            Some(index),
        )
        .await?;
        assert_ne!(refusal, UseDisposition::Committed);
        assert_eq!(positive::item_quantity(harness, 71).await?, 1);
        assert_eq!(positive::grant_count(harness).await?, 0);
        admitted = next;
    }
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let target = if glooth {
        Some(reference("oteryn:item.tibia.i37398").ok_or("Wind-Up Loco definition")?)
    } else {
        None
    };
    let (next, result, actual) =
        exchange(owner, admitted, command, &payload, Some(index), target).await?;
    assert_eq!(result, UseDisposition::Committed);
    let (next, replay, _) = exchange(owner, next, command, &actual, None, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    admitted = next;
    assert_eq!(positive::receipt_count(harness).await?, 1);
    assert_eq!(positive::grant_count(harness).await?, 1);
    let expected_source_remaining = i64::from(!glooth);
    assert_eq!(
        positive::item_quantity(harness, 71).await?,
        expected_source_remaining
    );
    if glooth {
        assert_eq!(positive::item_quantity(harness, 72).await?, 1);
        assert_eq!(positive::item_quantity(harness, 73).await?, 0);
    } else {
        let (next, repeat, _) = exchange(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(index),
            None,
        )
        .await?;
        assert_eq!(repeat, UseDisposition::Committed);
        assert_eq!(positive::item_quantity(harness, 71).await?, 1);
        assert_eq!(positive::grant_count(harness).await?, 1);
        admitted = next;
    }
    let achievements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(achievement)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(achievements, 1);

    let seen = observe(admitted).await?;
    assert!(seen.catalogue.mounts.iter().any(|key| key == mount));
    let change = appearance::Change {
        context: seen.catalogue.context,
        expected_root_revision: seen.catalogue.root_revision,
        prior: seen.catalogue.selected,
        selection: appearance::Selection {
            outfit_key: seen
                .catalogue
                .outfits
                .first()
                .ok_or("starter outfit")?
                .key
                .clone(),
            colours: [0; 4],
            addons: 0,
            mount_key: Some(mount.into()),
        },
        content_revision: owner
            .appearance_context()
            .ok_or("appearance content")?
            .1
            .content_revision()
            .into(),
    };
    let command = admitted.continuity.next_command_id;
    let response = super::super::quest_world_callbacks_fixtures::wire(
        owner,
        &mut admitted,
        command,
        appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        appearance::ChangeResult::decode(&response).map_err(debug)?,
        appearance::ChangeResult::Committed(_)
    ));
    let saved = observe(admitted).await?;
    assert_eq!(
        saved
            .catalogue
            .selected
            .as_ref()
            .and_then(|selection| selection.selection.mount_key.as_deref()),
        Some(mount)
    );
    Ok(())
}
