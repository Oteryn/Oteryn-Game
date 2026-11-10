//! M03: native USE and STEP entrances consume live, fenced admission predicates.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::*;

#[test]
fn qm03_native_inferno_step_refuses_without_live_premium() -> TestResult {
    scenario("qm03/pits")
}

#[test]
fn qm03_native_inferno_step_commits_with_live_premium() -> TestResult {
    scenario("qm03/pits-premium")
}

#[test]
fn qm03_native_anniversary_use_requires_live_blessings_promotion_and_premium() -> TestResult {
    scenario("qm03/anniversary-premium")
}

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            callback(name)?
                .selector
                .position
                .ok_or("entrance position")?,
            false,
        )
    })
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    oteryn_gameplay_quests::quest::world_callbacks::load_for(state.catalogue())?
        .into_iter()
        .find(|c| {
            c.key
                == if name.contains("anniversary") {
                    "oteryn:quest-callback/qm03/anniversary"
                } else {
                    "oteryn:quest-callback/qm03/pits"
                }
        })
        .ok_or_else(|| "Inferno entrance callback".into())
}

pub(super) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop, native_scene};
    if name.contains("anniversary") {
        return native_scene(
            native,
            vec![
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z08-x126-y125.b3"
                    )),
                    region: (8, 126, 125),
                    bounds: (32428, 32163, 32434, 32169),
                    start: (32432, 32165),
                    expected: vec![ExpectedTile {
                        position: (32431, 32165),
                        ids: vec![355, 25096],
                        counted_entry: None,
                    }],
                },
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z08-x130-y121.b3"
                    )),
                    region: (8, 130, 121),
                    bounds: (33452, 31030, 33455, 31032),
                    start: (33453, 31030),
                    expected: vec![ExpectedTile {
                        position: (33453, 31030),
                        ids: vec![15534, 15620, 25100],
                        counted_entry: None,
                    }],
                },
            ],
            &[],
        );
    }
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z10-x128-y126.b3"
            )),
            region: (10, 128, 126),
            bounds: (32789, 32329, 32794, 32334),
            start: (32792, 32331),
            expected: vec![ExpectedTile {
                position: (32791, 32331),
                ids: vec![354],
                counted_entry: None,
            }],
        },
    )
}

async fn state(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_quest_tracks),(SELECT count(*) FROM game_character_quest_receipts)")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

async fn use_machine(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: &Callback,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    use oteryn_protocol_oteryn::world_object::*;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace native USE")?;
    let native = callback.selector.native.as_ref().ok_or("native selector")?;
    let map = owner.bundle_map().ok_or("native map")?;
    let key = oteryn_world_bundle::bundle::placement_key(
        native.position.floor,
        native.position.x,
        native.position.y,
        native.ordinal,
    )
    .ok_or("machine key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
        .map_err(debug)?,
    )
    .await?;
    decode_use_result(&reply).map_err(debug)
}

async fn anniversary(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let before = state(h).await?;
    assert_eq!(
        use_machine(owner, admitted, callback).await?,
        UseDisposition::Rejected
    );
    assert_eq!(state(h).await?, before);
    // Fixture setup represents prior purchases; no production mutation is bypassed by the command.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(91).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        use_machine(owner, admitted, callback).await?,
        UseDisposition::Rejected
    );
    assert_eq!(state(h).await?, before);
    let catalogue = crate::content::blessing_catalogue::BlessingCatalogue::load()?;
    let regular: Vec<_> = catalogue
        .blessings
        .iter()
        .filter(|b| b.storage_key != "oteryn:blessing.twist_of_fate")
        .collect();
    for (index, blessing) in regular.iter().enumerate() {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'qm03-fixture')")
            .bind(id(41).as_slice()).bind(&blessing.storage_key).execute(&mut *tx).await?;
        tx.commit().await?;
        if index + 1 < regular.len() {
            assert_eq!(
                use_machine(owner, admitted, callback).await?,
                UseDisposition::Rejected
            );
            assert_eq!(state(h).await?, before);
        }
    }
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    assert_eq!(
        use_machine(owner, admitted, callback).await?,
        UseDisposition::Rejected,
        "all blessings and Premium cannot replace promotion"
    );
    assert_eq!(state(h).await?, before);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(91).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        use_machine(owner, admitted, callback).await?,
        UseDisposition::Rejected
    );
    assert_eq!(state(h).await?, before);
    owner.premium_coordinator = premium;
    assert_eq!(
        use_machine(owner, admitted, callback).await?,
        UseDisposition::Committed
    );
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?;
    assert_eq!(
        position.position(),
        super::quest_world_callbacks::local(callback.teleport.ok_or("machine destination")?)
    );
    assert_eq!(state(h).await?, (before.0 + 1, before.1 + 1, before.2 + 1));
    eprintln!("M03_DISPATCH_PG_EXECUTED anniversary live predicates");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    name: &str,
) -> TestResult {
    // Arrange the level only; commands use the runtime role and production catalogue.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=80 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    if name.contains("anniversary") {
        return anniversary(owner, admitted, h, callback).await;
    }
    let before = state(h).await?;
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::West),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?;
    if name.ends_with("-premium") {
        assert_eq!(
            position.position(),
            super::quest_world_callbacks::local(callback.selector.position.ok_or("entrance")?)
        );
        let progress: i64 = sqlx::query_scalar(
            "SELECT value::bigint FROM game_character_quest_tracks WHERE track_key=$1",
        )
        .bind(&callback.storage[0].track)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(progress, 1);
        assert_eq!(state(h).await?, (before.0 + 1, before.1 + 1, before.2 + 1));
    } else {
        assert_eq!(
            position.position(),
            super::quest_world_callbacks::local(
                oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 32792,
                    y: 32331,
                    floor: -10
                }
            ),
            "refused STEP returns to its origin"
        );
        assert_eq!(
            state(h).await?,
            before,
            "refusal cannot advance progress or receipts"
        );
    }
    eprintln!("M03_DISPATCH_PG_EXECUTED {name}");
    Ok(())
}
