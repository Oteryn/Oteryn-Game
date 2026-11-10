//! Capability 67 / command 84 read-only catalogue extension, explicitly prefixed ff 01.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field as bytes, push_varint_field as scalar,
    read_bytes, read_varint,
};
type Result<T> = std::result::Result<T, Error>;
pub const PAGE_SIZE: usize = 20;
pub const MAX_CATEGORIES: usize = 64;
pub const MAX_BYTES: usize = super::MAX_REPLY_BYTES;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub after: u16,
    pub race: u16,
    pub category: u8,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Card {
    /// First-species Warden completion; absent on non-creature/legacy cards.
    pub echo_warden_completed: Option<bool>,
    pub outfit_look_type: u32,
    pub object_look_type: Option<u32>,
    pub race: u16,
    pub name: String,
    pub category: String,
    pub detail: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub next: Option<u16>,
    pub categories: Vec<String>,
    pub cards: Vec<Card>,
}
pub fn encode_request(request: Request) -> Result<Vec<u8>> {
    if request.race > 1024
        || request.after > 8192
        || (request.race == 0 && request.after > 1024)
        || request.category > 64
    {
        return Err(Error::LimitExceeded);
    }
    let mut out = vec![255, 1];
    out.extend(request.after.to_le_bytes());
    out.extend(request.race.to_le_bytes());
    out.push(request.category);
    Ok(out)
}
/// Opt in to the T185 card field; legacy seven-byte requests keep their old reply shape.
pub fn encode_echo_request(request: Request) -> Result<Vec<u8>> {
    let mut body = encode_request(request)?;
    body.push(1);
    Ok(body)
}
pub fn decode_request(body: &[u8]) -> Result<Request> {
    let body = match body {
        [255, 1, _, _, _, _, _, 1] => &body[..7],
        _ => body,
    };
    let [255, 1, a, b, c, d, category] = body else {
        return Err(Error::Malformed);
    };
    let request = Request {
        after: u16::from_le_bytes([*a, *b]),
        race: u16::from_le_bytes([*c, *d]),
        category: *category,
    };
    encode_request(request)?;
    Ok(request)
}
fn text(raw: &[u8], max: usize, empty: bool) -> Result<String> {
    if raw.len() > max || (!empty && raw.is_empty()) {
        return Err(Error::LimitExceeded);
    }
    let value = std::str::from_utf8(raw).map_err(|_| Error::Malformed)?;
    if value.contains('\0') {
        return Err(Error::Malformed);
    }
    Ok(value.into())
}
pub fn encode_page(page: &Page) -> Result<Vec<u8>> {
    if page.cards.len() > PAGE_SIZE
        || page.categories.len() > MAX_CATEGORIES
        || page.next == Some(0)
        || page.next.is_some_and(|n| n > 8192)
    {
        return Err(Error::LimitExceeded);
    }
    if page.cards.windows(2).any(|w| w[0].race >= w[1].race)
        || page.categories.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(Error::Malformed);
    }
    if page
        .cards
        .iter()
        .any(|card| !page.categories.contains(&card.category))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    if let Some(next) = page.next {
        scalar(&mut out, 1, next.into());
    }
    for card in &page.cards {
        if !(1..=1024).contains(&card.race) {
            return Err(Error::Malformed);
        }
        text(card.name.as_bytes(), 128, false)?;
        text(card.category.as_bytes(), 128, false)?;
        text(card.detail.as_bytes(), 16000, true)?;
        if card.object_look_type == Some(0)
            || (card.object_look_type.is_some() && card.outfit_look_type != 0)
        {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        scalar(&mut row, 1, card.race.into());
        bytes(&mut row, 2, card.name.as_bytes());
        bytes(&mut row, 3, card.category.as_bytes());
        bytes(&mut row, 4, card.detail.as_bytes());
        if card.outfit_look_type != 0 {
            scalar(&mut row, 5, card.outfit_look_type.into());
        }
        if let Some(object) = card.object_look_type {
            scalar(&mut row, 6, object.into());
        }
        if let Some(completed) = card.echo_warden_completed {
            scalar(&mut row, 7, u64::from(completed));
        }
        bytes(&mut out, 2, &row);
    }
    for category in &page.categories {
        text(category.as_bytes(), 128, false)?;
        bytes(&mut out, 3, category.as_bytes());
    }
    if out.len() > MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
fn card(body: &[u8]) -> Result<Card> {
    let (mut cursor, mut seen) = (0, 0u8);
    let mut card = Card::default();
    while cursor < body.len() {
        let key = read_varint(body, &mut cursor)?;
        let tag = key >> 3;
        if !(1..=7).contains(&tag) || seen & (1 << tag) != 0 {
            return Err(Error::Malformed);
        }
        seen |= 1 << tag;
        match key {
            8 => {
                card.race = read_varint(body, &mut cursor)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?
            }
            18 => card.name = text(read_bytes(body, &mut cursor)?, 128, false)?,
            26 => card.category = text(read_bytes(body, &mut cursor)?, 128, false)?,
            34 => card.detail = text(read_bytes(body, &mut cursor)?, 16000, true)?,
            40 => {
                card.outfit_look_type = read_varint(body, &mut cursor)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?
            }
            48 => {
                card.object_look_type = Some(
                    read_varint(body, &mut cursor)?
                        .try_into()
                        .map_err(|_| Error::Malformed)?,
                )
            }
            56 => {
                card.echo_warden_completed = Some(match read_varint(body, &mut cursor)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                });
            }
            _ => return Err(Error::Malformed),
        }
    }
    if seen & 30 != 30 {
        return Err(Error::Malformed);
    }
    Ok(card)
}
pub fn decode_page(body: &[u8]) -> Result<Page> {
    if body.len() > MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut page = Page::default();
    let mut cursor = 0;
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            8 if page.next.is_none() => {
                page.next = Some(
                    read_varint(body, &mut cursor)?
                        .try_into()
                        .map_err(|_| Error::Malformed)?,
                )
            }
            18 if page.cards.len() < PAGE_SIZE => {
                page.cards.push(card(read_bytes(body, &mut cursor)?)?)
            }
            26 if page.categories.len() < MAX_CATEGORIES => {
                page.categories
                    .push(text(read_bytes(body, &mut cursor)?, 128, false)?)
            }
            _ => return Err(Error::Malformed),
        }
    }
    encode_page(&page)?;
    Ok(page)
}

/// Charm names use the same bounded card/page reply, with a distinct opt-in request prefix.
pub fn encode_charm_request(after: u16) -> Result<Vec<u8>> {
    if u32::from(after) > crate::charm::MAX_CHARM {
        return Err(Error::LimitExceeded);
    }
    Ok(vec![
        254,
        1,
        u8::try_from(after).map_err(|_| Error::LimitExceeded)?,
    ])
}
pub fn decode_charm_request(bytes: &[u8]) -> Result<u16> {
    let [254, 1, after] = bytes else {
        return Err(Error::Malformed);
    };
    let after = u16::from(*after);
    encode_charm_request(after)?;
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn charm_catalogue_prefix_is_bounded_and_distinct_from_creatures() -> Result<()> {
        assert_eq!(encode_charm_request(20)?, vec![254, 1, 20]);
        assert_eq!(decode_charm_request(&[254, 1, 20])?, 20);
        for len in 0..3 {
            assert!(decode_charm_request(&[254, 1, 20][..len]).is_err());
        }
        assert!(decode_charm_request(&[254, 1, 20, 0]).is_err());
        assert!(decode_charm_request(&[255, 1, 20]).is_err());
        assert!(encode_charm_request(33).is_err());
        assert!(decode_charm_request(&[254, 1, 33]).is_err());
        Ok(())
    }

    #[test]
    fn echo_request_opts_in_without_changing_legacy_requests() -> Result<()> {
        let request = Request {
            after: 0,
            race: 7,
            category: 0,
        };
        let legacy = encode_request(request)?;
        assert_eq!(legacy, [255, 1, 0, 0, 7, 0, 0]);
        let mut echo = encode_echo_request(request)?;
        assert_eq!(&echo[..7], legacy);
        assert_eq!(decode_request(&echo)?, request);
        echo[7] = 2;
        assert!(decode_request(&echo).is_err());
        echo[7] = 1;
        echo.push(1);
        assert!(decode_request(&echo).is_err());
        Ok(())
    }

    #[test]
    fn echo_warden_boolean_is_optional_strict_and_not_duplicated() -> Result<()> {
        let prefix = [8, 7, 18, 1, b'R', 26, 1, b'F', 34, 0];
        assert_eq!(card(&prefix)?.echo_warden_completed, None);
        for (value, completed) in [(0, false), (1, true)] {
            let mut row = prefix.to_vec();
            row.extend([56, value]);
            assert_eq!(card(&row)?.echo_warden_completed, Some(completed));
            let page = Page {
                next: None,
                categories: vec!["F".into()],
                cards: vec![card(&row)?],
            };
            assert_eq!(decode_page(&encode_page(&page)?)?, page);
            row.extend([56, value]);
            assert!(card(&row).is_err());
        }
        let mut row = prefix.to_vec();
        row.extend([56, 2]);
        assert!(card(&row).is_err());
        Ok(())
    }

    #[test]
    fn catalogue_wire_checks_bounds_duplicates_and_round_trip() -> Result<()> {
        let request = Request {
            after: 0,
            race: 7,
            category: 1,
        };
        let bytes = encode_request(request)?;
        assert_eq!(decode_request(&bytes)?, request);
        for end in 0..bytes.len() {
            assert!(decode_request(&bytes[..end]).is_err());
        }
        assert!(
            encode_request(Request {
                after: 1025,
                race: 0,
                category: 0
            })
            .is_err()
        );
        let page = Page {
            next: Some(20),
            categories: vec!["Forest dwellers".into()],
            cards: vec![Card {
                echo_warden_completed: None,
                outfit_look_type: 128,
                object_look_type: None,
                race: 7,
                name: "Grove watcher".into(),
                category: "Forest dwellers".into(),
                detail: "Health: 90".into(),
            }],
        };
        let bytes = encode_page(&page)?;
        assert_eq!(decode_page(&bytes)?, page);
        let mut duplicate = bytes.clone();
        duplicate.extend([8, 20]);
        assert!(decode_page(&duplicate).is_err());
        let mut invalid = page.clone();
        invalid.cards[0].detail = "x".repeat(16001);
        assert!(encode_page(&invalid).is_err());
        invalid = page.clone();
        invalid.cards.push(page.cards[0].clone());
        assert!(encode_page(&invalid).is_err());
        invalid = page;
        invalid.categories.clear();
        assert!(encode_page(&invalid).is_err());
        assert!(decode_page(&vec![0; MAX_BYTES + 1]).is_err());
        Ok(())
    }
}
