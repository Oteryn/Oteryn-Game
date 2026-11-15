//! W09FX exercises the real wire USE and canonical durable burn for each missing identity.
use super::*;
use oteryn_gameplay_items::concoctions::Concoction;

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
    kind: Concoction,
) -> TestResult {
    let key = kind.production_key();
    let definition = index
        .definition_ref("Item", key, index.revision(key).ok_or("revision")?)
        .ok_or("definition")?;
    let used = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let result = wire::exchange(
        owner,
        cases::admitted(owner, actor, session)?,
        &[used.clone(), used],
    )
    .await?;
    assert_eq!(
        result.results,
        vec![UseDisposition::Committed, UseDisposition::Exhausted],
        "{kind:?}"
    );
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(110).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(quantity, 1, "{kind:?}");
    assert_eq!(harness.count("game_character_concoction_effects").await?, 1);
    let durations = owner
        .root
        .read_active_potion_durations(owner.character, owner.holder, item_fence()?, id(43))
        .await
        .map_err(debug)?;
    let timer: bool = sqlx::query_scalar("SELECT started_at IS NOT NULL AND expires_at=started_at+interval '1 hour' FROM game_character_concoction_effects WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
    assert!(timer, "{kind:?} activation timer");
    if kind != Concoction::StaminaExtension {
        assert!(
            durations
                .iter()
                .any(|line| line.item_id == kind.item_id() && line.remaining_ms > 0),
            "{kind:?}"
        );
    } else {
        assert!(
            !durations.iter().any(|line| line.item_id == kind.item_id()),
            "instant stamina has a reuse cooldown, not an ongoing effect"
        );
    }
    if kind == Concoction::StaminaExtension {
        assert_eq!(
            owner
                .root
                .read_hunting_stamina(owner.character, item_fence()?.character_id)
                .await
                .map_err(debug)?,
            1060
        );
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .live_stamina(actor, session),
            Some(1060)
        );
    }
    if kind == Concoction::BestiaryBetterment {
        use crate::durability::bestiary_progress::{
            BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
        };
        let request = BestiaryKillRequest {
            occurrence: BestiaryKillOccurrence::from_bytes(id(151)).map_err(debug)?,
            race: crate::domain::bestiary::BestiaryRace::new(
                "oteryn:creature.rat",
                "definition-r1",
                vec![25, 250, 500],
            )
            .map_err(debug)?,
            context: postgres_fixture::context(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
        };
        let fence = postgres_fixture::fence(1)?;
        let capture = owner
            .root
            .capture_bestiary_talisman(
                owner.character,
                owner.holder,
                fence,
                request.occurrence,
                &request.race,
            )
            .await
            .map_err(debug)?;
        let outcome = owner
            .root
            .commit_bestiary_kill_with_talisman(
                owner.character,
                owner.holder,
                fence,
                request.clone(),
                Some(capture.clone()),
            )
            .await
            .map_err(debug)?;
        let BestiaryKillOutcome::Committed(receipt) = outcome else {
            return Err("missing Bestiary receipt".into());
        };
        assert_eq!(
            (receipt.kill_count_before, receipt.kill_count_after),
            (0, 2)
        );
        assert!(matches!(
            owner
                .root
                .commit_bestiary_kill_with_talisman(
                    owner.character,
                    owner.holder,
                    fence,
                    request,
                    Some(capture)
                )
                .await
                .map_err(debug)?,
            BestiaryKillOutcome::AlreadyCommitted(_)
        ));
    }
    if kind == Concoction::StaminaExtension {
        return Ok(());
    }
    // Two offline wall hours do not consume the hour. The same two online hours do.
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_concoction_effects SET started_at=started_at-interval '2 hours',expires_at=expires_at-interval '2 hours'")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    let timers = owner
        .root
        .read_active_potion_durations(owner.character, owner.holder, item_fence()?, id(43))
        .await
        .map_err(debug)?;
    assert!(
        timers
            .iter()
            .any(|line| line.item_id == kind.item_id() && line.remaining_ms > 3_500_000),
        "offline {kind:?}"
    );
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET training_created_at_us=training_created_at_us-7200000000 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let timers = owner
        .root
        .read_active_potion_durations(owner.character, owner.holder, item_fence()?, id(43))
        .await
        .map_err(debug)?;
    assert!(
        !timers.iter().any(|line| line.item_id == kind.item_id()),
        "expired {kind:?}"
    );
    Ok(())
}
