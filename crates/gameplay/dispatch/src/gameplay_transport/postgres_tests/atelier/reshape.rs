//! WU26-10 Mod Reshaping through the real Atelier writer: fee cause 483, the
//! RESHAPE receipt and its lesser fragment BURN commit in one transaction.
use super::*;
use crate::durability::character_atelier::AtelierResources;
use crate::durability::item_fee_burn::test_support::{definition, request as fee_request};
use crate::gameplay_transport::atelier::{GemBuildCommand, GemBuildOutcome};

async fn lesser_fragments(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(229).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_atelier_item_records),(SELECT count(*) FROM game_item_fee_burns),(SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid)").bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}

async fn stashed_lesser(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(quantity),0)::bigint FROM game_character_stash_balances
         WHERE character_id=encode($1,'hex')::uuid AND definition_key='oteryn:item.tibia.i46625'",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn carried_small_stacks(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances
         WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)",
    )
    .bind(id(148).as_slice())
    .bind(id(151).as_slice())
    .bind(id(154).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let content = fixture::MechanicContent;
    let resources = Box::new(AtelierResources {
        content: &content,
        coins: fee_request(2, 1)?.change,
        fragment: definition("oteryn:item.tibia.i46625"),
        supply_stash: false,
    });
    let session = crate::bestiary_postgres_harness::fence(1)?.game_session_id;
    // The paid scenario unlocked its first initial Regular gem; it keeps two modifiers.
    let before = stored(h).await?;
    let subject = before
        .gems
        .values()
        .find(|gem| gem.quality == GemQuality::Regular && gem.id != u128::from_be_bytes(id(181)))
        .ok_or("initial regular")?
        .clone();
    assert!(!subject.locked);
    assert_eq!(subject.mods.len(), 2);
    let gem = subject.id.to_be_bytes();

    // A Regular gem has no third modifier: nothing is paid, burned or recorded.
    let fp = footprint(h).await?;
    let fragments = lesser_fragments(h).await?;
    let third = AtelierRequest {
        occurrence: id(160),
        expected_atelier_revision: before.revision,
        mutation: AtelierMutation::Reshape { gem, position: 3 },
        resources: Some(Box::new(AtelierResources {
            content: &content,
            coins: fee_request(2, 1)?.change,
            fragment: definition("oteryn:item.tibia.i46626"),
            supply_stash: false,
        })),
    };
    advance_ex_action(owner)?;
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(third))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Refused)
    ));
    assert_eq!(stored(h).await?, before);
    assert_eq!(footprint(h).await?, fp);
    assert_eq!(lesser_fragments(h).await?, fragments);

    for (seed, position, burned) in [(161, 1_u8, 1_i64), (162, 2, 2)] {
        let prior = stored(h).await?;
        let fragments = lesser_fragments(h).await?;
        let request = AtelierRequest {
            occurrence: id(seed),
            expected_atelier_revision: prior.revision,
            mutation: AtelierMutation::Reshape { gem, position },
            resources: Some(resources.clone()),
        };
        advance_ex_action(owner)?;
        let outcome = owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(request.clone()))
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                outcome,
                GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
            ),
            "{outcome:?}"
        );
        assert_eq!(lesser_fragments(h).await?, fragments - burned);
        let after = stored(h).await?;
        let old = &prior.gems[&subject.id];
        let new = &after.gems[&subject.id];
        // Only the paid position may change; identity, quality and lock stay.
        assert_eq!(
            (new.id, new.quality, new.locked, new.mods.len()),
            (old.id, old.quality, old.locked, old.mods.len())
        );
        let kept = usize::from(2 - position);
        assert_eq!(new.mods[kept], old.mods[kept]);
        assert_eq!(after.gold, 0);
        assert!(after.unrevealed.is_empty() && after.fragments.is_empty());

        let fp = footprint(h).await?;
        let replay = owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(request))
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                replay,
                GemBuildOutcome::Atelier(AtelierOutcome::AlreadyCommitted { .. })
            ),
            "{replay:?}"
        );
        assert_eq!(stored(h).await?, after);
        assert_eq!(footprint(h).await?, fp);
    }

    let fees: Vec<i64> = sqlx::query_scalar(
        "SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=483 ORDER BY fee_gold_units",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(fees, vec![125_000, 1_000_000]);
    let (receipts, bound): (i64, bool) = sqlx::query_as(
        "SELECT count(*),coalesce(bool_and(fee_transaction_id=item_transaction_id),false)
         FROM game_character_atelier_receipts WHERE kind='RESHAPE'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 2);
    assert!(bound, "one logical transaction binds the fragments and gold fee");
    let envelopes: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id)
         WHERE f.cause_kind=483 ORDER BY f.transaction_id LIMIT 3",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(envelopes.len(), 2);
    for wire in envelopes {
        use crate::durability::item_fee_burn_audit::{FeeBurnCauseV1, decode_fee_burn_envelope};
        let (_, event) = decode_fee_burn_envelope(&wire).map_err(debug)?;
        let cause = event.cause.and_then(|c| c.cause).ok_or("fee cause")?;
        let FeeBurnCauseV1::GemReshape(cause) = cause else {
            return Err("unexpected reshape cause".into());
        };
        assert_eq!(cause.service_key, "oteryn:atelier.reshape");
        assert!(!cause.atelier_proof.is_empty());
    }

    // Owner decision WU26-10: away from a depot only carried fragments count; at a
    // depot with the Supply Stash open, carried fragments go first and the stash
    // pays the remainder. Move every carried lesser fragment into the stash.
    let carried = lesser_fragments(h).await?;
    super::wire_paid::stow_asset_command(owner, admitted, 229, u32::try_from(carried)?, 160)
        .await?;
    assert_eq!(stashed_lesser(h).await?, carried);
    let still_carried: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
         WHERE i.definition_production_key='oteryn:item.tibia.i46625' AND i.lifecycle=1 AND e.character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(still_carried, 0, "no lesser fragment is left in the bag");
    let at_depot = |supply_stash: bool| {
        Some(Box::new(AtelierResources {
            content: &content,
            coins: resources.coins.clone(),
            fragment: resources.fragment.clone(),
            supply_stash,
        }))
    };
    let away = AtelierRequest {
        occurrence: id(165),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::Reshape { gem, position: 1 },
        resources: at_depot(false),
    };
    let before = stored(h).await?;
    let fp = footprint(h).await?;
    advance_ex_action(owner)?;
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(away))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Refused)
    ));
    assert_eq!(stored(h).await?, before);
    assert_eq!(footprint(h).await?, fp);
    assert_eq!(stashed_lesser(h).await?, carried);

    // At the depot with nothing carried the stash pays both fragments.
    let from_stash = AtelierRequest {
        occurrence: id(166),
        expected_atelier_revision: before.revision,
        mutation: AtelierMutation::Reshape { gem, position: 2 },
        resources: at_depot(true),
    };
    advance_ex_action(owner)?;
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(from_stash))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
        ),
        "{outcome:?}"
    );
    assert_eq!(stashed_lesser(h).await?, carried - 2);
    let debited: i64 = sqlx::query_scalar(
        "SELECT (r.stash_input->>'count')::bigint FROM game_atelier_item_records r
         JOIN game_character_stash_entries e ON e.atelier_transaction_id=r.transaction_id
         WHERE r.cause='RESHAPE' AND r.occurrence=encode($1,'hex')::uuid AND e.amount=2",
    )
    .bind(id(166).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(debited, 2);

    // Carried fragments go first: the stash is untouched while the bag holds enough.
    fixture::small_fragment_stacks(h).await?;
    let bag = carried_small_stacks(h).await?;
    let carried_first = AtelierRequest {
        occurrence: id(167),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::Reshape { gem, position: 2 },
        resources: at_depot(true),
    };
    advance_ex_action(owner)?;
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(carried_first))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
        ),
        "{outcome:?}"
    );
    assert_eq!(carried_small_stacks(h).await?, bag - 2);
    assert_eq!(stashed_lesser(h).await?, carried - 2);

    // A locked gem must be unlocked first: the redraw refuses without custody.
    let lock = AtelierRequest {
        occurrence: id(163),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::SetLock { gem, locked: true },
        resources: None,
    };
    advance_ex_action(owner)?;
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(lock))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
    ));
    let locked = stored(h).await?;
    let fp = footprint(h).await?;
    let fragments = lesser_fragments(h).await?;
    let refused = AtelierRequest {
        occurrence: id(164),
        expected_atelier_revision: locked.revision,
        mutation: AtelierMutation::Reshape { gem, position: 1 },
        resources: Some(resources),
    };
    advance_ex_action(owner)?;
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(refused))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Refused)
    ));
    assert_eq!(stored(h).await?, locked);
    assert_eq!(footprint(h).await?, fp);
    assert_eq!(lesser_fragments(h).await?, fragments);
    Ok(())
}
