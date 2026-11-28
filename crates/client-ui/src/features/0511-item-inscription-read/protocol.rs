use super::{
    CAPABILITY_ITEM_INSCRIPTION_ATTRIBUTION_V1, CAPABILITY_ITEM_INSCRIPTION_READ_V1,
    COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION, COMMAND_TYPE_ITEM_INSCRIPTION_READ, Inscription,
    Reply, Request,
};
use oteryn_client_runtime::item_inscription::Reply as AttributedReply;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};

/// What the client knows about one item's current text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Known {
    /// Nothing has been read.
    #[default]
    Unread,
    /// A read of this handle was sent.
    Reading(u64),
    /// The server's current text of this handle was read.
    Read {
        handle: u64,
        max_length: u32,
        writable: bool,
        attribution: Option<Attribution>,
    },
    /// The server refused to read this handle.
    Refused(u64),
    /// The server's text of this handle exceeds the writing window and cannot be shown,
    /// so it is never replaced.
    TooLong(u64),
}

/// The recorded date and optional writer of a read inscription.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribution {
    pub writer_name: Option<String>,
    pub written_at_unix_ms: i64,
}

impl Inscription {
    fn read_route(&self) -> (u32, u32) {
        if self.attribution_selected {
            (
                CAPABILITY_ITEM_INSCRIPTION_ATTRIBUTION_V1,
                COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION,
            )
        } else {
            (
                CAPABILITY_ITEM_INSCRIPTION_READ_V1,
                COMMAND_TYPE_ITEM_INSCRIPTION_READ,
            )
        }
    }
}

/// Whether a Write of `text` to an item may be sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteCheck {
    Allowed,
    /// The item's text has not been read: read it first, and send nothing now.
    Read,
    Refused(&'static str),
}

/// The read of `handle`'s text; `None` without capability 511 or for handle 0.
pub fn request(state: &mut Inscription, handle: u64) -> Option<FeatureCommand> {
    if !state.selected {
        return None;
    }
    let payload = Request {
        item_handle: handle,
    }
    .encode()?;
    state.known = Known::Reading(handle);
    let (capability, kind) = state.read_route();
    Some(FeatureCommand {
        capability,
        kind,
        payload,
    })
}

/// Applies the reply to the read in flight; `Some` only for that read's accepted, valid reply.
/// A refused or invalid reply marks the item refused, so no Write can replace its text.
pub fn apply_result(
    state: &mut Inscription,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) -> Option<Reply> {
    let handle = Request::decode(&command.payload)?.item_handle;
    if (command.capability, command.kind) != state.read_route()
        || state.known != Known::Reading(handle)
    {
        return None;
    }
    let decoded = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| decode_reply(command.kind, &result.payload))
    .flatten();
    let Some((reply, attribution)) = decoded else {
        state.known = Known::Refused(handle);
        return None;
    };
    state.known = Known::Read {
        handle,
        max_length: reply.max_length,
        writable: reply.writable,
        attribution,
    };
    Some(reply)
}

fn decode_reply(kind: u32, payload: &[u8]) -> Option<(Reply, Option<Attribution>)> {
    if kind == COMMAND_TYPE_ITEM_INSCRIPTION_READ {
        return Some((Reply::decode(payload)?, None));
    }
    let reply = AttributedReply::decode(payload)?;
    let attribution = reply
        .written_at_unix_ms
        .map(|written_at_unix_ms| Attribution {
            writer_name: reply.writer_name,
            written_at_unix_ms,
        });
    Some((
        Reply {
            text: reply.text,
            max_length: reply.max_length,
            writable: reply.writable,
        },
        attribution,
    ))
}

/// The Write gate for `handle`. With capability 511 only a read, writable text within its
/// maximum is sent; without it, an empty text is never sent because the item's current text
/// is unknown and the empty replacement would erase it.
#[must_use]
pub fn write_check(state: &Inscription, handle: u64, text: &str) -> WriteCheck {
    if !state.selected {
        return if text.is_empty() {
            WriteCheck::Refused(tr!(
                "The item's current text is unknown; an empty text would erase it"
            ))
        } else {
            WriteCheck::Allowed
        };
    }
    match state.known {
        Known::Read {
            handle: read,
            max_length,
            writable,
            ..
        } if read == handle => {
            if !writable {
                WriteCheck::Refused(tr!("This item cannot be written on"))
            } else if text.chars().count() > max_length as usize {
                WriteCheck::Refused(tr!("The letter text is too long"))
            } else {
                WriteCheck::Allowed
            }
        }
        Known::TooLong(read) if read == handle => {
            WriteCheck::Refused(tr!("The letter text is too long"))
        }
        Known::Refused(refused) if refused == handle => {
            WriteCheck::Refused(tr!("The server refused to read the item's text"))
        }
        // A read still marked in flight here was lost (the form only submits while idle).
        _ => WriteCheck::Read,
    }
}
