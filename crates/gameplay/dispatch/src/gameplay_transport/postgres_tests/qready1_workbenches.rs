//! M20: native laboratory stations consume every declared input atomically.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) fn matches(name: &str) -> bool {
    matches!(name, "qready1-cradle-brew" | "qready1-scatter-blend")
}

pub(super) fn start(name: &str) -> TestResult<Position> {
    match name {
        "qready1-cradle-brew" => Ok(Position {
            x: 33786,
            y: 32571,
            floor: -12,
        }),
        "qready1-scatter-blend" => Ok(Position {
            x: 32292,
            y: 32293,
            floor: -8,
        }),
        _ => Err("unknown M20 workbench".into()),
    }
}

#[test]
fn qready1_cradle_cauldron_three_inputs_one_antimutagen_atomic_and_repeatable() -> TestResult {
    run(
        "qready1-cradle-brew",
        Position {
            x: 33786,
            y: 32570,
            floor: -12,
        },
    )
}

#[test]
fn qready1_scatter_blender_five_inputs_one_flakes_atomic_and_repeatable() -> TestResult {
    run(
        "qready1-scatter-blend",
        Position {
            x: 32293,
            y: 32293,
            floor: -8,
        },
    )
}

fn run(name: &'static str, at: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, at, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("M20 workbench worker panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    let crop = match name {
        "qready1-cradle-brew" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z12-x131-y127.b3"
            )),
            region: (12, 131, 127),
            bounds: (33784, 32568, 33788, 32573),
            start: (33786, 32571),
            expected: vec![
                ExpectedTile {
                    position: (33786, 32570),
                    ids: vec![40232, 2343, 8997],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33786, 32571),
                    ids: vec![40231, 8698],
                    counted_entry: None,
                },
            ],
        },
        "qready1-scatter-blend" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x126-y126.b3"
            )),
            region: (8, 126, 126),
            bounds: (32290, 32290, 32295, 32295),
            start: (32292, 32293),
            expected: vec![
                ExpectedTile {
                    position: (32293, 32293),
                    ids: vec![416, 2345, 9108],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32292, 32293),
                    ids: vec![416],
                    counted_entry: None,
                },
            ],
        },
        _ => return Err("unknown M20 native workbench".into()),
    };
    native_crop(native, crop)
}

async fn ledger(h: &Harness, callback: &Callback) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$2)")
        .bind(&callback.storage[0].track).bind(&callback.key).fetch_one(&h.pool).await?)
}

async fn use_(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: &WorldObjectTarget,
    id: u64,
) -> TestResult<UseDisposition> {
    super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(id, target)])
        .await?
        .first()
        .map(|(_, disposition)| *disposition)
        .ok_or_else(|| "workbench USE result".into())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let native = callback.selector.native.as_ref().ok_or("native station")?;
    let at = native.position;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, native.ordinal)
        .ok_or("station key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let original = target.clone();
    let carried = callback.item_on_target.is_some();
    let inputs: Vec<_> = if let Some(primary) = &callback.item_on_target {
        std::iter::once((primary.item.clone(), primary.count))
            .chain(
                primary
                    .ingredients
                    .iter()
                    .map(|input| (input.item.clone(), input.count)),
            )
            .collect()
    } else {
        callback
            .offerings
            .iter()
            .map(|input| (input.item.clone(), input.count))
            .collect()
    };
    assert_eq!(inputs.len(), if carried { 3 } else { 5 });
    let cost: i64 = inputs.iter().map(|(_, count)| i64::from(*count)).sum();
    let fence = admitted.item_fence.ok_or("fence")?;
    let seed_position = if carried { start(name)? } else { at };
    // Missing the final ingredient must leave every earlier ingredient and the counter untouched.
    for (ordinal, (item, count)) in inputs[..inputs.len() - 1].iter().enumerate() {
        super::quest_world_callbacks::input_item(
            owner,
            fence,
            &Offering {
                position: seed_position,
                item: item.clone(),
                count: *count,
            },
            600 + ordinal as u64,
            carried,
        )
        .await?;
    }
    assert_eq!(
        use_(owner, admitted, &target, 1).await?,
        UseDisposition::Rejected
    );
    assert_eq!(ledger(h, callback).await?, (0, 0, 0, 0));
    let (item, count) = inputs.last().ok_or("last ingredient")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: seed_position,
            item: item.clone(),
            count: *count,
        },
        700,
        carried,
    )
    .await?;
    for (command, epoch) in [(2, 1), (4, 2)] {
        assert_eq!(
            use_(owner, admitted, &target, command).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            ledger(h, callback).await?,
            (epoch, epoch, epoch, epoch * cost)
        );
        target.expected_revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
        assert_eq!(
            use_(owner, admitted, &target, command + 1).await?,
            UseDisposition::Rejected
        );
        assert_eq!(
            ledger(h, callback).await?,
            (epoch, epoch, epoch, epoch * cost)
        );
        if epoch == 1 {
            for (ordinal, (item, count)) in inputs.iter().enumerate() {
                super::quest_world_callbacks::input_item(
                    owner,
                    fence,
                    &Offering {
                        position: seed_position,
                        item: item.clone(),
                        count: *count,
                    },
                    800 + ordinal as u64,
                    carried,
                )
                .await?;
            }
        }
    }
    let reward = callback
        .effects
        .item_reward
        .as_ref()
        .ok_or("recipe output")?;
    let outputs: Vec<(String,i64)> = sqlx::query_as("SELECT i.definition_production_key,sum(m.quantity)::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) GROUP BY i.definition_production_key")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        outputs,
        vec![(reward.item.clone(), 2 * i64::from(reward.count))]
    );
    let burned: Vec<(String,i64)> = sqlx::query_as("SELECT before_item->>'definition_production_key',sum(quantity)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1 GROUP BY 1 ORDER BY 1")
        .bind(&callback.key).fetch_all(&h.pool).await?;
    let mut expected: Vec<_> = inputs
        .into_iter()
        .map(|(key, count)| (key, 2 * i64::from(count)))
        .collect();
    expected.sort();
    assert_eq!(burned, expected);
    let transactions: i64 = sqlx::query_scalar("SELECT count(DISTINCT tx) FROM (SELECT created_xact_id::text tx FROM game_character_quest_receipts WHERE transition_key=$1 UNION ALL SELECT created_xact_id::text FROM game_quest_callback_consumptions WHERE transition_key=$1 UNION ALL SELECT created_xact_id::text FROM game_reward_claim_mint_receipts) proofs")
        .bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(
        transactions, 2,
        "inputs, counter and output share their crafting transaction"
    );
    let after = ledger(h, callback).await?;
    assert!(
        use_(owner, admitted, &original, 2)
            .await
            .err()
            .is_some_and(|error| error.to_string().contains("CommandOutcomeExpired"))
    );
    assert_eq!(ledger(h, callback).await?, after);
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        2
    );
    Ok(())
}
