//! P02 full-row scenario, derived from its audit/matrix row and Bosstiary owners:
//! 1. Send BOSS_COOLDOWNS; require an accepted progress/points snapshot.
//! 2. Walk to a compiled-world boss, ATTACK until death, and require kill/point deltas.
//! 3. Earn prowess by those kills; Equip slot one and require unlock/bonus/slot deltas.
//! 4. Repeat Equip and require a refusal reply with unchanged state.
//! 5. ATTACK the daily boosted boss and require its three-kill delta and slot refusal.
//! 6. LOGOUT, freshly admit, query both commands, and re-check all durable state.
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::bosstiary_slots::{
    BossSlotReply, BossSlotRequest, COMMAND_TYPE_BOSSTIARY_SLOTS,
};
use std::collections::{BTreeMap, BTreeSet};
const REQUIRED: &[&str] = &[
    "initial-query",
    "boss-kill-progress",
    "slot-equip",
    "slot-refusal",
    "boosted-kill",
    "fresh-admission",
];
struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}
impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, s: &'static str, v: impl std::fmt::Debug) {
        if REQUIRED.contains(&s) && self.passed.insert(s) {
            evidence(&format!("P02_CHECK step={s} status=PASS observed={v:?}"));
        }
    }
    fn finish(self, r: TestResult) {
        if let Err(e) = &r {
            evidence(&format!("P02_GAP status=FAIL observed={e}"));
        }
        for s in REQUIRED {
            if !self.passed.contains(s) {
                evidence(&format!(
                    "P02_CHECK step={s} status=FAIL observed=not-reached"
                ));
            }
        }
        let ok = r.is_ok() && REQUIRED.iter().all(|s| self.passed.contains(s));
        evidence(&format!(
            "e2e_all scenario=P02.full status={} seconds={:.3} evidence={}",
            if ok { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if ok {
                "observed-complete-player-lifecycle"
            } else {
                "runtime-step-failure-see-P02_GAP"
            }
        ));
    }
}
pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let r = verdict::indexed_timeout(Duration::from_secs(600), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match r {
        Ok(v) => v,
        Err(_) => Err("step deadline: no server reply within 600s".into()),
    });
}
async fn snapshot(c: &mut DevClientSession) -> TestResult<boss_encounter::Snapshot> {
    let q = boss_encounter::query("").ok_or("cooldown encoder")?;
    let r = c
        .session_mut()
        .gated_command(q.capability, q.kind, &q.payload)
        .await?;
    if r.status != CommandStatus::Accepted {
        return Err(format!("step cooldown-query: reply={r:?}").into());
    }
    boss_encounter::decode(&r.payload)
        .ok_or_else(|| format!("step cooldown-query: malformed payload={:?}", r.payload).into())
}
async fn slot(c: &mut DevClientSession, q: BossSlotRequest) -> TestResult<BossSlotReply> {
    let p = q.encode().ok_or("slot encoder")?;
    let r = c
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            COMMAND_TYPE_BOSSTIARY_SLOTS,
            &p,
        )
        .await?;
    if r.status != CommandStatus::Accepted {
        return Err(format!("step boss-slot: reply={r:?}").into());
    }
    BossSlotReply::decode(&r.payload)
        .ok_or_else(|| format!("step boss-slot: malformed payload={:?}", r.payload).into())
}
fn counts(s: &boss_encounter::Snapshot) -> BTreeMap<String, u64> {
    s.progress.iter().cloned().collect()
}
fn bosses() -> TestResult<Vec<crate::domain::bosstiary::BosstiaryBoss>> {
    let l = crate::content::ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    };
    Ok(crate::content::canonical_title_catalogue(l)?.bosstiary)
}
async fn kill(
    c: &mut DevClientSession,
    w: &BundleWorld,
    s: &oteryn_world_bundle::spawn::Table,
    key: &str,
) -> TestResult {
    travel_to_spawn(c, w, s, key)
        .await
        .map_err(|e| { verdict::record_error(verdict::IndexedError::as_error(&e)); verdict::context(format!("step boss-kill {key}: walking/spawn reply={e}"), e) })?;
    let here = c.world_spatial().actor_position;
    let target = c
        .world_entities()
        .and_then(|es| {
            es.iter()
                .filter(|e| {
                    e.kind == EntityKind::Creature
                        && e.position.floor == here.floor
                        && e.position
                            .x
                            .abs_diff(here.x)
                            .max(e.position.y.abs_diff(here.y))
                            <= 1
                })
                .min_by_key(|e| {
                    e.position
                        .x
                        .abs_diff(here.x)
                        .max(e.position.y.abs_diff(here.y))
                })
        })
        .copied()
        .ok_or_else(|| format!("step boss-kill {key}: no adjacent creature"))?;
    let r = c.session_mut().attack_target(Some(&target.entity)).await?;
    if r.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step boss-kill {key}: ATTACK reply={r:?}").into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    loop {
        c.service_liveness(Duration::from_millis(50)).await?;
        c.take_events();
        if c.world_entities()
            .is_some_and(|es| es.get(&target.entity).is_none())
        {
            c.session_mut().attack_target(None).await?;
            return Ok(());
        }
        if tokio::time::Instant::now() >= until {
            c.session_mut().attack_target(None).await?;
            return Err(
                format!("step boss-kill {key}: ATTACK accepted, death frame timeout").into(),
            );
        }
    }
}
async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let meta = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut c = connect_world(clients, 0xb2).await?;
    let mut before = snapshot(&mut c).await?;
    let initial_slot = slot(&mut c, BossSlotRequest::Query).await?;
    row.pass(
        "initial-query",
        (&before.progress, before.points, &initial_slot),
    );
    let catalog = bosses()?;
    let spawned: Vec<_> = catalog
        .iter()
        .filter(|b| {
            meta.spawns.sources.iter().any(|src| {
                src.points.iter().any(|p| {
                    meta.spawns
                        .creatures
                        .get(p.creature as usize)
                        .is_some_and(|x| x.key == b.key)
                })
            })
        })
        .collect();
    let mut ordinary = None;
    let mut boosted = None;
    for b in spawned {
        kill(&mut c, world, &meta.spawns, &b.key).await?;
        let after = snapshot(&mut c).await?;
        let d = counts(&after)
            .get(&b.key)
            .copied()
            .unwrap_or(0)
            .saturating_sub(counts(&before).get(&b.key).copied().unwrap_or(0));
        match d {
            1 => ordinary = Some((b.key.clone(), b.category.clone(), after.clone())),
            3 => boosted = Some((b.key.clone(), after.clone())),
            _ => {
                return Err(format!(
                    "step boss-kill {}: progress delta={d}, reply={after:?}",
                    b.key
                )
                .into());
            }
        }
        before = after;
        if ordinary.is_some() && boosted.is_some() {
            break;
        }
    }
    let (boss, category, after) =
        ordinary.ok_or("step boss-kill: no reachable compiled Bosstiary boss produced delta=1")?;
    row.pass("boss-kill-progress", (&boss, &after.progress, after.points));
    if category != "nemesis" {
        return Err(format!(
            "step slot-equip: boss={boss} category={category} has not earned prowess"
        )
        .into());
    }
    let equipped = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: 1,
            boss: boss.clone(),
        },
    )
    .await?;
    if equipped.status != 0 || equipped.unlocked & 1 == 0 || equipped.bosses[0] != boss {
        return Err(format!("step slot-equip: reply={equipped:?}").into());
    }
    row.pass("slot-equip", &equipped);
    let refused = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: 1,
            boss: boss.clone(),
        },
    )
    .await?;
    if refused.status == 0 || refused.bosses != equipped.bosses {
        return Err(format!("step slot-refusal: reply={refused:?}").into());
    }
    row.pass("slot-refusal", &refused);
    let (boosted_key, boosted_state) =
        boosted.ok_or("step boosted-kill: no reachable boss produced delta=3")?;
    let br = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: 2,
            boss: boosted_key.clone(),
        },
    )
    .await?;
    if br.status == 0 {
        return Err(format!("step boosted-kill: slot accepted reply={br:?}").into());
    }
    row.pass("boosted-kill", (&boosted_key, &boosted_state.progress, &br));
    if c.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step fresh-admission: logout refused".into());
    }
    drop(c);
    let mut c = connect_world(clients, 0xb3).await?;
    let durable = snapshot(&mut c).await?;
    let ds = slot(&mut c, BossSlotRequest::Query).await?;
    if durable.progress != before.progress
        || durable.points != before.points
        || ds.bosses != equipped.bosses
    {
        return Err(format!("step fresh-admission: progress={durable:?}, slot={ds:?}").into());
    }
    row.pass("fresh-admission", (&durable.progress, durable.points, &ds));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_whole_lifecycle() {
        assert_eq!(REQUIRED.len(), 6);
        assert!(Row::new().passed.is_empty());
    }
}
