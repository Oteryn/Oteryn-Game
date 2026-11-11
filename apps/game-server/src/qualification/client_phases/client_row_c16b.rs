//! Client replay capture for SCENARIO_MATRIX C16b (Paralyze rune).
//!
//! A caster casts the real Paralyze rune on an independently admitted player. The paralysed
//! player then queries its own condition status; the raw non-empty reply (icon 14, positive
//! remaining time) is the captured payload. The consumer is `ConditionStatusPanel::finish` in
//! `crates/client-ui/src/features/0276-condition-status/state.rs`, projected by `panel_text` in
//! that feature's `view.rs`.

use super::*;
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};

const PARALYSED_ICON: u8 = 14;

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let result = row_complete(journey(clients, world)).await;
    match result.unwrap_or_else(|_| Err("C16b.client journey timeout".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=C16b.client status=PASS evidence=paralyze-condition-status-reply",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C16b.client status=FAIL evidence={error}"
        )),
    }
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let mut caster = connect_world(clients, 0x16).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        capture(&mut caster, clients, world).await
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = full_e2ef5::rune_cleanup(&mut caster, world).await;
    logout_result?;
    row_result
}

async fn capture(
    caster: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let book = native_spell_book()?;
    let spell = book
        .rune(3165)
        .ok_or("C16b Paralyze rune not in pinned spell book")?;
    let (reference, _) = full_e2ef5::acquire_rune(caster, world, 3165).await?;
    let mut target = connect_character(clients, 0x17, 0).await?;
    let row_result: TestResult<_> = async {
        let result = tokio::time::timeout(Duration::from_secs(120), async {
            let path = route(world, tile(target.world_spatial().actor_position)?, |at| {
                full_e2ef5::rune_corridor(world, at).is_some()
            })
            .ok_or("C16b no reachable non-PZ equal-ground target corridor")?;
            full_e2ef5::with_observer_liveness(caster, walk(&mut target, world, &path)).await?;
            let cells =
                full_e2ef5::rune_corridor(world, tile(target.world_spatial().actor_position)?)
                    .ok_or("C16b target corridor vanished")?;
            let approach = route(world, tile(caster.world_spatial().actor_position)?, |at| {
                at.floor == cells[0].floor
                    && !cells.contains(&at)
                    && at.x.abs_diff(cells[0].x).max(at.y.abs_diff(cells[0].y)) <= 3
                    && world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| !tile.protection_zone())
            })
            .ok_or("C16b caster cannot approach the independent target")?;
            full_e2ef5::with_observer_liveness(&mut target, walk(caster, world, &approach)).await?;
            caster.service_liveness(Duration::from_millis(50)).await?;
            let entity = target
                .world_entities()
                .and_then(|entities| {
                    entities
                        .iter()
                        .find(|entity| &entity.entity.identity == entities.own_identity())
                })
                .map(|entity| entity.entity)
                .ok_or("C16b target own native entity absent")?;
            if query(&mut target)
                .await?
                .1
                .conditions
                .iter()
                .any(|c| c.icon == PARALYSED_ICON)
            {
                return Err("C16b target was paralysed before the cast".into());
            }
            full_e2ef5::cast_rune(caster, spell, reference, Some(entity), None).await?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
            let (payload, status) = loop {
                let (payload, status) = query(&mut target).await?;
                if status.conditions.iter().any(|c| c.icon == PARALYSED_ICON) {
                    break (payload, status);
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err("C16b target never reported the Paralysed condition".into());
                }
                full_e2ef5::with_observer_liveness(&mut target, async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok(())
                })
                .await?;
            };
            let condition = status
                .conditions
                .iter()
                .find(|c| c.icon == PARALYSED_ICON)
                .ok_or("C16b Paralysed condition absent")?;
            if condition.remaining_ms <= 0 {
                return Err(format!(
                    "C16b Paralysed has no positive duration: {}",
                    condition.remaining_ms
                )
                .into());
            }
            let seconds =
                condition.remaining_ms / 1_000 + i64::from(condition.remaining_ms % 1_000 != 0);
            if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let path = Path::new(&path);
                std::fs::write(path.join("c16b-condition.bin"), &payload)?;
                std::fs::write(
                    path.join("c16b-expected.txt"),
                    format!("≪ Paralysed: {seconds}s\r\n"),
                )?;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await;
        let cleanup = full_e2ef5::with_observer_liveness(
            caster,
            full_e2ef5::rune_cleanup(&mut target, world),
        )
        .await;
        result.map_err(|_| "C16b journey exceeded 120s")??;
        cleanup
    }
    .await;
    row_result
}

async fn query(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<(Vec<u8>, ConditionStatus)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("C16b condition query refused".into());
    }
    let status = ConditionStatus::decode(&reply.payload).ok_or("C16b malformed condition reply")?;
    Ok((reply.payload, status))
}
