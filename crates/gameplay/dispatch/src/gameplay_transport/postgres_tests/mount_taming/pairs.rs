//! Each admitted creature/item pair reaches the same production USE consumer.
use super::*;
use crate::domain::appearance::{AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact};
use oteryn_protocol_oteryn::world_object::{UseTarget, encode_item_use_intent};

async fn item_quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn equip_mask(h: &Harness, index: &ItemDefinitionIndex, key: &str) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
        .bind(id(240).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).ok_or("mask revision")?)
        .bind(id(241).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(240).as_slice()).bind(id(42).as_slice()).bind(id(242).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn payload(
    index: &ItemDefinitionIndex,
    key: &str,
    entity: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult<Vec<u8>> {
    let source = index
        .definition_ref("Item", key, index.revision(key).ok_or("source revision")?)
        .ok_or("definition")?;
    Ok(encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(source),
        use_with: Some(entity),
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?)
}

fn feedback(owner: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId, text: &str) {
    assert_eq!(
        owner.states.chat.drain(session),
        [oteryn_protocol_oteryn::chat::ChatLine::Private {
            speaker_name: "Game".into(),
            text: text.into()
        }]
    );
}

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
    music: bool,
) -> TestResult {
    let rules = crate::content::native_gameplay::mount_taming::rules();
    assert_eq!(rules.len(), 44, "every qualified pair remains covered");
    let mut retained = admitted(owner, actor, session)?;
    owner
        .states
        .chat
        .admit_as(session, "Tamer", Some(id(41)), owner.owner_now());
    let cosmetics = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .and_then(|n| n.cosmetics())
        .ok_or("cosmetics")?;
    let mut successes = 0_i64;
    let mut attempts = 0_i64;
    for rule in rules
        .iter()
        .filter(|r| (r.item_key == "oteryn:item.tibia.i16244") == music)
    {
        if let Some(key) = &rule.required_equipment {
            let (target, entity) = spawn(owner, actor, &rule.creature_key, 1).await?;
            let before = item_quantity(h, &rule.item_key).await?;
            let (results, next) =
                wire::exchange(owner, retained, &[payload(index, &rule.item_key, entity)?]).await?;
            assert_eq!(results, [UseDisposition::RequirementNotMet]);
            retained = next;
            assert_eq!(item_quantity(h, &rule.item_key).await?, before);
            assert_eq!(h.count("game_mount_taming_attempts").await?, attempts);
            owner
                .runtime
                .lock()
                .await
                .remove_test_actor(target)
                .map_err(debug)?;
            equip_mask(h, index, key).await?;
        }
        // Each authored failure is reached through the wire, including ACTION_ALL and
        // BREAK=false. A single retained draw selects its weighted outcome.
        for (i, failure) in rule.failures.iter().enumerate() {
            let n = u128::try_from(rule.failures.len())?;
            let bucket = u128::from(rule.chance) * n + u128::try_from(i)?;
            let draw = u64::try_from((bucket << 64).div_ceil(100 * n))?;
            let (target, entity) = spawn(owner, actor, &rule.creature_key, 1).await?;
            let before = item_quantity(h, &rule.item_key).await?;
            crate::gameplay_transport::mount_taming::test_draw::set(draw);
            let (results, next) =
                wire::exchange(owner, retained, &[payload(index, &rule.item_key, entity)?]).await?;
            assert_eq!(
                results,
                [UseDisposition::Committed],
                "failure of {}",
                rule.item_key
            );
            retained = next;
            let consumes = rule.break_item && matches!(failure.action, 2 | 4);
            let removes = matches!(failure.action, 1 | 4);
            assert_eq!(
                item_quantity(h, &rule.item_key).await?,
                before - i64::from(consumes)
            );
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_identity(target)
                    .is_err(),
                removes
            );
            if !removes {
                owner
                    .runtime
                    .lock()
                    .await
                    .remove_test_actor(target)
                    .map_err(debug)?;
            }
            feedback(owner, session, &failure.text);
            assert_eq!(h.count("game_account_appearance_unlocks").await?, successes);
            attempts += 1;
        }
        let (target, entity) = spawn(owner, actor, &rule.creature_key, 1).await?;
        let payload = payload(index, &rule.item_key, entity)?;
        let before = item_quantity(h, &rule.item_key).await?;
        crate::gameplay_transport::mount_taming::test_draw::set(if music { u64::MAX } else { 0 });
        let command = retained.continuity.next_command_id;
        let (results, next) =
            wire::exchange(owner, retained, std::slice::from_ref(&payload)).await?;
        assert_eq!(
            results,
            [UseDisposition::Committed],
            "{} on {}",
            rule.item_key,
            rule.creature_key
        );
        retained = next;
        assert!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_identity(target)
                .is_err()
        );
        assert_eq!(item_quantity(h, &rule.item_key).await?, before - 1);
        feedback(owner, session, &rule.success_text);
        successes += 1;
        attempts += 1;
        let current = owner
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
                facts: &current.earned
            }
            .mount(&rule.mount_key),
            UnlockFact::Compatible
        );
        // New transport: the removed target is absent, yet its immutable receipt wins
        // before current visibility and source resolution.
        let mut replay = retained;
        replay.continuity.next_command_id = command;
        let (results, _) = wire::exchange(owner, replay, std::slice::from_ref(&payload)).await?;
        assert_eq!(results, [UseDisposition::Committed]);
        assert_eq!(item_quantity(h, &rule.item_key).await?, before - 1);
        assert_eq!(h.count("game_mount_taming_attempts").await?, attempts);
    }
    if music {
        let lifecycle:i16=sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i16244'").fetch_one(&h.pool).await?;
        assert_eq!(
            lifecycle, 2,
            "the ninth Music Box retires its last source unit"
        );
    }
    assert_eq!(successes, if music { 9 } else { 35 });
    assert_eq!(h.count("game_account_appearance_unlocks").await?, successes);
    assert_eq!(h.count("game_account_achievements").await?, successes + 1);
    Ok(())
}
