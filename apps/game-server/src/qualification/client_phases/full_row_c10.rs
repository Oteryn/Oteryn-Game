//! C10 concrete player scenario, derived from the matrix/re-audit and the
//! `bed_regen`/`spell_periodic` server owners:
//! 1. Admit the provisioned character and cast an available soul-costing learned
//!    spell; require a `Cast` reply and the exact soul debit in an actor-vitals delta.
//! 2. Immediately repeat that player command; require the cooldown refusal and no
//!    soul delta (the negative path must not charge the player).
//! 3. Walk to a native creature spawn whose reward meets the character level,
//!    and kill it through ordinary combat, thereby
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
use super::full_row_c01::admission_error;
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_catalog as spell_wire;
use oteryn_protocol_oteryn::bed::COMMAND_TYPE_BED_SLEEP_INTENT;
use oteryn_protocol_oteryn::item_view::{ItemHandle, encode_item_target};
use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
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
        Self { passed: BTreeSet::new(), invalid: false, started: std::time::Instant::now() }
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
            if result.is_ok() && !self.invalid && STEPS.iter().all(|s| self.passed.contains(s)) { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(420), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| Err("step deadline: C10 journey timed out waiting for a server reply/delta".into()));
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

async fn bed_command(client: &mut DevClientSession, handle: ItemHandle) -> TestResult<UseDisposition> {
    let reply = client.session_mut().gated_command(
        CAPABILITY_WORLD_INTERACTION_V1,
        COMMAND_TYPE_BED_SLEEP_INTENT,
        &bed_payload(handle, 6),
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step bed command: transport status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step bed command: malformed reply={error:?} bytes={:?}", reply.payload).into())
}

async fn soul_spell(client: &mut DevClientSession) -> TestResult<spell_wire::SpellCatalogEntry> {
    let snapshot = client.session_mut().gated_snapshot(spell_wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step soul debit: admitted spell catalogue missing")?;
    let catalog = spell_wire::decode_catalog(&snapshot.payload)
        .map_err(|e| format!("step soul debit: catalogue reply={e:?}"))?;
    let vitals = client.actor_vitals().ok_or("step=soul-debit observed=no-vitals")?;
    select_soul_spell(&catalog, vitals.mana, vitals.soul).cloned().ok_or_else(|| {
        let candidates: Vec<_> = catalog.iter().filter(|spell| spell.rune_item.is_none() && spell.soul > 0)
            .map(|spell| (spell.words.as_str(), spell.level, spell.mana, spell.soul, spell.available)).collect();
        format!("step=soul-debit observed=no-affordable-eligible-fixed-cost-spell mana={} soul={} candidates={candidates:?} need=ordinary-soul-spell-eligibility", vitals.mana, vitals.soul).into()
    })
}

fn select_soul_spell(
    catalog: &[spell_wire::SpellCatalogEntry],
    mana: u32,
    soul: u32,
) -> Option<&spell_wire::SpellCatalogEntry> {
    catalog.iter().find(|spell| {
        spell.available && spell.rune_item.is_none() && !spell.mana_percent
            && spell.mana <= mana && (1..=soul).contains(&spell.soul)
    })
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let generation = load_world_generation(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let world = &generation.bundle;
    let mut client = connect_world(clients, 0xca)
        .await
        .map_err(|error| admission_error("initial-admission", error))?;
    let spell = soul_spell(&mut client).await?;
    let before = client.actor_vitals().copied().ok_or("step soul debit: vitals absent")?;
    let cast = client.cast_spell(NonZeroU32::new(spell.cast_index).ok_or("step soul debit: zero cast index")?, SpellTarget::None, false).await?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("step soul debit: server reply={cast:?}").into());
    }
    let expected = before.soul.checked_sub(spell.soul)
        .ok_or("step=soul-debit observed=insufficient-soul need=affordable-soul-cost")?;
    let after = cast.actor_vitals_delta.as_ref().map(|delta| delta.value.soul)
        .filter(|&soul| soul == expected)
        .ok_or_else(|| format!("step=soul-debit observed={cast:?} need=command-vitals-delta-soul-{expected}"))?;
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

    let (xp_before, level) = progress(&mut client).await?;
    let creature = soul_window_target(world, &metadata.spawns, &generation.rewards,
        tile(client.world_spatial().actor_position)?, level)?;
    travel_to_spawn(&mut client, world, &metadata.spawns, creature).await?;
    client_hunt::hunt(&mut client).await.map_err(|e| { verdict::record_error(verdict::IndexedError::as_error(&e)); verdict::context(format!("step kill window: combat reply={e}"), e) })?;
    let xp_after = progress(&mut client).await?.0;
    if xp_after.checked_sub(xp_before).is_none_or(|earned| earned < u64::from(level)) {
        return Err(format!("step=kill-window creature={creature} observed=xp-{xp_before}->{xp_after} need=applied-xp-at-least-level-{level}").into());
    }
    let regenerated = wait_soul(&mut client, after, |soul| soul == after + 1, "kill-window regeneration", Duration::from_secs(130)).await?;
    row.pass("kill-window-regeneration", (xp_before, xp_after, after, regenerated));

    let carried = client.inventory().and_then(|i| i.main_backpack).map(|i| i.handle)
        .ok_or("step=non-bed-refusal observed=no-backpack-handle need=real-carried-item")?;
    let vitals = client.actor_vitals().copied().ok_or("step non-bed refusal: vitals absent")?;
    let custody = client.inventory().cloned();
    let refusal = bed_command(&mut client, carried).await?;
    if refusal != UseDisposition::NotPossible || client.actor_vitals() != Some(&vitals) || client.inventory().cloned() != custody {
        return Err(format!("step non-bed refusal: reply={refusal:?} vitals={:?}", client.actor_vitals()).into());
    }
    row.pass("non-bed-refusal", refusal);

    let (bed, definition, _) = plan_bed(world, tile(client.world_spatial().actor_position)?)?;
    let food = full_e2ef7::acquire(&mut client, world, &provisioning::item_index()?, "oteryn:item.tibia.i3577", 1)
        .await.map_err(|error| format!("step=rest-food observed={error} need=earned-edible-food"))?;
    let handle = client.inventory().and_then(|inventory| inventory.entries.iter()
        .find(|item| item.item_definition_ref == food)).ok_or("step=rest-food observed=no-carried-food-handle")?.handle;
    let used = client.use_item(handle).await?;
    if used.disposition != UseDisposition::Committed {
        return Err(format!("step=rest-food observed={used:?} need=consumed-food").into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == bed.floor && u32::from(at.x.abs_diff(bed.x)) + u32::from(at.y.abs_diff(bed.y)) == 1
    }).ok_or("step=bed-route observed=no-route-from-food-vendor need=reachable-house-bed")?;
    walk(&mut client, world, &path).await?;
    let basis = spatial_basis::current(&mut client)?;
    let position = basis.position(bed);
    let map = client.session_mut().world_map_snapshot().ok_or("step=bed-sleep observed=no-live-world-map")?;
    let origin = map.tiles.iter().find(|entry| entry.position == position)
        .and_then(|entry| entry.items.iter().find(|item| item.definition == MapDefinition::Item(definition)))
        .map(|item| item.origin)
        .ok_or("step=bed-sleep observed=no-live-bed-map-item")?;
    let mut before_rest = client.actor_vitals().copied().ok_or("step=bed-sleep observed=no-vitals")?;
    if before_rest.health == before_rest.max_health && before_rest.mana == before_rest.max_mana {
        let index = NonZeroU32::new(spell.cast_index).ok_or("step=rest-deficit observed=zero-spell-index")?;
        let cast = client.cast_spell(index, SpellTarget::None, false).await?;
        let paid = cast.actor_vitals_delta.as_ref().map(|delta| delta.value)
            .filter(|after| cast.status == CommandStatus::Accepted
                && cast.disposition == SpellCastDisposition::Cast && after.mana < before_rest.mana)
            .ok_or_else(|| format!("step=rest-deficit observed={cast:?} need=ordinary-mana-debit-before-bed"))?;
        before_rest = paid;
    }
    client.take_events();
    sleep_at_bed(&mut client, bed, map.header.bundle_digest, origin).await?;
    if client.take_events().into_iter().any(|event| matches!(event,
        SessionEvent::ActorVitals(delta) if delta.value != before_rest
    )) {
        return Err("step=saved-rest observed=vitals-changed-before-terminal-bed-ack need=known-saved-vitals".into());
    }
    row.pass("bed-sleep", position);
    let prior = client.join_snapshot().game_session_id;
    drop(client);
    let started = tokio::time::Instant::now();
    let wake = started + Duration::from_secs(31);
    verdict::Harness::global().bounded_wait(wake, "saved-rest");
    tokio::time::sleep_until(wake).await;
    fresh_admission(row, clients, prior, before_rest, started).await
}

fn fixed_bed_target(
    bed: TilePos,
    digest: [u8; 32],
    ordinal: u8,
    object_revision: u64,
) -> TestResult<oteryn_protocol_oteryn::world_object::WorldObjectTarget> {
    let key = crate::map::view::placement_key(bed, ordinal).ok_or("step=bed-sleep observed=invalid-bed-ordinal")?;
    let mut placement = digest.to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(oteryn_protocol_oteryn::world_object::WorldObjectTarget { placement, expected_revision: object_revision })
}

async fn sleep_at_bed(client: &mut DevClientSession, bed: TilePos, digest: [u8; 32], origin: MapOrigin) -> TestResult {
    use oteryn_protocol_oteryn::server_modal as modal;
    match origin {
        MapOrigin::Handle(handle) => {
            let reply = bed_command(client, handle).await?;
            if reply != UseDisposition::OfflineTraining {
                return Err(format!("step=bed-sleep observed={reply:?} need=OfflineTraining").into());
            }
        }
        MapOrigin::BaseOrdinal { ordinal, object_revision } => {
            let target = fixed_bed_target(bed, digest, ordinal, object_revision)?;
            let opt_in = client.session_mut().gated_command(modal::CAPABILITY_SERVER_MODAL_V1,
                modal::COMMAND_TYPE_SERVER_MODAL_QUERY, modal::QUERY_WITH_TERMINAL_REPLY).await?;
            if opt_in.status != CommandStatus::Accepted || modal::Reply::decode(&opt_in.payload) != Ok(modal::Reply::None) {
                return Err(format!("step=bed-modal-opt-in observed={opt_in:?} need=empty-accepted-modal-query").into());
            }
            let used = client.use_object(&target.placement, target.expected_revision).await?;
            if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::BedChooseSkill {
                return Err(format!("step=bed-use observed={used:?} need=BedChooseSkill").into());
            }
            let query = client.session_mut().gated_command(modal::CAPABILITY_SERVER_MODAL_V1,
                modal::COMMAND_TYPE_SERVER_MODAL_QUERY, &[]).await?;
            let Ok(modal::Reply::Dialog(dialog)) = modal::Reply::decode(&query.payload) else {
                return Err(format!("step=bed-modal observed={query:?} need=Choose-a-Skill-dialog").into());
            };
            if query.status != CommandStatus::Accepted || dialog.title != "Choose a Skill"
                || !dialog.buttons.iter().any(|(id, _)| *id == 1)
                || !dialog.choices.iter().any(|(id, _)| *id == 6)
            {
                return Err(format!("step=bed-modal observed={dialog:?} need=magic-training-choice").into());
            }
            let answer = modal::Answer { dialog_id: dialog.id, button_id: 1, choice_id: Some(6) };
            let reply = client.session_mut().gated_command(modal::CAPABILITY_SERVER_MODAL_V1,
                modal::COMMAND_TYPE_SERVER_MODAL_ANSWER, &answer.encode()?).await?;
            if reply.status != CommandStatus::Accepted
                || modal::Reply::decode(&reply.payload) != Ok(modal::Reply::AnsweredAndLoggedOut(answer))
            {
                return Err(format!("step=bed-modal-answer observed={reply:?} need=AnsweredAndLoggedOut").into());
            }
        }
        MapOrigin::DisplayOnly => return Err("step=bed-sleep observed=display-only-bed need=usable-native-map-target".into()),
    }
    Ok(())
}

fn soul_window_target<'a>(
    world: &BundleWorld,
    spawns: &'a oteryn_world_bundle::spawn::Table,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    from: TilePos,
    level: u32,
) -> TestResult<&'a str> {
    let mut candidates: Vec<_> = spawns.creatures.iter().enumerate().filter_map(|(index, creature)| {
        let xp = rewards.row(&creature.key).ok()?.xp_amount.get();
        (xp >= i64::from(level)).then_some((xp, index, creature.key.as_str()))
    }).collect();
    candidates.sort_unstable();
    for (_, index, key) in candidates {
        if route(world, from, |at| {
            world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone())
                && spawns.sources.iter().any(|source| source.floor == at.floor
                    && source.points.iter().any(|point| point.creature as usize == index
                        && point.x.abs_diff(at.x).max(point.y.abs_diff(at.y)) <= 1))
        }).is_some() {
            return Ok(key);
        }
    }
    Err(format!("step=kill-window observed=no-reachable-spawn-with-xp-at-least-{level} need=level-appropriate-creature").into())
}

fn plan_bed(world: &BundleWorld, from: TilePos) -> TestResult<(TilePos, NonZeroU32, Vec<StepDirection>)> {
    use oteryn_gameplay_world::map::view::MapFacts;

    let profiles = native_input()?.item_profiles.as_ref().ok_or("step=bed-route observed=no-native-item-profiles")?;
    let profiles: serde_json::Value = serde_json::from_slice(&profiles.bytes)?;
    let definitions = provisioning::item_index()?;
    let heads: BTreeSet<_> = profiles["records"].as_array().ok_or("step=bed-route observed=no-item-profile-records")?
        .iter().filter_map(|entry| {
            let bed = &entry["semantics"]["bed"];
            if bed["state"] != "KNOWN" || bed["value"]["part"] != "HEAD" {
                return None;
            }
            let key = entry["production_definition"]["production_key"].as_str()?;
            definitions.definition_ref("Item", key, definitions.revision(key)?)
        }).collect();
    let facts = world.facts();
    let houses: BTreeSet<_> = facts.house_ids().collect();
    let mut sites = std::collections::BTreeMap::new();
    for bed in houses.into_iter().flat_map(|house| facts.house_tiles(house)) {
        let Some(stack) = world.base().tile(bed.x, bed.y, bed.floor) else { continue };
        for (ordinal, (&id, _)) in stack.ids().iter().zip(stack.depths()).filter(|&(_, &depth)| depth == 0).enumerate() {
            let Ok(ordinal) = u8::try_from(ordinal) else { break };
            if let Some(entry) = facts.base_entry(bed, ordinal, id)
                && let MapDefinition::Item(reference) = entry.definition
                && heads.contains(&reference)
            {
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    if let (Some(x), Some(y)) = (bed.x.checked_add_signed(dx), bed.y.checked_add_signed(dy)) {
                        sites.entry(TilePos { x, y, floor: bed.floor }).or_insert((bed, reference));
                    }
                }
            }
        }
    }
    if sites.is_empty() {
        return Err("step=bed-route observed=no-authored-house-bed-head need=placed-reciprocal-bed-pair".into());
    }
    let path = route(world, from, |at| sites.contains_key(&at))
        .ok_or("step=bed-route observed=no-walk-route-to-authored-bed need=reachable-house-bed")?;
    let at = path.iter().try_fold(from, |at, &direction| destination(world, at, direction))
        .ok_or("step=bed-route observed=invalid-planned-step")?;
    let &(bed, definition) = sites.get(&at).ok_or("step=bed-route observed=non-bed-route-endpoint")?;
    Ok((bed, definition, path))
}

async fn wait_soul(client: &mut DevClientSession, from: u32, wanted: impl Fn(u32) -> bool, step: &str, duration: Duration) -> TestResult<u32> {
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                if delta.value.soul < from && step.contains("regeneration") {
                    return Err(format!("step {step}: unexpected soul delta {from}->{}", delta.value.soul).into());
                }
                if wanted(delta.value.soul) { return Ok(delta.value.soul); }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step {step}: timeout; last vitals={:?}", client.actor_vitals()).into());
        }
    }
}

async fn progress(client: &mut DevClientSession) -> TestResult<(u64, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[1, 0, 0, 1],
    ).await?;
    if reply.status != CommandStatus::Accepted || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err() || reply.payload[0] != 1 {
        return Err(format!("step kill window: progression reply={reply:?}").into());
    }
    Ok((u64::from_le_bytes(reply.payload[1..9].try_into()?), u32::from_le_bytes(reply.payload[9..13].try_into()?)))
}

async fn fresh_admission(row: &mut Row, clients: &SeamClients<'_>, prior: crate::foundation::GameSessionId, before: oteryn_protocol_oteryn::actor_spell::ActorVitals, started: tokio::time::Instant) -> TestResult {
    let elapsed = started.elapsed();
    let client = connect_world(clients, 0xcb)
        .await
        .map_err(|error| admission_error("fresh-admission", error))?;
    let after = client.actor_vitals().ok_or("step saved rest: fresh-admission vitals absent")?;
    if client.join_snapshot().game_session_id == prior || after.soul != before.soul
        || after.health < before.health || after.mana < before.mana
        || !(rest_credited(elapsed, before.health, after.health, before.max_health)
            || rest_credited(elapsed, before.mana, after.mana, before.max_mana))
    {
        return Err(format!("step=saved-rest elapsed={elapsed:?} need=positive-offline-credit prior={prior:?} admitted={:?} before={before:?} after={after:?}", client.join_snapshot().game_session_id).into());
    }
    row.pass("fresh-admission-saved-rest", after);
    Ok(())
}

fn rest_credited(elapsed: Duration, before: u32, after: u32, maximum: u32) -> bool {
    elapsed >= Duration::from_secs(30) && before < maximum && after > before && after <= maximum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_bed_uses_bundle_ordinal_and_live_revision() -> TestResult {
        let bed = TilePos { x: 32311, y: 32804, floor: -7 };
        let target = fixed_bed_target(bed, [7; 32], 1, 23)?;
        assert_eq!(&target.placement[..32], &[7; 32]);
        assert_eq!(&target.placement[32..], &crate::map::view::placement_key(bed, 1)
            .ok_or("test placement")?.to_be_bytes());
        assert_eq!(target.expected_revision, 23);
        Ok(())
    }

    #[test]
    fn independent_review_c10_rejects_unproven_evidence() {
        assert!(!rest_credited(Duration::from_secs(31), 30, 30, 50));
        assert!(!rest_credited(Duration::from_secs(29), 30, 31, 50));
        assert!(!rest_credited(Duration::from_secs(31), 50, 50, 50));
        assert!(rest_credited(Duration::from_secs(31), 30, 31, 50));
    }

    #[test]
    fn soul_spell_needs_both_live_resource_pools() {
        let entry = spell_wire::SpellCatalogEntry {
            cast_index: 1, name: "Conjure Arrow".into(), words: "exevo con".into(), level: 13,
            mana: 100, mana_percent: false, soul: 1, learned: true, available: true,
            cooldown_groups: vec![], rune_item: None,
        };
        let catalog = [entry];
        assert!(select_soul_spell(&catalog, 55, 100).is_none());
        assert!(select_soul_spell(&catalog, 100, 0).is_none());
        assert_eq!(select_soul_spell(&catalog, 100, 1), Some(&catalog[0]));
    }

    #[test]
    fn bed_intent_encodes_handle_and_skill() -> TestResult {
        let target = NonZeroU64::new(99).ok_or("handle")?;
        let decoded = oteryn_protocol_oteryn::bed::decode_bed_sleep_intent(&bed_payload(target, 6))
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(decoded, oteryn_protocol_oteryn::bed::BedSleepIntent { target, skill: 6 });
        Ok(())
    }
    #[test]
    fn row_needs_every_lifecycle_step_once() {
        let mut row = Row::new();
        for step in STEPS { row.pass(step, ()); }
        assert!(!row.invalid && STEPS.iter().all(|step| row.passed.contains(step)));
        row.pass(STEPS[0], ());
        assert!(row.invalid);
    }
}
