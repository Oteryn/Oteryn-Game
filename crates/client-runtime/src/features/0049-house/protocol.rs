//! HOUSE-WIRE-1 accepted differences: native UUID ACLs, revision CAS, 20-row keyset pages.
//! Donor: Crystal 201189f8 src/server/network/protocol/protocolgame.cpp:11338,2238.
//! ACL/doors/kick: Crystal src/map/house/house.cpp:230,252,523,806.
//! Bounds/UUIDs/CAS: accepted HOUSE-WIRE-1 D1261/D1350/D1354/D1384 (task parity table).
pub use oteryn_session::property_services::house::{
    HouseAclRead, HouseAclSelector, HouseAclSnapshot, decode_house_acl_snapshot,
    encode_house_acl_snapshot,
};
pub use oteryn_session::property_services::house_sale;

use crate::raw_feature::FeatureCommand;
type Result<T> = std::result::Result<T, &'static str>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Bid(u64),
    Notice(u64),
    Cancel,
    Guests(Vec<[u8; 16]>),
    Subowners(Vec<[u8; 16]>),
    Door {
        x: u16,
        y: u16,
        floor: i8,
        revision: u64,
        members: Vec<[u8; 16]>,
    },
    Kick([u8; 16]),
    Leave,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub size: u32,
    pub beds: u16,
    pub entrance: Option<(u16, u16, i8)>,
    pub house: String,
    pub state: u64,
    pub rent: u64,
    pub revision: u64,
    pub acl_revision: u64,
    pub owned: bool,
    pub price: u64,
    pub notice: Option<u64>,
    pub ends: Option<u64>,
    pub maximum: Option<u64>,
    pub kind: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub rows: Vec<Row>,
    pub cursor: Option<String>,
}
fn var(out: &mut Vec<u8>, mut n: u64) {
    while n > 127 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
}
fn number(tag: u8, n: u64) -> Vec<u8> {
    let mut out = vec![tag << 3];
    var(&mut out, n);
    out
}
fn bytes(tag: u8, b: &[u8]) -> Vec<u8> {
    let mut out = vec![tag << 3 | 2];
    var(&mut out, b.len() as u64);
    out.extend_from_slice(b);
    out
}
fn require(ok: bool, message: &'static str) -> Result<()> {
    ok.then_some(()).ok_or(message)
}
fn text(b: &[u8]) -> Result<&str> {
    require(!b.is_empty() && b.len() <= 512, "Invalid text length")?;
    std::str::from_utf8(b).map_err(|_| "Invalid UTF-8")
}
fn id(id: &[u8; 16]) -> Result<()> {
    require(id[6] >> 4 == 7 && id[8] & 192 == 128, "Invalid UUIDv7")
}
pub fn query(world: [u8; 16], town: &str, cursor: Option<&str>) -> Result<FeatureCommand> {
    id(&world)?;
    text(town.as_bytes())?;
    let suffix = town
        .strip_prefix("oteryn:content.area.city.")
        .ok_or("Invalid town key")?;
    require(
        !suffix.is_empty()
            && suffix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "Invalid town key",
    )?;
    let mut payload = [bytes(1, &world), bytes(2, town.as_bytes())].concat();
    payload.extend(number(4, 1));
    if let Some(c) = cursor {
        text(c.as_bytes())?;
        payload.extend(bytes(3, c.as_bytes()));
    }
    Ok(FeatureCommand {
        capability: 49,
        kind: 57,
        payload,
    })
}
pub fn mutation(
    world: [u8; 16],
    house: &str,
    revision: u64,
    action: &Action,
) -> Result<FeatureCommand> {
    id(&world)?;
    text(house.as_bytes())?;
    let positive = !matches!(action, Action::Bid(0) | Action::Notice(0));
    require(positive, "Enter a positive amount or time")?;
    let binding = [bytes(1, &world), bytes(2, house.as_bytes())].concat();
    let (tag, mut body) = match action {
        Action::Bid(n) => (1, number(1, *n)),
        Action::Notice(n) => (2, number(1, *n)),
        Action::Cancel => (3, Vec::new()),
        Action::Guests(_) | Action::Subowners(_) => {
            let kind = 1 + u64::from(matches!(action, Action::Subowners(_)));
            (4, [number(1, kind), number(2, revision)].concat())
        }
        Action::Door {
            x,
            y,
            floor,
            revision,
            ..
        } => {
            require((-15..=0).contains(floor), "Invalid floor")?;
            let floor = i64::from(*floor);
            let position = [
                number(1, (*x).into()),
                number(2, (*y).into()),
                number(3, ((floor << 1) ^ (floor >> 63)) as u64),
            ]
            .concat();
            (5, [bytes(1, &position), number(2, *revision)].concat())
        }
        Action::Kick(target) => {
            id(target)?;
            (6, bytes(1, target))
        }
        Action::Leave => (7, Vec::new()),
    };
    if let Action::Guests(members) | Action::Subowners(members) | Action::Door { members, .. } =
        action
    {
        require(members.len() <= 200, "At most 200 members")?;
        for (i, member) in members.iter().enumerate() {
            id(member)?;
            require(!members[..i].contains(member), "Duplicate member")?;
            body.extend(bytes(3, member));
        }
    }
    Ok(FeatureCommand {
        capability: 49,
        kind: 56,
        payload: [bytes(8, &binding), bytes(tag, &body)].concat(),
    })
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn var(&mut self) -> Result<u64> {
        let mut n = 0;
        for shift in (0..70).step_by(7) {
            let (&b, rest) = self.0.split_first().ok_or("Truncated varint")?;
            self.0 = rest;
            require(!(shift == 63 && b > 1), "Varint overflow")?;
            n |= u64::from(b & 127) << shift;
            if b < 128 {
                return Ok(n);
            }
        }
        Err("Invalid varint")
    }
    fn bytes(&mut self) -> Result<&'a [u8]> {
        let len = usize::try_from(self.var()?).map_err(|_| "Length overflow")?;
        require(len <= self.0.len(), "Truncated field")?;
        let (b, rest) = self.0.split_at(len);
        self.0 = rest;
        Ok(b)
    }
}
fn row(body: &[u8]) -> Result<Row> {
    let mut r = Reader(body);
    let mut row = Row {
        kind: 1,
        ..Row::default()
    };
    let mut seen = 0u32;
    let (mut x, mut y, mut floor) = (0, 0, 0);
    while !r.0.is_empty() {
        let key = r.var()?;
        let tag = key >> 3;
        require(
            (1..=17).contains(&tag) && seen & (1 << tag) == 0,
            "Invalid row field",
        )?;
        seen |= 1 << tag;
        if tag == 1 || tag == 12 {
            require(key & 7 == 2, "Invalid house text field")?;
            let value = text(r.bytes()?)?.into();
            if tag == 1 {
                row.house = value;
            } else {
                row.name = value;
            }
            continue;
        }
        require(key & 7 == 0, "Invalid scalar")?;
        let n = r.var()?;
        match tag {
            2 if (1..=6).contains(&n) => row.state = n,
            3 => row.rent = n,
            4 => row.revision = n,
            5 if n <= 1 => row.owned = n == 1,
            6 => row.notice = Some(n),
            7 => row.price = n,
            8 => row.ends = Some(n),
            9 => row.maximum = Some(n),
            10 => row.acl_revision = n,
            11 if (1..=3).contains(&n) => row.kind = n,
            13 if n > 0 => row.size = n.try_into().map_err(|_| "Invalid size")?,
            14 => row.beds = n.try_into().map_err(|_| "Invalid beds")?,
            15 => x = n.try_into().map_err(|_| "Invalid entrance x")?,
            16 => y = n.try_into().map_err(|_| "Invalid entrance y")?,
            17 if n == 0 || (n <= 29 && n & 1 == 1) => floor = ((n >> 1) as i8) ^ -((n & 1) as i8),
            _ => return Err("Invalid row value"),
        }
    }
    // Required tags: house, state, rent, revision, owned, price, ACL revision.
    require(seen & 1214 == 1214, "Missing row field")?;
    let details = (12..=17).fold(0u32, |mask, tag| mask | (1 << tag));
    require(
        seen & details == 0 || seen & details == details,
        "Incomplete house metadata",
    )?;
    if seen & details != 0 {
        row.entrance = Some((x, y, floor));
    }
    Ok(row)
}
pub fn page(body: &[u8]) -> Result<Page> {
    require(body.len() <= 32768, "Page too large")?;
    let mut r = Reader(body);
    let mut page = Page::default();
    let mut disposition = None;
    while !r.0.is_empty() {
        match r.var()? {
            8 if disposition.is_none() => disposition = Some(r.var()?),
            26 if page.rows.len() < 20 => {
                let row = row(r.bytes()?)?;
                require(
                    page.rows.last().is_none_or(|old| old.house < row.house),
                    "Unordered page",
                )?;
                page.rows.push(row);
            }
            34 if page.cursor.is_none() => page.cursor = Some(text(r.bytes()?)?.into()),
            _ => return Err("Invalid page field"),
        }
    }
    require(disposition == Some(1), "Server refused catalogue")?;
    require(
        !(page
            .cursor
            .as_ref()
            .is_some_and(|c| page.rows.last().is_none_or(|r| &r.house != c))),
        "Invalid page cursor",
    )?;
    Ok(page)
}
pub fn result(body: &[u8]) -> Result<(u64, u64)> {
    let mut r = Reader(body);
    require(r.var()? == 8, "Missing disposition")?;
    let d = r.var()?;
    require((1..=11).contains(&d) && r.var()? == 16, "Invalid result")?;
    let revision = r.var()?;
    require(r.0.is_empty(), "Trailing result")?;
    Ok((d, revision))
}

pub fn acl_query(world: [u8; 16], town: &str, read: &HouseAclRead) -> Result<FeatureCommand> {
    let payload =
        oteryn_session::property_services::house::encode_house_acl_read_query(world, town, read)
            .map_err(|_| "Invalid house ACL query")?;
    Ok(FeatureCommand {
        capability: 49,
        kind: 57,
        payload,
    })
}
