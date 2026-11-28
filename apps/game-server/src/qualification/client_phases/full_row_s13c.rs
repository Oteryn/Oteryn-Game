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
    let result = verdict::indexed_timeout(Duration::from_secs(240), journey(&mut row, clients))
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

pub(super) async fn effects(
    client: &mut DevClientSession,
) -> TestResult<party_effects::PartyEffects> {
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

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "step=logout session={} observed={outcome:?} need=accepted-logout",
            uuid_text(session.as_bytes()),
        )
        .into());
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

pub(super) fn spell_book() -> TestResult<crate::spell::SpellBook> {
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

pub(super) fn spell_index(
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<std::num::NonZeroU32> {
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

const CASTER_NAME: &str = "S Thirteen C Caster";

pub(super) fn clean_party(state: &parties::PartyResult) -> bool {
    state.members.is_empty() && state.invitations.is_empty()
}

pub(super) fn caster_invitation(state: &parties::PartyResult) -> TestResult<u64> {
    match state.invitations.as_slice() {
        [entry]
            if state.members.is_empty() && entry.name == CASTER_NAME && entry.expires_at > 0 =>
        {
            Ok(entry.handle)
        }
        _ => Err(format!(
            "step=invite expected one invitation from {CASTER_NAME}, observed {state:?}"
        )
        .into()),
    }
}

pub(super) fn same_roster(a: &parties::PartyResult, b: &parties::PartyResult) -> bool {
    let expected = BTreeSet::from([
        (CASTER_NAME.to_owned(), true),
        ("Packet One".to_owned(), false),
    ]);
    [a, b].into_iter().all(|state| {
        state.members.len() == 2
            && state.invitations.is_empty()
            && state
                .members
                .iter()
                .map(|m| (m.name.clone(), m.leader))
                .collect::<BTreeSet<_>>()
                == expected
    })
}

fn cast_requirement(disposition: casts::SpellCastDisposition) -> &'static str {
    use casts::SpellCastDisposition as D;
    match disposition {
        D::LevelTooLow => "player-earned-party-spell-level",
        D::NotEnoughMana => "party-spell-mana",
        D::MagicLevelTooLow => "player-trained-party-spell-magic-level",
        D::NotAvailable => "learned-party-spell-and-current-premium",
        _ => "cast-and-mana-delta",
    }
}

pub(super) async fn prepare_caster(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "S13c",
        "sorcerer-level-32,premium",
        "Ordinary leveling to the party spell prerequisite exceeds the row budget",
    );
    let mut initial = connect_character(clients, tag, 1).await?;
    let session = initial.join_snapshot().game_session_id;
    let before = full_e2ef5::level(&mut initial).await?;
    if before != 1 {
        return Err(format!(
            "step=fixture expected level-1 caster before grant, observed {before}"
        )
        .into());
    }
    let receipt = request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        "s13-party".into(),
    )
    .await?;
    evidence(&format!(
        "S13C_FIXTURE profile=s13-party level_before={before} target_level=32 receipt={receipt:?}"
    ));
    character_prep::grant_premium(&initial).await?;
    logout(&mut initial).await?;
    drop(initial);
    let mut caster = connect_character(clients, tag, 1).await?;
    if caster.join_snapshot().game_session_id == session {
        return Err("seeded caster reused its previous session".into());
    }
    let level = full_e2ef5::level(&mut caster).await?;
    let vocation = full_e2ef5::current_vocation(&mut caster).await?;
    let vitals = caster
        .actor_vitals()
        .ok_or("fixture caster vitals absent")?;
    evidence(&format!(
        "S13C_FIXTURE session={} level={level} vocation={vocation:?} max_health={} max_mana={}",
        uuid_text(caster.join_snapshot().game_session_id.as_bytes()),
        vitals.max_health,
        vitals.max_mana
    ));
    if level != 32
        || vocation != crate::spell::Vocation::Sorcerer
        || vitals.max_health != 305
        || vitals.max_mana != 810
    {
        return Err("step=fixture seeded caster did not serve level-32 Sorcerer stats".into());
    }
    use oteryn_protocol_oteryn::{character_info, premium_overview};
    let reply = caster
        .session_mut()
        .gated_command(
            character_info::CAPABILITY_CHARACTER_INFO_V1,
            character_info::COMMAND_TYPE_CHARACTER_INFO,
            &[premium_overview::OPERATION],
        )
        .await?;
    let overview = premium_overview::Overview::decode(&reply.payload)
        .ok_or("S13c fixture Premium overview malformed")?;
    evidence(&format!(
        "S13C_FIXTURE premium_status={:?} premium_state={:?} authority_revision={} observed_at_us={} valid_for_us={} sequence={}",
        reply.status,
        overview.state,
        overview.authority_revision,
        overview.observed_at_us,
        overview.valid_for_us,
        reply.server_sequence,
    ));
    if reply.status != CommandStatus::Accepted
        || overview.state != premium_overview::State::Active
        || overview.valid_for_us == 0
    {
        return Err("step=fixture caster did not serve current Premium".into());
    }
    Ok(caster)
}

pub(super) async fn require_seeded_caster(
    caster: &mut DevClientSession,
    spell: &crate::spell::SpellDefinition,
) -> TestResult {
    let level = full_e2ef5::prepare_spell_access(caster, spell).await?;
    let mana = caster.actor_vitals().ok_or("caster vitals absent")?.mana;
    if level != 32 || mana < 216 {
        return Err(format!(
            "step=seeded-caster expected level 32 and 216 mana, observed level={level} mana={mana}"
        )
        .into());
    }
    Ok(())
}

pub(super) async fn walk_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    direction: StepDirection,
) -> TestResult {
    let before = tile(client.world_spatial().actor_position)?;
    let expected = destination(world, before, direction).ok_or("planned step has no landing")?;
    let outcome = client.step(direction).await?;
    let actual = tile(client.world_spatial().actor_position)?;
    if outcome.disposition != StepDisposition::Moved || before == expected || actual != expected {
        return Err(format!(
            "step=walk from={before:?} expected={expected:?} actual={actual:?} disposition={:?}",
            outcome.disposition
        )
        .into());
    }
    Ok(())
}

pub(super) async fn require_clear_effects(client: &mut DevClientSession) -> TestResult {
    if effects(client).await?.party_spell_remaining_ms != [0; 5] {
        return Err("party spell condition already present before cast".into());
    }
    Ok(())
}

pub(super) fn enchant_effect(snapshot: party_effects::PartyEffects) -> TestResult<u32> {
    let [0, 0, remaining @ 1..=120_000, 0, 0] = snapshot.party_spell_remaining_ms else {
        return Err(format!(
            "unexpected Enchant Party timers: {:?}",
            snapshot.party_spell_remaining_ms
        )
        .into());
    };
    Ok(remaining)
}

pub(super) async fn saved_enchant_timers(clients: &SeamClients<'_>) -> TestResult<[u32; 2]> {
    use oteryn_gameplay_domain::domain::condition::ConditionValues;
    use oteryn_gameplay_foundation::foundation::condition::ConditionStore;

    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut saved = [0; 2];
    let characters = clients
        .characters
        .first_chunk::<2>()
        .ok_or("missing party character identities")?;
    for (timer, character) in saved.iter_mut().zip(characters) {
        let snapshot: serde_json::Value = sqlx::query_scalar(
            "SELECT snapshot FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_slice())
        .fetch_one(&mut database)
        .await?;
        let store = ConditionStore::from_persistent_snapshot(&snapshot, 0)
            .map_err(|error| format!("invalid saved condition snapshot: {error:?}"))?;
        let mut enchant = store.instances().iter().filter(|instance| {
            instance.definition().key() == "candidate:spell/enchant_party/effect-attributes"
        });
        let effect = enchant
            .next()
            .ok_or("logout did not persist Enchant Party")?;
        if enchant.next().is_some()
            || !matches!(
                effect.definition().values(),
                ConditionValues::SpellSkills {
                    duration_ms: 120_000,
                    magic_level: 1,
                    ..
                }
            )
        {
            return Err("logout did not persist the exact Enchant Party skill effect".into());
        }
        *timer = u32::try_from(
            effect
                .remaining_duration_ms(0)
                .ok_or("saved timer absent")?,
        )?;
    }
    database.close().await?;
    evidence(&format!(
        "S13C_PG_SNAPSHOT peer_ms={} caster_ms={} duration_ms=120000 magic_level=1",
        saved[0], saved[1]
    ));
    Ok(saved)
}

pub(super) async fn require_reloaded_effect(
    client: &mut DevClientSession,
    old_session: oteryn_protocol_oteryn::GameSessionId,
    remaining_before: u32,
    saved_remaining: u32,
) -> TestResult {
    let remaining = enchant_effect(effects(client).await?)?;
    if client.join_snapshot().game_session_id == old_session
        || remaining >= remaining_before
        || remaining > saved_remaining
    {
        return Err(format!("step=fresh-admission expected continued countdown below {remaining_before}, observed {remaining}").into());
    }
    Ok(())
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
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_Q_S13C_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let scope = SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let clients = &scope;
    let mut caster = prepare_caster(clients, 0xc3).await?;
    let mut peer = connect_character(clients, 0xc4, 0).await?;
    if caster.join_snapshot().game_session_id == peer.join_snapshot().game_session_id
        || !clean_party(&party(&mut caster, None).await?)
        || !clean_party(&party(&mut peer, None).await?)
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
    if path.is_empty() {
        return Err("party staging route does not move the caster".into());
    }
    for direction in path {
        let (moved, serviced) = join(
            walk_once(&mut caster, world, direction),
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
    let invitation = caster_invitation(&party(&mut peer, None).await?)?;
    party(&mut peer, Some(parties::PartyIntent::Accept(invitation))).await?;
    if !same_roster(
        &party(&mut caster, None).await?,
        &party(&mut peer, None).await?,
    ) {
        return Err("step=accept observed=roster-mismatch need=matching-two-member-views".into());
    }
    row.pass("walk-invite-accept");

    let words = "utori mas sio";
    let book = spell_book()?;
    let index = spell_index(&book, words)?;
    let spell = book.indexed(index).ok_or("indexed party spell absent")?;
    require_seeded_caster(&mut caster, spell).await?;
    require_clear_effects(&mut caster).await?;
    require_clear_effects(&mut peer).await?;
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
        || before.mana.checked_sub(216) != Some(after.mana)
    {
        return Err(format!(
            "step=cast spell={words:?} required_level={} mana={:?} premium={} learned_required={} observed={cast:?} before={before:?} after={after:?} need={}",
            spell.level, spell.mana, spell.premium, spell.learning_required, cast_requirement(cast.disposition)
        )
        .into());
    }
    row.pass("cast-reply-and-mana");
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
    let caster_remaining = enchant_effect(effects(&mut caster).await?)?;
    let peer_remaining = enchant_effect(effects(&mut peer).await?)?;
    row.pass("member-condition-deltas");

    let sessions = [
        caster.join_snapshot().game_session_id,
        peer.join_snapshot().game_session_id,
    ];
    logout(&mut peer).await?;
    logout(&mut caster).await?;
    drop((caster, peer));
    let [saved_peer, saved_caster] = saved_enchant_timers(clients).await?;
    let mut caster = connect_character(clients, 0xc5, 1).await?;
    let mut peer = connect_character(clients, 0xc6, 0).await?;
    require_reloaded_effect(&mut caster, sessions[0], caster_remaining, saved_caster).await?;
    require_reloaded_effect(&mut peer, sessions[1], peer_remaining, saved_peer).await?;
    row.pass("fresh-admission-condition");
    for client in [&mut peer, &mut caster] {
        if !party(client, None).await?.members.is_empty() {
            party(client, Some(parties::PartyIntent::Leave)).await?;
        }
        if !clean_party(&party(client, None).await?) {
            return Err("party state remained after leave".into());
        }
        logout(client).await?;
    }
    row.pass("final-logout");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starter_spell_gate_identifies_which_prerequisite_failed() {
        assert_eq!(
            cast_requirement(casts::SpellCastDisposition::LevelTooLow),
            "player-earned-party-spell-level"
        );
        assert_eq!(
            cast_requirement(casts::SpellCastDisposition::NotEnoughMana),
            "party-spell-mana"
        );
    }

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
