//! Spoken casts on the ordinary client connection; no direct runtime mutation.
use super::client_combat_journey::Checks;
use super::*;
use oteryn_dev_client::{
    CastOutcome, ChatDisposition, ChatIntent, ChatOutcome, ChatSpeechMode, DevClientSession,
};
use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::cyclopedia_character as character;
use tokio::time::Instant;

const INDIVIDUAL_COOLDOWN_WORDS: &str = "exana pox";

pub(super) async fn spell_journey(
    client: &mut DevClientSession,
    pz_result: TestResult,
) -> TestResult {
    let mut checks = Checks::default();
    checks.record("C04", "pz-aggression", pz_result);
    let book = client_world::native_spell_book();
    match book {
        Ok(book) => {
            if book.spoken("exura").is_some() {
                let result = words_and_cooldown(client, book).await;
                match result {
                    Ok((mana, cooldown, group_cooldown)) => {
                        checks.record(
                            "C04",
                            "words-mana",
                            if mana {
                                Ok(())
                            } else {
                                Err("spoken exura did not spend mana".into())
                            },
                        );
                        checks.record("C04", "spell-cooldown", if cooldown {
                            Ok(())
                        } else {
                            Err("post-group retries did not prove resource-free individual cooldown refusals".into())
                        });
                        checks.record("C04", "group-cooldown", if group_cooldown {
                            Ok(())
                        } else {
                            Err("a distinct healing spell did not share the resource-free cooldown refusal".into())
                        });
                    }
                    Err(error) => {
                        let reason = error.to_string();
                        checks.record("C04", "words-mana", Err(reason.clone().into()));
                        checks.record("C04", "spell-cooldown", Err(reason.clone().into()));
                        checks.record("C04", "group-cooldown", Err(reason.into()));
                    }
                }
            } else {
                for case in ["words-mana", "spell-cooldown", "group-cooldown"] {
                    checks.skip("C04", case, "candidate:spell/light_healing");
                }
            }
        }
        Err(error) => {
            let reason = format!("pinned spell catalog failed qualification: {error}");
            for case in ["words-mana", "spell-cooldown", "group-cooldown"] {
                checks.record("C04", case, Err(reason.clone().into()));
            }
        }
    }
    checks.finish()
}

async fn say(client: &mut DevClientSession, words: &str) -> TestResult<ChatOutcome> {
    Ok(client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        })
        .await?)
}

async fn words_and_cooldown(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
) -> TestResult<(bool, bool, bool)> {
    let group_index = spell_index(book, "exura gran")?;
    if spell_index(book, "exura")? == group_index {
        return Err("group cooldown probe must use a distinct spell".into());
    }
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("caster vitals unavailable")?;
    let cast = say(client, "exura").await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!(
            "spoken exura refused: {cast:?}; qualification needs a classed caster"
        )
        .into());
    }
    // Probe the one-second healing group on the first round trip after payment.
    let group = client
        .cast_spell(group_index, SpellTarget::None, false)
        .await?;
    let group_cooldown = group.disposition == SpellCastDisposition::CoolingDown
        && group.actor_vitals_delta.is_none();
    let group_checked_at = Instant::now();
    let deadline = group_checked_at + Duration::from_secs(5);
    let mana = loop {
        if client
            .actor_vitals()
            .is_some_and(|after| after.mana < before.mana)
        {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    };

    let individual = individual_cooldown(client, book, group_checked_at).await?;
    Ok((mana, individual, group_cooldown))
}

async fn individual_cooldown(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    group_checked_at: Instant,
) -> TestResult<bool> {
    // Crystal's Cure Poison has a six-second individual and one-second healing-group
    // cooldown. Read the served definitions so changes cannot silently close this window.
    let seed_index = spell_index(book, INDIVIDUAL_COOLDOWN_WORDS)?;
    let seed_spell = book
        .indexed(seed_index)
        .ok_or("individual cooldown seed missing")?;
    let light_healing = book.spoken("exura").ok_or("light healing missing")?.spell;
    let group = longest_group_cooldown(seed_spell);
    let individual = Duration::from_micros(seed_spell.cooldown_micros);
    if group.is_zero() || group >= individual {
        return Err("individual cooldown seed must outlast its groups".into());
    }
    // Finish the earlier healing group's window before synchronizing with regeneration.
    service_until(
        client,
        group_checked_at + longest_group_cooldown(light_healing),
    )
    .await?;
    let before_next_credit = wait_for_mana_regeneration(client).await?;
    let previous_mana = client
        .actor_vitals()
        .ok_or("pre-seed caster vitals unavailable")?
        .mana;
    let seed_sent = Instant::now();
    let seed = client
        .cast_spell(seed_index, SpellTarget::None, false)
        .await?;
    let seed_received = Instant::now();
    let paid = seed
        .actor_vitals_delta
        .as_ref()
        .map(|delta| delta.value)
        .ok_or_else(|| format!("individual cooldown seed has no vitals delta: {seed:?}"))?;
    if seed.status != CommandStatus::Accepted
        || seed.disposition != SpellCastDisposition::Cast
        || paid.mana >= previous_mana
    {
        return Err(format!("individual cooldown seed did not pay mana: {seed:?}").into());
    }

    let window = probe_window(
        seed_sent,
        seed_received,
        group,
        individual,
        before_next_credit,
    );
    service_until(client, window.start).await?;
    if !window.contains(&Instant::now()) {
        return Err(
            "individual cooldown probe has no time between group expiry and next deadline".into(),
        );
    }

    // These known spell words and the canonical index enter the same cast gate.
    // Chat rate limiting reports Exhausted or Muted, separately from Rejected.
    let spoken_retry = say(client, INDIVIDUAL_COOLDOWN_WORDS).await?;
    let retry = client
        .cast_spell(seed_index, SpellTarget::None, false)
        .await?;
    // The indexed result follows any spoken effects. A read-only command then
    // lets the client apply any vitals update sent after the indexed result.
    let after_spoken = client.actor_vitals().copied();
    stats_barrier(client).await?;
    if !window.contains(&Instant::now()) {
        return Err(
            "individual cooldown readback exceeded the spell or regeneration deadline".into(),
        );
    }
    Ok(same_spell_refusals_match(
        &spoken_retry,
        &retry,
        &paid,
        after_spoken.as_ref(),
        client.actor_vitals(),
    ))
}

fn longest_group_cooldown(spell: &crate::spell::SpellDefinition) -> Duration {
    Duration::from_micros(
        spell
            .groups
            .iter()
            .map(|group| group.cooldown_micros)
            .max()
            .unwrap_or(0),
    )
}

fn probe_window(
    seed_sent: Instant,
    seed_received: Instant,
    group: Duration,
    individual: Duration,
    before_next_credit: Instant,
) -> std::ops::Range<Instant> {
    (seed_received + group)..(seed_sent + individual).min(before_next_credit)
}

async fn service_until(client: &mut DevClientSession, deadline: Instant) -> TestResult {
    client
        .service_liveness(deadline.saturating_duration_since(Instant::now()))
        .await?;
    client.take_events();
    Ok(())
}

async fn wait_for_mana_regeneration(client: &mut DevClientSession) -> TestResult<Instant> {
    use oteryn_gameplay_progression::regeneration::{FED_MANA_PERIOD_MS, UNFED_MANA_PERIOD_MS};

    let mut unchanged_since = Instant::now();
    stats_barrier(client).await?;
    let mana = client
        .actor_vitals()
        .ok_or("caster vitals unavailable before regeneration")?
        .mana;
    let deadline = Instant::now() + Duration::from_millis(u64::from(2 * UNFED_MANA_PERIOD_MS));
    loop {
        service_until(client, Instant::now() + Duration::from_millis(100)).await?;
        let sent = Instant::now();
        stats_barrier(client).await?;
        let current = client
            .actor_vitals()
            .ok_or("caster vitals unavailable during regeneration")?
            .mana;
        if current > mana {
            // The owner serializes periodic vitals with these readbacks. The credit
            // followed the last unchanged barrier. Native mana credits are separated
            // by at least the fed period, even on this unfed fixture.
            return Ok(unchanged_since + Duration::from_millis(u64::from(FED_MANA_PERIOD_MS)));
        }
        if current < mana || sent >= deadline {
            return Err(
                "could not synchronize individual cooldown with a passive mana credit".into(),
            );
        }
        unchanged_since = sent;
    }
}

async fn stats_barrier(client: &mut DevClientSession) -> TestResult {
    let stats = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[character::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if stats.status != CommandStatus::Accepted {
        return Err(format!("same-spell vitals barrier refused: {:?}", stats.status).into());
    }
    Ok(())
}

fn same_spell_refusals_match(
    spoken_retry: &ChatOutcome,
    retry: &CastOutcome,
    paid: &ActorVitals,
    after_spoken: Option<&ActorVitals>,
    after_indexed: Option<&ActorVitals>,
) -> bool {
    // Independent combat and vitals refreshes can change health during the probe.
    let resources_unchanged = |after: &ActorVitals| {
        ActorVitals {
            health: paid.health,
            ..*after
        } == *paid
    };
    spoken_retry.status == CommandStatus::Rejected
        && spoken_retry.disposition == ChatDisposition::Rejected
        && spoken_retry.wait_seconds == 0
        && retry.status == CommandStatus::Accepted
        && retry.disposition == SpellCastDisposition::CoolingDown
        && retry.actor_vitals_delta.is_none()
        && after_spoken.is_some_and(resources_unchanged)
        && after_indexed.is_some_and(resources_unchanged)
}

pub(super) fn spell_index(
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<std::num::NonZeroU32> {
    let spell = book
        .spoken(words)
        .ok_or("spell words missing from book")?
        .spell;
    (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| {
            book.indexed(*index)
                .is_some_and(|entry| entry.key == spell.key)
        })
        .ok_or_else(|| format!("{words} has no canonical spell index").into())
}

pub(super) async fn protection_zone_probe(
    client: &mut DevClientSession,
    inside: bool,
) -> TestResult {
    let book = client_world::native_spell_book()?;
    let words = "exevo vis lux";
    let spell = book
        .spoken(words)
        .ok_or("energy beam missing from book")?
        .spell;
    if !spell.aggressive {
        return Err("PZ probe must use an aggressive spell".into());
    }
    let index = spell_index(book, words)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let before = client
            .actor_vitals()
            .copied()
            .ok_or("PZ probe has no vitals")?;
        let outcome = client.cast_spell(index, SpellTarget::None, false).await?;
        if !inside && outcome.disposition == SpellCastDisposition::CoolingDown {
            if tokio::time::Instant::now() >= deadline {
                return Err("outside-PZ comparison remained on cooldown".into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
            client.take_events();
            continue;
        }
        let matched = if inside {
            outcome.status == CommandStatus::Rejected
                && outcome.disposition == SpellCastDisposition::Rejected
                && outcome.actor_vitals_delta.is_none()
                && client.actor_vitals().copied() == Some(before)
        } else {
            outcome.status == CommandStatus::Accepted
                && outcome.disposition == SpellCastDisposition::Cast
                && outcome.actor_vitals_delta.is_some()
                && client
                    .actor_vitals()
                    .is_some_and(|after| after.mana < before.mana)
        };
        if !matched {
            return Err(
                format!("PZ aggressive cast comparison inside={inside}: {outcome:?}").into(),
            );
        }
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_spell_cooldown_probe_requires_resource_free_refusals() {
        let paid = ActorVitals {
            health: 120,
            max_health: 150,
            mana: 80,
            max_mana: 100,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        let spoken = ChatOutcome {
            command_id: 1,
            status: CommandStatus::Rejected,
            disposition: ChatDisposition::Rejected,
            wait_seconds: 0,
            result_server_sequence: 1,
        };
        let indexed = CastOutcome {
            command_id: 2,
            status: CommandStatus::Accepted,
            disposition: SpellCastDisposition::CoolingDown,
            result_server_sequence: 2,
            actor_vitals_delta: None,
        };
        assert!(same_spell_refusals_match(
            &spoken,
            &indexed,
            &paid,
            Some(&paid),
            Some(&paid)
        ));
        let after_damage = ActorVitals {
            health: 110,
            ..paid
        };
        assert!(
            same_spell_refusals_match(&spoken, &indexed, &paid, Some(&after_damage), Some(&paid))
                && same_spell_refusals_match(
                    &spoken,
                    &indexed,
                    &paid,
                    Some(&paid),
                    Some(&after_damage),
                ),
            "an independent health update must not invalidate a cooldown refusal"
        );

        let mana_debit = ActorVitals { mana: 60, ..paid };
        let soul_debit = ActorVitals { soul: 99, ..paid };
        let mut accepted = Vec::new();
        for (case, after_spoken, after_indexed) in [
            ("spoken mana debit", Some(&mana_debit), Some(&paid)),
            ("indexed soul debit", Some(&paid), Some(&soul_debit)),
            ("missing spoken vitals", None, Some(&paid)),
            ("missing indexed vitals", Some(&paid), None),
        ] {
            if same_spell_refusals_match(&spoken, &indexed, &paid, after_spoken, after_indexed) {
                accepted.push(case);
            }
        }

        for (case, changed) in [
            (
                "max health",
                ActorVitals {
                    max_health: 151,
                    ..paid
                },
            ),
            (
                "max mana",
                ActorVitals {
                    max_mana: 101,
                    ..paid
                },
            ),
            ("harmony", ActorVitals { harmony: 1, ..paid }),
            (
                "serene",
                ActorVitals {
                    serene: true,
                    ..paid
                },
            ),
            ("unattributed mana credit", ActorVitals { mana: 81, ..paid }),
        ] {
            if same_spell_refusals_match(&spoken, &indexed, &paid, Some(&changed), Some(&paid))
                || same_spell_refusals_match(&spoken, &indexed, &paid, Some(&paid), Some(&changed))
            {
                accepted.push(case);
            }
        }

        let wrong_spoken_status = ChatOutcome {
            status: CommandStatus::Accepted,
            ..spoken.clone()
        };
        let wrong_indexed_status = CastOutcome {
            status: CommandStatus::Rejected,
            ..indexed.clone()
        };
        let unexpected_delta = CastOutcome {
            actor_vitals_delta: Some(oteryn_dev_client::AppliedDelta {
                server_sequence: 3,
                base_revision: 2,
                new_revision: 3,
                value: soul_debit,
            }),
            ..indexed.clone()
        };
        for (case, spoken, indexed) in [
            ("wrong spoken status", &wrong_spoken_status, &indexed),
            ("wrong indexed status", &spoken, &wrong_indexed_status),
            ("indexed vitals delta", &spoken, &unexpected_delta),
        ] {
            if same_spell_refusals_match(spoken, indexed, &paid, Some(&paid), Some(&paid)) {
                accepted.push(case);
            }
        }
        assert!(accepted.is_empty(), "probe accepted {accepted:?}");
    }

    #[test]
    fn individual_cooldown_seed_outlasts_every_group() -> TestResult {
        let book = client_world::native_spell_book()?;
        let seed = book
            .spoken(INDIVIDUAL_COOLDOWN_WORDS)
            .ok_or("seed absent")?
            .spell;
        assert!(!seed.groups.is_empty());
        assert!(
            seed.groups
                .iter()
                .all(|group| group.cooldown_micros < seed.cooldown_micros)
        );
        Ok(())
    }

    #[test]
    fn periodic_credit_during_either_readback_is_outside_probe_window() {
        let last_unchanged = Instant::now();
        let credit = last_unchanged + Duration::from_millis(100);
        let next_credit = last_unchanged
            + Duration::from_millis(u64::from(
                oteryn_gameplay_progression::regeneration::FED_MANA_PERIOD_MS,
            ));
        let seed_sent = credit;
        let seed_received = seed_sent + Duration::from_millis(100);
        let group = Duration::from_secs(1);
        let individual = Duration::from_secs(6);
        let window = probe_window(seed_sent, seed_received, group, individual, next_credit);
        assert!(!window.contains(&(seed_received + group - Duration::from_micros(1))));
        assert!(window.contains(&(seed_received + group)));
        assert!(window.contains(&(next_credit - Duration::from_micros(1))));
        assert!(!window.contains(&next_credit));
        let no_regen_limit = seed_received + individual * 2;
        let window = probe_window(seed_sent, seed_received, group, individual, no_regen_limit);
        assert!(!window.contains(&(seed_sent + individual)));
    }

    #[test]
    fn spell_journey_resolves_words_from_pinned_catalog() -> TestResult {
        let book = client_world::native_spell_book()?;
        for words in [
            "exura",
            "exura gran",
            INDIVIDUAL_COOLDOWN_WORDS,
            "utani hur",
            "utana vid",
            "exori vis",
            "exevo vis lux",
        ] {
            let index = spell_index(book, words)?;
            assert_eq!(
                book.indexed(index).map(|spell| &spell.key),
                book.spoken(words).map(|spoken| &spoken.spell.key)
            );
        }
        Ok(())
    }
}
