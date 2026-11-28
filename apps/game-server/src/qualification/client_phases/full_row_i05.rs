//! I05 full-row journey: general timed items, equipment charges and transforms.
//!
//! Concrete player scenario, derived from matrix row I05, Global Energy Ring behavior,
//! and `timed_item_clock` (the live server owner):
//! 1. Admit the node-boot character and send TIMED_ITEM Inspect for an unknown
//!    server handle; expect a Rejected command envelope carrying typed Stale
//!    and no inventory delta (the refusal path must not mutate custody).
//! 2. Grant a dedicated Knight one inactive Energy Ring as fixture input, then
//!    confirm its carried form and the empty ring slot on fresh admission.
//! 3. Send ITEM_MOVE for that carried ring into the ring equipment slot; expect
//!    Moved plus an inventory delta whose equipped view is transformed to active
//!    energy ring 3088.
//! 4. Send TIMED_ITEM Inspect for the equipped handle; expect Ok, revision zero and
//!    a positive remaining budget no greater than 600,000 ms. Send Use
//!    with an impossible expected revision; expect Stale and no equipment
//!    delta, then inspect with the reported revision and expect Ok.
//! 5. Log out, obtain a fresh admission, and re-check durable equipped custody: the
//!    active 3088 view remains in the ring slot and its inspected remaining budget
//!    is positive and no greater than the pre-logout budget.
//! 6. Unequip the ring, require its base item view and verify that its timer pauses.
use super::*;
use oteryn_protocol_oteryn::{item_view as items, timed_item as timed};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const ENERGY_RING: u32 = 3051;
const ACTIVE_ENERGY_RING: u32 = 3088;

struct Journey {
    step: usize,
    started: std::time::Instant,
}

impl Journey {
    fn new() -> Self {
        Self {
            step: 0,
            started: std::time::Instant::now(),
        }
    }
    fn passed(&mut self, step: usize, observation: &str) {
        self.step = step;
        evidence(&format!(
            "I05_CHECK step={step} PASS observed={observation}"
        ));
    }
    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("I05_GAP step={} observed={error}", self.step + 1));
        }
        evidence(&format!(
            "e2e_all scenario=I05.full status={} seconds={:.3}",
            if result.is_ok() && self.step == 6 {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Journey::new();
    let result = verdict::indexed_timeout(
        Duration::from_secs(420),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("journey-timeout:no-server-reply-within-420s".into()),
    });
}

async fn timed_command(
    client: &mut oteryn_dev_client::DevClientSession,
    action: timed::TimedAction,
    target: items::ItemHandle,
    expected_revision: u64,
) -> TestResult<timed::TimedItemResult> {
    let payload = timed::encode_timed_item_intent(timed::TimedItemIntent {
        action,
        target,
        expected_revision,
        npc: None,
    })
    .map_err(|error| format!("timed-intent-encode:{error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            timed::CAPABILITY_TIMED_ITEM_V1,
            timed::COMMAND_TYPE_TIMED_ITEM_INTENT,
            &payload,
        )
        .await?;
    let result = timed::decode_timed_item_result(&reply.payload)
        .map_err(|error| format!("timed-result-decode:{error:?}:envelope={reply:?}"))?;
    let expected = if result.disposition == timed::TimedDisposition::Ok {
        CommandStatus::Accepted
    } else {
        CommandStatus::Rejected
    };
    if reply.status != expected {
        return Err(format!(
            "timed-command-status-mismatch:expected={expected:?}:envelope={reply:?}"
        )
        .into());
    }
    if action == timed::TimedAction::Inspect && target.get() == u64::MAX {
        evidence(&format!("I05_UNKNOWN_HANDLE envelope={reply:?}"));
    }
    Ok(result)
}

fn reference(id: u32) -> TestResult<NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn bag_handles(
    client: &oteryn_dev_client::DevClientSession,
    key: NonZeroU32,
) -> TestResult<BTreeSet<items::ItemHandle>> {
    let inventory = client
        .inventory()
        .ok_or("step=ring-custody missing=inventory")?;
    Ok(inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == key)
        .map(|entry| entry.handle)
        .collect())
}

fn item_count(client: &oteryn_dev_client::DevClientSession, key: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("item-count-inventory-missing")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == key)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn wait_for_item(
    client: &mut oteryn_dev_client::DevClientSession,
    key: NonZeroU32,
    previous_count: u64,
    previous_handles: &BTreeSet<items::ItemHandle>,
) -> TestResult<items::ItemHandle> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if previous_count.checked_add(1) == Some(item_count(client, key)?)
            && let Some(handle) =
                added_handle(bag_handles(client, key)?.into_iter(), previous_handles)
        {
            return Ok(handle);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "purchase-inventory-delta-timeout:item={key}:previous-count={previous_count}:observed-count={}",
                item_count(client, key)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

fn equipped_ring(
    client: &oteryn_dev_client::DevClientSession,
    active: NonZeroU32,
) -> TestResult<items::ItemHandle> {
    client
        .inventory()
        .and_then(|view| {
            view.equipment.iter().find(|entry| {
                entry.slot == items::EquipmentSlot::Ring && entry.item.item_definition_ref == active
            })
        })
        .map(|entry| entry.item.handle)
        .ok_or("active-energy-ring-equipment-delta-missing".into())
}

const STALE_PROBE_ACTION: timed::TimedAction = timed::TimedAction::Use;

fn has_paused_budget(
    first: &timed::TimedItemResult,
    later: &timed::TimedItemResult,
    previous_ms: u64,
) -> bool {
    first.disposition == timed::TimedDisposition::Ok
        && first
            .remaining_ms
            .is_some_and(|ms| (1..=previous_ms).contains(&ms))
        && first == later
}

async fn journey(row: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I05_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let mut client = connect_character(clients, 0xb0, 0).await?;
    let result: TestResult = async {
        if !client
            .selected_capabilities()
            .contains(&timed::CAPABILITY_TIMED_ITEM_V1)
        {
            return Err("timed-item-capability-not-offered".into());
        }
        let before = client
            .inventory()
            .cloned()
            .ok_or("refusal-inventory-missing")?;
        let unknown = items::ItemHandle::new(u64::MAX).ok_or("invalid-test-handle")?;
        let refused = timed_command(&mut client, timed::TimedAction::Inspect, unknown, 0).await?;
        if refused != timed::TimedItemResult::refused(timed::TimedDisposition::Stale)
            || client.inventory() != Some(&before)
        {
            return Err(format!("unknown-handle-not-rollback-safe:reply={refused:?}").into());
        }
        row.passed(1, "rejected-envelope+typed-stale+inventory-unchanged");

        let base = reference(ENERGY_RING)?;
        if item_count(&client, base)? != 0 {
            return Err("i05-base-ring-preexists-fixture".into());
        }
        character_prep::declare_seeded(
            "I05.full",
            "one carried inactive Energy Ring",
            "timed-item lifecycle",
        );
        character_prep::grant_item(&client, ENERGY_RING, 1).await?;
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("i05-fixture-logout-refused".into());
        }
        client = connect_character(clients, 0xaf, 0).await?;
        let handles = bag_handles(&client, base)?;
        if item_count(&client, base)? != 1
            || client.inventory().is_none_or(|inventory| {
                inventory
                    .equipment
                    .iter()
                    .any(|entry| entry.slot == items::EquipmentSlot::Ring)
            })
        {
            return Err("i05-fixture-durable-prestate-mismatch".into());
        }
        let carried = added_handle(handles.into_iter(), &BTreeSet::new())
            .ok_or("i05-fixture-expected-one-handle")?;
        row.passed(2, "fresh-admission+one-inactive-ring+empty-ring-slot");

        let delta = super::client_row_i05::move_ring(
            client.session_mut(),
            carried,
            items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
        )
        .await?;
        let active = reference(ACTIVE_ENERGY_RING)?;
        let handle = equipped_ring(&client, active)?;
        if !delta.value.equipment.iter().any(|entry| {
            entry.slot == items::EquipmentSlot::Ring
                && entry.item.handle == handle
                && entry.item.item_definition_ref == active
                && entry.item.count.get() == 1
        }) {
            return Err("energy-ring-first-equip-delta-mismatch".into());
        }
        row.passed(3, "item-move-moved+3051-to-3088-equipment-delta");

        let inspected = timed_command(&mut client, timed::TimedAction::Inspect, handle, 0).await?;
        let remaining = inspected
            .remaining_ms
            .ok_or("inspect-missing-remaining-ms")?;
        if inspected.disposition != timed::TimedDisposition::Ok
            || inspected.revision != 0
            || remaining == 0
            || remaining > 600_000
        {
            return Err(format!("equipped-inspect-invalid:reply={inspected:?}").into());
        }
        let equipment = client
            .inventory()
            .map(|view| view.equipment.clone())
            .ok_or("stale-equipment-missing")?;
        let stale = timed_command(&mut client, STALE_PROBE_ACTION, handle, u64::MAX).await?;
        if stale.disposition != timed::TimedDisposition::Stale
            || client
                .inventory()
                .is_none_or(|view| view.equipment != equipment)
        {
            return Err(format!("wrong-revision-not-stale-or-mutated:reply={stale:?}").into());
        }
        let current = timed_command(
            &mut client,
            timed::TimedAction::Inspect,
            handle,
            inspected.revision,
        )
        .await?;
        if current.disposition != timed::TimedDisposition::Ok {
            return Err(format!("current-revision-inspect-refused:reply={current:?}").into());
        }
        row.passed(
            4,
            "inspect-ok+revision-zero+bounded-budget+stale-refusal+no-delta+current-revision-ok",
        );

        let old_session = client.join_snapshot().game_session_id;
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("logout-refused-before-persistence-check".into());
        }
        client = connect_character(clients, 0xb1, 0).await?;
        if client.join_snapshot().game_session_id == old_session {
            return Err("fresh-admission-reused-session".into());
        }
        let durable_handle = equipped_ring(&client, active)?;
        let durable = timed_command(
            &mut client,
            timed::TimedAction::Inspect,
            durable_handle,
            current.revision,
        )
        .await?;
        if durable.disposition != timed::TimedDisposition::Ok
            || durable
                .remaining_ms
                .is_none_or(|value| value == 0 || value > remaining)
        {
            return Err(format!(
                "fresh-admission-timer-not-durable:before={remaining}:reply={durable:?}"
            )
            .into());
        }
        row.passed(5, "fresh-session+active-transform+nonincreasing-timer");
        let last_equipped_ms = durable
            .remaining_ms
            .ok_or("step=unequip missing=last-equipped-budget")?;
        let base = reference(ENERGY_RING)?;
        let previous_count = item_count(&client, base)?;
        let previous_handles = bag_handles(&client, base)?;
        let moved = client
            .move_item(&items::ItemMoveIntent {
                source: durable_handle,
                destination: items::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != items::ItemMoveOutcome::Moved {
            return Err(format!("step=unequip observed={moved:?}").into());
        }
        let paused_handle =
            wait_for_item(&mut client, base, previous_count, &previous_handles).await?;
        if equipped_ring(&client, active).is_ok() {
            return Err("step=unequip observed=active-ring-still-equipped".into());
        }
        let paused =
            timed_command(&mut client, timed::TimedAction::Inspect, paused_handle, 0).await?;
        client.service_liveness(Duration::from_secs(2)).await?;
        let later = timed_command(
            &mut client,
            timed::TimedAction::Inspect,
            paused_handle,
            paused.revision,
        )
        .await?;
        if !has_paused_budget(&paused, &later, last_equipped_ms) {
            return Err(
                format!("step=unequip-pause observed=initial:{paused:?}/later:{later:?}").into(),
            );
        }
        row.passed(6, "unequip-transformed-to-base+paused-duration-stable");
        Ok(())
    }
    .await;
    result?;
    let release = async {
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("final-logout-refused".into());
        }
        Ok(())
    }
    .await;
    release
}

fn added_handle(
    handles: impl Iterator<Item = items::ItemHandle>,
    previous: &BTreeSet<items::ItemHandle>,
) -> Option<items::ItemHandle> {
    let mut added = handles.filter(|handle| !previous.contains(handle));
    let first = added.next()?;
    added.next().is_none().then_some(first)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_second_unequip_identifies_the_added_ring() -> TestResult {
        let old = items::ItemHandle::new(1).ok_or("old handle")?;
        let moved = items::ItemHandle::new(2).ok_or("moved handle")?;
        let previous = BTreeSet::from([old]);
        assert_eq!(
            added_handle([old, moved].into_iter(), &previous),
            Some(moved)
        );
        assert_eq!(added_handle([old].into_iter(), &previous), None);
        assert_eq!(
            added_handle([old, moved, moved].into_iter(), &previous),
            None
        );
        Ok(())
    }

    #[test]
    fn review_repair_stale_probe_uses_revision_checked_operation() {
        assert_eq!(STALE_PROBE_ACTION, timed::TimedAction::Use);
    }

    #[test]
    fn paused_ring_rejects_countdown_and_reset() {
        let paused = timed::TimedItemResult {
            remaining_ms: Some(50),
            ..timed::TimedItemResult::refused(timed::TimedDisposition::Ok)
        };
        let countdown = timed::TimedItemResult {
            remaining_ms: Some(49),
            ..paused
        };
        assert!(has_paused_budget(&paused, &paused, 60));
        assert!(!has_paused_budget(&paused, &countdown, 60));
        assert!(!has_paused_budget(&paused, &paused, 40));
    }

    #[test]
    fn required_item_ids_are_fixed() {
        assert_eq!((ENERGY_RING, ACTIVE_ENERGY_RING), (3051, 3088));
    }
}
