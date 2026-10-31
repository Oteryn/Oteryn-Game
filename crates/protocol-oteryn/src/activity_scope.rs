//! ACTIVITY_SCOPE: a committed fresh-session barrier inside StateSnapshot.
//! Accepted by PR #2769, answer 6543120139. This is server output, never a grant.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field as bytes, push_varint_field as scalar,
    read_bytes, read_varint, set_once,
};

pub const STATE_DOMAIN_ACTIVITY_SCOPE: u32 = 99;
pub const MAX_ACTIVITY_SCOPE_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityScope {
    Channel { world: [u8; 16], channel: [u8; 16] },
    Instance { world: [u8; 16], instance: [u8; 16] },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityScopeSnapshot {
    pub handoff: [u8; 16],
    pub source_session: [u8; 16],
    pub session: [u8; 16],
    pub scope: ActivityScope,
    pub scope_generation: u64,
    pub connection_generation: u64,
    pub lease_generation: u64,
    pub selection: Vec<u32>,
}

impl ActivityScopeSnapshot {
    fn validate(&self) -> Result<(), Error> {
        let (world, identity) = match self.scope {
            ActivityScope::Channel { world, channel } => (world, channel),
            ActivityScope::Instance { world, instance } => (world, instance),
        };
        if [
            self.handoff,
            self.source_session,
            self.session,
            world,
            identity,
        ]
        .iter()
        .any(|id| id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80)
            || self.source_session == self.session
            || self.scope_generation == 0
            || self.connection_generation == 0
            || self.lease_generation == 0
            || self.selection.len() > crate::MAX_CAPABILITY_COUNT
            || self.selection.first() == Some(&0)
            || self.selection.windows(2).any(|pair| pair[0] >= pair[1])
            || self
                .selection
                .binary_search(&crate::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
                .is_err()
        {
            return Err(Error::Malformed);
        }
        Ok(())
    }
}

/// The enclosing frame belongs to the source connection's next sequence. Only
/// after this complete frame does the client reset its stream to the new session
/// and accept the destination's complete snapshots, starting with sequence one.
pub fn encode_activity_scope(value: &ActivityScopeSnapshot) -> Result<Vec<u8>, Error> {
    value.validate()?;
    let mut output = Vec::new();
    bytes(&mut output, 1, &value.handoff);
    bytes(&mut output, 2, &value.source_session);
    bytes(&mut output, 3, &value.session);
    let (tag, world, identity) = match value.scope {
        ActivityScope::Channel { world, channel } => (1, world, channel),
        ActivityScope::Instance { world, instance } => (2, world, instance),
    };
    scalar(&mut output, 4, tag);
    bytes(&mut output, 5, &world);
    bytes(&mut output, 6, &identity);
    scalar(&mut output, 7, value.scope_generation);
    scalar(&mut output, 8, value.connection_generation);
    scalar(&mut output, 9, value.lease_generation);
    for capability in &value.selection {
        scalar(&mut output, 10, u64::from(*capability));
    }
    if output.len() > MAX_ACTIVITY_SCOPE_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(output)
}

pub fn decode_activity_scope(body: &[u8]) -> Result<ActivityScopeSnapshot, Error> {
    if body.len() > MAX_ACTIVITY_SCOPE_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut cursor, mut handoff, mut source, mut session, mut tag, mut world, mut identity) =
        (0, None, None, None, None, None, None);
    let (mut scope_generation, mut connection_generation, mut lease_generation) =
        (None, None, None);
    let mut selection = Vec::new();
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            10 => set_once(&mut handoff, id(read_bytes(body, &mut cursor)?)?)?,
            18 => set_once(&mut source, id(read_bytes(body, &mut cursor)?)?)?,
            26 => set_once(&mut session, id(read_bytes(body, &mut cursor)?)?)?,
            32 => set_once(&mut tag, read_varint(body, &mut cursor)?)?,
            42 => set_once(&mut world, id(read_bytes(body, &mut cursor)?)?)?,
            50 => set_once(&mut identity, id(read_bytes(body, &mut cursor)?)?)?,
            56 => set_once(&mut scope_generation, read_varint(body, &mut cursor)?)?,
            64 => set_once(&mut connection_generation, read_varint(body, &mut cursor)?)?,
            72 => set_once(&mut lease_generation, read_varint(body, &mut cursor)?)?,
            80 => {
                if selection.len() == crate::MAX_CAPABILITY_COUNT {
                    return Err(Error::LimitExceeded);
                }
                selection.push(
                    u32::try_from(read_varint(body, &mut cursor)?).map_err(|_| Error::Malformed)?,
                );
            }
            _ => return Err(Error::Malformed),
        }
    }
    let (world, identity) = (
        world.ok_or(Error::Malformed)?,
        identity.ok_or(Error::Malformed)?,
    );
    let scope = match tag {
        Some(1) => ActivityScope::Channel {
            world,
            channel: identity,
        },
        Some(2) => ActivityScope::Instance {
            world,
            instance: identity,
        },
        _ => return Err(Error::Malformed),
    };
    let result = ActivityScopeSnapshot {
        handoff: handoff.ok_or(Error::Malformed)?,
        source_session: source.ok_or(Error::Malformed)?,
        session: session.ok_or(Error::Malformed)?,
        scope,
        scope_generation: scope_generation.ok_or(Error::Malformed)?,
        connection_generation: connection_generation.ok_or(Error::Malformed)?,
        lease_generation: lease_generation.ok_or(Error::Malformed)?,
        selection,
    };
    result.validate()?;
    Ok(result)
}

fn id(bytes: &[u8]) -> Result<[u8; 16], Error> {
    bytes.try_into().map_err(|_| Error::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: u8) -> [u8; 16] {
        let mut id = [value; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    fn snapshot() -> ActivityScopeSnapshot {
        ActivityScopeSnapshot {
            handoff: id(1),
            source_session: id(2),
            session: id(3),
            scope: ActivityScope::Instance {
                world: id(4),
                instance: id(5),
            },
            scope_generation: 8,
            connection_generation: 1,
            lease_generation: 4,
            selection: vec![1, 117],
        }
    }
    #[test]
    fn scope_namespace_survives_round_trip_and_never_becomes_a_channel() {
        let instance = snapshot();
        let mut channel = instance.clone();
        channel.scope = ActivityScope::Channel {
            world: id(4),
            channel: id(5),
        };
        let instance_bytes = encode_activity_scope(&instance).expect("instance");
        let channel_bytes = encode_activity_scope(&channel).expect("channel");
        assert_ne!(instance_bytes, channel_bytes);
        assert_eq!(decode_activity_scope(&instance_bytes), Ok(instance));
        assert_eq!(decode_activity_scope(&channel_bytes), Ok(channel));
    }
    #[test]
    fn duplicate_fields_unknown_tags_truncation_and_widened_selection_are_rejected() {
        let encoded = encode_activity_scope(&snapshot()).expect("valid");
        for end in 0..encoded.len() {
            assert!(decode_activity_scope(&encoded[..end]).is_err());
        }
        let mut duplicate = encoded.clone();
        scalar(&mut duplicate, 8, 2);
        assert!(decode_activity_scope(&duplicate).is_err());
        let mut unknown = encoded;
        scalar(&mut unknown, 11, 1);
        assert!(decode_activity_scope(&unknown).is_err());
        let mut value = snapshot();
        value.selection = vec![117, 1];
        assert!(encode_activity_scope(&value).is_err());
        value.selection = vec![117, 117];
        assert!(encode_activity_scope(&value).is_err());
        value.selection = vec![1];
        assert!(encode_activity_scope(&value).is_err());
    }
}
