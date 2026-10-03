//! Current actor condition projection candidate. No routing or capability ID is allocated.
//! Native coordinates retain their signed qualified frame; this is not a VIS2 coordinate cast.
use crate::world_spatial_entities::EntityRef;
pub const CAPABILITY_NAME: &str = "ACTOR_CURRENT_CONDITIONS_CANDIDATE_V1";
pub const MAX_BYTES: usize = 2048;
pub const MAX_KEY_BYTES: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Malformed,
    Limit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Light {
    pub level: u8,
    pub color: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporaryAppearance {
    Outfit {
        key: String,
        colours: [u16; 4],
        addons: u8,
        mount: Option<String>,
    },
    Item {
        key: String,
        revision: String,
        artifact: [u8; 32],
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub entity: EntityRef,
    pub content: [u8; 32],
    pub map: [u8; 32],
    pub frame: [u8; 32],
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub owner_time_us: u64,
    pub player_revision: u64,
    pub invisible: bool,
    pub condition_light: Option<Light>,
    pub temporary_appearance: Option<TemporaryAppearance>,
}
fn key(out: &mut Vec<u8>, s: &str) -> Result<(), Error> {
    if s.is_empty() || s.len() > MAX_KEY_BYTES || s.chars().any(char::is_control) {
        return Err(Error::Limit);
    }
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
    Ok(())
}
pub fn encode(s: &Snapshot) -> Result<Vec<u8>, Error> {
    if s.entity.identity == [0; 16]
        || s.entity.generation == 0
        || s.content == [0; 32]
        || s.map == [0; 32]
        || s.frame == [0; 32]
        || s.player_revision == 0
    {
        return Err(Error::Malformed);
    }
    let mut out = b"ACS1".to_vec();
    out.extend_from_slice(&s.entity.identity);
    out.extend_from_slice(&s.entity.generation.to_be_bytes());
    for pin in [s.content, s.map, s.frame] {
        out.extend_from_slice(&pin);
    }
    out.extend_from_slice(&s.x.to_be_bytes());
    out.extend_from_slice(&s.y.to_be_bytes());
    out.extend_from_slice(&s.floor.to_be_bytes());
    out.extend_from_slice(&s.owner_time_us.to_be_bytes());
    out.extend_from_slice(&s.player_revision.to_be_bytes());
    out.push(u8::from(s.invisible));
    match s.condition_light {
        None => out.push(0),
        Some(light) => out.extend_from_slice(&[1, light.level, light.color]),
    }
    match &s.temporary_appearance {
        None => out.push(0),
        Some(TemporaryAppearance::Outfit {
            key: k,
            colours,
            addons,
            mount,
        }) => {
            if *addons > 3 {
                return Err(Error::Malformed);
            }
            out.push(1);
            key(&mut out, k)?;
            for c in colours {
                out.extend_from_slice(&c.to_be_bytes());
            }
            out.push(*addons);
            match mount {
                None => out.push(0),
                Some(m) => {
                    out.push(1);
                    key(&mut out, m)?;
                }
            }
        }
        Some(TemporaryAppearance::Item {
            key: k,
            revision,
            artifact,
        }) => {
            if *artifact != s.content {
                return Err(Error::Malformed);
            }
            out.push(2);
            key(&mut out, k)?;
            key(&mut out, revision)?;
            out.extend_from_slice(artifact);
        }
    }
    if out.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    Ok(out)
}
struct Reader<'a> {
    input: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.at.checked_add(N).ok_or(Error::Limit)?;
        let v = self
            .input
            .get(self.at..end)
            .ok_or(Error::Malformed)?
            .try_into()
            .map_err(|_| Error::Malformed)?;
        self.at = end;
        Ok(v)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.bytes::<1>()?[0])
    }
    fn key(&mut self) -> Result<String, Error> {
        let n = usize::from(u16::from_be_bytes(self.bytes()?));
        if n == 0 || n > MAX_KEY_BYTES {
            return Err(Error::Limit);
        }
        let end = self.at.checked_add(n).ok_or(Error::Limit)?;
        let s = std::str::from_utf8(self.input.get(self.at..end).ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?
            .to_owned();
        self.at = end;
        if s.chars().any(char::is_control) {
            return Err(Error::Malformed);
        }
        Ok(s)
    }
}
pub fn decode(input: &[u8]) -> Result<Snapshot, Error> {
    if input.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    let mut r = Reader { input, at: 0 };
    if r.bytes::<4>()? != *b"ACS1" {
        return Err(Error::Malformed);
    }
    let entity = EntityRef {
        identity: r.bytes()?,
        generation: u64::from_be_bytes(r.bytes()?),
    };
    let content = r.bytes()?;
    let map = r.bytes()?;
    let frame = r.bytes()?;
    let x = i32::from_be_bytes(r.bytes()?);
    let y = i32::from_be_bytes(r.bytes()?);
    let floor = i16::from_be_bytes(r.bytes()?);
    let owner_time_us = u64::from_be_bytes(r.bytes()?);
    let player_revision = u64::from_be_bytes(r.bytes()?);
    let invisible = match r.byte()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Malformed),
    };
    let condition_light = match r.byte()? {
        0 => None,
        1 => Some(Light {
            level: r.byte()?,
            color: r.byte()?,
        }),
        _ => return Err(Error::Malformed),
    };
    let temporary_appearance = match r.byte()? {
        0 => None,
        1 => {
            let key = r.key()?;
            let mut colours = [0; 4];
            for c in &mut colours {
                *c = u16::from_be_bytes(r.bytes()?);
            }
            let addons = r.byte()?;
            let mount = match r.byte()? {
                0 => None,
                1 => Some(r.key()?),
                _ => return Err(Error::Malformed),
            };
            Some(TemporaryAppearance::Outfit {
                key,
                colours,
                addons,
                mount,
            })
        }
        2 => Some(TemporaryAppearance::Item {
            key: r.key()?,
            revision: r.key()?,
            artifact: r.bytes()?,
        }),
        _ => return Err(Error::Malformed),
    };
    if r.at != input.len() {
        return Err(Error::Malformed);
    }
    let s = Snapshot {
        entity,
        content,
        map,
        frame,
        x,
        y,
        floor,
        owner_time_us,
        player_revision,
        invisible,
        condition_light,
        temporary_appearance,
    };
    encode(&s)?;
    Ok(s)
}
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn signed_native_frame_and_closed_item_projection_roundtrip() {
        let s = Snapshot {
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            content: [2; 32],
            map: [3; 32],
            frame: [4; 32],
            x: -120,
            y: 300,
            floor: -7,
            owner_time_us: 50_000,
            player_revision: 2,
            invisible: true,
            condition_light: Some(Light {
                level: 6,
                color: 215,
            }),
            temporary_appearance: Some(TemporaryAppearance::Item {
                key: "oteryn:item.tibia.i3264".into(),
                revision: "source-r1".into(),
                artifact: [2; 32],
            }),
        };
        let bytes = encode(&s).expect("typed candidate");
        assert_eq!(decode(&bytes), Ok(s.clone()));
        let mut extra = bytes;
        extra.push(0);
        assert_eq!(decode(&extra), Err(Error::Malformed));
        let mut foreign = s;
        foreign.content = [5; 32];
        assert_eq!(encode(&foreign), Err(Error::Malformed));
    }
}
