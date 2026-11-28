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

async fn effects(client: &mut DevClientSession) -> TestResult<(Vec<u8>, u64)> {
    let reply = client
        .session_mut()
        .gated_command(
            party_effects::CAPABILITY_PARTY_EFFECTS_V1,
            party_effects::COMMAND_TYPE_PARTY_EFFECTS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || party_effects::PartyEffects::decode(&reply.payload).is_none()
    {
        return Err(format!("step=effects status={:?} malformed", reply.status).into());
    }
    Ok((reply.payload, reply.server_sequence))
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

fn spell_index(words: &str) -> TestResult<std::num::NonZeroU32> {
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
    let book = catalog.into_spell_book_with_selections(&selected)?;
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

/// Casts until the server stops answering `CoolingDown` (an earlier row may have cast).
async fn cast_through_cooldown(
    client: &mut DevClientSession,
    index: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::CastOutcome> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        let outcome = client
            .cast_spell(index, casts::SpellTarget::None, false)
            .await?;
        if outcome.disposition != casts::SpellCastDisposition::CoolingDown
            || tokio::time::Instant::now() >= deadline
        {
            return Ok(outcome);
        }
        client.service_liveness(Duration::from_secs(2)).await?;
        client.take_events();
    }
}

async fn flow(clients: &SeamClients<'_>, strict: bool) -> TestResult<Observed> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut caster = if strict {
        full_row_s13c::prepare_caster(clients, 0x65).await?
    } else {
        connect_character(clients, 0x65, 1).await?
    };
    use full_row_s13c as full;
    let mut peer = connect_character(clients, 0x66, 0).await?;
    for client in [&mut caster, &mut peer] {
        let state = party(client, None).await?;
        if strict && !full::clean_party(&state) {
            return Err("party state already present".into());
        }
        if !state.members.is_empty() {
            party(client, Some(parties::PartyIntent::Leave)).await?;
        }
    }
    let (words, slot, index) = if strict {
        let words = "utori mas sio";
        let book = full_row_s13c::spell_book()?;
        let index = full_row_s13c::spell_index(&book, words)?;
        let spell = book.indexed(index).ok_or("indexed party spell absent")?;
        full_row_s13c::require_seeded_caster(&mut caster, spell).await?;
        (words, 2, index)
    } else {
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
        let index = spell_index(words)?;
        (words, slot, index)
    };

    // The party gate: the same spell cast while solo must not be cast.
    let vitals = caster
        .actor_vitals()
        .copied()
        .ok_or("caster vitals absent")?;
    if strict {
        full::require_clear_effects(&mut caster).await?;
        full::require_clear_effects(&mut peer).await?;
    }
    let solo = if strict {
        caster
            .cast_spell(index, casts::SpellTarget::None, false)
            .await?
    } else {
        cast_through_cooldown(&mut caster, index).await?
    };
    if (strict
        && (solo.disposition != casts::SpellCastDisposition::Rejected
            || solo.status != CommandStatus::Rejected
            || solo.actor_vitals_delta.is_some()))
        || solo.disposition == casts::SpellCastDisposition::Cast
        || caster.actor_vitals().copied() != Some(vitals)
    {
        return Err(format!(
            "step=solo-gate observed {solo:?}, expected a refusal without a vitals change"
        )
        .into());
    }

    let caster_start = tile(caster.world_spatial().actor_position)?;
    let peer_start = tile(peer.world_spatial().actor_position)?;
    let path = route(world, caster_start, |at| {
        at != peer_start
            && at.floor == peer_start.floor
            && at.x.abs_diff(peer_start.x).max(at.y.abs_diff(peer_start.y)) <= 1
    })
    .ok_or("step=walk no route beside the member")?;
    if strict && path.is_empty() {
        return Err("party staging route does not move the caster".into());
    }
    for direction in path {
        let (moved, serviced) = join(
            async {
                if strict {
                    full::walk_once(&mut caster, world, direction).await
                } else {
                    walk(&mut caster, world, &[direction]).await
                }
            },
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
    let invitation = if strict {
        full::caster_invitation(&invitations)?
    } else {
        invitations
            .invitations
            .first()
            .ok_or("step=invite no invitation")?
            .handle
    };
    party(&mut peer, Some(parties::PartyIntent::Accept(invitation))).await?;

    if strict
        && !full::same_roster(
            &party(&mut caster, None).await?,
            &party(&mut peer, None).await?,
        )
    {
        return Err("party accept did not produce matching two-member rosters".into());
    }
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("caster vitals absent")?;
    let cast = if strict {
        caster
            .cast_spell(index, casts::SpellTarget::None, false)
            .await?
    } else {
        cast_through_cooldown(&mut caster, index).await?
    };
    let after = caster.actor_vitals().copied().ok_or("caster vitals lost")?;
    if cast.disposition != casts::SpellCastDisposition::Cast
        || after.mana >= before.mana
        || (strict
            && (cast.status != CommandStatus::Accepted
                || cast.actor_vitals_delta.as_ref().map(|delta| delta.value) != Some(after)
                || before.mana.checked_sub(216) != Some(after.mana)))
    {
        return Err(
            format!("step=cast observed {cast:?} before={before:?} after={after:?}").into(),
        );
    }
    let shared_effects = if strict {
        None
    } else {
        Some((effects(&mut caster).await?.0, effects(&mut peer).await?.0))
    };
    let refused_vitals = caster.actor_vitals().copied();
    let refused = caster
        .cast_spell(index, casts::SpellTarget::None, false)
        .await?;
    if refused.disposition != casts::SpellCastDisposition::CoolingDown
        || (strict
            && (refused.status != CommandStatus::Accepted || refused.actor_vitals_delta.is_some()))
        || caster.actor_vitals().copied() != refused_vitals
    {
        return Err(format!("step=cooldown observed {refused:?}").into());
    }
    let (caster_effects, member_effects) = match shared_effects {
        Some(pair) => pair,
        None => {
            let (caster_effects, caster_sequence) = effects(&mut caster).await?;
            let caster_binding = (caster.join_snapshot().game_session_id, caster_sequence);
            let (member_effects, member_sequence) = effects(&mut peer).await?;
            let member_binding = (peer.join_snapshot().game_session_id, member_sequence);
            if caster_binding.0 == member_binding.0
                || caster_binding.1 == 0
                || member_binding.1 == 0
            {
                return Err("invalid party effects capture binding".into());
            }
            let remaining = |payload: &[u8]| {
                full::enchant_effect(
                    party_effects::PartyEffects::decode(payload)
                        .ok_or("malformed party effects capture")?,
                )
            };
            let caster_remaining = remaining(&caster_effects)?;
            let member_remaining = remaining(&member_effects)?;
            let root = std::env::temp_dir()
                .join(format!("s13c-{}", uuid_text(caster_binding.0.as_bytes())));
            std::fs::create_dir(&root)?;
            let capture = |role,
                           (session, sequence): (oteryn_protocol_oteryn::GameSessionId, u64),
                           payload: &[u8]|
             -> TestResult<_> {
                use std::io::Write;
                let path = root.join(format!(
                    "{role}-{}-{sequence}.bin",
                    uuid_text(session.as_bytes())
                ));
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)?
                    .write_all(payload)?;
                Ok(path)
            };
            let caster_file = capture("caster", caster_binding, &caster_effects)?;
            let member_file = capture("member", member_binding, &member_effects)?;
            let caster_replay = std::fs::read(caster_file)?;
            let member_replay = std::fs::read(member_file)?;
            if caster_replay != caster_effects || member_replay != member_effects {
                return Err("session-bound capture differs from live response".into());
            }
            verify_panel(slot, &caster_replay, &member_replay)?;
            evidence(&format!(
                "S13C_CLIENT_REPLAY caster_session={} caster_sequence={} member_session={} member_sequence={} status=PASS",
                uuid_text(caster_binding.0.as_bytes()),
                caster_binding.1,
                uuid_text(member_binding.0.as_bytes()),
                member_binding.1,
            ));
            full::logout(&mut peer).await?;
            full::logout(&mut caster).await?;
            drop((caster, peer));
            let [saved_peer, saved_caster] = full::saved_enchant_timers(clients).await?;
            caster = connect_character(clients, 0x67, 1).await?;
            peer = connect_character(clients, 0x68, 0).await?;
            full::require_reloaded_effect(
                &mut caster,
                caster_binding.0,
                caster_remaining,
                saved_caster,
            )
            .await?;
            full::require_reloaded_effect(
                &mut peer,
                member_binding.0,
                member_remaining,
                saved_peer,
            )
            .await?;
            (caster_replay, member_replay)
        }
    };
    for client in [&mut peer, &mut caster] {
        if !strict || !party(client, None).await?.members.is_empty() {
            party(client, Some(parties::PartyIntent::Leave)).await?;
        }
        if strict {
            if !full::clean_party(&party(client, None).await?) {
                return Err("party state remained after leave".into());
            }
            full::logout(client).await?;
        } else {
            let _ = client.session_mut().logout().await;
        }
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
            match verdict::indexed_timeout(Duration::from_secs(420), flow(clients, false)).await {
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

fn verify_panel(slot: usize, caster: &[u8], member: &[u8]) -> TestResult {
    use oteryn_dev_client::client_ui::party_effects::{PartyEffectsPanel, panel_text, request};

    let label = [
        "Train Party",
        "Protect Party",
        "Enchant Party",
        "Heal Party",
        "Enlighten Party",
    ]
    .get(slot)
    .ok_or("party spell slot out of range")?;
    for (who, payload) in [("caster", caster), ("member", member)] {
        let mut panel = PartyEffectsPanel {
            selected: true,
            ..PartyEffectsPanel::default()
        };
        if request(&mut panel) != Some(Vec::new()) || !panel.finish(true, payload) {
            return Err(format!("step=panel {who} did not decode live party effects").into());
        }
        let snapshot = panel.snapshot.ok_or("party effects snapshot absent")?;
        let remaining = snapshot.party_spell_remaining_ms[slot];
        if !snapshot.in_party
            || remaining == 0
            || snapshot
                .party_spell_remaining_ms
                .iter()
                .enumerate()
                .any(|(index, ms)| index != slot && *ms != 0)
        {
            return Err(
                format!("step=panel {who} has wrong party spell slots: {snapshot:?}").into(),
            );
        }
        let text = panel_text(&panel);
        let mut shown = text.split("\r\n").filter(|line| line.contains(" Party: "));
        let expected = format!("{label}: {}s", remaining.div_ceil(1000));
        if shown.next() != Some(expected.as_str()) || shown.next().is_some() {
            return Err(format!("step=panel {who} did not show {expected:?}: {text:?}").into());
        }
        if panel.finish(false, &[])
            || panel.snapshot != Some(snapshot)
            || panel.finish(true, &[0])
            || panel.snapshot != Some(snapshot)
            || panel_text(&panel) != "Party effects could not be refreshed. Try Refresh."
        {
            return Err(
                format!("step=panel {who} did not retain its snapshot on refresh failure").into(),
            );
        }
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = async {
        let dedicated =
            character_prep::dedicated_character(clients, "NODE_BOOT_Q_S13C_ACCOUNT").await?;
        let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
        let scope = SeamClients {
            accounts: &accounts,
            characters: &characters,
            ..*clients
        };
        flow(&scope, true).await
    }
    .await;
    let value = match result {
        Ok(value) => value,
        Err(error) => {
            evidence(&format!(
                "e2e_all scenario=S13c.client status=FAIL seconds={:.3} evidence={error}",
                started.elapsed().as_secs_f64()
            ));
            return;
        }
    };
    capture("s13c-caster-effects.bin", &value.caster_effects);
    capture("s13c-member-effects.bin", &value.member_effects);
    capture(
        "s13c-expected.txt",
        format!("slot={}\nwords={}\n", value.slot, value.words).as_bytes(),
    );
    evidence(&format!(
        "e2e_all scenario=S13c.client status=PASS seconds={:.3} evidence=live-caster-and-member-party-effects-panel",
        started.elapsed().as_secs_f64()
    ));
}
