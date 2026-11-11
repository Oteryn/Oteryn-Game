//! E2E-C17b — Invisibility and targeting, derived from the C17b matrix/audit row
//! and the ordinary-combat/monster-AI owners named there.
//!
//! Player scenario:
//! 1. Admit the provisioned level-100 caster and an ordinary second character, walk both to a
//!    rat spawn, and observe the rat damage the visible caster (normal monster targeting).
//! 2. Have the second player select the visible caster with ATTACK; assert the accepted reply and
//!    exact combat-state target delta, then clear that target.
//! 3. Cast `utana vid` through ACTOR_SPELL; assert Cast, the exact mana delta, and a nonzero server
//!    sequence. Assert the rat produces no further attributed damage and the caster disappears
//!    from the other player's entity view.
//! 4. Re-send ATTACK from the other player using the previously observed entity reference; assert
//!    the TargetNotVisible refusal and that no target is installed. Immediately recast Invisible;
//!    assert the Cooldown refusal and no mana delta (the negative/resource-free path).
//! 5. Log the invisible caster out and freshly admit the same character. Assert a new session with
//!    the paid mana and position retained, and assert the observer still receives no caster entity
//!    (durable active condition). Walk home, log both characters out, and report one row verdict.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};

const ACTIONS: &[&str] = &[
    "monster-targets-visible",
    "player-targets-visible",
    "cast-and-cost",
    "monster-drops-invisible",
    "observer-loses-entity",
    "known-reference-refused",
    "cooldown-no-cost",
    "fresh-admission-condition",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
}
impl Row {
    fn new() -> Self {
        Self {
            passed: std::collections::BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, action: &'static str, sequence: u64) -> TestResult {
        if sequence == 0 || !ACTIONS.contains(&action) || !self.passed.insert(action) {
            return Err(format!(
                "C17b invalid/duplicate receipt for step {action}: sequence={sequence}"
            )
            .into());
        }
        evidence(&format!(
            "C17B_CHECK invisible-targeting PASS step={action} server_sequence={sequence}"
        ));
        Ok(())
    }
    fn finish(self, result: &TestResult) {
        if let Err(error) = result {
            evidence(&format!("C17B_GAP step-failure observed={error}"));
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("C17B_GAP step={action} observed=not-reached"));
            }
        }
        let pass = result.is_ok() && ACTIONS.iter().all(|a| self.passed.contains(a));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C17b.full status={} seconds={:.3}{}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if pass {
                ""
            } else {
                " evidence=runtime-step-failure-see-C17B_GAP"
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result =
        match row_complete_with_timeout(Duration::from_secs(180), journey(&mut row, clients)).await {
            Ok(result) => result,
            Err(_) => Err(
                "step=journey observed=timeout-after-180s need=responsive-spell/AI/visibility-path"
                    .into(),
            ),
        };
    row.finish(&result);
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    Ok(catalog.into_spell_book_with_selections(&selections)?)
}
fn spell_index(book: &crate::spell::SpellBook) -> TestResult<std::num::NonZeroU32> {
    (1..=book.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| {
            book.indexed(*i)
                .is_some_and(|s| s.name.eq_ignore_ascii_case("Invisibility"))
        })
        .ok_or_else(|| "step=cast observed=served-Invisibility-absent need=pinned-spell".into())
}
fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("step=visibility observed=entity-snapshot-absent")?;
    entities
        .iter()
        .find(|e| &e.entity.identity == entities.own_identity())
        .map(|e| e.entity)
        .ok_or_else(|| "step=visibility observed=own-entity-absent".into())
}
fn visible(client: &DevClientSession, target: oteryn_dev_client::EntityRef) -> bool {
    client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|e| e.entity == target))
}
fn damage_sequence(
    client: &mut DevClientSession,
    attacker: oteryn_dev_client::EntityRef,
    target: oteryn_dev_client::EntityRef,
) -> u64 {
    use oteryn_protocol_oteryn::presentation::PresentationEventKind;
    client.take_events().into_iter().filter_map(|event| match event {
        SessionEvent::Presentation(delta) if delta.value.iter().any(|event| matches!(event.kind, PresentationEventKind::ValueText { attacker: Some(a), target: t, primary, .. } if a == attacker && t == target && primary.value > 0)) => Some(delta.server_sequence),
        _ => None,
    }).max().unwrap_or(0)
}
async fn wait_visible(
    client: &mut DevClientSession,
    target: oteryn_dev_client::EntityRef,
    expected: bool,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut sequence = 0;
    loop {
        sequence = client
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                SessionEvent::WorldSpatialEntities(delta) => Some(delta.server_sequence),
                _ => None,
            })
            .max()
            .unwrap_or(sequence);
        if visible(client, target) == expected && sequence != 0 {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=observer-visibility observed={} expected={expected} need=visibility-delta",
                visible(client, target)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}
async fn logout(client: &mut RowClient) -> TestResult {
    let reply = client.row_logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?} need=accepted-logout").into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut caster = row_connect_character(clients, 0xb1, 1).await?;
    let mut observer = row_connect_character(clients, 0xb2, 0).await?;
    travel_to_spawn(&mut caster, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let caster_tile = tile(caster.world_spatial().actor_position)?;
    let path = route(
        world,
        tile(observer.world_spatial().actor_position)?,
        |at| {
            at.floor == caster_tile.floor
                && at
                    .x
                    .abs_diff(caster_tile.x)
                    .max(at.y.abs_diff(caster_tile.y))
                    <= 1
        },
    )
    .ok_or("step=meet observed=no-walk-route need=reachable-rat-spawn")?;
    walk(&mut observer, world, &path).await?;
    let caster_ref = own(&caster)?;
    let rat = caster
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Creature
                    && e.position.floor == caster.world_spatial().actor_position.floor
                    && e.position
                        .x
                        .abs_diff(caster.world_spatial().actor_position.x)
                        .max(
                            e.position
                                .y
                                .abs_diff(caster.world_spatial().actor_position.y),
                        )
                        <= 1
            })
        })
        .map(|e| e.entity)
        .ok_or("step=monster-targets-visible observed=no-adjacent-rat need=live-rat-spawn")?;
    caster.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let pre_damage = loop {
        caster.service_liveness(Duration::from_millis(100)).await?;
        let seq = damage_sequence(&mut caster, rat, caster_ref);
        if seq != 0 {
            break seq;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=monster-targets-visible observed=no-attributed-damage-within-5s need=rat-AI-target/swing".into());
        }
    };
    row.pass("monster-targets-visible", pre_damage)?;
    let modes = FightModes {
        fight_mode: FightMode::Defensive,
        chase: ChaseMode::Stand,
        secure: true,
    };
    let mode = observer.session_mut().set_fight_modes(&modes).await?;
    let selected = observer
        .session_mut()
        .attack_target(Some(&caster_ref))
        .await?;
    if mode.disposition != AttackIntentDisposition::Ok
        || selected.disposition != AttackIntentDisposition::Ok
        || observer
            .session_mut()
            .combat_state()
            .is_none_or(|s| s.target != Some(caster_ref))
    {
        return Err(format!("step=player-targets-visible observed=mode:{:?}/attack:{:?}/state:{:?} need=accepted-target-delta", mode.disposition, selected.disposition, observer.session_mut().combat_state()).into());
    }
    row.pass("player-targets-visible", selected.result_server_sequence)?;
    observer.session_mut().attack_target(None).await?;
    let book = spell_book()?;
    let spell = book
        .spoken("utana vid")
        .ok_or("step=cast observed=words-unresolved need=Invisibility-catalog")?
        .spell;
    let crate::spell::ManaCost::Fixed(cost) = spell.mana else {
        return Err("step=cast observed=non-fixed-mana need=assertable-cost".into());
    };
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("step=cast observed=vitals-absent")?;
    caster.take_events();
    observer.take_events();
    let cast = caster
        .cast_spell(spell_index(&book)?, SpellTarget::None, false)
        .await?;
    if cast.status != CommandStatus::Accepted
        || cast.disposition != SpellCastDisposition::Cast
        || cast.actor_vitals_delta.as_ref().is_none_or(|d| {
            d.server_sequence == 0 || before.mana.checked_sub(cost) != Some(d.value.mana)
        })
    {
        return Err(format!("step=cast-and-cost observed=status:{:?}/disposition:{:?}/vitals:{:?} need=Cast-and-exact-mana-delta", cast.status, cast.disposition, cast.actor_vitals_delta).into());
    }
    row.pass("cast-and-cost", cast.result_server_sequence)?;
    let lost_seq = wait_visible(&mut observer, caster_ref, false).await?;
    row.pass(
        "observer-loses-entity",
        lost_seq.max(cast.result_server_sequence),
    )?;
    let no_damage_until = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < no_damage_until {
        caster.service_liveness(Duration::from_millis(100)).await?;
        if damage_sequence(&mut caster, rat, caster_ref) != 0 {
            return Err("step=monster-drops-invisible observed=attributed-rat-damage-after-cast need=AI-invisibility-filter".into());
        }
    }
    row.pass("monster-drops-invisible", cast.result_server_sequence)?;
    let refused = observer
        .session_mut()
        .attack_target(Some(&caster_ref))
        .await?;
    if refused.disposition != AttackIntentDisposition::TargetNotVisible
        || observer
            .session_mut()
            .combat_state()
            .is_some_and(|s| s.target.is_some())
    {
        return Err(format!("step=known-reference-refused observed={refused:?}/state:{:?} need=TargetNotVisible-without-target", observer.session_mut().combat_state()).into());
    }
    row.pass("known-reference-refused", refused.result_server_sequence)?;
    let paid = caster
        .actor_vitals()
        .copied()
        .ok_or("step=cooldown observed=vitals-absent")?;
    let cooldown = caster
        .cast_spell(spell_index(&book)?, SpellTarget::None, false)
        .await?;
    if cooldown.disposition != SpellCastDisposition::CoolingDown
        || cooldown.actor_vitals_delta.is_some()
        || caster
            .actor_vitals()
            .is_none_or(|v| v.mana != paid.mana || v.soul != paid.soul)
    {
        return Err(format!(
            "step=cooldown-no-cost observed={cooldown:?} need=Cooldown-without-vitals-delta"
        )
        .into());
    }
    row.pass("cooldown-no-cost", cooldown.result_server_sequence)?;
    let session = caster.join_snapshot().game_session_id;
    let position = caster.world_spatial().actor_position;
    logout(&mut caster).await?;
    caster = row_connect_character(clients, 0xb3, 1).await?;
    if caster.join_snapshot().game_session_id == session
        || caster.world_spatial().actor_position != position
        || caster
            .actor_vitals()
            .is_none_or(|v| v.mana != paid.mana || v.soul != paid.soul)
    {
        return Err("step=fresh-admission-condition observed=session/position/vitals-mismatch need=durable-condition-state".into());
    }
    observer
        .service_liveness(Duration::from_millis(100))
        .await?;
    if visible(&observer, own(&caster)?) {
        return Err("step=fresh-admission-condition observed=caster-visible need=reloaded-invisibility-condition".into());
    }
    row.pass("fresh-admission-condition", cast.result_server_sequence)?;
    let back = route(world, tile(caster.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("step=cleanup observed=no-caster-route-home")?;
    walk(&mut caster, world, &back).await?;
    let back = route(
        world,
        tile(observer.world_spatial().actor_position)?,
        |at| at == TOWN,
    )
    .ok_or("step=cleanup observed=no-observer-route-home")?;
    walk(&mut observer, world, &back).await?;
    logout(&mut caster).await?;
    logout(&mut observer).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_requires_every_lifecycle_receipt() -> TestResult {
        let mut row = Row::new();
        for (index, action) in ACTIONS.iter().enumerate() {
            assert!(row.pass(action, u64::try_from(index + 1)?).is_ok());
        }
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        Ok(())
    }
}
