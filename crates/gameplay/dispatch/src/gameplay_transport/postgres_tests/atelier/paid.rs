use super::*;
use crate::durability::character_atelier::AtelierResources;
use crate::durability::character_wheel::{
    WheelChangeRequest, WheelCommitOutcome, WheelOccurrence, WheelSlots,
};
use crate::durability::item_fee_burn::test_support::{definition, request as fee_request};
use crate::gameplay_transport::atelier::{GemBuildCommand, GemBuildOutcome};

async fn quantity(h: &Harness, seed: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(seed).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_atelier_item_records),(SELECT count(*) FROM game_item_fee_burns),(SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid)").bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    fixture::paid_assets(h).await?;
    let preserves_historical_fees:bool=sqlx::query_scalar("SELECT NOT convalidated FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause'").fetch_one(&h.pool).await?;
    assert!(
        preserves_historical_fees,
        "0298's historical fee receipts remain valid"
    );
    let content = fixture::MechanicContent;
    let resources = Box::new(AtelierResources {
        content: &content,
        coins: fee_request(2, 1)?.change,
        fragment: definition("oteryn:item.tibia.i46625"),
    });
    let session = crate::bestiary_postgres_harness::fence(1)?.game_session_id;
    let mut requests = Vec::new();
    for (seed, quality, input) in [
        (180, GemQuality::Lesser, 220),
        (181, GemQuality::Regular, 223),
        (182, GemQuality::Greater, 226),
    ] {
        let request = AtelierRequest {
            occurrence: id(seed),
            expected_atelier_revision: stored(h).await?.revision,
            mutation: AtelierMutation::Reveal { quality },
            resources: Some(resources.clone()),
        };
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
        assert_eq!(quantity(h, input).await?, 1);
        let revealed = stored(h).await?;
        assert_eq!(
            revealed.gems[&u128::from_be_bytes(id(seed))].quality,
            quality
        );
        assert_eq!(revealed.gold, 0);
        assert!(revealed.unrevealed.is_empty() && revealed.fragments.is_empty());
        let fp = footprint(h).await?;
        let replay = owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(request.clone()))
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                replay,
                GemBuildOutcome::Atelier(AtelierOutcome::AlreadyCommitted { .. })
            ),
            "{replay:?}"
        );
        assert_eq!(footprint(h).await?, fp);
        assert_eq!(stored(h).await?, revealed);
        requests.push(request);
    }
    // One fee is coins-only; later fees consume remaining coins then debit the existing bank.
    let fees:Vec<(i64,i64)>=sqlx::query_as("SELECT fee_gold_units,bank_debit_gold_units FROM game_item_fee_burns WHERE cause_kind=9 ORDER BY fee_gold_units").fetch_all(&h.pool).await?;
    assert_eq!(
        fees,
        vec![(125_000, 0), (1_000_000, 125_000), (6_000_000, 6_000_000)]
    );
    let mut conflict = requests[0].clone();
    conflict.mutation = AtelierMutation::Reveal {
        quality: GemQuality::Greater,
    };
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(conflict))
            .await,
        Err(CharacterProgressionError::ConflictingOccurrence)
    ));
    // Full fragment stacks force a fresh-entry MINT. The next grade burns that small stack whole.
    let fresh = AtelierRequest {
        occurrence: id(178),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::Dismantle { gem: id(181) },
        resources: Some(resources.clone()),
    };
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(fresh))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
        ),
        "{outcome:?}"
    );
    let fresh_yield:i64=sqlx::query_scalar("SELECT (outputs->0->>'count')::bigint FROM game_atelier_item_records WHERE occurrence=encode($1,'hex')::uuid").bind(id(178).as_slice()).fetch_one(&h.pool).await?;
    assert!((2..=5).contains(&fresh_yield));
    fixture::small_fragment_stacks(h).await?;
    let fp = footprint(h).await?;
    let before = stored(h).await?;
    let stale = AtelierRequest {
        occurrence: id(183),
        expected_atelier_revision: 0,
        mutation: AtelierMutation::Reveal {
            quality: GemQuality::Lesser,
        },
        resources: Some(resources.clone()),
    };
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(stale))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Refused)
    ));
    assert_eq!(stored(h).await?, before);
    assert_eq!(footprint(h).await?, fp);
    // A fee refusal after the planned material BURN rolls back that material and its audit.
    let mut bad = resources.clone();
    bad.coins.backpack.definition.revision_ref = "incompatible".into();
    let mod_id = before
        .gems
        .values()
        .find(|gem| gem.domain == WheelDomain::Green && gem.quality == GemQuality::Regular)
        .ok_or("green regular")?
        .mods[1];
    let refused = AtelierRequest {
        occurrence: id(184),
        expected_atelier_revision: before.revision,
        mutation: AtelierMutation::GradeUp {
            kind: FragmentKind::Basic,
            mod_id,
        },
        resources: Some(bad),
    };
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(refused))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Refused)
    ));
    assert_eq!(quantity(h, 229).await?, 100);
    assert_eq!(footprint(h).await?, fp);
    assert_eq!(stored(h).await?, before);
    for seed in 185..188 {
        let request = AtelierRequest {
            occurrence: id(seed),
            expected_atelier_revision: stored(h).await?.revision,
            mutation: AtelierMutation::GradeUp {
                kind: FragmentKind::Basic,
                mod_id,
            },
            resources: Some(resources.clone()),
        };
        let outcome = owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(request))
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                outcome,
                GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
            ),
            "{outcome:?}"
        );
    }
    assert_eq!(stored(h).await?.grades[&(FragmentKind::Basic, mod_id)], 3);
    assert_eq!(quantity(h, 229).await?, 56 + fresh_yield);
    let retired:(i64,i16,bool)=sqlx::query_as("SELECT i.quantity,i.lifecycle,NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id) FROM game_atelier_item_records r JOIN game_item_instances i ON i.item_instance_id=(r.outputs->0->>'item')::uuid WHERE r.occurrence=encode($1,'hex')::uuid").bind(id(178).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(retired, (0, 2, true));
    let green = stored(h)
        .await?
        .gems
        .values()
        .find(|gem| gem.domain == WheelDomain::Green && gem.quality == GemQuality::Regular)
        .ok_or("green regular")?
        .id
        .to_be_bytes();
    let wrong = WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(179)).map_err(debug)?,
        expected_wheel_revision: 0,
        slots: WheelSlots::ZERO,
        vessels: Some([None, Some(green), None, None]),
    };
    assert!(
        matches!(owner.execute_gem_atelier(session,GemBuildCommand::Wheel(wrong)).await.map_err(debug)?,GemBuildOutcome::Wheel(ref change) if matches!(**change,WheelCommitOutcome::Refused(_)))
    );
    let mut points = [0; 36];
    points[14] = 11;
    let earned = WheelSlots::new(points).map_err(debug)?;
    let wheel = WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(188)).map_err(debug)?,
        expected_wheel_revision: 0,
        slots: earned,
        vessels: Some([Some(green), None, None, None]),
    };
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Wheel(wheel))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Wheel(ref change) if matches!(**change, WheelCommitOutcome::Committed(_))
        ),
        "{outcome:?}"
    );
    assert!(
        owner
            .states
            .atelier_sessions
            .lock()
            .map_err(debug)?
            .get(&session)
            .and_then(|s| s.as_ref())
            .ok_or("projection")?
            .gems[&u128::from_be_bytes(green)]
            .vessel
    );
    let replay = owner
        .execute_gem_atelier(session, GemBuildCommand::Wheel(wheel))
        .await
        .map_err(debug)?;
    assert!(matches!(
        replay,
        GemBuildOutcome::Wheel(ref change) if matches!(**change, WheelCommitOutcome::AlreadyCommitted(_))
    ));
    let mut too_many = points;
    too_many[14] = 12;
    let over = WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(193)).map_err(debug)?,
        expected_wheel_revision: 1,
        slots: WheelSlots::new(too_many).map_err(debug)?,
        vessels: None,
    };
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Wheel(over))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Wheel(ref change) if matches!(**change, WheelCommitOutcome::Refused(crate::durability::character_wheel::WheelRefusal::OverPoints))
    ));
    let fp = footprint(h).await?;
    let blocked = AtelierRequest {
        occurrence: id(189),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::SwitchDomain { gem: green },
        resources: Some(resources.clone()),
    };
    assert!(matches!(
        owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(blocked))
            .await
            .map_err(debug)?,
        GemBuildOutcome::Atelier(AtelierOutcome::Refused)
    ));
    assert_eq!(footprint(h).await?, fp);
    let remove = WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(190)).map_err(debug)?,
        expected_wheel_revision: 1,
        slots: earned,
        vessels: Some([None; 4]),
    };
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Wheel(remove))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Wheel(ref change) if matches!(**change, WheelCommitOutcome::Committed(_))
        ),
        "{outcome:?}"
    );
    let unlock = AtelierRequest {
        occurrence: id(194),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::SetLock {
            gem: green,
            locked: false,
        },
        resources: None,
    };
    owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(unlock))
        .await
        .map_err(debug)?;
    let switch = AtelierRequest {
        occurrence: id(191),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::SwitchDomain { gem: green },
        resources: Some(resources.clone()),
    };
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(switch))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
        ),
        "{outcome:?}"
    );
    assert_eq!(
        stored(h).await?.gems[&u128::from_be_bytes(green)].domain,
        WheelDomain::Red
    );
    let dismantle = AtelierRequest {
        occurrence: id(192),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::Dismantle { gem: id(180) },
        resources: Some(resources),
    };
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(dismantle.clone()))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
        ),
        "{outcome:?}"
    );
    assert!(
        !stored(h)
            .await?
            .gems
            .contains_key(&u128::from_be_bytes(id(180)))
    );
    assert!(
        (57 + fresh_yield..=59 + fresh_yield).contains(&quantity(h, 229).await?),
        "dismantle merges into the compatible fragment stack"
    );
    let fp = footprint(h).await?;
    owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(dismantle))
        .await
        .map_err(debug)?;
    assert_eq!(footprint(h).await?, fp);
    fixture::higher_level(h).await?;
    let data = WheelGemData::embedded().map_err(debug)?;
    let mut full = [0; 36];
    for (index, slot) in data.wheel()["topology"]
        .as_array()
        .ok_or("topology")?
        .iter()
        .enumerate()
    {
        if slot["domain"] == "red" {
            full[index] = u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?;
        }
    }
    let wheel = WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(195)).map_err(debug)?,
        expected_wheel_revision: 2,
        slots: WheelSlots::new(full).map_err(debug)?,
        vessels: Some([None, Some(green), None, None]),
    };
    // Keep the existing 11-point central slice too: moving points still needs a temple.
    let mut wheel = wheel;
    full[14] = 11;
    wheel.slots = WheelSlots::new(full).map_err(debug)?;
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Wheel(wheel))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Wheel(ref change) if matches!(**change, WheelCommitOutcome::Committed(_))
        ),
        "{outcome:?}"
    );
    let effects = owner
        .wheel_stat_bonuses(session, "sorcerer", 2_000, true, true)
        .ok_or("gem effects")?;
    assert_eq!(effects.sockets.grade_iv_points, 1);
    assert_eq!(
        effects
            .sockets
            .effective_grades
            .get(&u128::from_be_bytes(green)),
        Some(&vec![0, 0])
    );
    assert!(!effects.sockets.gem_values_milli.is_empty());
    let supreme_mod = stored(h).await?.gems[&u128::from_be_bytes(id(182))].mods[2];
    for seed in 196..199 {
        let request = AtelierRequest {
            occurrence: id(seed),
            expected_atelier_revision: stored(h).await?.revision,
            mutation: AtelierMutation::GradeUp {
                kind: FragmentKind::Supreme,
                mod_id: supreme_mod,
            },
            resources: Some(Box::new(AtelierResources {
                content: &content,
                coins: fee_request(3, 1)?.change,
                fragment: definition("oteryn:item.tibia.i46626"),
            })),
        };
        let outcome = owner
            .execute_gem_atelier(session, GemBuildCommand::Atelier(request))
            .await
            .map_err(debug)?;
        assert!(
            matches!(
                outcome,
                GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
            ),
            "{outcome:?}"
        );
    }
    assert_eq!(quantity(h, 232).await?, 50);
    assert_eq!(
        owner
            .wheel_stat_bonuses(session, "sorcerer", 2000, true, true)
            .ok_or("projection")?
            .sockets
            .grade_iv_points,
        2
    );
    let fees:Vec<i64>=sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=11 ORDER BY fee_gold_units").fetch_all(&h.pool).await?;
    assert_eq!(
        fees,
        vec![
            2_000_000, 5_000_000, 5_000_000, 12_000_000, 30_000_000, 75_000_000
        ]
    );
    let dismantle = AtelierRequest {
        occurrence: id(199),
        expected_atelier_revision: stored(h).await?.revision,
        mutation: AtelierMutation::Dismantle { gem: id(182) },
        resources: Some(Box::new(AtelierResources {
            content: &content,
            coins: fee_request(4, 1)?.change,
            fragment: definition("oteryn:item.tibia.i46626"),
        })),
    };
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(dismantle))
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            outcome,
            GemBuildOutcome::Atelier(AtelierOutcome::Committed { .. })
        ),
        "{outcome:?}"
    );
    assert!((51..=53).contains(&quantity(h, 232).await?));
    let same_transaction:bool=sqlx::query_scalar("SELECT bool_and(fee_transaction_id=item_transaction_id) FROM game_character_atelier_receipts WHERE kind IN ('REVEAL','GRADE_UP')").fetch_one(&h.pool).await?;
    assert!(
        same_transaction,
        "one logical transaction binds the materials and gold fee"
    );
    let one_event:bool=sqlx::query_scalar("SELECT bool_and((SELECT count(*) FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id)=1) FROM game_atelier_item_records r").fetch_one(&h.pool).await?;
    assert!(one_event, "each combined action emits one audit event");
    let envelopes:Vec<Vec<u8>>=sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_kind IN (9,11) ORDER BY f.transaction_id LIMIT 9").fetch_all(&h.pool).await?;
    assert_eq!(envelopes.len(), 9);
    for wire in envelopes {
        use crate::durability::item_fee_burn_audit::{FeeBurnCauseV1, decode_fee_burn_envelope};
        let (_, event) = decode_fee_burn_envelope(&wire).map_err(debug)?;
        let cause = event.cause.and_then(|c| c.cause).ok_or("fee cause")?;
        let proof = match cause {
            FeeBurnCauseV1::GemReveal(cause) | FeeBurnCauseV1::GemGradeUp(cause) => {
                cause.atelier_proof
            }
            _ => return Err("unexpected combined cause".into()),
        };
        assert!(!proof.is_empty());
    }

    let state = stored(h).await?;
    let fp = footprint(h).await?;
    let mut replay = requests.remove(0);
    replay.resources = None;
    let outcome = owner
        .execute_gem_atelier(session, GemBuildCommand::Atelier(replay))
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        GemBuildOutcome::Atelier(AtelierOutcome::AlreadyCommitted { .. })
    ));
    assert_eq!(stored(h).await?, state);
    assert_eq!(footprint(h).await?, fp);
    Ok(())
}
