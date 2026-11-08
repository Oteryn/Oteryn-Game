//! M20 World crafting against the unchanged native wooden grinder room.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "qready1-cursed-grind";
const AT: Position = Position {
    x: 32012,
    y: 32908,
    floor: -7,
};

#[test]
fn qready1_native_grinder_spends_each_splinter_and_mints_each_dust_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(NAME, AT, false).map_err(|e| e.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("M20 grinder PG worker panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x125-y128.b3"
            )),
            region: (7, 125, 128),
            bounds: (32010, 32906, 32014, 32910),
            start: (32011, 32907),
            expected: vec![
                ExpectedTile {
                    position: (32012, 32908),
                    ids: vec![4404, 4572, 15858, 21573],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32011, 32907),
                    ids: vec![4405],
                    counted_entry: None,
                },
            ],
        },
    )
}

async fn use_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: &WorldObjectTarget,
    id: u64,
) -> TestResult<UseDisposition> {
    super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(id, target)])
        .await?
        .first()
        .map(|(_, disposition)| *disposition)
        .ok_or_else(|| "grinder USE reply".into())
}
async fn snapshot(h: &Harness, callback: &Callback) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$2)")
        .bind(&callback.storage[0].track).bind(&callback.key).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    let binding = callback
        .selector
        .native
        .as_ref()
        .ok_or("production native grinder")?;
    assert_eq!(binding.ordinal, 3);
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, binding.ordinal)
        .ok_or("grinder key")?;
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
    assert_eq!(
        use_command(owner, admitted, &target, 1).await?,
        UseDisposition::Rejected
    );
    assert_eq!(snapshot(h, callback).await?, (0, 0, 0, 0));
    super::quest_world_callbacks::input_item(
        owner,
        admitted.item_fence.ok_or("fence")?,
        &Offering {
            position: Position {
                x: 32011,
                y: 32907,
                floor: -7,
            },
            item: "oteryn:item.tibia.i16122".into(),
            count: 2,
        },
        301,
        true,
    )
    .await?;
    for id in [2, 3] {
        assert_eq!(
            use_command(owner, admitted, &target, id).await?,
            UseDisposition::Committed
        );
        let count = i64::try_from(id - 1)?;
        assert_eq!(snapshot(h, callback).await?, (count, count, count, count));
        target.expected_revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    }
    let after = snapshot(h, callback).await?;
    assert_eq!(
        use_command(owner, admitted, &target, 4).await?,
        UseDisposition::Rejected
    );
    assert_eq!(
        snapshot(h, callback).await?,
        after,
        "missing material leaves the epoch and grant untouched"
    );
    let replay = use_command(owner, admitted, &original, 2).await;
    assert!(
        replay
            .err()
            .is_some_and(|e| e.to_string().contains("CommandOutcomeExpired"))
    );
    assert_eq!(snapshot(h, callback).await?, after);
    let outputs:(String,i64)=sqlx::query_as("SELECT min(i.definition_production_key),sum(m.quantity)::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id)").fetch_one(&h.pool).await?;
    assert_eq!(outputs, ("oteryn:item.tibia.i21507".into(), 2));
    let remaining:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i16122'").fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0);
    let transactions:i64=sqlx::query_scalar("SELECT count(DISTINCT tx) FROM (SELECT created_xact_id::text tx FROM game_character_quest_receipts WHERE transition_key=$1 UNION ALL SELECT created_xact_id::text FROM game_quest_callback_consumptions WHERE transition_key=$1 UNION ALL SELECT created_xact_id::text FROM game_reward_claim_mint_receipts) proofs").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(
        transactions, 2,
        "each input, counter and MINT share their one SQL transaction"
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        2,
        "station remains at its native identity"
    );
    Ok(())
}
