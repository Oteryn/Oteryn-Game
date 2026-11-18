//! S13c client replay capture, and the shared party-spell observation S13a and S13b reuse. Two
//! admitted characters (caster Character one, member Character zero) form a party; the caster
//! first casts its vocation's party spell while still solo (the party gate refusal), then casts it
//! inside the party (a mana delta and a condition on both members), queries PARTY EFFECTS on both
//! sockets, and casts again during the cooldown (a refusal). The raw PARTY EFFECTS payloads are
//! written byte for byte (`s13c-caster-effects.bin`, `s13c-member-effects.bin`); the observed cast
//! outcomes and vitals go to `s13c-expected.txt`. The journey runs once per process; S13a and S13b
//! read its observation instead of repeating the party formation.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{actor_spell as casts, party as parties, party_effects};
use tokio::sync::OnceCell;

pub(super) struct Observed {
    pub(super) words: &'static str,
    pub(super) slot: usize,
    pub(super) solo_disposition: casts::SpellCastDisposition,
    pub(super) solo_status: CommandStatus,
    pub(super) mana_before: u32,
    pub(super) mana_after: u32,
    pub(super) vitals_before: String,
    pub(super) vitals_after: String,
    pub(super) caster_effects: Vec<u8>,
    pub(super) member_effects: Vec<u8>,
    pub(super) refusal_disposition: casts::SpellCastDisposition,
    pub(super) refusal_status: CommandStatus,
}

static OBSERVED: OnceCell<Result<Observed, String>> = OnceCell::const_new();

fn vitals_line(vitals: &oteryn_protocol_oteryn::actor_spell::ActorVitals) -> String {
    format!(
        "{} {} {} {} {} {} {}",
        vitals.health,
        vitals.max_health,
        vitals.mana,
        vitals.max_mana,
        vitals.soul,
        vitals.harmony,
        u8::from(vitals.serene)
    )
}

pub(super) fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "S13_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
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
    let state =
        parties::decode_party_result(&reply.payload).map_err(|e| format!("party decode: {e:?}"))?;
    if reply.status != CommandStatus::Accepted || state.disposition != parties::PartyDisposition::Ok
    {
        return Err(format!(
            "step=party status={:?} disposition={:?}",
            reply.status, state.disposition
        )
        .into());
    }
    Ok(state)
}

fn party_effect_present(in_party: bool, remaining: &[u32], slot: usize) -> bool {
    in_party && remaining.get(slot).is_some_and(|ms| *ms > 0)
}

fn party_mana_paid(before: u32, after: u32, expected: u32) -> bool {
    before.checked_sub(expected) == Some(after)
}

fn solo_gate_proven(
    disposition: casts::SpellCastDisposition,
    status: CommandStatus,
    unchanged: bool,
) -> bool {
    disposition == casts::SpellCastDisposition::Rejected
        && status == CommandStatus::Rejected
        && unchanged
}

#[test]
fn solo_gate_requires_wire_rejection_and_unchanged_vitals() {
    use casts::SpellCastDisposition as D;
    assert!(!solo_gate_proven(
        D::CoolingDown,
        CommandStatus::Accepted,
        true
    ));
    assert!(!solo_gate_proven(
        D::TargetRequired,
        CommandStatus::Accepted,
        true
    ));
    assert!(!solo_gate_proven(
        D::Rejected,
        CommandStatus::Rejected,
        false
    ));
    assert!(solo_gate_proven(D::Rejected, CommandStatus::Rejected, true));
}

#[test]
fn party_spell_requires_exact_cost_and_effect_on_each_participant() {
    assert!(!party_mana_paid(100, 99, 60));
    assert!(party_mana_paid(100, 40, 60));
    assert!(!party_effect_present(true, &[15, 0], 1));
    assert!(!party_effect_present(false, &[0, 15], 1));
    assert!(party_effect_present(true, &[0, 15], 1));
}

async fn effects(
    client: &mut DevClientSession,
) -> TestResult<(Vec<u8>, party_effects::PartyEffects)> {
    let reply = client
        .session_mut()
        .gated_command(
            party_effects::CAPABILITY_PARTY_EFFECTS_V1,
            party_effects::COMMAND_TYPE_PARTY_EFFECTS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=effects status={:?}", reply.status).into());
    }
    let view = party_effects::PartyEffects::decode(&reply.payload)
        .ok_or("step=effects malformed reply")?;
    Ok((reply.payload, view))
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

fn spell_parameters(words: &str) -> TestResult<(std::num::NonZeroU32, u32)> {
    use crate::spell::{Execution, party::PartyBuffSpecExt as _};
    let book = native_spell_book()?;
    let spell = &book
        .spoken(words)
        .ok_or_else(|| format!("compiled spell missing: {words}"))?
        .spell;
    let Execution::PartyBuff(buff) = &spell.execution else {
        return Err(format!("compiled spell is not a party buff: {words}").into());
    };
    let mana = buff
        .mana_cost(2)
        .map_err(|error| format!("two-member party cost: {error:?}"))?;
    let index = (1..=book.source_len())
        .filter_map(|n| u32::try_from(n).ok().and_then(std::num::NonZeroU32::new))
        .find(|n| book.indexed(*n).is_some_and(|s| s.key == spell.key))
        .ok_or_else(|| format!("compiled spell has no index: {words}"))?;
    Ok((index, mana))
}

/// A previous row may have cast the same spell. Bound retries and retain the vitals
/// immediately before the attempt that produced the terminal result.
async fn cast_through_cooldown(
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    index: std::num::NonZeroU32,
) -> TestResult<(casts::ActorVitals, oteryn_dev_client::CastOutcome)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    tokio::time::timeout_at(deadline, async {
        loop {
            let before = client
                .actor_vitals()
                .copied()
                .ok_or("caster vitals absent")?;
            let (outcome, serviced) = join(
                client.cast_spell(index, casts::SpellTarget::None, false),
                peer.service_liveness(Duration::from_millis(50)),
            )
            .await;
            let outcome = outcome?;
            serviced?;
            peer.take_events();
            if outcome.disposition != casts::SpellCastDisposition::CoolingDown {
                return Ok((before, outcome));
            }
            let (caster_idle, peer_idle) = join(
                client.service_liveness(Duration::from_secs(2)),
                peer.service_liveness(Duration::from_secs(2)),
            )
            .await;
            caster_idle?;
            peer_idle?;
            client.take_events();
            peer.take_events();
        }
    })
    .await
    .map_err(|_| "step=cast still cooling down after 150s")?
}

async fn flow(clients: &SeamClients<'_>) -> TestResult<Observed> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut caster = connect_character(clients, 0x65, 1).await?;
    let mut peer = connect_character(clients, 0x66, 0).await?;
    for client in [&mut caster, &mut peer] {
        if !party(client, None).await?.members.is_empty() {
            party(client, Some(parties::PartyIntent::Leave)).await?;
        }
    }
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
    let own = page.own_row.ok_or("step=vocation no own ranking row")?;
    let (words, slot) = match oteryn_gameplay_progression::wheel::base_vocation(&own.vocation) {
        Some("knight") => ("utito mas sio", 0),
        Some("paladin") => ("utamo mas sio", 1),
        Some("sorcerer") => ("utori mas sio", 2),
        Some("druid") => ("utura mas sio", 3),
        Some("monk") => ("utevo mas sio", 4),
        _ => return Err(format!("step=vocation {} has no party spell", own.vocation).into()),
    };
    let (index, mana_cost) = spell_parameters(words)?;

    let caster_start = tile(caster.world_spatial().actor_position)?;
    let peer_start = tile(peer.world_spatial().actor_position)?;
    let path = route(world, caster_start, |at| {
        at != peer_start
            && at.floor == peer_start.floor
            && at.x.abs_diff(peer_start.x).max(at.y.abs_diff(peer_start.y)) <= 1
    })
    .ok_or("step=walk no route beside the member")?;
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
    if !party(&mut caster, None).await?.members.is_empty()
        || !party(&mut peer, None).await?.members.is_empty()
    {
        return Err("step=solo-gate the prior party still exists".into());
    }
    // The party gate: the same spell cast while solo must not be cast.
    let (vitals, solo) = cast_through_cooldown(&mut caster, &mut peer, index).await?;
    if !solo_gate_proven(
        solo.disposition,
        solo.status,
        caster.actor_vitals().copied() == Some(vitals),
    ) {
        return Err(format!(
            "step=solo-gate observed {solo:?}, expected a refusal without a vitals change"
        )
        .into());
    }

    party(
        &mut caster,
        Some(parties::PartyIntent::Invite(own_ref(&peer)?)),
    )
    .await?;
    let invitation = party(&mut peer, None)
        .await?
        .invitations
        .first()
        .ok_or("step=invite no invitation")?
        .handle;
    party(&mut peer, Some(parties::PartyIntent::Accept(invitation))).await?;

    let caster_roster = party(&mut caster, None).await?;
    let peer_roster = party(&mut peer, None).await?;
    if caster_roster.members.len() != 2
        || peer_roster.members.len() != 2
        || !caster_roster.members.iter().all(|member| {
            peer_roster
                .members
                .iter()
                .any(|peer| peer.name == member.name && peer.leader == member.leader)
        })
    {
        return Err("step=party expected both views of the two-member roster".into());
    }
    let (_, caster_before) = effects(&mut caster).await?;
    let (_, member_before) = effects(&mut peer).await?;
    let (before, cast) = cast_through_cooldown(&mut caster, &mut peer, index).await?;
    let after = caster.actor_vitals().copied().ok_or("caster vitals lost")?;
    if cast.status != CommandStatus::Accepted
        || cast.disposition != casts::SpellCastDisposition::Cast
        || cast.actor_vitals_delta.as_ref().map(|delta| delta.value) != Some(after)
        || !party_mana_paid(before.mana, after.mana, mana_cost)
    {
        return Err(
            format!("step=cast observed {cast:?} before={before:?} after={after:?}").into(),
        );
    }
    let (caster_effects, caster_after) = effects(&mut caster).await?;
    let (member_effects, member_after) = effects(&mut peer).await?;
    for (before, after) in [(caster_before, caster_after), (member_before, member_after)] {
        if !party_effect_present(after.in_party, &after.party_spell_remaining_ms, slot)
            || after.party_spell_remaining_ms[slot] <= before.party_spell_remaining_ms[slot]
        {
            return Err(format!(
                "step=effects slot {slot} did not refresh: {before:?} -> {after:?}"
            )
            .into());
        }
    }
    let refused_vitals = caster.actor_vitals().copied();
    let refused = caster
        .cast_spell(index, casts::SpellTarget::None, false)
        .await?;
    if refused.disposition != casts::SpellCastDisposition::CoolingDown
        || caster.actor_vitals().copied() != refused_vitals
    {
        return Err(format!("step=cooldown observed {refused:?}").into());
    }
    for client in [&mut peer, &mut caster] {
        party(client, Some(parties::PartyIntent::Leave)).await?;
        let _ = client.session_mut().logout().await;
    }
    Ok(Observed {
        words,
        slot,
        solo_disposition: solo.disposition,
        solo_status: solo.status,
        mana_before: before.mana,
        mana_after: after.mana,
        vitals_before: vitals_line(&before),
        vitals_after: vitals_line(&after),
        caster_effects,
        member_effects,
        refusal_disposition: refused.disposition,
        refusal_status: refused.status,
    })
}

/// The shared observation, computed at most once per process.
pub(super) async fn observed(clients: &SeamClients<'_>) -> &'static Result<Observed, String> {
    let result = OBSERVED
        .get_or_init(|| async {
            match verdict::indexed_timeout(Duration::from_secs(420), flow(clients)).await {
                Ok(Ok(value)) => Ok(value),
                Ok(Err(error)) => Err(error.to_string().replace(' ', "_")),
                Err(_) => Err("timeout".to_owned()),
            }
        })
        .await;
    if result.is_err() {
        verdict::Harness::global().shared_failure("S13-party-observation");
    }
    result
}

pub(super) fn report(
    row: &str,
    observed: &Result<Observed, String>,
    detail: impl FnOnce(&Observed) -> String,
) {
    match observed {
        Ok(value) => evidence(&format!(
            "e2e_all scenario={row}.client status=CAPTURED evidence={}",
            detail(value)
        )),
        Err(error) => evidence(&format!(
            "e2e_all scenario={row}.client status=FAIL evidence={error}"
        )),
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let observed = observed(clients).await;
    if let Ok(value) = observed {
        capture("s13c-caster-effects.bin", &value.caster_effects);
        capture("s13c-member-effects.bin", &value.member_effects);
        capture(
            "s13c-expected.txt",
            format!("slot={}\nwords={}\n", value.slot, value.words).as_bytes(),
        );
    }
    report("S13c", observed, |value| {
        format!("slot={} words={}", value.slot, value.words)
    });
}
