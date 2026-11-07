//! Adversarial mature owner call: a reserved Source key cannot bypass its child.
//! This proves durable conservation, not a native portal/travel play test.
use super::*;
#[test]
fn srv7_summer_source_parent_cannot_omit_private_room_child() -> TestResult {
    run(async |admin| {
        let h = super::super::fixture::Harness::create_with_admission_seed(
            admin,
            "srv7_source_parent_omission",
            true,
            false,
        )
        .await?;
        let (_evidence, mut request) =
            super::super::activity_runtime::prepare_activity_fixture(&h).await?;
        request.activity_key = "oteryn:boss.rakesh_moonfang".into();
        request.template_revision =
            "source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105".into();
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let result = crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
            .enter(&authority, &h.node, request)
            .await;
        assert!(
            !matches!(
                result,
                Ok(
                    crate::durability::activity_handoff::ActivityEnterOutcome::Entered(_)
                        | crate::durability::activity_handoff::ActivityEnterOutcome::Replayed(_)
                )
            ),
            "a mature core Source parent committed without its source room child: {result:?}"
        );
        let committed: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups WHERE state=2")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            committed, 0,
            "a missing Source child rolls back its entire parent/successor batch"
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
