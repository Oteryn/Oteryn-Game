//! Durable invalidation covers buffers even after a bystander left the node.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};

#[test]
fn s17_cast_postgres_cross_worker_consent_barrier() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|e| e.to_string())?
                .block_on(privacy_case(admin))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "cross-worker PG thread panicked")?
        .map_err(Into::into)
}
async fn privacy_case(admin: String) -> TestResult {
    use crate::bestiary_postgres_harness::{debug, fence, id};
    use crate::durability::cast_consent::{CastConsent, CastConsentOutcome as Outcome};
    use oteryn_gameplay_social::{
        cast::{CastOwner, Content, Error, Scene, Settings, test_support::admission},
        spectate::spectate_test_support::facts,
    };
    let started = std::time::Instant::now();
    let h = Harness::create(admin, "s17_privacy_barrier", true).await?;
    let result = async {
        let table: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('game_cast_consent_workers')::text")
                .fetch_one(&h.pool)
                .await?;
        assert!(
            table.is_some(),
            "cross-worker withdrawal must have a durable lease/cursor barrier"
        );
        assert!(!sqlx::query_scalar::<_,bool>("SELECT has_function_privilege('oteryn_game_runtime','game_expire_cast_consent_metadata(integer)','EXECUTE')").fetch_one(&h.pool).await?);
        assert!(sqlx::query_scalar::<_,bool>("SELECT has_function_privilege('oteryn_game_control','game_expire_cast_consent_metadata(integer)','EXECUTE')").fetch_one(&h.pool).await?);
        assert!(sqlx::query_scalar::<_,i32>("SELECT game_expire_cast_consent_metadata(NULL)").fetch_one(&h.pool).await.is_err());
        assert!(sqlx::query_scalar::<_,i32>("SELECT game_expire_cast_consent_metadata(129)").fetch_one(&h.pool).await.is_err());
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = fence(1)?;
        let bystander = fence.character_id;
        let on = CastConsent {
            allowed: true,
            revision: 1,
        };
        assert_eq!(
            h.root
                .character_cast_consent(&authority, &h.node, fence, Some((0, true)))
                .await?,
            Outcome::Written(on)
        );
        let target = facts(1, 1, 1);
        let mut owners = [CastOwner::new(8)?, CastOwner::new(8)?];
        let mut viewers = Vec::new();
        for (index, owner) in owners.iter_mut().enumerate() {
            let state = owner.start(
                target,
                Settings {
                    delay_seconds: 1,
                    ..Settings::default()
                },
                0,
            )?;
            owner.persist_audit(&h.root, &h.node).await?;
            let viewer = owner.join(
                &admission(state, [index as u8 + 1; 16], [index as u8 + 1; 16], 1),
                0,
            )?;
            owner.persist_audit(&h.root, &h.node).await?;
            owner.consent(bystander, on)?;
            owner.publish(
                target,
                &[bystander],
                Scene {
                    tiles: vec![(1, 2, 3)],
                    actors: vec![],
                },
                0,
            )?;
            viewers.push(viewer);
        }
        let mut workers = [
            h.root.register_cast_consent_worker(&h.node, id(97)).await?,
            h.root.register_cast_consent_worker(&h.node, id(98)).await?,
        ];
        assert!(
            h.root
                .register_cast_consent_worker(&h.node, id(97))
                .await
                .is_err(),
            "a live worker cannot reset/fast-forward its cursor"
        );
        let mut withdrawal = std::pin::pin!(h.root.character_cast_consent(
            &authority,
            &h.node,
            fence,
            Some((1, false))
        ));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(40), &mut withdrawal)
                .await
                .is_err()
        );
        suspend_with_idle_holder(withdrawal.as_mut(), &h.root).await;
        // New captures already deny the old preference; a fresh empty worker can
        // start at this prefix without replaying history or resurrecting old media.
        assert_eq!(
            h.root.read_cast_consents(&h.node, &[bystander]).await?,
            vec![(
                bystander,
                CastConsent {
                    allowed: false,
                    revision: 1
                }
            )]
        );
        let fresh = h.root.register_cast_consent_worker(&h.node, id(99)).await?;
        h.root.close_cast_consent_worker(&h.node, fresh).await?;
        for index in 0..2 {
            let batch = h
                .root
                .refresh_cast_consent_worker(&h.node, &mut workers[index])
                .await?;
            assert_eq!(batch.events().len(), 1);
            for event in batch.events() {
                owners[index].invalidate(event.character, event.minimum_revision - 1)?;
            }
            h.root
                .acknowledge_cast_consent(&h.node, &mut workers[index], batch)
                .await?;
            if index == 0 {
                assert!(
                    tokio::time::timeout(std::time::Duration::from_millis(40), &mut withdrawal)
                        .await
                        .is_err(),
                    "one worker's ACK must not commit another worker's buffered permission"
                );
                suspend_with_idle_holder(withdrawal.as_mut(), &h.root).await;
                let allowed: bool =
                    sqlx::query_scalar("SELECT allowed FROM game_character_cast_consent")
                        .fetch_one(&h.pool)
                        .await?;
                assert!(allowed);
            }
        }
        let off = CastConsent {
            allowed: false,
            revision: 2,
        };
        assert_eq!(withdrawal.await?, Outcome::Written(off));
        for (owner, viewer) in owners.iter_mut().zip(&viewers) {
            assert_eq!(owner.consent(bystander, on), Err(Error::Conflict));
            let mut saw_clear = false;
            owner.flush(*viewer, 1000, &mut |content| {
                saw_clear |= matches!(content, Content::Clear);
                assert!(!matches!(content, Content::Scene(_)));
                true
            })?;
            assert!(saw_clear);
        }
        assert_eq!(
            h.root
                .character_cast_consent(&authority, &h.node, fence, Some((2, true)))
                .await?,
            Outcome::Written(CastConsent {
                allowed: true,
                revision: 3
            })
        );
        for (owner, viewer) in owners.iter_mut().zip(&viewers) {
            owner.consent(
                bystander,
                CastConsent {
                    allowed: true,
                    revision: 3,
                },
            )?;
            owner.publish(
                target,
                &[bystander],
                Scene {
                    tiles: vec![(1, 2, 3)],
                    actors: vec![],
                },
                1000,
            )?;
            let mut shown = false;
            owner.flush(*viewer, 2000, &mut |content| {
                shown |= matches!(content, Content::Scene(_));
                true
            })?;
            assert!(shown);
        }
        for worker in workers {
            h.root.close_cast_consent_worker(&h.node, worker).await?;
        }
        Ok(())
    }
    .await;
    h.cleanup().await?;
    eprintln!(
        "S17 cross-worker privacy PG executed in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    result
}

// A timeout leaves the borrowed future alive and may suspend it during a DB
// pass. Keep driving it until that pass returns the single holder; otherwise
// the next operation races a holder stranded inside this same test's future.
async fn suspend_with_idle_holder<F: std::future::Future>(
    mut withdrawal: std::pin::Pin<&mut F>,
    root: &crate::durability::DurabilityRoot,
) {
    std::future::poll_fn(|cx| {
        assert!(
            withdrawal.as_mut().poll(cx).is_pending(),
            "withdrawal must remain pending before every worker acknowledges"
        );
        if root.is_ready() {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
}
