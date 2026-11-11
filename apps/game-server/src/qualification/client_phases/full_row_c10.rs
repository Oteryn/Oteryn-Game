//! C10 concrete player scenario, derived from the matrix/re-audit and the
//! `bed_regen`/`spell_periodic` server owners:
//! 1. Admit the provisioned character and cast an available soul-costing learned
//!    spell; require a `Cast` reply and the exact soul debit in an actor-vitals delta.
//! 2. Immediately repeat that player command; require the cooldown refusal and no
//!    soul delta (the negative path must not charge the player).
//! 3. Walk to a native rat spawn and kill it through ordinary combat, thereby
//!    opening the server's kill-funded soul window; require earned XP and then the
//!    periodic `+1` soul actor-vitals delta on the real server clock.
//! 4. Send BED_SLEEP_INTENT with an ordinary carried-item handle; require the
//!    `NotPossible` refusal and unchanged vitals/custody, proving a non-bed cannot
//!    enter saved rest.
//! 5. Find a visible native bed through the admitted item view, walk within reach,
//!    consume ordinary acquired food if needed, and send BED_SLEEP_INTENT for that
//!    bed; require the offline-training reply/session close.
//! 6. Freshly admit the character after saved rest and require a new game session,
//!    retained soul, and nondecreasing saved health/mana (food-funded rest is
//!    settled by the server owner exactly once).
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_catalog as spell_wire;
use oteryn_protocol_oteryn::bed::COMMAND_TYPE_BED_SLEEP_INTENT;
use oteryn_protocol_oteryn::item_view::{ItemHandle, encode_item_target};
use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroU64};

const STEPS: &[&str] = &[
    "soul-debit",
    "cooldown-refusal",
    "kill-window-regeneration",
    "non-bed-refusal",
    "bed-sleep",
    "fresh-admission-saved-rest",
];

struct Row {
    passed: BTreeSet<&'static str>,
    invalid: bool,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            invalid: false,
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if !STEPS.contains(&step) || !self.passed.insert(step) {
            self.invalid = true;
        } else {
            evidence(&format!("C10_CHECK {step} PASS observed={observed:?}"));
        }
    }
    fn finish(self, result: TestResult) {
        for step in STEPS {
            if !self.passed.contains(step) {
                evidence(&format!("C10_CHECK {step} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("C10_GAP {error}"));
        }
        evidence(&format!(
            "e2e_all scenario=C10.full status={} seconds={:.3}",
            if result.is_ok() && !self.invalid && STEPS.iter().all(|s| self.passed.contains(s)) {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete(journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| {
            Err("step deadline: C10 journey timed out waiting for a server reply/delta".into())
        });
    row.finish(result);
}

fn bed_payload(handle: ItemHandle, skill: u8) -> Vec<u8> {
    let target = encode_item_target(handle);
    let mut payload = Vec::with_capacity(target.len() + 4);
    payload.push(0x0a);
    payload.push(u8::try_from(target.len()).unwrap_or(0));
    payload.extend(target);
    payload.extend([0x10, skill]);
    payload
}

async fn bed_command(
    client: &mut DevClientSession,
    handle: ItemHandle,
) -> TestResult<UseDisposition> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_WORLD_INTERACTION_V1,
            COMMAND_TYPE_BED_SLEEP_INTENT,
            &bed_payload(handle, 6),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step bed command: transport status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into());
    }
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload).map_err(|error| {
        format!(
            "step bed command: malformed reply={error:?} bytes={:?}",
            reply.payload
        )
        .into()
    })
}

async fn soul_spell(client: &mut DevClientSession) -> TestResult<spell_wire::SpellCatalogEntry> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(spell_wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step soul debit: admitted spell catalogue missing")?;
    spell_wire::decode_catalog(&snapshot.payload)
        .map_err(|e| format!("step soul debit: catalogue reply={e:?}"))?
        .into_iter()
        .find(|spell| spell.available && spell.rune_item.is_none() && spell.soul > 0)
        .ok_or_else(|| {
            "step soul debit: no available player-earned soul spell in server catalogue".into()
        })
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xca).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let spell = soul_spell(&mut client).await?;
    let before = client.actor_vitals().copied().ok_or("step soul debit: vitals absent")?;
    let cast = client.cast_spell(NonZeroU32::new(spell.cast_index).ok_or("step soul debit: zero cast index")?, SpellTarget::None, false).await?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("step soul debit: server reply={cast:?}").into());
    }
    let after = wait_soul(&mut client, before.soul, |soul| soul == before.soul.saturating_sub(spell.soul), "soul debit", Duration::from_secs(5)).await?;
    row.pass("soul-debit", (before.soul, after));

    let held = client.inventory().cloned();
    let refused = client.cast_spell(NonZeroU32::new(spell.cast_index).ok_or("step cooldown refusal: zero index")?, SpellTarget::None, false).await?;
    if refused.disposition != SpellCastDisposition::CoolingDown
        || client.actor_vitals().is_none_or(|v| v.soul != after)
        || client.inventory().cloned() != held
    {
        return Err(format!("step cooldown refusal: reply={refused:?} vitals={:?}", client.actor_vitals()).into());
    }
    row.pass("cooldown-refusal", refused.disposition);

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let xp_before = progress(&mut client).await?.0;
    client_hunt::hunt(&mut client).await.map_err(|e| format!("step kill window: combat reply={e}"))?;
    let xp_after = progress(&mut client).await?.0;
    if xp_after <= xp_before {
        return Err(format!("step kill window: no XP delta {xp_before}->{xp_after}").into());
    }
    let regenerated = wait_soul(&mut client, after, |soul| soul == after + 1, "kill-window regeneration", Duration::from_secs(130)).await?;
    row.pass("kill-window-regeneration", (xp_before, xp_after, after, regenerated));

    let carried = client.inventory().and_then(|i| i.main_backpack).map(|i| i.handle)
        .unwrap_or(NonZeroU64::MIN);
    let vitals = client.actor_vitals().copied().ok_or("step non-bed refusal: vitals absent")?;
    let custody = client.inventory().cloned();
    let refusal = bed_command(&mut client, carried).await?;
    if refusal != UseDisposition::NotPossible || client.actor_vitals() != Some(&vitals) || client.inventory().cloned() != custody {
        return Err(format!("step non-bed refusal: reply={refusal:?} vitals={:?}", client.actor_vitals()).into());
    }
    row.pass("non-bed-refusal", refusal);

    // The production spatial item view exposes dynamic ground items, while native
    // house beds must likewise arrive with a session-local handle. Exercise every
    // visible map-item candidate; only OfflineTraining is a successful bed reply.
    let handles: Vec<_> = client.world_entities().into_iter().flat_map(|entities| entities.iter())
        .filter_map(|entity| match entity.detail {
            oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Object { item_handle: Some(handle), .. } => Some(handle),
            _ => None,
        }).collect();
    if handles.is_empty() {
        return Err("step bed sleep: actual native item-view observation contained no bed handle; server needs an admitted reachable house-bed item in the node-boot world".into());
    }
    for handle in handles {
        let reply = bed_command(&mut client, handle).await?;
        if reply == UseDisposition::OfflineTraining {
            row.pass("bed-sleep", (handle, reply));
            return fresh_admission(row, clients, client.join_snapshot().game_session_id, regenerated, vitals).await;
        }
    }
    Err("step bed sleep: all actual visible item handles returned non-bed refusal; server needs a reachable ACL-authorized bed in node-boot content".into())

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

async fn wait_soul(
    client: &mut DevClientSession,
    from: u32,
    wanted: impl Fn(u32) -> bool,
    step: &str,
    duration: Duration,
) -> TestResult<u32> {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                if delta.value.soul < from && step.contains("regeneration") {
                    return Err(format!(
                        "step {step}: unexpected soul delta {from}->{}",
                        delta.value.soul
                    )
                    .into());
                }
                if wanted(delta.value.soul) {
                    return Ok(delta.value.soul);
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step {step}: timeout; last vitals={:?}",
                client.actor_vitals()
            )
            .into());
        }
    }
}

async fn progress(client: &mut DevClientSession) -> TestResult<(u64, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload.len() != 42 || reply.payload[0] != 1
    {
        return Err(format!("step kill window: progression reply={reply:?}").into());
    }
    Ok((
        u64::from_le_bytes(reply.payload[1..9].try_into()?),
        u32::from_le_bytes(reply.payload[9..13].try_into()?),
    ))
}

async fn fresh_admission(
    row: &mut Row,
    clients: &SeamClients<'_>,
    prior: crate::foundation::GameSessionId,
    soul: u32,
    before: oteryn_protocol_oteryn::actor_spell::ActorVitals,
) -> TestResult {
    let mut client = connect_world(clients, 0xcb).await?;
    let row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
        let after = client
            .actor_vitals()
            .ok_or("step saved rest: fresh-admission vitals absent")?;
        if client.join_snapshot().game_session_id == prior
            || after.soul < soul
            || after.health < before.health
            || after.mana < before.mana
        {
            return Err(format!(
                "step saved rest: prior={prior:?} admitted={:?} before={before:?} after={after:?}",
                client.join_snapshot().game_session_id
            )
            .into());
        }
        row.pass("fresh-admission-saved-rest", after);
        Ok(())
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bed_intent_encodes_handle_and_skill() -> TestResult {
        let target = NonZeroU64::new(99).ok_or("handle")?;
        let decoded = oteryn_protocol_oteryn::bed::decode_bed_sleep_intent(&bed_payload(target, 6))
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decoded,
            oteryn_protocol_oteryn::bed::BedSleepIntent { target, skill: 6 }
        );
        Ok(())
    }
    #[test]
    fn row_needs_every_lifecycle_step_once() {
        let mut row = Row::new();
        for step in STEPS {
            row.pass(step, ());
        }
        assert!(!row.invalid && STEPS.iter().all(|step| row.passed.contains(step)));
        row.pass(STEPS[0], ());
        assert!(row.invalid);
    }
}
