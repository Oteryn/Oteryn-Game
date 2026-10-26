//! PODIUM-1 bounded display commands. Capability 133 remains unoffered.
use crate::charm_wire::CyclopediaWireError as Error;

pub const CAPABILITY_PODIUM_V1: u32 = 133;
pub const COMMAND_TYPE_PODIUM: u32 = 154;
pub const DISPLAY_BYTES: usize = 15;
pub const MAX_REQUEST_BYTES: usize = 556;
/// A result prefix, version, palette and bounded outfit/mount choices.
pub const MAX_WINDOW_BYTES: usize = 20_911;
const _: () = assert!(MAX_WINDOW_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);

/// Mutable Item presentation. Visibility never grants cosmetic ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    pub look_type: u16,
    pub addons: u8,
    pub colours: [u8; 4],
    pub mount: u16,
    pub mount_colours: [u8; 4],
    /// North, east, south, west, in that order.
    pub direction: u8,
    pub show_outfit: bool,
    pub show_mount: bool,
    pub show_podium: bool,
}
impl Default for Display {
    fn default() -> Self {
        Self {
            look_type: 0,
            addons: 0,
            colours: [0; 4],
            mount: 0,
            mount_colours: [0; 4],
            direction: 2,
            show_outfit: false,
            show_mount: false,
            show_podium: true,
        }
    }
}

pub fn encode_display(value: Display) -> Result<[u8; DISPLAY_BYTES], Error> {
    if value.direction > 3
        || value.addons > 3
        || value
            .colours
            .iter()
            .chain(&value.mount_colours)
            .any(|v| *v > 132)
        || (value.show_outfit && value.look_type == 0)
        || (value.show_mount && value.mount == 0)
        || (value.look_type == 0 && value.addons != 0)
    {
        return Err(Error::Malformed);
    }
    let mut bytes = [0; DISPLAY_BYTES];
    bytes[..2].copy_from_slice(&value.look_type.to_be_bytes());
    bytes[2] = value.addons;
    bytes[3..7].copy_from_slice(&value.colours);
    bytes[7..9].copy_from_slice(&value.mount.to_be_bytes());
    bytes[9..13].copy_from_slice(&value.mount_colours);
    bytes[13] = value.direction;
    bytes[14] = u8::from(value.show_outfit)
        | (u8::from(value.show_mount) << 1)
        | (u8::from(value.show_podium) << 2);
    Ok(bytes)
}
pub fn decode_display(bytes: &[u8]) -> Result<Display, Error> {
    if bytes.len() != DISPLAY_BYTES || bytes[14] > 7 {
        return Err(Error::Malformed);
    }
    let value = Display {
        look_type: u16::from_be_bytes([bytes[0], bytes[1]]),
        addons: bytes[2],
        colours: bytes[3..7].try_into().map_err(|_| Error::Malformed)?,
        mount: u16::from_be_bytes([bytes[7], bytes[8]]),
        mount_colours: bytes[9..13].try_into().map_err(|_| Error::Malformed)?,
        direction: bytes[13],
        show_outfit: bytes[14] & 1 != 0,
        show_mount: bytes[14] & 2 != 0,
        show_podium: bytes[14] & 4 != 0,
    };
    encode_display(value)?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub house: String,
    pub item: [u8; 16],
    /// 0: window, 1: configure, 2: rotate clockwise.
    pub action: u8,
    pub expected_revision: u64,
    /// Window/rotate requests carry the default display, avoiding ignored inputs.
    pub display: Display,
}
pub fn encode_request(request: &Request) -> Result<Vec<u8>, Error> {
    let valid_house = request
        .house
        .strip_prefix("oteryn:content.house.")
        .is_some_and(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        });
    if !valid_house
        || request.house.len() > 512
        || request.action > 2
        || (request.action == 0 && request.expected_revision != 0)
        || request.item[6] >> 4 != 7
        || request.item[8] & 0xc0 != 0x80
        || (request.action != 1 && request.display != Display::default())
    {
        return Err(Error::Malformed);
    }
    let mut bytes = vec![1, request.action];
    bytes.extend_from_slice(&(request.house.len() as u16).to_be_bytes());
    bytes.extend_from_slice(request.house.as_bytes());
    bytes.extend_from_slice(&request.item);
    bytes.extend_from_slice(&request.expected_revision.to_be_bytes());
    bytes.extend_from_slice(&encode_display(request.display)?);
    Ok(bytes)
}
pub fn decode_request(bytes: &[u8]) -> Result<Request, Error> {
    if bytes.len() < 43 || bytes.len() > MAX_REQUEST_BYTES || bytes[0] != 1 {
        return Err(Error::Malformed);
    }
    let n = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
    if bytes.len() != 43 + n {
        return Err(Error::Malformed);
    }
    let request = Request {
        action: bytes[1],
        house: std::str::from_utf8(&bytes[4..4 + n])
            .map_err(|_| Error::Malformed)?
            .into(),
        item: bytes[4 + n..20 + n]
            .try_into()
            .map_err(|_| Error::Malformed)?,
        expected_revision: u64::from_be_bytes(
            bytes[20 + n..28 + n]
                .try_into()
                .map_err(|_| Error::Malformed)?,
        ),
        display: decode_display(&bytes[28 + n..])?,
    };
    encode_request(&request)?;
    Ok(request)
}

/// Command result: refusal code (zero on success), revision and exact Item display.
/// This is not a map-domain broadcast; the map owner must publish the resulting delta.
pub fn encode_result(refusal: u16, revision: u64, display: Display) -> Result<Vec<u8>, Error> {
    if refusal != 0 && !(1630..=1639).contains(&refusal) {
        return Err(Error::Malformed);
    }
    let mut bytes = refusal.to_be_bytes().to_vec();
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(&encode_display(display)?);
    Ok(bytes)
}

/// Successful Renown window: the stored Item presentation and the actor's currently
/// authorized choices. Look identities refer to the session's qualified content.
/// Choices are sorted numerically, unique and bounded by the native content limit.
pub fn encode_window(
    revision: u64,
    display: Display,
    palette: &[[u8; 3]],
    outfits: &[(u16, u8)],
    mounts: &[u16],
) -> Result<Vec<u8>, Error> {
    if palette.len() != 133
        || outfits.len() > 4096
        || mounts.len() > 4096
        || outfits
            .iter()
            .any(|(look, addons)| *look == 0 || *addons > 3)
        || outfits.windows(2).any(|v| v[0].0 >= v[1].0)
        || mounts.contains(&0)
        || mounts.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(Error::Malformed);
    }
    let mut bytes = encode_result(0, revision, display)?;
    bytes.push(1);
    bytes.extend_from_slice(&(palette.len() as u16).to_be_bytes());
    bytes.extend(palette.iter().flatten());
    bytes.extend_from_slice(&(outfits.len() as u16).to_be_bytes());
    for (look, addons) in outfits {
        bytes.extend_from_slice(&look.to_be_bytes());
        bytes.push(*addons);
    }
    bytes.extend_from_slice(&(mounts.len() as u16).to_be_bytes());
    for look in mounts {
        bytes.extend_from_slice(&look.to_be_bytes());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn podium_registry_window_limit_matches_codec() -> Result<(), Box<dyn std::error::Error>> {
        let registry: serde_json::Value =
            serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)?;
        let commands = registry["command_types"].as_array().ok_or("commands")?;
        let entry = commands
            .iter()
            .find(|v| v["id"] == COMMAND_TYPE_PODIUM)
            .ok_or("podium command")?;
        assert_eq!(entry["capability"], CAPABILITY_PODIUM_V1);
        assert_eq!(entry["max_result_payload_bytes"], MAX_WINDOW_BYTES);
        assert_eq!(entry["max_payload_bytes"], MAX_REQUEST_BYTES);
        Ok(())
    }
    #[test]
    fn podium_window_bounds_and_counterfactual() -> Result<(), Error> {
        let palette = [[7, 8, 9]; 133];
        let bytes = encode_window(4, Display::default(), &palette, &[(128, 3)], &[139])?;
        assert_eq!(&bytes[..25], encode_result(0, 4, Display::default())?);
        assert_eq!(&bytes[25..28], &[1, 0, 133]);
        assert_eq!(&bytes[28..427], palette.as_flattened());
        assert_eq!(&bytes[427..], &[0, 1, 0, 128, 3, 0, 1, 0, 139]);
        for outfits in [
            vec![(0, 0)],
            vec![(128, 4)],
            vec![(128, 0), (128, 1)],
            vec![(129, 0), (128, 0)],
        ] {
            assert!(encode_window(0, Display::default(), &palette, &outfits, &[]).is_err());
        }
        for mounts in [vec![0], vec![139, 139], vec![140, 139]] {
            assert!(encode_window(0, Display::default(), &palette, &[], &mounts).is_err());
        }
        assert!(encode_window(0, Display::default(), &palette[..132], &[], &[]).is_err());
        let outfits: Vec<_> = (1..=4096).map(|look| (look, 3)).collect();
        let mut mounts: Vec<_> = (1..=4096).collect();
        assert_eq!(
            encode_window(0, Display::default(), &palette, &outfits, &mounts)?.len(),
            MAX_WINDOW_BYTES
        );
        mounts.push(4097);
        assert!(encode_window(0, Display::default(), &palette, &outfits, &mounts).is_err());
        Ok(())
    }
    #[test]
    fn podium_codec_roundtrip_and_counterfactual() -> Result<(), Error> {
        let mut item = [0; 16];
        item[6] = 0x70;
        item[8] = 0x80;
        let request = Request {
            house: "oteryn:content.house.test".into(),
            item,
            action: 1,
            expected_revision: 4,
            display: Display {
                look_type: 128,
                addons: 3,
                colours: [0, 1, 2, 132],
                mount: 139,
                mount_colours: [132, 2, 1, 0],
                direction: 3,
                show_outfit: true,
                show_mount: true,
                show_podium: false,
            },
        };
        let bytes = encode_request(&request)?;
        assert_eq!(decode_request(&bytes)?, request);
        for n in 0..bytes.len() {
            assert!(decode_request(&bytes[..n]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode_request(&trailing).is_err());
        let mut invalid = request.clone();
        invalid.display.direction = 4;
        assert!(encode_request(&invalid).is_err());
        invalid = request.clone();
        invalid.display.colours[0] = 133;
        assert!(encode_request(&invalid).is_err());
        invalid = request;
        invalid.display.show_mount = true;
        invalid.display.mount = 0;
        assert!(encode_request(&invalid).is_err());
        Ok(())
    }
}
