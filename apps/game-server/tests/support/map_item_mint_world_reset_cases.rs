//! Admission at the legacy epoch and across RETIRING publication.
use super::*;

#[test]
fn world_reset_map_mint_admits_without_reset_state_and_fences_retiring() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "reset_admission").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for relation in [
            "game_world_reset_epochs",
            "game_world_reset_active",
            "game_world_reset_jobs",
        ] {
            assert_eq!(harness.count(relation).await?, 0);
        }
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let first = harness
            .taken(
                &authority,
                fence()?,
                pickup(command(1)?, entry(300, 200, 0)?, 0, 1),
            )
            .await?;
        assert!(harness.ground_of(first.item_instance_id).await?.is_some());

        // An uninitialized World admits epoch zero, never a caller-chosen epoch.
        let before = harness.footprint().await?;
        assert!(matches!(
            harness
                .take(
                    &authority,
                    fence()?,
                    pickup(command(2)?, entry(301, 200, 0)?, 1, 1)
                )
                .await,
            Err(MapItemMintError::AuthorityRejected)
        ));
        assert_eq!(harness.footprint().await?, before);

        let mut prepared = harness
            .root
            .freeze_map_item_mint(
                &authority,
                &harness.node,
                fence()?,
                pickup(command(3)?, entry(302, 200, 0)?, 0, 1),
            )
            .await
            .map_err(debug)?;
        crate::durability::world_reset::test_support::seed_retiring(&harness.pool, world).await?;
        assert!(harness.root.retiring_world_reset(world).await?.is_some());
        let before = harness.footprint().await?;
        // Leave the session and assignment unchanged: only the reset fence closes.
        assert!(matches!(
            harness
                .take(
                    &authority,
                    fence()?,
                    pickup(command(4)?, entry(303, 200, 0)?, 0, 1)
                )
                .await,
            Err(MapItemMintError::AuthorityRejected)
        ));
        assert!(matches!(
            harness
                .root
                .commit_map_item_mint(
                    &SpellLanePermit::of_fresh_scope(fence()?.runtime_scope).await?,
                    &authority,
                    &harness.node,
                    fence()?,
                    &mut prepared
                )
                .await,
            Err(MapItemMintError::AuthorityRejected)
        ));
        assert_eq!(harness.footprint().await?, before);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
