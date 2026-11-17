//! Versioned account visibility; the legacy empty badge request stays unchanged.
use super::{BADGE_COUNT, Badge, BadgesWireError};
pub const PREFIX: u8 = 0xf0;
pub const MAX_REQUEST_BYTES: usize = 18;
pub const MAX_REPLY_BYTES: usize = 3 + 2 * BADGE_COUNT;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Query(Option<[u8; 16]>),
    Set { badge: Badge, public: bool },
}
impl Request {
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Query(None) => vec![PREFIX, 0],
            Self::Query(Some(target)) => {
                let mut bytes = vec![PREFIX, 1];
                bytes.extend(target);
                bytes
            }
            Self::Set { badge, public } => vec![PREFIX, 2, badge.id(), u8::from(public)],
        }
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, BadgesWireError> {
        match bytes {
            [PREFIX, 0] => Ok(Self::Query(None)),
            [PREFIX, 1, target @ ..] if target.len() == 16 => Ok(Self::Query(Some(
                target.try_into().map_err(|_| BadgesWireError)?,
            ))),
            [PREFIX, 2, badge, public @ 0..=1] => Ok(Self::Set {
                badge: Badge::from_id(*badge).ok_or(BadgesWireError)?,
                public: *public == 1,
            }),
            _ => Err(BadgesWireError),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub earned: Vec<Badge>,
    pub hidden: Vec<Badge>,
}
impl Reply {
    pub fn encode(&self) -> Result<Vec<u8>, BadgesWireError> {
        let valid = |badges: &[Badge]| {
            badges.len() <= BADGE_COUNT && badges.windows(2).all(|pair| pair[0].id() < pair[1].id())
        };
        if !valid(&self.earned)
            || !valid(&self.hidden)
            || self.hidden.iter().any(|badge| !self.earned.contains(badge))
        {
            return Err(BadgesWireError);
        }
        let mut bytes = vec![PREFIX, self.earned.len() as u8];
        bytes.extend(self.earned.iter().map(|badge| badge.id()));
        bytes.push(self.hidden.len() as u8);
        bytes.extend(self.hidden.iter().map(|badge| badge.id()));
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, BadgesWireError> {
        if bytes.first() != Some(&PREFIX) || bytes.len() > MAX_REPLY_BYTES {
            return Err(BadgesWireError);
        }
        let count = usize::from(*bytes.get(1).ok_or(BadgesWireError)?);
        let hidden = usize::from(*bytes.get(count + 2).ok_or(BadgesWireError)?);
        if bytes.len() != 3 + count + hidden {
            return Err(BadgesWireError);
        }
        let parse = |ids: &[u8]| {
            ids.iter()
                .map(|id| Badge::from_id(*id).ok_or(BadgesWireError))
                .collect::<Result<Vec<_>, _>>()
        };
        let reply = Self {
            earned: parse(bytes.get(2..count + 2).ok_or(BadgesWireError)?)?,
            hidden: parse(bytes.get(count + 3..).ok_or(BadgesWireError)?)?,
        };
        reply.encode()?;
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cyclopedia_character::Badge;
    #[test]
    fn badge_visibility_round_trip_rejects_unearned_hidden_and_ambiguous_requests()
    -> Result<(), crate::cyclopedia_character::BadgesWireError> {
        for request in [
            Request::Query(None),
            Request::Query(Some([7; 16])),
            Request::Set {
                badge: Badge::FledgelingHero,
                public: false,
            },
        ] {
            assert_eq!(Request::decode(&request.encode()), Ok(request));
        }
        let reply = Reply {
            earned: vec![Badge::FledgelingHero],
            hidden: vec![Badge::FledgelingHero],
        };
        assert_eq!(Reply::decode(&reply.encode()?), Ok(reply));
        assert!(
            Reply {
                earned: vec![],
                hidden: vec![Badge::FledgelingHero]
            }
            .encode()
            .is_err()
        );
        assert!(Request::decode(&[PREFIX, 2, 1, 2]).is_err());
        assert!(Request::decode(&[PREFIX, 0, 0]).is_err());
        Ok(())
    }
}
