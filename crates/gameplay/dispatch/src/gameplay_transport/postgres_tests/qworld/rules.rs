//! Current-occurrence schedule decisions and immutable payload recovery proof.
use super::super::*;
use oteryn_gameplay_world_change::rules::{Counter, Definition, Schedule};
use sha2::{Digest, Sha256};

pub(crate) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    world: crate::foundation::WorldId,
    epoch: u64,
) -> TestResult {
    let before = h.root.world_change_snapshot(world, epoch).await?;
    let (month, day) = h.root.world_change_calendar(world, epoch).await?;
    for (key, schedule, expected) in [
        (
            "qworld_always",
            Schedule::AtReset {
                chance_permille: 1000,
            },
            true,
        ),
        (
            "qworld_never",
            Schedule::AtReset { chance_permille: 0 },
            false,
        ),
        (
            "qworld_rotation",
            Schedule::Rotation {
                period_epochs: 2,
                active_epoch: u16::try_from(epoch % 2)?,
            },
            true,
        ),
        (
            "qworld_rotation_off",
            Schedule::Rotation {
                period_epochs: 2,
                active_epoch: u16::try_from((epoch + 1) % 2)?,
            },
            false,
        ),
        (
            "qworld_calendar",
            Schedule::Calendar {
                months: vec![month],
                days: vec![day],
            },
            true,
        ),
    ] {
        let definition = definition(key, schedule, None);
        h.root
            .bind_world_change_definition(owner.character, world, epoch, definition.clone())
            .await?;
        h.root
            .bind_world_change_definition(owner.character, world, epoch, definition)
            .await?;
        assert_eq!(h.root.world_change_active(world, key).await?, expected);
    }
    let mut selected = 0;
    for suffix in 0..8 {
        let key = format!("{}{:03}", "k".repeat(97), suffix);
        assert_eq!(key.len(), 100);
        h.root
            .bind_world_change_definition(
                owner.character,
                world,
                epoch,
                definition(
                    &key,
                    Schedule::AtReset {
                        chance_permille: 999,
                    },
                    None,
                ),
            )
            .await?;
        selected += usize::from(h.root.world_change_active(world, &key).await?);
    }
    assert!(
        selected > 0,
        "accepted long keys must not hit the RNG purpose limit and become permanently inactive"
    );
    h.root
        .bind_world_change_definition(
            owner.character,
            world,
            epoch,
            definition(
                "qworld_rat_exhaustion",
                Schedule::AtReset {
                    chance_permille: 1000,
                },
                Some(Counter {
                    creature: "oteryn:creature.rat".into(),
                    kills: 2,
                    initial_active: true,
                    active_on_goal: false,
                }),
            ),
        )
        .await?;
    assert!(
        h.root
            .world_change_active(world, "qworld_rat_exhaustion")
            .await?
    );
    let conflict = definition(
        "qworld_always",
        Schedule::AtReset { chance_permille: 0 },
        None,
    );
    assert!(
        h.root
            .bind_world_change_definition(owner.character, world, epoch, conflict)
            .await
            .is_err()
    );
    assert!(h.root.world_change_active(world, "qworld_always").await?);
    assert!(
        h.root
            .bind_world_change_definition(
                owner.character,
                world,
                epoch + 1,
                definition(
                    "qworld_wrong_epoch",
                    Schedule::AtReset {
                        chance_permille: 1000
                    },
                    None
                )
            )
            .await
            .is_err()
    );
    assert!(
        h.root
            .bind_world_change_definition(
                owner.character,
                world,
                epoch,
                definition(
                    "qworld_bad_rotation",
                    Schedule::Rotation {
                        period_epochs: 0,
                        active_epoch: 0
                    },
                    None
                )
            )
            .await
            .is_err()
    );
    let (bytes,digest):(Vec<u8>,Vec<u8>)=sqlx::query_as("SELECT decision,decision_digest FROM game_world_change_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&h.pool).await?;
    let mut forged: serde_json::Value = serde_json::from_slice(&bytes)?;
    forged["world"] = serde_json::json!(id(99));
    let forged = serde_json::to_vec(&forged)?;
    let forged_digest: [u8; 32] = Sha256::digest(&forged).into();
    sqlx::query("UPDATE game_world_change_occurrences SET decision=$3,decision_digest=$4 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(forged).bind(forged_digest.as_slice()).execute(&h.pool).await?;
    assert!(
        h.root.world_change_snapshot(world, epoch).await.is_err(),
        "a valid checksum cannot substitute another World"
    );
    sqlx::query("UPDATE game_world_change_occurrences SET decision=$3,decision_digest=$4 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&bytes).bind(&digest).execute(&h.pool).await?;
    sqlx::query("DELETE FROM game_world_change_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    assert!(
        h.root.world_change_calendar(world, epoch).await.is_err(),
        "boot cannot synthesize a lost bound decision"
    );
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_change_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0);
    sqlx::query("INSERT INTO game_world_change_occurrences(world_id,reset_epoch,decision,decision_digest) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(bytes).bind(digest).execute(&h.pool).await?;
    assert_eq!(h.root.world_change_snapshot(world, epoch).await?, before);
    Ok(())
}

fn definition(key: &str, schedule: Schedule, counter: Option<Counter>) -> Definition {
    Definition {
        key: key.into(),
        schedule,
        counter,
        sources: vec!["oteryn:test/qworld-owned-native-fixture".into()],
    }
}
