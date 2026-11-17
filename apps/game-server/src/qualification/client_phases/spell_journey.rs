//! Spoken casts on the ordinary client connection; no direct runtime mutation.
use super::client_combat_journey::Checks;
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};

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
                            Err("immediate retry did not prove a resource-free spell cooldown refusal".into())
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

async fn say(
    client: &mut DevClientSession,
    words: &str,
) -> TestResult<oteryn_dev_client::ChatOutcome> {
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
    let index = spell_index(book, "exura")?;
    let group_index = spell_index(book, "exura gran")?;
    if index == group_index {
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
    let spoken_retry = say(client, "exura").await?;
    // Distinct indexed probes distinguish cooldown from chat flood control.
    let retry = client.cast_spell(index, SpellTarget::None, false).await?;
    let cooldown = spoken_retry.disposition == ChatDisposition::Rejected
        && retry.disposition == SpellCastDisposition::CoolingDown
        && retry.actor_vitals_delta.is_none();
    let group = client
        .cast_spell(group_index, SpellTarget::None, false)
        .await?;
    let group_cooldown = group.disposition == SpellCastDisposition::CoolingDown
        && group.actor_vitals_delta.is_none();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client
            .actor_vitals()
            .is_some_and(|after| after.mana < before.mana)
        {
            return Ok((true, cooldown, group_cooldown));
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok((false, cooldown, group_cooldown));
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

fn spell_index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
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
        let matched = outcome.status == CommandStatus::Accepted
            && if inside {
                outcome.disposition == SpellCastDisposition::Rejected
                    && outcome.actor_vitals_delta.is_none()
                    && client.actor_vitals().copied() == Some(before)
            } else {
                outcome.disposition == SpellCastDisposition::Cast
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
    fn spell_journey_resolves_words_from_pinned_catalog() -> TestResult {
        let book = client_world::native_spell_book()?;
        for words in [
            "exura",
            "exura gran",
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
