//! Full Task Board actions for command 89. Legacy actions 0..=3 keep their original codec.
use super::{Error, TALISMAN_PATHS, valid_race};
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};

pub const MAX_REQUEST_BYTES: usize = 192;
pub const MAX_REPLY_BYTES: usize = 16_384;
pub const MAX_KILL_TASKS: usize = 9;
pub const MAX_DELIVERY_TASKS: usize = 9;
pub const MAX_BOUNTY_OFFERS: usize = 3;
pub const MAX_LIST_SLOTS: usize = 5;
pub const MAX_SHOP_OFFERS: usize = 32;

/// Full actions use zero-based slots and difficulty values. A blank race clears a list slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub action: u8,
    pub selection: u16,
    pub race: String,
}

impl Request {
    pub fn query() -> Self {
        Self {
            action: 4,
            selection: 0,
            race: String::new(),
        }
    }

    fn valid(&self) -> bool {
        let selection = usize::from(self.selection);
        let selected = match self.action {
            4 | 7 | 9 | 11 | 12 | 15 | 17 => selection == 0,
            5 | 10 | 18 => selection < 4,
            6 => selection < MAX_DELIVERY_TASKS,
            8 => selection < MAX_BOUNTY_OFFERS,
            13 | 14 => selection < MAX_LIST_SLOTS,
            16 => selection < MAX_SHOP_OFFERS,
            _ => false,
        };
        selected
            && if matches!(self.action, 13 | 14) {
                self.race.is_empty() || valid_race(&self.race)
            } else {
                self.race.is_empty()
            }
    }
}

pub fn encode_request(request: &Request) -> Result<Vec<u8>, Error> {
    if !request.valid() {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(request.action));
    push_varint_field(&mut out, 2, u64::from(request.selection));
    push_message_field(&mut out, 3, request.race.as_bytes());
    if out.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_request(bytes: &[u8]) -> Result<Request, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut selection, mut race) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut action, u8_value(bytes, &mut at)?)?,
            16 => set_once(&mut selection, u16_value(bytes, &mut at)?)?,
            26 => set_once(&mut race, string_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let request = Request {
        action: action.ok_or(Error::Malformed)?,
        selection: selection.ok_or(Error::Malformed)?,
        race: race.ok_or(Error::Malformed)?,
    };
    if encode_request(&request)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(request)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillTask {
    /// Empty only for the any-creature task.
    pub race: String,
    pub required: u32,
    pub current: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryTask {
    /// Donor item identifier; custody resolves it to the admitted canonical Item identity.
    pub item: u16,
    pub required: u32,
    /// Undelivered: recorded carried + stash + depot availability, modulo 2^32.
    /// Delivered: exactly `required`. Crystal protocolgame.cpp:6210-6222.
    pub current: u32,
    pub delivered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weekly {
    pub started: bool,
    pub difficulty: u8,
    pub expanded: bool,
    pub multiplier: u8,
    pub kill_experience: u64,
    pub delivery_experience: u64,
    pub pending_points: u64,
    pub pending_soulseals: u64,
    pub kills: Vec<KillTask>,
    pub deliveries: Vec<DeliveryTask>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BountyOffer {
    pub race: String,
    pub required: u32,
    pub experience: u64,
    pub points: u32,
    pub grade: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListSlot {
    pub unlocked: bool,
    pub preferred: String,
    pub unwanted: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounty {
    pub state: u8,
    pub difficulty: u8,
    /// Zero when no task is selected; otherwise the original offer slot plus one.
    pub selected: u8,
    pub current: u32,
    pub rerolls: u64,
    pub daily_reroll_at: u64,
    pub offers: Vec<BountyOffer>,
    /// The selected offer remains available even after the generated list changes.
    pub active: Option<BountyOffer>,
    pub lists: Vec<ListSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TalismanPath {
    pub level: u16,
    pub bonus: u16,
    pub next_cost: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopOffer {
    pub index: u16,
    pub kind: u8,
    pub price: u32,
    pub primary: u32,
    pub secondary: u32,
    pub addon: u8,
    pub bought: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// 0 success; other values name the contract's durable refusal category.
    pub refusal: u8,
    /// Calendar week identity derived from a committed World reset epoch.
    pub period: u64,
    pub points: u64,
    pub bounty_points: u64,
    pub soulseals: u64,
    pub weekly: Weekly,
    pub bounty: Bounty,
    pub talisman: [TalismanPath; TALISMAN_PATHS],
    pub shop: Vec<ShopOffer>,
}

fn u8_value(bytes: &[u8], at: &mut usize) -> Result<u8, Error> {
    u8::try_from(read_varint(bytes, at)?).map_err(|_| Error::Malformed)
}
fn u16_value(bytes: &[u8], at: &mut usize) -> Result<u16, Error> {
    u16::try_from(read_varint(bytes, at)?).map_err(|_| Error::Malformed)
}
fn u32_value(bytes: &[u8], at: &mut usize) -> Result<u32, Error> {
    u32::try_from(read_varint(bytes, at)?).map_err(|_| Error::Malformed)
}
fn bool_value(bytes: &[u8], at: &mut usize) -> Result<bool, Error> {
    match read_varint(bytes, at)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Malformed),
    }
}
fn string_value(bytes: &[u8], at: &mut usize) -> Result<String, Error> {
    let raw = read_bytes(bytes, at)?;
    if raw.len() > 128 {
        return Err(Error::LimitExceeded);
    }
    Ok(std::str::from_utf8(raw)
        .map_err(|_| Error::Malformed)?
        .to_owned())
}
fn bounded_push<T>(rows: &mut Vec<T>, value: T, limit: usize) -> Result<(), Error> {
    if rows.len() == limit {
        return Err(Error::LimitExceeded);
    }
    rows.push(value);
    Ok(())
}
fn race_or_empty(race: &str) -> bool {
    race.is_empty() || valid_race(race)
}

fn encode_kill(task: &KillTask) -> Result<Vec<u8>, Error> {
    if !race_or_empty(&task.race) || task.required == 0 || task.current > task.required {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, task.race.as_bytes());
    push_varint_field(&mut out, 2, u64::from(task.required));
    push_varint_field(&mut out, 3, u64::from(task.current));
    Ok(out)
}
fn decode_kill(bytes: &[u8]) -> Result<KillTask, Error> {
    let (mut race, mut required, mut current) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(&mut race, string_value(bytes, &mut at)?)?,
            16 => set_once(&mut required, u32_value(bytes, &mut at)?)?,
            24 => set_once(&mut current, u32_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(KillTask {
        race: race.ok_or(Error::Malformed)?,
        required: required.ok_or(Error::Malformed)?,
        current: current.ok_or(Error::Malformed)?,
    })
}
fn encode_delivery(task: &DeliveryTask) -> Result<Vec<u8>, Error> {
    if task.item == 0 || task.required == 0 || (task.delivered && task.current != task.required) {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(task.item));
    push_varint_field(&mut out, 2, u64::from(task.required));
    push_varint_field(&mut out, 3, u64::from(task.current));
    push_varint_field(&mut out, 4, u64::from(task.delivered));
    Ok(out)
}
fn decode_delivery(bytes: &[u8]) -> Result<DeliveryTask, Error> {
    let (mut item, mut required, mut current, mut delivered) = (None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut item, u16_value(bytes, &mut at)?)?,
            16 => set_once(&mut required, u32_value(bytes, &mut at)?)?,
            24 => set_once(&mut current, u32_value(bytes, &mut at)?)?,
            32 => set_once(&mut delivered, bool_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(DeliveryTask {
        item: item.ok_or(Error::Malformed)?,
        required: required.ok_or(Error::Malformed)?,
        current: current.ok_or(Error::Malformed)?,
        delivered: delivered.ok_or(Error::Malformed)?,
    })
}
fn encode_weekly(board: &Weekly) -> Result<Vec<u8>, Error> {
    if board.kills.len() > MAX_KILL_TASKS || board.deliveries.len() > MAX_DELIVERY_TASKS {
        return Err(Error::LimitExceeded);
    }
    if board.difficulty > 3
        || !matches!(board.multiplier, 1 | 2 | 3 | 5 | 8)
        || (!board.started && (!board.kills.is_empty() || !board.deliveries.is_empty()))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, value) in [
        (1, u64::from(board.started)),
        (2, u64::from(board.difficulty)),
        (3, u64::from(board.expanded)),
        (4, u64::from(board.multiplier)),
        (5, board.kill_experience),
        (6, board.delivery_experience),
        (7, board.pending_points),
        (8, board.pending_soulseals),
    ] {
        push_varint_field(&mut out, field, value);
    }
    for task in &board.kills {
        push_message_field(&mut out, 9, &encode_kill(task)?);
    }
    for task in &board.deliveries {
        push_message_field(&mut out, 10, &encode_delivery(task)?);
    }
    Ok(out)
}
fn decode_weekly(bytes: &[u8]) -> Result<Weekly, Error> {
    let (mut started, mut difficulty, mut expanded, mut multiplier) = (None, None, None, None);
    let (mut kill_experience, mut delivery_experience, mut pending_points, mut pending_soulseals) =
        (None, None, None, None);
    let (mut kills, mut deliveries) = (Vec::new(), Vec::new());
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut started, bool_value(bytes, &mut at)?)?,
            16 => set_once(&mut difficulty, u8_value(bytes, &mut at)?)?,
            24 => set_once(&mut expanded, bool_value(bytes, &mut at)?)?,
            32 => set_once(&mut multiplier, u8_value(bytes, &mut at)?)?,
            40 => set_once(&mut kill_experience, read_varint(bytes, &mut at)?)?,
            48 => set_once(&mut delivery_experience, read_varint(bytes, &mut at)?)?,
            56 => set_once(&mut pending_points, read_varint(bytes, &mut at)?)?,
            64 => set_once(&mut pending_soulseals, read_varint(bytes, &mut at)?)?,
            74 => bounded_push(
                &mut kills,
                decode_kill(read_bytes(bytes, &mut at)?)?,
                MAX_KILL_TASKS,
            )?,
            82 => bounded_push(
                &mut deliveries,
                decode_delivery(read_bytes(bytes, &mut at)?)?,
                MAX_DELIVERY_TASKS,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(Weekly {
        started: started.ok_or(Error::Malformed)?,
        difficulty: difficulty.ok_or(Error::Malformed)?,
        expanded: expanded.ok_or(Error::Malformed)?,
        multiplier: multiplier.ok_or(Error::Malformed)?,
        kill_experience: kill_experience.ok_or(Error::Malformed)?,
        delivery_experience: delivery_experience.ok_or(Error::Malformed)?,
        pending_points: pending_points.ok_or(Error::Malformed)?,
        pending_soulseals: pending_soulseals.ok_or(Error::Malformed)?,
        kills,
        deliveries,
    })
}
fn encode_bounty_offer(offer: &BountyOffer) -> Result<Vec<u8>, Error> {
    if !valid_race(&offer.race) || offer.required == 0 || offer.grade > 2 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, offer.race.as_bytes());
    for (field, value) in [
        (2, u64::from(offer.required)),
        (3, offer.experience),
        (4, u64::from(offer.points)),
        (5, u64::from(offer.grade)),
    ] {
        push_varint_field(&mut out, field, value);
    }
    Ok(out)
}
fn decode_bounty_offer(bytes: &[u8]) -> Result<BountyOffer, Error> {
    let (mut race, mut required, mut experience, mut points, mut grade) =
        (None, None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(&mut race, string_value(bytes, &mut at)?)?,
            16 => set_once(&mut required, u32_value(bytes, &mut at)?)?,
            24 => set_once(&mut experience, read_varint(bytes, &mut at)?)?,
            32 => set_once(&mut points, u32_value(bytes, &mut at)?)?,
            40 => set_once(&mut grade, u8_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(BountyOffer {
        race: race.ok_or(Error::Malformed)?,
        required: required.ok_or(Error::Malformed)?,
        experience: experience.ok_or(Error::Malformed)?,
        points: points.ok_or(Error::Malformed)?,
        grade: grade.ok_or(Error::Malformed)?,
    })
}
fn encode_list(slot: &ListSlot) -> Result<Vec<u8>, Error> {
    if !race_or_empty(&slot.preferred)
        || !race_or_empty(&slot.unwanted)
        || (!slot.unlocked && (!slot.preferred.is_empty() || !slot.unwanted.is_empty()))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(slot.unlocked));
    push_message_field(&mut out, 2, slot.preferred.as_bytes());
    push_message_field(&mut out, 3, slot.unwanted.as_bytes());
    Ok(out)
}
fn decode_list(bytes: &[u8]) -> Result<ListSlot, Error> {
    let (mut unlocked, mut preferred, mut unwanted) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut unlocked, bool_value(bytes, &mut at)?)?,
            18 => set_once(&mut preferred, string_value(bytes, &mut at)?)?,
            26 => set_once(&mut unwanted, string_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(ListSlot {
        unlocked: unlocked.ok_or(Error::Malformed)?,
        preferred: preferred.ok_or(Error::Malformed)?,
        unwanted: unwanted.ok_or(Error::Malformed)?,
    })
}
fn encode_bounty(board: &Bounty) -> Result<Vec<u8>, Error> {
    if board.offers.len() > MAX_BOUNTY_OFFERS || board.lists.len() > MAX_LIST_SLOTS {
        return Err(Error::LimitExceeded);
    }
    if board.state > 3
        || board.difficulty > 3
        || usize::from(board.selected) > MAX_BOUNTY_OFFERS
        || board
            .active
            .as_ref()
            .map_or(board.current != 0, |offer| board.current > offer.required)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, value) in [
        (1, u64::from(board.state)),
        (2, u64::from(board.difficulty)),
        (3, u64::from(board.selected)),
        (4, u64::from(board.current)),
        (5, board.rerolls),
        (6, board.daily_reroll_at),
    ] {
        push_varint_field(&mut out, field, value);
    }
    for offer in &board.offers {
        push_message_field(&mut out, 7, &encode_bounty_offer(offer)?);
    }
    if let Some(active) = &board.active {
        push_message_field(&mut out, 8, &encode_bounty_offer(active)?);
    }
    for slot in &board.lists {
        push_message_field(&mut out, 9, &encode_list(slot)?);
    }
    Ok(out)
}
fn decode_bounty(bytes: &[u8]) -> Result<Bounty, Error> {
    let (mut state, mut difficulty, mut selected, mut current, mut rerolls, mut daily_reroll_at) =
        (None, None, None, None, None, None);
    let (mut offers, mut active, mut lists) = (Vec::new(), None, Vec::new());
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut state, u8_value(bytes, &mut at)?)?,
            16 => set_once(&mut difficulty, u8_value(bytes, &mut at)?)?,
            24 => set_once(&mut selected, u8_value(bytes, &mut at)?)?,
            32 => set_once(&mut current, u32_value(bytes, &mut at)?)?,
            40 => set_once(&mut rerolls, read_varint(bytes, &mut at)?)?,
            48 => set_once(&mut daily_reroll_at, read_varint(bytes, &mut at)?)?,
            58 => bounded_push(
                &mut offers,
                decode_bounty_offer(read_bytes(bytes, &mut at)?)?,
                MAX_BOUNTY_OFFERS,
            )?,
            66 => set_once(
                &mut active,
                decode_bounty_offer(read_bytes(bytes, &mut at)?)?,
            )?,
            74 => bounded_push(
                &mut lists,
                decode_list(read_bytes(bytes, &mut at)?)?,
                MAX_LIST_SLOTS,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(Bounty {
        state: state.ok_or(Error::Malformed)?,
        difficulty: difficulty.ok_or(Error::Malformed)?,
        selected: selected.ok_or(Error::Malformed)?,
        current: current.ok_or(Error::Malformed)?,
        rerolls: rerolls.ok_or(Error::Malformed)?,
        daily_reroll_at: daily_reroll_at.ok_or(Error::Malformed)?,
        offers,
        active,
        lists,
    })
}
fn encode_talisman(path: &TalismanPath) -> Result<Vec<u8>, Error> {
    if path.bonus > 10_000 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, value) in [(1, path.level), (2, path.bonus), (3, path.next_cost)] {
        push_varint_field(&mut out, field, u64::from(value));
    }
    Ok(out)
}
fn decode_talisman(bytes: &[u8]) -> Result<TalismanPath, Error> {
    let (mut level, mut bonus, mut next_cost) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut level, u16_value(bytes, &mut at)?)?,
            16 => set_once(&mut bonus, u16_value(bytes, &mut at)?)?,
            24 => set_once(&mut next_cost, u16_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(TalismanPath {
        level: level.ok_or(Error::Malformed)?,
        bonus: bonus.ok_or(Error::Malformed)?,
        next_cost: next_cost.ok_or(Error::Malformed)?,
    })
}
fn encode_shop(offer: &ShopOffer) -> Result<Vec<u8>, Error> {
    if usize::from(offer.index) >= MAX_SHOP_OFFERS || offer.kind > 5 || offer.addon > 2 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, value) in [
        (1, u64::from(offer.index)),
        (2, u64::from(offer.kind)),
        (3, u64::from(offer.price)),
        (4, u64::from(offer.primary)),
        (5, u64::from(offer.secondary)),
        (6, u64::from(offer.addon)),
        (7, u64::from(offer.bought)),
    ] {
        push_varint_field(&mut out, field, value);
    }
    Ok(out)
}
fn decode_shop(bytes: &[u8]) -> Result<ShopOffer, Error> {
    let (mut index, mut kind, mut price, mut primary, mut secondary, mut addon, mut bought) =
        (None, None, None, None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut index, u16_value(bytes, &mut at)?)?,
            16 => set_once(&mut kind, u8_value(bytes, &mut at)?)?,
            24 => set_once(&mut price, u32_value(bytes, &mut at)?)?,
            32 => set_once(&mut primary, u32_value(bytes, &mut at)?)?,
            40 => set_once(&mut secondary, u32_value(bytes, &mut at)?)?,
            48 => set_once(&mut addon, u8_value(bytes, &mut at)?)?,
            56 => set_once(&mut bought, bool_value(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(ShopOffer {
        index: index.ok_or(Error::Malformed)?,
        kind: kind.ok_or(Error::Malformed)?,
        price: price.ok_or(Error::Malformed)?,
        primary: primary.ok_or(Error::Malformed)?,
        secondary: secondary.ok_or(Error::Malformed)?,
        addon: addon.ok_or(Error::Malformed)?,
        bought: bought.ok_or(Error::Malformed)?,
    })
}

pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    if reply.shop.len() > MAX_SHOP_OFFERS {
        return Err(Error::LimitExceeded);
    }
    if reply.refusal > 17
        || reply.points > i64::MAX.unsigned_abs()
        || reply.bounty_points > u64::from(u32::MAX)
        || reply.soulseals > u64::from(u32::MAX)
        || reply.bounty.rerolls > i64::MAX.unsigned_abs()
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, value) in [
        (1, u64::from(reply.refusal)),
        (2, reply.period),
        (3, reply.points),
        (4, reply.bounty_points),
        (5, reply.soulseals),
    ] {
        push_varint_field(&mut out, field, value);
    }
    push_message_field(&mut out, 6, &encode_weekly(&reply.weekly)?);
    push_message_field(&mut out, 7, &encode_bounty(&reply.bounty)?);
    for path in &reply.talisman {
        push_message_field(&mut out, 8, &encode_talisman(path)?);
    }
    for offer in &reply.shop {
        push_message_field(&mut out, 9, &encode_shop(offer)?);
    }
    if out.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (
        mut refusal,
        mut period,
        mut points,
        mut bounty_points,
        mut soulseals,
        mut weekly,
        mut bounty,
    ) = (None, None, None, None, None, None, None);
    let (mut talisman, mut shop) = (Vec::new(), Vec::new());
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut refusal, u8_value(bytes, &mut at)?)?,
            16 => set_once(&mut period, read_varint(bytes, &mut at)?)?,
            24 => set_once(&mut points, read_varint(bytes, &mut at)?)?,
            32 => set_once(&mut bounty_points, read_varint(bytes, &mut at)?)?,
            40 => set_once(&mut soulseals, read_varint(bytes, &mut at)?)?,
            50 => set_once(&mut weekly, decode_weekly(read_bytes(bytes, &mut at)?)?)?,
            58 => set_once(&mut bounty, decode_bounty(read_bytes(bytes, &mut at)?)?)?,
            66 => bounded_push(
                &mut talisman,
                decode_talisman(read_bytes(bytes, &mut at)?)?,
                TALISMAN_PATHS,
            )?,
            74 => bounded_push(
                &mut shop,
                decode_shop(read_bytes(bytes, &mut at)?)?,
                MAX_SHOP_OFFERS,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let reply = Reply {
        refusal: refusal.ok_or(Error::Malformed)?,
        period: period.ok_or(Error::Malformed)?,
        points: points.ok_or(Error::Malformed)?,
        bounty_points: bounty_points.ok_or(Error::Malformed)?,
        soulseals: soulseals.ok_or(Error::Malformed)?,
        weekly: weekly.ok_or(Error::Malformed)?,
        bounty: bounty.ok_or(Error::Malformed)?,
        talisman: talisman.try_into().map_err(|_| Error::Malformed)?,
        shop,
    };
    if encode_reply(&reply)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(reply)
}

#[cfg(test)]
#[path = "task_board_full_tests.rs"]
mod tests;
