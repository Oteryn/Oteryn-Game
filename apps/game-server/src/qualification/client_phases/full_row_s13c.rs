//! S13c full row: party-spell conditions applied to real party members.
//!
//! Concrete player journey derived from the S13c audit/matrix row and the
//! ordinary-combat condition application owner:
//! 1. Admit two distinct node-boot characters; query both party views and
//!    require empty rosters.
//! 2. Walk the caster beside the peer with ordinary STEP commands, invite the
//!    visible peer, accept on the peer, and require matching two-member views.
//! 3. Select the admitted caster's vocation-specific party spell from the
//!    compiled spell book, CAST it, and require an accepted Cast reply plus a
//!    mana delta.
//! 4. Query PARTY EFFECTS on both sockets and require the matching condition
//!    timer slot (Train, Protect, Enchant, Heal, or Enlighten) on caster and
//!    member, proving member application rather than merely a successful cast.
//! 5. CAST again during cooldown; require CoolingDown, no vitals delta, and no
//!    mana/state change (the refusal path).
//! 6. Log both characters out, freshly admit both, require new session ids,
//!    and re-query PARTY EFFECTS to prove each unexpired condition survived the
//!    durable logout/admission boundary; then leave the restored party and log
//!    out cleanly.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{actor_spell as casts, party as parties, party_effects};
use std::collections::BTreeSet;

const STEPS: &[&str] = &[
    "two-admissions",
    "walk-invite-accept",
    "cast-reply-and-mana",
    "member-condition-deltas",
    "cooldown-refusal",
    "fresh-admission-condition",
    "final-logout",
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
    fn pass(&mut self, step: &'static str) {
        if !STEPS.contains(&step) || !self.passed.insert(step) {
            self.invalid = true;
        } else {
            evidence(&format!("S13C_CHECK {step} PASS server-frames-validated"));
        }
    }
    fn finish(self, result: &TestResult) {
        for step in STEPS {
            if !self.passed.contains(step) {
                evidence(&format!(
                    "S13C_GAP step={step} observed=missing-delta-or-reply"
                ));
            }
        }
        if let Err(error) = result {
            evidence(&format!("S13C_GAP observed={error}"));
        }
        let pass = result.is_ok() && !self.invalid && STEPS.iter().all(|s| self.passed.contains(s));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=S13c.full status={} seconds={:.3}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(240), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| {
            Err("step=journey observed=timeout need=responsive-party-spell-server".into())
        });
    row.finish(&result);
}

async fn party(
    client: &mut DevClientSession,
    intent: Option<parties::PartyIntent>,
) -> TestResult<parties::PartyResult> {
    let (kind, payload) = match intent {
        Some(value) => (
            parties::COMMAND_TYPE_PARTY_INTENT,
            parties::encode_party_intent(value).map_err(|e| format!("party encode: {e:?}"))?,
        ),
        None => (parties::COMMAND_TYPE_PARTY_QUERY, Vec::new()),
    };
    let reply = client
        .session_mut()
        .gated_command(parties::CAPABILITY_PARTY_V1, kind, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=party-command observed=status-{:?} need=accepted-party-reply",
            reply.status
        )
        .into());
    }
    let state =
        parties::decode_party_result(&reply.payload).map_err(|e| format!("party decode: {e:?}"))?;
    if state.disposition != parties::PartyDisposition::Ok {
        return Err(format!(
            "step=party-command observed=disposition-{:?} need=party-operation-ok",
            state.disposition
        )
        .into());
    }
    Ok(state)
}

async fn effects(client: &mut DevClientSession) -> TestResult<party_effects::PartyEffects> {
    let reply = client
        .session_mut()
        .gated_command(
            party_effects::CAPABILITY_PARTY_EFFECTS_V1,
            party_effects::COMMAND_TYPE_PARTY_EFFECTS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=effects-query observed=status-{:?} need=accepted-party-effects",
            reply.status
        )
        .into());
    }
    party_effects::PartyEffects::decode(&reply.payload).ok_or_else(|| {
        "step=effects-query observed=malformed-payload need=party-effects-snapshot".into()
    })
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused need=accepted-logout".into());
    }
    Ok(())
}

fn own_ref(
    client: &DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    let entities = client.world_entities().ok_or("own entities absent")?;
    entities
        .iter()
        .find(|e| e.entity.identity == *entities.own_identity())
        .map(|e| e.entity)
        .ok_or_else(|| "own entity absent".into())
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selected = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    Ok(catalog.into_spell_book_with_selections(&selected)?)
}

fn spell_index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
    let key = &book
        .spoken(words)
        .ok_or_else(|| format!("compiled spell missing: {words}"))?
        .spell
        .key;
    (1..=book.source_len())
        .filter_map(|n| u32::try_from(n).ok().and_then(std::num::NonZeroU32::new))
        .find(|n| book.indexed(*n).is_some_and(|s| &s.key == key))
        .ok_or_else(|| format!("compiled spell has no index: {words}").into())
}

fn same_roster(a: &parties::PartyResult, b: &parties::PartyResult) -> bool {
    let names = |v: &parties::PartyResult| {
        v.members
            .iter()
            .map(|m| (m.name.clone(), m.leader))
            .collect::<BTreeSet<_>>()
    };
    a.members.len() == 2 && b.members.len() == 2 && names(a) == names(b)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut caster = connect_character(clients, 0xc3, 1).await?;
    let mut peer = connect_character(clients, 0xc4, 0).await?;
    if caster.join_snapshot().game_session_id == peer.join_snapshot().game_session_id
        || !party(&mut caster, None).await?.members.is_empty()
        || !party(&mut peer, None).await?.members.is_empty()
    {
        return Err("step=two-admissions observed=duplicate-session-or-nonempty-party need=two-clean-characters".into());
    }
    row.pass("two-admissions");

    let caster_start = tile(caster.world_spatial().actor_position)?;
    let peer_start = tile(peer.world_spatial().actor_position)?;
    let path = route(world, caster_start, |at| {
        at != peer_start
            && at.floor == peer_start.floor
            && at.x.abs_diff(peer_start.x).max(at.y.abs_diff(peer_start.y)) <= 1
    })
    .ok_or("step=walk observed=no-route need=walkable-adjacent-tile")?;
    for direction in path {
        let (moved, serviced) = join(
            walk(&mut caster, world, &[direction]),
            peer.service_liveness(Duration::from_millis(10)),
        )
        .await;
        moved?;
        serviced?;
        caster.take_events();
        peer.take_events();
    }
    caster.service_liveness(Duration::from_millis(100)).await?;
    peer.service_liveness(Duration::from_millis(100)).await?;
    party(
        &mut caster,
        Some(parties::PartyIntent::Invite(own_ref(&peer)?)),
    )
    .await?;
    let invitations = party(&mut peer, None).await?;
    let invitation = invitations
        .invitations
        .first()
        .ok_or("step=invite observed=no-invitation need=peer-party-delta")?
        .handle;
    party(&mut peer, Some(parties::PartyIntent::Accept(invitation))).await?;
    if !same_roster(
        &party(&mut caster, None).await?,
        &party(&mut peer, None).await?,
    ) {
        return Err("step=accept observed=roster-mismatch need=matching-two-member-views".into());
    }
    row.pass("walk-invite-accept");

    let ranking = oteryn_dev_client::client_runtime::highscores::command(
        oteryn_dev_client::client_runtime::highscores::wire::HighscoresQuery {
            world: None,
            details: false,
            category: 1,
            vocation: 0,
            page: 1,
            snapshot_id: 0,
        },
    )
    .map_err(|e| format!("ranking input: {e:?}"))?;
    let reply = caster
        .session_mut()
        .gated_command(ranking.capability, ranking.command_type, &ranking.payload)
        .await?;
    let page = oteryn_dev_client::client_runtime::highscores::wire::decode_highscores_result(
        &reply.payload,
    )
    .map_err(|e| format!("ranking result: {e:?}"))?;
    let own = page
        .own_row
        .ok_or("step=spell-select observed=no-own-ranking-row need=durable-vocation")?;
    let (words, slot) = match oteryn_gameplay_progression::wheel::base_vocation(&own.vocation) {
        Some("knight") => ("utito mas sio", 0),
        Some("paladin") => ("utamo mas sio", 1),
        Some("sorcerer") => ("utori mas sio", 2),
        Some("druid") => ("utura mas sio", 3),
        Some("monk") => ("utevo mas sio", 4),
        _ => {
            return Err(format!(
                "step=spell-select observed=vocation-{} need=party-spell-vocation",
                own.vocation
            )
            .into());
        }
    };
    let index = spell_index(&spell_book()?, words)?;
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("caster vitals absent")?;
    let cast = caster
        .cast_spell(index, casts::SpellTarget::None, false)
        .await?;
    let after = caster.actor_vitals().copied().ok_or("caster vitals lost")?;
    if cast.status != CommandStatus::Accepted
        || cast.disposition != casts::SpellCastDisposition::Cast
        || cast.actor_vitals_delta.as_ref().map(|d| d.value) != Some(after)
        || after.mana >= before.mana
    {
        return Err(format!(
            "step=cast observed={cast:?} before={before:?} after={after:?} need=cast-and-mana-delta"
        )
        .into());
    }
    row.pass("cast-reply-and-mana");
    let caster_effects = effects(&mut caster).await?;
    let peer_effects = effects(&mut peer).await?;
    if caster_effects.party_spell_remaining_ms[slot] == 0
        || peer_effects.party_spell_remaining_ms[slot] == 0
    {
        return Err(format!("step=member-condition observed=caster-{caster_effects:?}-peer-{peer_effects:?} need=nonzero-slot-{slot}").into());
    }
    row.pass("member-condition-deltas");
    let refused_before = caster
        .actor_vitals()
        .copied()
        .ok_or("refusal vitals absent")?;
    let refused = caster
        .cast_spell(index, casts::SpellTarget::None, false)
        .await?;
    if refused.status != CommandStatus::Accepted
        || refused.disposition != casts::SpellCastDisposition::CoolingDown
        || refused.actor_vitals_delta.is_some()
        || caster.actor_vitals().copied() != Some(refused_before)
    {
        return Err(format!(
            "step=cooldown-refusal observed={refused:?} need=CoolingDown-without-delta"
        )
        .into());
    }
    row.pass("cooldown-refusal");

    let sessions = [
        caster.join_snapshot().game_session_id,
        peer.join_snapshot().game_session_id,
    ];
    logout(&mut peer).await?;
    logout(&mut caster).await?;
    drop((caster, peer));
    let mut caster = connect_character(clients, 0xc5, 1).await?;
    let mut peer = connect_character(clients, 0xc6, 0).await?;
    if caster.join_snapshot().game_session_id == sessions[0]
        || peer.join_snapshot().game_session_id == sessions[1]
        || effects(&mut caster).await?.party_spell_remaining_ms[slot] == 0
        || effects(&mut peer).await?.party_spell_remaining_ms[slot] == 0
    {
        return Err(format!("step=fresh-admission observed=missing-slot-{slot}-or-reused-session need=durable-condition-reload").into());
    }
    row.pass("fresh-admission-condition");
    if !party(&mut peer, None).await?.members.is_empty() {
        party(&mut peer, Some(parties::PartyIntent::Leave)).await?;
    }
    if !party(&mut caster, None).await?.members.is_empty() {
        party(&mut caster, Some(parties::PartyIntent::Leave)).await?;
    }
    logout(&mut peer).await?;
    logout(&mut caster).await?;
    row.pass("final-logout");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_requires_every_unique_step() {
        let mut row = Row::new();
        for step in &STEPS[..STEPS.len() - 1] {
            row.pass(step);
        }
        assert!(!STEPS.iter().all(|s| row.passed.contains(s)));
        row.pass(STEPS[STEPS.len() - 1]);
        assert!(STEPS.iter().all(|s| row.passed.contains(s)) && !row.invalid);
    }
}
