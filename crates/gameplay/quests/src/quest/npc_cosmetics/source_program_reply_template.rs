//! Closed source renderers over captured clock and authoritative Hazard facts.
use super::Blocked;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplyTemplate {
    SourceDuration {
        storage: String,
        subtract: CapturedNow,
        source_function: DurationFunction,
    },
    SourceHazardStatus {
        owner: HazardOwner,
        current_field: HazardCurrent,
        maximum_field: HazardMaximum,
        prefix: String,
        between: String,
        suffix: String,
    },
    SourceHazardSelected {
        selection: HazardSelection,
        prefix: String,
        suffix: String,
    },
}
#[derive(Debug, Deserialize)]
pub enum CapturedNow {
    #[serde(rename = "now")]
    Now,
}
#[derive(Debug, Deserialize)]
pub enum DurationFunction {
    #[serde(rename = "string.diff")]
    StringDiff,
}
#[derive(Debug, Deserialize)]
pub enum HazardOwner {
    #[serde(rename = "hazard.gnomprona-gardens")]
    Gardens,
}
#[derive(Debug, Deserialize)]
pub enum HazardCurrent {
    #[serde(rename = "selected")]
    Selected,
}
#[derive(Debug, Deserialize)]
pub enum HazardMaximum {
    #[serde(rename = "unlocked")]
    Unlocked,
}
#[derive(Debug, Deserialize)]
pub enum HazardSelection {
    #[serde(rename = "hazard_level")]
    Level,
}
#[derive(Clone, Copy)]
pub struct HazardReplyFacts {
    pub selected: u16,
    pub unlocked: u16,
    pub requested: Option<u16>,
}
impl ReplyTemplate {
    pub fn storage(&self) -> Option<&str> {
        match self {
            Self::SourceDuration { storage, .. } => Some(storage),
            _ => None,
        }
    }
    pub fn is_hazard(&self) -> bool {
        !matches!(self, Self::SourceDuration { .. })
    }
    pub fn has_hazard_followup(&self, text: &str) -> bool {
        matches!(self, Self::SourceHazardSelected { .. }) && !text.is_empty()
    }
    pub fn valid_for(&self, text: &str) -> bool {
        match self {
            Self::SourceDuration { storage, .. } => {
                !storage.is_empty() && text.matches("{source_duration}").count() == 1
            }
            Self::SourceHazardStatus {
                prefix,
                between,
                suffix,
                ..
            } => text.is_empty() && prefix.len() + between.len() + suffix.len() <= 2048,
            Self::SourceHazardSelected { prefix, suffix, .. } => {
                (text.is_empty()
                    || text
                        == "You've achieved the necessary hazard level. As a reward, you've received the Noxious Ripptor mount and a primal bag.")
                    && prefix.len() + suffix.len() <= 2048
            }
        }
    }
    pub fn render(
        &self,
        text: &str,
        captured_now: i64,
        read: impl FnOnce(&str) -> Result<i64, Blocked>,
        source_duration: impl FnOnce(i64) -> String,
    ) -> Result<String, Blocked> {
        if !self.valid_for(text) {
            return Err(Blocked::InvalidProgram);
        }
        let Some(storage) = self.storage() else {
            return Err(Blocked::InvalidProgram);
        };
        let seconds = read(storage)?
            .checked_sub(captured_now)
            .ok_or(Blocked::ArithmeticBound)?;
        let rendered = text.replace("{source_duration}", &source_duration(seconds));
        if rendered.len() > 8192 {
            return Err(Blocked::ArithmeticBound);
        }
        Ok(rendered)
    }
    pub fn render_hazard(&self, text: &str, facts: HazardReplyFacts) -> Result<String, Blocked> {
        if !self.valid_for(text)
            || !(1..=12).contains(&facts.unlocked)
            || !(1..=facts.unlocked).contains(&facts.selected)
        {
            return Err(Blocked::InvalidProgram);
        }
        let rendered = match self {
            Self::SourceHazardStatus {
                owner: HazardOwner::Gardens,
                current_field: HazardCurrent::Selected,
                maximum_field: HazardMaximum::Unlocked,
                prefix,
                between,
                suffix,
            } => format!(
                "{prefix}{}{between}{}{suffix}",
                facts.selected, facts.unlocked
            ),
            Self::SourceHazardSelected {
                selection: HazardSelection::Level,
                prefix,
                suffix,
            } => {
                let requested = facts
                    .requested
                    .filter(|value| (1..=facts.unlocked).contains(value))
                    .ok_or(Blocked::MissingSourceFact)?;
                format!("{prefix}{requested}{suffix}")
            }
            _ => return Err(Blocked::InvalidProgram),
        };
        if rendered.len() > 8192 {
            return Err(Blocked::ArithmeticBound);
        }
        Ok(rendered)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_source_duration_template_is_closed_and_uses_captured_clock() -> Result<(), String> {
        let template: ReplyTemplate = serde_json::from_str(r#"{"kind":"source_duration","storage":"known.daily","subtract":"now","source_function":"string.diff"}"#)
            .map_err(|error| error.to_string())?;
        let text = "Wait {source_duration}.";
        assert_eq!(
            template.render(
                text,
                100,
                |key| {
                    assert_eq!(key, "known.daily");
                    Ok(101)
                },
                |seconds| format!("{seconds}s")
            ),
            Ok("Wait 1s.".into())
        );
        assert_eq!(
            template.render(text, 100, |_| Ok(99), |seconds| format!("{seconds}s")),
            Ok("Wait -1s.".into())
        );
        assert!(
            template
                .render(text, i64::MIN, |_| Ok(i64::MAX), |_| "bad".into())
                .is_err()
        );
        assert!(
            template
                .render(
                    text,
                    0,
                    |_| Err(Blocked::MissingSourceFact),
                    |_| "bad".into()
                )
                .is_err()
        );
        for json in [
            r#"{"kind":"source_duration","storage":"known.daily","subtract":"client_now","source_function":"string.diff"}"#,
            r#"{"kind":"source_duration","storage":"known.daily","subtract":"now","source_function":"eval"}"#,
            r#"{"kind":"source_duration","storage":"known.daily","subtract":"now","source_function":"string.diff","authority":true}"#,
        ] {
            assert!(serde_json::from_str::<ReplyTemplate>(json).is_err());
        }
        assert!(!template.valid_for("raw source placeholder omitted"));
        Ok(())
    }
}

#[cfg(test)]
mod hazard_tests {
    use super::*;
    #[test]
    fn source_hazard_reply_uses_actual_owner_facts_and_rejects_forged_templates()
    -> Result<(), String> {
        let template: ReplyTemplate=serde_json::from_str(r#"{"kind":"source_hazard_status","owner":"hazard.gnomprona-gardens","current_field":"selected","maximum_field":"unlocked","prefix":"current ","between":" max ","suffix":"."}"#).map_err(|e|e.to_string())?;
        let facts = HazardReplyFacts {
            selected: 3,
            unlocked: 6,
            requested: None,
        };
        assert_eq!(
            template
                .render_hazard("", facts)
                .map_err(|e| format!("{e:?}"))?,
            "current 3 max 6."
        );
        for facts in [
            HazardReplyFacts {
                selected: 0,
                ..facts
            },
            HazardReplyFacts {
                selected: 7,
                ..facts
            },
            HazardReplyFacts {
                unlocked: 13,
                ..facts
            },
        ] {
            assert!(template.render_hazard("", facts).is_err());
        }
        let selected:ReplyTemplate=serde_json::from_str(r#"{"kind":"source_hazard_selected","selection":"hazard_level","prefix":"set ","suffix":"."}"#).map_err(|e|e.to_string())?;
        assert!(selected.render_hazard("", facts).is_err());
        assert_eq!(
            selected
                .render_hazard(
                    "",
                    HazardReplyFacts {
                        requested: Some(6),
                        ..facts
                    }
                )
                .map_err(|e| format!("{e:?}"))?,
            "set 6."
        );
        for json in [
            r#"{"kind":"source_hazard_status","owner":"client","current_field":"selected","maximum_field":"unlocked","prefix":"","between":"","suffix":""}"#,
            r#"{"kind":"source_hazard_selected","selection":"hazard_level","prefix":"","suffix":"","authority":true}"#,
        ] {
            assert!(serde_json::from_str::<ReplyTemplate>(json).is_err());
        }
        Ok(())
    }
}
