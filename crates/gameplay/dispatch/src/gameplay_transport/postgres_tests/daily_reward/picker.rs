use super::*;
#[test]
fn daily_picker_real_dispatch_validates_rolls_back_delivers_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_feature(false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "picker scenario panicked")?
        .map_err(Into::into)
}
pub(super) async fn case(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    shrine: &[u8],
) -> TestResult {
    use wire::picker::{CLAIM, Claim, Offer, Pick, QUERY};
    let (_, output) = serve(owner, admitted, &[(2, shrine), (QUERY, &[])]).await?;
    let offer = Offer::decode(&results(&output)?[1].2)?;
    assert_eq!((offer.day, offer.maximum), (1, 5));
    assert!(offer.items.contains(&266));
    let claim = |item, count| Claim {
        epoch: offer.epoch,
        day: offer.day,
        picks: vec![Pick { item, count }],
    };
    for bad in [
        claim(266, 6),
        claim(1, 1),
        Claim {
            epoch: offer.epoch + 1,
            ..claim(266, 5)
        },
    ] {
        let bytes = bad.encode()?;
        let (_, output) = serve(owner, admitted, &[(CLAIM, &bytes)]).await?;
        assert_eq!(
            results(&output)?[0].1,
            crate::foundation::CommandStatus::Rejected
        );
        let claims: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_daily_reward_claims")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            claims, 0,
            "invalid selection must roll back its daily claim"
        );
    }
    let good = Claim {
        epoch: offer.epoch,
        day: 1,
        picks: vec![
            Pick {
                item: 266,
                count: 3,
            },
            Pick {
                item: 268,
                count: 2,
            },
        ],
    }
    .encode()?;
    let (_, output) = serve(
        owner,
        admitted,
        &[(CLAIM, &good), (CLAIM, &good), (QUERY, &[])],
    )
    .await?;
    let values = results(&output)?;
    assert_eq!(values[0].2[1], 1);
    assert_eq!(values[1].2, [8, 2]);
    assert_eq!(Offer::decode(&values[2].2)?.maximum, 0);
    let outputs:(i64,i64,i64)=sqlx::query_as("SELECT count(*),sum(quantity)::bigint,(SELECT count(*) FROM game_item_bindings) FROM game_daily_reward_picks").fetch_one(&h.pool).await?;
    assert_eq!(outputs, (2, 5, 2));
    let physical:Vec<(String, i64)>=sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_daily_reward_picks p JOIN game_item_instances i USING(item_instance_id) ORDER BY p.source_id").fetch_all(&h.pool).await?;
    assert_eq!(
        physical,
        vec![
            ("oteryn:item.tibia.i266".into(), 3),
            ("oteryn:item.tibia.i268".into(), 2)
        ]
    );
    let (_, output) = serve(owner, admitted, &[(CLAIM, &claim(266, 1).encode()?)]).await?;
    assert_eq!(results(&output)?[0].2, [8, 6]);
    // Advance the same real owner through the item and non-item schedule.
    for day in 2..=5 {
        epoch(h, day).await?;
        let (_, output) = serve(
            owner,
            admitted,
            &[(wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[])],
        )
        .await?;
        assert_eq!(results(&output)?[0].2[1], 1);
    }
    epoch(h, 6).await?;
    let (_, output) = serve(owner, admitted, &[(QUERY, &[])]).await?;
    let exercise = Offer::decode(&results(&output)?[0].2)?;
    assert_eq!((exercise.day, exercise.maximum), (6, 1));
    assert!(exercise.items.contains(&28540));
    let training = Claim {
        epoch: exercise.epoch,
        day: 6,
        picks: vec![Pick {
            item: 28540,
            count: 1,
        }],
    }
    .encode()?;
    let (_, output) = serve(owner, admitted, &[(CLAIM, &training)]).await?;
    assert_eq!(results(&output)?[0].2[1], 1);
    let charged: (i16, i16) = sqlx::query_as(
        "SELECT quantity,charges FROM game_daily_reward_picks WHERE source_id=28540",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(charged, (1, 50));
    epoch(h, 7).await?;
    let (_, output) = serve(
        owner,
        admitted,
        &[(wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[])],
    )
    .await?;
    assert_eq!(results(&output)?[0].2[1], 1);
    epoch(h, 8).await?;
    let (_, output) = serve(owner, admitted, &[(QUERY, &[])]).await?;
    let available = Offer::decode(&results(&output)?[0].2)?;
    assert_eq!((available.day, available.maximum), (8, 5));
    let mut stale = *admitted;
    let mut fence = stale.item_fence.ok_or("fence")?;
    fence.connection_generation =
        crate::foundation::ConnectionGeneration::new(99).map_err(debug)?;
    stale.item_fence = Some(fence);
    let bytes = Claim {
        epoch: available.epoch,
        day: available.day,
        picks: vec![Pick {
            item: 266,
            count: 5,
        }],
    }
    .encode()?;
    let (_, output) = serve(owner, &mut stale, &[(CLAIM, &bytes)]).await?;
    assert!(results(&output)?.is_empty());
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_daily_reward_picks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(remaining, 3, "stale ingress must not mint another item");
    Ok(())
}
