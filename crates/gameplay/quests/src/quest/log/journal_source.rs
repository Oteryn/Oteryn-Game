//! Interpret admitted journal templates, preserving their source part order.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::super::loader::{LoweredQuestState, QuestTextRef};
use super::source_texts::SourceTexts;
use super::{JournalPiece, JournalText, QuestJournal};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Journal {
    None,
    Fixed {
        text_ref: QuestTextRef,
    },
    Template {
        parts: Vec<QuestTextRef>,
        reads: Vec<String>,
    },
    PerStage {
        stages: Vec<Stage>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stage {
    value: i64,
    text_ref: Option<QuestTextRef>,
    template: Option<Template>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Template {
    parts: Vec<QuestTextRef>,
    reads: Vec<String>,
}

pub(super) fn track(lowered: &LoweredQuestState, source: &str) -> Option<String> {
    let key = format!(
        "oteryn:quest-progress/{}",
        source.split_once(":quest-progress/")?.1
    );
    (lowered.source_track(&key) == Some(source)).then_some(key)
}

impl Journal {
    // None is a fact-only exclusion: no admitted, ordered binding. Errors are broken content.
    pub(super) fn lower(
        &self,
        lowered: &LoweredQuestState,
        texts: &SourceTexts,
    ) -> Result<Option<QuestJournal>, String> {
        Ok(Some(match self {
            Self::None => QuestJournal::None,
            Self::Fixed { text_ref } => {
                QuestJournal::Fixed(JournalText::Text(texts.resolve(text_ref)?.into()))
            }
            Self::Template { parts, reads } => {
                let Some(text) = template(parts, reads, lowered, texts)? else {
                    return Ok(None);
                };
                QuestJournal::Fixed(text)
            }
            Self::PerStage { stages } => {
                let mut entries = BTreeMap::new();
                for stage in stages {
                    let text = match (&stage.text_ref, &stage.template) {
                        (Some(reference), None) => {
                            JournalText::Text(texts.resolve(reference)?.into())
                        }
                        (None, Some(value)) => {
                            let Some(text) = template(&value.parts, &value.reads, lowered, texts)?
                            else {
                                return Ok(None);
                            };
                            text
                        }
                        _ => return Err("ambiguous journal stage".into()),
                    };
                    if entries.insert(stage.value, text).is_some() {
                        return Err("duplicate journal stage".into());
                    }
                }
                QuestJournal::PerStage(entries)
            }
        }))
    }
}

fn template(
    parts: &[QuestTextRef],
    reads: &[String],
    lowered: &LoweredQuestState,
    texts: &SourceTexts,
) -> Result<Option<JournalText>, String> {
    // The merged source format records a sorted set of reads, not argument order. Only
    // a single part with a single counter and single admitted read is unambiguous.
    let resolved = parts
        .iter()
        .map(|part| texts.resolve(part))
        .collect::<Result<Vec<_>, _>>()?;
    if resolved.len() != 1 || reads.len() != 1 {
        return Ok(None);
    }
    let text = resolved[0];
    if text.matches('%').count() != 1 || text.matches("%d").count() != 1 {
        return Ok(None);
    }
    let Some(key) = track(lowered, &format!("canary:quest-progress/{}", reads[0])) else {
        return Ok(None);
    };
    let Some((before, after)) = text.split_once("%d") else {
        return Ok(None);
    };
    Ok(Some(JournalText::Template(vec![
        JournalPiece::Text(before.into()),
        JournalPiece::Track(key),
        JournalPiece::Text(after.into()),
    ])))
}
