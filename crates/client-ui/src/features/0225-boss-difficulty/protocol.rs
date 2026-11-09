use super::BossDifficultyPanel;
use oteryn_client_runtime::{
    CommandStatus,
    boss_difficulty::{self, Reply, Request},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Open,
    Preview,
    Start,
    Cancel,
}
pub fn request(panel: &mut BossDifficultyPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Feature unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    let typed = match intent {
        Intent::Open => Request::Open {
            boss_key: panel.boss_key.clone(),
        },
        Intent::Preview => Request::Preview {
            context: panel.context?,
            difficulty: panel.difficulty,
        },
        Intent::Start => Request::Start {
            context: panel.context?,
            difficulty: panel.difficulty,
        },
        Intent::Cancel => Request::Cancel {
            context: panel.context?,
        },
    };
    let Ok(payload) = boss_difficulty::encode_request(&typed) else {
        panel.feedback = Some(tr!("Enter a valid boss id or open a server context first"));
        return None;
    };
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: 225,
        kind: 431,
        payload,
    })
}
pub fn apply_result(panel: &mut BossDifficultyPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(tr!("The server refused the difficulty action"));
        return false;
    }
    let Some(reply) = decode_reply(&result.payload) else {
        panel.feedback = Some(tr!("The server returned an invalid difficulty reply"));
        return false;
    };
    panel.context = match &reply {
        Reply::Window {
            context,
            boss_key,
            selected_difficulty,
            ..
        } => {
            panel.boss_key.clone_from(boss_key);
            panel.difficulty = *selected_difficulty;
            Some(*context)
        }
        Reply::Closed { .. } => None,
        Reply::Refused { context, .. } => *context,
    };
    panel.feedback = if matches!(reply, Reply::Refused { .. }) {
        Some(tr!("The server refused the difficulty choice"))
    } else {
        None
    };
    panel.reply = Some(reply);
    true
}
fn decode_reply(bytes: &[u8]) -> Option<Reply> {
    use boss_difficulty::Refusal;
    let mut at = 0;
    let (mut kind, mut context, mut key, mut revision, mut difficulty, mut refusal) =
        (None, None, None, None, None, None);
    while at < bytes.len() {
        let tag = var(bytes, &mut at)?;
        match tag {
            8 => kind = Some(var(bytes, &mut at)?),
            18 => context = Some(take(bytes, &mut at)?.try_into().ok()?),
            26 => key = Some(String::from_utf8(take(bytes, &mut at)?.to_vec()).ok()?),
            34 => revision = Some(String::from_utf8(take(bytes, &mut at)?.to_vec()).ok()?),
            40 => difficulty = Some(u16::try_from(var(bytes, &mut at)?).ok()?),
            48 => {
                var(bytes, &mut at)?;
            }
            56 => {
                refusal = Some(match var(bytes, &mut at)? {
                    1 => Refusal::NoAcceptedProfile,
                    2 => Refusal::ContextClosed,
                    3 => Refusal::ContextMissing,
                    4 => Refusal::ContentChanged,
                    5 => Refusal::BossUnavailable,
                    6 => Refusal::ReplayConflict,
                    _ => return None,
                })
            }
            _ => return None,
        }
    }
    match kind? {
        0 => Some(Reply::Window {
            context: context?,
            boss_key: key?,
            content_revision: revision?,
            selected_difficulty: difficulty?,
        }),
        1 => Some(Reply::Closed { context: context? }),
        2 => Some(Reply::Refused {
            context,
            reason: refusal?,
        }),
        _ => None,
    }
}
fn var(b: &[u8], a: &mut usize) -> Option<u64> {
    let (mut v, mut s) = (0, 0);
    loop {
        let x = *b.get(*a)?;
        *a += 1;
        v |= u64::from(x & 127) << s;
        if x & 128 == 0 {
            return Some(v);
        }
        s += 7;
        if s > 63 {
            return None;
        }
    }
}
fn take<'a>(b: &'a [u8], a: &mut usize) -> Option<&'a [u8]> {
    let n = usize::try_from(var(b, a)?).ok()?;
    let x = b.get(*a..*a + n)?;
    *a += n;
    Some(x)
}
