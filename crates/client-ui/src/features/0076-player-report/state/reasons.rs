//! Wire ordinals: Canary lua_definitions.hpp:181–201, commit
//! 666a60f6fef803b043cbaf1c7b5fab219da9bfa9. Labels and guidance are Oteryn-authored.
use super::Panel;
const REASONS: [(&str, &str); 21] = [
    (
        "Inappropriate name",
        "Quote the exact name and explain the naming-rule concern.",
    ),
    (
        "Name formatting",
        "Quote the name and describe its formatting issue.",
    ),
    (
        "Advertising in name",
        "Identify the advertised product or service in the name.",
    ),
    (
        "Unfitting name",
        "Explain which naming requirement the name conflicts with.",
    ),
    (
        "Name encourages rule breaking",
        "Quote the name and identify the rule it encourages breaking.",
    ),
    (
        "Insulting statement",
        "Quote the original statement and its context.",
    ),
    (
        "Spam",
        "Describe the repeated messages, channel and approximate time.",
    ),
    (
        "Advertising statement",
        "Quote the advertisement and identify its channel.",
    ),
    (
        "Unfitting statement",
        "Quote the statement and explain the channel's topic rules.",
    ),
    (
        "Channel language",
        "Identify the channel, its language rule and the original statement.",
    ),
    (
        "Disclosure of private information",
        "Describe the disclosure without repeating sensitive personal data.",
    ),
    (
        "Encouraging rule violations",
        "Quote the encouragement and name the relevant rule.",
    ),
    (
        "Bug abuse statement",
        "Quote the statement and describe the suspected exploit without sharing credentials.",
    ),
    (
        "Unofficial software",
        "Describe observable repeated behavior, location and approximate time.",
    ),
    (
        "Impersonation",
        "Identify the claimed role and quote the relevant words.",
    ),
    (
        "Harassing operators",
        "Describe the conduct, context and approximate time.",
    ),
    (
        "False information",
        "Quote the claim and explain the relevant context.",
    ),
    (
        "Account sharing",
        "Describe the observed evidence; do not include account credentials.",
    ),
    (
        "Stealing account data",
        "Describe the attempt without including passwords, tokens or recovery codes.",
    ),
    (
        "Attacking the service",
        "Describe the observed disruption and approximate time.",
    ),
    (
        "Service agreement",
        "Identify the agreement requirement and explain the observed conduct.",
    ),
];
impl Panel {
    pub fn cycle_reason(&mut self, forward: bool) {
        if self.pending {
            return;
        }
        let current = self.fields[0]
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|n| *n < REASONS.len());
        let next = current.map_or(0, |n| {
            if forward {
                (n + 1) % REASONS.len()
            } else {
                (n + REASONS.len() - 1) % REASONS.len()
            }
        });
        self.fields[0] = next.to_string();
        self.violation_reviewed = None;
        self.feedback = self.reason_help();
    }
    pub fn reason_help(&self) -> String {
        let reason = self.fields[0].trim().parse::<usize>().ok();
        match reason.and_then(|n| REASONS.get(n).map(|value| (n, value))) {
            Some((n, (label, example))) => format!("Reason {n}: {label}\r\nEvidence: {example}"),
            None => "Choose a report reason using Previous reason / Next reason.".into(),
        }
    }
}
