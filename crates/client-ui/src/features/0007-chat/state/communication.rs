//! Local communication exceptions never mutate the server's bounded chat history.
use oteryn_client_runtime::chat::{ChatLine, ChatSpeechMode};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommunicationRules {
    pub ignored: BTreeSet<String>,
    pub whitelist: BTreeSet<String>,
    pub blacklist: BTreeSet<String>,
    pub ignore_private: bool,
    pub ignore_yells: bool,
}

impl CommunicationRules {
    pub fn parse(
        ignored: &str,
        whitelist: &str,
        blacklist: &str,
        ignore_private: bool,
        ignore_yells: bool,
    ) -> Result<Self, &'static str> {
        fn names(value: &str) -> Result<BTreeSet<String>, &'static str> {
            let mut names = BTreeSet::new();
            for name in value
                .split([',', ';', '\n'])
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                if name.len() > 120 || name.chars().any(char::is_control) {
                    return Err("Names must contain at most 120 bytes and no control characters");
                }
                names.insert(name.to_lowercase());
                if names.len() > 100 {
                    return Err("Each communication list holds at most 100 names");
                }
            }
            Ok(names)
        }
        Ok(Self {
            ignored: names(ignored)?,
            whitelist: names(whitelist)?,
            blacklist: names(blacklist)?,
            ignore_private,
            ignore_yells,
        })
    }

    pub fn accepts(&self, line: &ChatLine) -> bool {
        let (speaker, private, yell) = match line {
            ChatLine::Local {
                speaker_name, mode, ..
            } => (speaker_name, false, *mode == ChatSpeechMode::Yell),
            ChatLine::Private { speaker_name, .. } => (speaker_name, true, false),
            ChatLine::Room { speaker_name, .. } | ChatLine::Party { speaker_name, .. } => {
                (speaker_name, false, false)
            }
            ChatLine::Guild(line) => (&line.speaker_name, false, false),
            ChatLine::PrivateChannel(line) => (&line.speaker_name, false, false),
            ChatLine::Dropped => return true,
        };
        let name = speaker.trim().to_lowercase();
        self.whitelist.contains(&name)
            || !(self.ignored.contains(&name)
                || self.blacklist.contains(&name)
                || (private && self.ignore_private)
                || (yell && self.ignore_yells))
    }
}
