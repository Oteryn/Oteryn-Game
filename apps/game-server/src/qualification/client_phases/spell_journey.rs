//! Spoken casts on the ordinary client connection; no direct runtime mutation.
use super::client_combat_journey::Checks;
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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

pub(super) async fn spell_journey(client: &mut DevClientSession) -> TestResult {
    let mut checks = Checks::default();
    let book = spell_book();
    match book {
        Ok(book) => {
            if book.spoken("exura").is_some() {
                let result = words_and_cooldown(client, &book).await;
                match result {
                    Ok((mana, cooldown)) => {
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
                    }
                    Err(error) => {
                        let reason = error.to_string();
                        checks.record("C04", "words-mana", Err(reason.clone().into()));
                        checks.record("C04", "spell-cooldown", Err(reason.into()));
                    }
                }
            } else {
                for case in ["words-mana", "spell-cooldown"] {
                    checks.skip("C04", case, "candidate:spell/light_healing");
                }
            }
        }
        Err(error) => {
            let reason = format!("pinned spell catalog failed qualification: {error}");
            for case in ["words-mana", "spell-cooldown"] {
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
) -> TestResult<(bool, bool)> {
    let spell = book.spoken("exura").ok_or("missing light healing")?.spell;
    let index = (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| {
            book.indexed(*index)
                .is_some_and(|entry| entry.key == spell.key)
        })
        .ok_or("light healing has no canonical index")?;
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
    // The typed probe distinguishes the spell owner's CoolingDown refusal from
    // chat flood control, vocation, level, mana and other unrelated refusals.
    let retry = client.cast_spell(index, SpellTarget::None, false).await?;
    let cooldown = spoken_retry.disposition == ChatDisposition::Rejected
        && retry.disposition == SpellCastDisposition::CoolingDown
        && retry.actor_vitals_delta.is_none();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client
            .actor_vitals()
            .is_some_and(|after| after.mana < before.mana)
        {
            return Ok((true, cooldown));
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok((false, cooldown));
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_journey_resolves_words_from_pinned_catalog() -> TestResult {
        let book = spell_book()?;
        for words in ["exura", "utani hur", "utana vid", "exori vis"] {
            assert!(book.spoken(words).is_some(), "missing {words}");
        }
        Ok(())
    }
}
