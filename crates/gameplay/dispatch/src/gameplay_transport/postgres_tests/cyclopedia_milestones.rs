use super::cyclopedia_fixtures::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::cyclopedia::{MapAction, Point};
use crate::durability::character_cyclopedia::{CyclopediaPremiumGate, MapRequest};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_revision_sequencer::{CharacterRevisionSequencer, RevisionSlot};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::{DurabilityError, character_authority::ReconciledCharacterAuthority};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_protocol_oteryn::cyclopedia::decode_retained_result;

const BUNDLE: [u8; 32] = [91; 32];
const BASE: &str = "oteryn:outfit.discoverer/quest-grant/0";
const ADDON_ONE: &str = "oteryn:outfit.discoverer/quest-grant/1";
const ADDON_TWO: &str = "oteryn:outfit.discoverer/quest-grant/2";
const WIDELY: &str = "oteryn:achievement/widely_travelled";
const MEASURING: &str = "oteryn:achievement/measuring_the_world";

struct NoPremium;
impl CyclopediaPremiumGate for NoPremium {
    async fn premium(
        &self,
        _tx: &mut Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        Ok(false)
    }
}

#[test]
fn discovery_milestones_grant_outfit_addons_and_achievements() -> TestResult {
    execute(
        false,
        "discovery_milestones_grant_outfit_addons_and_achievements",
    )
}

#[test]
fn discovery_milestones_replay_and_relogin_keep_one_grant() -> TestResult {
    execute(
        true,
        "discovery_milestones_replay_and_relogin_keep_one_grant",
    )
}

fn execute(relogin: bool, name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(relogin, name).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "discovery milestones PostgreSQL scenario panicked")??;
    eprintln!(
        "POSTGRES_EXECUTED {name} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn discovery_milestone_reconciliation_keeps_existing_progress_and_replies() -> TestResult {
    reconcile_cases(
        [MapAction::Query, MapAction::Find],
        &[],
        "discovery_milestone_reconciliation_keeps_existing_progress_and_replies",
    )
}

#[test]
fn discovery_milestone_map_percent_counts_only_required_subareas() -> TestResult {
    reconcile_cases(
        [MapAction::Query],
        &[(1, 12)],
        "discovery_milestone_map_percent_counts_only_required_subareas",
    )
}

#[test]
fn discovery_milestone_shuffle_reconciles_grandfathered_progress() -> TestResult {
    reconcile_cases(
        [MapAction::Shuffle],
        &[],
        "discovery_milestone_shuffle_reconciles_grandfathered_progress",
    )
}

fn reconcile_cases<const N: usize>(
    actions: [MapAction; N],
    extra_subareas: &'static [(u32, u32)],
    name: &'static str,
) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || reconcile_scenario(actions, extra_subareas, name).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "discovery reconciliation PostgreSQL scenario panicked")??;
    eprintln!(
        "POSTGRES_EXECUTED {name} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn reconcile_scenario(
    actions: impl IntoIterator<Item = MapAction>,
    extra_subareas: &[(u32, u32)],
    name: &str,
) -> TestResult {
    run(async |admin| {
        for action in actions {
            let progress = if action == MapAction::Query {
                serde_json::json!({"legacy": {"discovered": false, "exploration": null},
                    "subareas": {"11": {"discovered": true, "exploration": null}}})
            } else {
                serde_json::json!({"discovered": true, "exploration": null})
            };
            let h = Harness::create_with_admission_seed(
                admin.clone(),
                "discovery_reconcile",
                true,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            support::seed(&h, BUNDLE).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let session = GameSessionId::decode(&id(50)).map_err(debug)?;
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                &owner, session, false,
            )
            .await
            .ok_or("current discovery fence")?;
            let sequencer = CharacterRevisionSequencer::new();
            let mut slot = sequencer.acquire(fence.character_id).await;
            let roster = [(1, 11)];
            let start = request(session, 1, &roster, 1, 11, MapAction::Start, 0)?;
            let retained = call(&mut slot, &h, &authority, fence, start.clone()).await?;
            let revision = decode_retained_result(&retained).map_err(debug)?.revision;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_cyclopedia SET progress=$1::text::jsonb")
                .bind(progress.to_string())
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            assert_rewards(&h, &[], &[]).await?;
            assert_eq!(
                call(&mut slot, &h, &authority, fence, start.clone()).await?,
                retained
            );
            eprintln!("POSTGRES_STARTED {name} {action:?}");

            if action == MapAction::Shuffle {
                let shuffle = request(session, 2, &roster, 1, 11, action, 0)?;
                let reply = call(&mut slot, &h, &authority, fence, shuffle.clone()).await?;
                let result = decode_retained_result(&reply).map_err(debug)?;
                assert_eq!(result.code, 0);
                assert!(!result.discovered);
                assert_eq!(result.area_percent, 0);
                assert_eq!(result.revision, revision + 1);
                assert_rewards(&h, &[BASE, ADDON_ONE, ADDON_TWO], &[MEASURING, WIDELY]).await?;
                assert_eq!(call(&mut slot, &h, &authority, fence, shuffle).await?, reply);
                assert_rewards(&h, &[BASE, ADDON_ONE, ADDON_TWO], &[MEASURING, WIDELY]).await?;
            } else {
                for command in [2, 3] {
                    let mut read = request(session, command, &roster, 1, 11, action, 20)?;
                    read.subareas.extend_from_slice(extra_subareas);
                    let reply = call(&mut slot, &h, &authority, fence, read).await?;
                    let result = decode_retained_result(&reply).map_err(debug)?;
                    assert!(result.discovered);
                    assert_eq!(result.area_percent, 100);
                    assert_eq!(result.revision, revision);
                    assert_rewards(&h, &[BASE, ADDON_ONE, ADDON_TWO], &[MEASURING, WIDELY]).await?;
                }
            }
            assert_eq!(
                call(&mut slot, &h, &authority, fence, start).await?,
                retained
            );
            let after: String =
                sqlx::query_scalar("SELECT progress::text FROM game_character_cyclopedia")
                    .fetch_one(&h.pool)
                    .await?;
            let after: serde_json::Value = serde_json::from_str(&after)?;
            if action == MapAction::Shuffle {
                assert_eq!(after["subareas"]["11"]["discovered"], false);
                assert!(after["subareas"]["11"]["exploration"].is_object());
            } else {
                assert_eq!(after, progress);
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
        }
        Ok(())
    })
}

fn scenario(relogin: bool, name: &'static str) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "discovery_m12", true, false).await?;
        eprintln!("POSTGRES_STARTED {name}");
        prepare_current_session(&h).await?;
        support::seed(&h, BUNDLE).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let fence =
            crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
                .await
                .ok_or("current discovery fence")?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence.character_id).await;

        let subareas = if relogin {
            vec![(1, 11)]
        } else {
            vec![(1, 11), (1, 12), (2, 21), (3, 31), (4, 41)]
        };
        let mut number = 0;
        let mut completion = None;
        for &(area, subarea) in &subareas {
            let mut next = |action, x| -> TestResult<MapRequest> {
                number += 1;
                request(session, number, &subareas, area, subarea, action, x)
            };
            let start = next(MapAction::Start, 0)?;
            let reply = call(&mut slot, &h, &authority, fence, start).await?;
            assert_eq!(decode_retained_result(&reply).map_err(debug)?.code, 0);
            for (index, x) in (20..27).enumerate() {
                let find = next(MapAction::Find, x)?;
                let reply = call(&mut slot, &h, &authority, fence, find.clone()).await?;
                let result = decode_retained_result(&reply).map_err(debug)?;
                assert_eq!(result.code, 0);
                assert_eq!(result.found, (index + 1) as u8);
                assert_eq!(result.discovered, index == 6);
                if index < 6 && subarea == 11 {
                    assert_rewards(&h, &[], &[]).await?;
                }
                if index == 6 {
                    completion = Some((find, reply));
                }
            }
            match (relogin, subarea) {
                (false, 11) => assert_rewards(&h, &[], &[]).await?,
                (false, 12) => assert_rewards(&h, &[BASE], &[WIDELY]).await?,
                (false, 21 | 31) => assert_rewards(&h, &[BASE, ADDON_ONE], &[WIDELY]).await?,
                _ => {
                    assert_rewards(&h, &[BASE, ADDON_ONE, ADDON_TWO], &[MEASURING, WIDELY]).await?
                }
            }
        }
        if relogin {
            let (find, committed) = completion.ok_or("completion receipt")?;
            let replayed = call(&mut slot, &h, &authority, fence, find).await?;
            assert_eq!(replayed, committed);
            drop(slot);
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let old = store.current_session(session).await?;
            assert!(matches!(
                store
                    .release_logout_session(
                        session,
                        &crate::gameplay_transport::canonical_uuid(&id(40)),
                        old.current_transport().ok_or("authenticated transport")?,
                    )
                    .await?,
                ExpiredLossReleaseV1::Released { .. }
            ));
            let successor = super::full_world_support::relogin(&h).await?;
            assert_ne!(successor, session);
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                &owner, successor, false,
            )
            .await
            .ok_or("successor discovery fence")?;
            let mut slot = sequencer.acquire(fence.character_id).await;
            let find = request(successor, 1, &subareas, 1, 11, MapAction::Find, 26)?;
            let result = call(&mut slot, &h, &authority, fence, find).await?;
            let result = decode_retained_result(&result).map_err(debug)?;
            assert!(result.discovered);
            assert_eq!(result.area_percent, 100);
            assert_rewards(&h, &[BASE, ADDON_ONE, ADDON_TWO], &[MEASURING, WIDELY]).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn request(
    session: GameSessionId,
    number: u8,
    subareas: &[(u32, u32)],
    area: u32,
    subarea: u32,
    action: MapAction,
    x: i32,
) -> TestResult<MapRequest> {
    let occurrence = crate::gameplay_transport::npc_trade_command::derived(
        b"discovery-milestone-fixture",
        session,
        u64::from(number),
    );
    let point = |x| Point { x, y: 0, floor: -7 };
    Ok(MapRequest {
        command: CommandRef::new(session, CommandId::new(u64::from(number)).map_err(debug)?),
        action,
        binding: [number; 32],
        occurrence,
        bundle: BUNDLE,
        epoch: 0,
        area,
        legacy_area: 1,
        subarea: Some(subarea),
        subareas: subareas.to_vec(),
        milestone_subareas: subareas.into(),
        pool: (20..27).map(point).collect(),
        at: point(x),
        fee: None,
        transaction: occurrence,
        event: occurrence,
        now: 1,
    })
}

async fn call(
    slot: &mut RevisionSlot,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: MapRequest,
) -> TestResult<Vec<u8>> {
    slot.cyclopedia(&h.root, authority, &h.node, fence, request, &NoPremium)
        .await
        .map_err(debug)
}

async fn assert_rewards(h: &Harness, outfits: &[&str], achievements: &[&str]) -> TestResult {
    let (unlocks, unlock_receipts, facts, grant_requests): (Vec<String>, Vec<String>, Vec<String>, Vec<String>) =
        sqlx::query_as("SELECT \
            ARRAY(SELECT unlock_key FROM game_account_appearance_unlocks WHERE account_id=$1 ORDER BY unlock_key), \
            ARRAY(SELECT unlock_key FROM game_account_appearance_unlock_receipts WHERE account_id=$1 ORDER BY unlock_key), \
            ARRAY(SELECT achievement_key FROM game_account_achievements WHERE account_id=$1 ORDER BY achievement_key), \
            ARRAY(SELECT achievement_key FROM game_account_achievement_grant_requests WHERE account_id=$1 ORDER BY achievement_key)")
            .bind(sqlx::types::Uuid::from_bytes(id(40)))
            .fetch_one(&h.pool).await?;
    assert_eq!(unlocks, outfits, "Discoverer entitlements");
    assert_eq!(
        unlock_receipts, outfits,
        "one receipt per Discoverer entitlement"
    );
    assert_eq!(facts, achievements, "account achievement facts");
    assert_eq!(grant_requests, achievements, "one request per achievement");
    Ok(())
}

#[test]
fn discovery_milestone_roster_is_playable_on_the_world_pin() -> TestResult {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("admitted native content")?;
    let world = super::qc_pinned_world::world(native)?;
    let table = &world.base().subareas;
    let sampling = crate::domain::cyclopedia::BasicArea::embedded().ok_or("sampling catalogue")?;
    let door_at = |pos| world.facts().doors().at(pos);
    let roster =
        crate::gameplay_transport::cyclopedia::discoverable_subareas(table, &sampling, door_at);
    let playable =
        crate::gameplay_transport::cyclopedia::discoverable_subareas(table, &sampling, |pos| {
            door_at(pos) || !world.enterable(pos)
        });
    assert_eq!(
        roster, playable,
        "each required subarea must support the native POI pool"
    );
    let mains = roster
        .iter()
        .map(|&(main, _)| main)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        mains.into_iter().collect::<Vec<_>>(),
        [
            1, 2, 3, 4, 7, 8, 9, 11, 14, 16, 17, 19, 20, 22, 23, 24, 25, 27, 355, 434
        ],
    );
    Ok(())
}
