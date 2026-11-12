//! Source construction items are ordinary self-USE, without a creature/map target.
use super::*;
use crate::domain::appearance::{AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact};
use oteryn_protocol_oteryn::world_object::{UseTarget, encode_item_use_intent};

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let mut retained = admitted(owner, actor, session)?;
    let cosmetics = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .and_then(|native| native.cosmetics())
        .ok_or("production cosmetics")?;
    let mut count = 0_i64;
    for rule in crate::content::native_gameplay::mount_taming::rules()
        .iter()
        .filter(|rule| rule.target_item_key.is_some() && rule.single_item_use())
    {
        assert_eq!(
            rule.target_item_key.as_deref(),
            Some(rule.item_key.as_str())
        );
        let definition = index
            .definition_ref(
                "Item",
                &rule.item_key,
                index
                    .revision(&rule.item_key)
                    .ok_or("construction revision")?,
            )
            .ok_or("construction ref")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(definition),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let before = quantity(h, &rule.item_key).await?;
        let (target, entity) = spawn(owner, actor, "oteryn:creature.bear", 1).await?;
        let explicit_target = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(definition),
            use_with: Some(entity),
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let (results, next) = wire::exchange(owner, retained, &[explicit_target]).await?;
        assert_eq!(results, [UseDisposition::NothingToUse]);
        retained = next;
        assert_eq!(quantity(h, &rule.item_key).await?, before);
        assert_eq!(h.count("game_mount_taming_attempts").await?, count);
        owner
            .runtime
            .lock()
            .await
            .remove_test_actor(target)
            .map_err(debug)?;
        let command = retained.continuity.next_command_id;
        let (results, next) =
            wire::exchange(owner, retained, std::slice::from_ref(&payload)).await?;
        assert_eq!(results, [UseDisposition::Committed], "{}", rule.mount_key);
        retained = next;
        count += 1;
        assert_eq!(quantity(h, &rule.item_key).await?, before - 1);
        let state = owner
            .root
            .read_current_character_appearance_state(
                owner.character,
                owner.holder,
                postgres_fixture::fence(1)?,
                cosmetics.earned_grant_keys(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            EarnedCosmetics {
                content: cosmetics,
                facts: &state.earned
            }
            .mount(&rule.mount_key),
            UnlockFact::Compatible
        );
        let mut replay = retained;
        replay.continuity.next_command_id = command;
        assert_eq!(
            wire::exchange(owner, replay, std::slice::from_ref(&payload))
                .await?
                .0,
            [UseDisposition::Committed]
        );
        assert_eq!(quantity(h, &rule.item_key).await?, before - 1);
        let (results, next) = wire::exchange(owner, retained, &[payload]).await?;
        assert_eq!(results, [UseDisposition::RequirementNotMet]);
        retained = next;
        assert_eq!(h.count("game_mount_taming_attempts").await?, count);
        assert_eq!(h.count("game_account_appearance_unlocks").await?, count);
    }
    assert_eq!(count, 3, "all one-item source construction pairs exercised");
    Ok(())
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}
