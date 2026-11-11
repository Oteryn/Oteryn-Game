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

pub(super) fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
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

async fn effects(client: &mut DevClientSession) -> TestResult<Vec<u8>> {
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
    Ok(reply.payload)
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

async fn flow(clients: &SeamClients<'_>) -> TestResult<Observed> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut caster = connect_character(clients, 0x65, 1).await?;
    let row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
        let mut peer = connect_character(clients, 0x66, 0).await?;
        let row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
            let page =
                oteryn_dev_client::client_runtime::highscores::wire::decode_highscores_result(
                    &reply.payload,
                )
                .map_err(|e| format!("ranking result: {e:?}"))?;
            let own = page.own_row.ok_or("step=vocation no own ranking row")?;
            let (words, slot) =
                match oteryn_gameplay_progression::wheel::base_vocation(&own.vocation) {
                    Some("knight") => ("utito mas sio", 0),
                    Some("paladin") => ("utamo mas sio", 1),
                    Some("sorcerer") => ("utori mas sio", 2),
                    Some("druid") => ("utura mas sio", 3),
                    Some("monk") => ("utevo mas sio", 4),
                    _ => {
                        return Err(
                            format!("step=vocation {} has no party spell", own.vocation).into()
                        );
                    }
                };
            let index = spell_index(words)?;

            // The party gate: the same spell cast while solo must not be cast.
            let vitals = caster
                .actor_vitals()
                .copied()
                .ok_or("caster vitals absent")?;
            let solo = cast_through_cooldown(&mut caster, index).await?;
            if solo.disposition == casts::SpellCastDisposition::Cast
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
            let invitation = party(&mut peer, None)
                .await?
                .invitations
                .first()
                .ok_or("step=invite no invitation")?
                .handle;
            party(&mut peer, Some(parties::PartyIntent::Accept(invitation))).await?;

            let before = caster
                .actor_vitals()
                .copied()
                .ok_or("caster vitals absent")?;
            let cast = cast_through_cooldown(&mut caster, index).await?;
            let after = caster.actor_vitals().copied().ok_or("caster vitals lost")?;
            if cast.disposition != casts::SpellCastDisposition::Cast || after.mana >= before.mana {
                return Err(format!(
                    "step=cast observed {cast:?} before={before:?} after={after:?}"
                )
                .into());
            }
            let caster_effects = effects(&mut caster).await?;
            let member_effects = effects(&mut peer).await?;
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
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            row_logout(&mut peer).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut caster).await?;
    }
    row_result
}

/// The shared observation, computed at most once per process.
pub(super) async fn observed(clients: &SeamClients<'_>) -> &'static Result<Observed, String> {
    OBSERVED
        .get_or_init(|| async {
            match row_complete(flow(clients)).await {
                Ok(Ok(value)) => Ok(value),
                Ok(Err(error)) => Err(error.to_string().replace(' ', "_")),
                Err(_) => Err("timeout".to_owned()),
            }
        })
        .await
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
        let written = (|| -> TestResult {
            capture("s13c-caster-effects.bin", &value.caster_effects)?;
            capture("s13c-member-effects.bin", &value.member_effects)?;
            capture(
                "s13c-expected.txt",
                format!("slot={}\nwords={}\n", value.slot, value.words).as_bytes(),
            )?;
            Ok(())
        })();
        if let Err(error) = written {
            evidence(&format!(
                "e2e_all scenario=S13c.client status=FAIL evidence=capture-write:{error}"
            ));
            return;
        }
    }
    report("S13c", observed, |value| {
        format!("slot={} words={}", value.slot, value.words)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vitals_text_preserves_boundaries_and_boolean() {
        let vitals = oteryn_protocol_oteryn::actor_spell::ActorVitals {
            health: 0,
            max_health: u32::MAX,
            mana: 1,
            max_mana: 2,
            soul: 3,
            harmony: 4,
            serene: true,
        };
        assert_eq!(vitals_line(&vitals), "0 4294967295 1 2 3 4 1");
    }
}
