use std::sync::Arc;

use crate::achievement_catalogue::AchievementCatalogue;
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, id, runtime, scope,
};
use crate::durability::highscore_snapshot::SnapshotCatalogues;
use crate::foundation::CharacterId;

#[test]
fn ranking_cut_admits_creation_vocation_after_progression_initializes() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let harness =
                        Harness::create_with_bootstrap_vocation(admin, "rank_birth", "sorcerer")
                            .await?;
                    let result = exercise(&harness).await;
                    harness.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "ranking creation-vocation thread failed")?
        .map_err(Into::into)
}

async fn exercise(harness: &Harness) -> TestResult {
    let world = scope()?.world_id();
    let character = CharacterId::decode(&id(CHARACTER)).map_err(debug)?;
    let achievements = AchievementCatalogue::embedded().map_err(debug)?;
    let catalogues = SnapshotCatalogues::activated(Arc::new(achievements)).map_err(debug)?;

    harness.root.await_ready().await?;
    harness
        .root
        .refresh_highscore_schedule(world, catalogues.clone(), true)
        .await
        .map_err(debug)?;
    harness.root.await_ready().await?;
    let before = harness
        .root
        .read_highscore_page(world, 1, None, 1, 0, character)
        .await
        .map_err(debug)?;
    assert!(before.rows.is_empty());
    assert!(before.own_row.is_none());

    sqlx::query(
        "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',\
          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    )
    .bind(character.as_bytes().as_slice())
    .execute(&harness.pool)
    .await?;

    harness.root.await_ready().await?;
    harness
        .root
        .refresh_highscore_schedule(world, catalogues, true)
        .await
        .map_err(debug)?;
    harness.root.await_ready().await?;
    let after = harness
        .root
        .read_highscore_page(world, 1, None, 1, 0, character)
        .await
        .map_err(debug)?;
    assert!(after.snapshot.snapshot_id > before.snapshot.snapshot_id);
    let own = after
        .own_row
        .as_ref()
        .ok_or("initialized character is missing from the cut")?;
    assert_eq!(after.rows.as_slice(), std::slice::from_ref(own));
    assert_eq!(own.character_id, character);
    assert_eq!(own.vocation, "sorcerer");
    assert_eq!(own.level, 50);
    assert_eq!(own.value, (1_000, 0));
    Ok(())
}
