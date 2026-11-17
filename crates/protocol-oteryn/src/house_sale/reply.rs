use super::{Error, MAX_GOLD, PAGE_SIZE, codec, valid_house};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseSaleRow {
    pub house: String,
    pub name: String,
    pub revision: u64,
    pub owner: String,
    pub recipient: String,
    /// Active transfer: 0 none, 1 offered, 2 accepted.
    pub transfer_state: u8,
    pub due_at_unix_ms: i64,
    pub price: u64,
    pub rent: u64,
    /// Bits 1..6 correspond to request actions, projected from the current actor.
    pub allowed: u8,
    pub move_out_state: u8,
}
impl HouseSaleRow {
    fn valid(&self) -> bool {
        valid_house(&self.house)
            && !self.name.is_empty()
            && self.name.len() <= 128
            && self.owner.len() <= 64
            && self.recipient.len() <= 64
            && self.revision > 0
            && self.transfer_state <= 2
            && self.due_at_unix_ms >= 0
            && self.move_out_state <= 4
            && self.price <= MAX_GOLD
            && self.rent <= MAX_GOLD
            && self.price + self.rent <= MAX_GOLD
            && self.allowed & !0x7e == 0
            && (self.transfer_state == 0 || (!self.recipient.is_empty() && self.due_at_unix_ms > 0))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseSaleReply {
    pub now_unix_ms: i64,
    pub rows: Vec<HouseSaleRow>,
    pub has_more: bool,
}
pub fn encode_reply(reply: &HouseSaleReply) -> Result<Vec<u8>, Error> {
    if reply.now_unix_ms <= 0
        || reply.rows.len() > PAGE_SIZE
        || reply.rows.iter().any(|r| !r.valid())
        || reply.rows.windows(2).any(|r| r[0].house >= r[1].house)
    {
        return Err(Error::Malformed);
    }
    let mut out = vec![1];
    out.extend_from_slice(&reply.now_unix_ms.to_be_bytes());
    out.extend_from_slice(&[u8::from(reply.has_more), reply.rows.len() as u8]);
    for r in &reply.rows {
        for text in [&r.house, &r.name, &r.owner, &r.recipient] {
            codec::text(&mut out, text)?;
        }
        for number in [r.revision, r.due_at_unix_ms as u64, r.price, r.rent] {
            out.extend_from_slice(&number.to_be_bytes());
        }
        out.extend_from_slice(&[r.transfer_state, r.allowed, r.move_out_state]);
    }
    Ok(out)
}
pub fn decode_reply(bytes: &[u8]) -> Result<HouseSaleReply, Error> {
    if bytes.len() > 16384 {
        return Err(Error::LimitExceeded);
    }
    let mut input = codec::Reader::new(bytes);
    if input.byte()? != 1 {
        return Err(Error::Malformed);
    }
    let now_unix_ms = input.signed()?;
    let has_more = match input.byte()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Malformed),
    };
    let count = input.byte()? as usize;
    if count > PAGE_SIZE {
        return Err(Error::LimitExceeded);
    }
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        let house = input.text(512)?;
        let name = input.text(128)?;
        let owner = input.text(64)?;
        let recipient = input.text(64)?;
        rows.push(HouseSaleRow {
            house,
            name,
            owner,
            recipient,
            revision: input.number()?,
            due_at_unix_ms: input.signed()?,
            price: input.number()?,
            rent: input.number()?,
            transfer_state: input.byte()?,
            allowed: input.byte()?,
            move_out_state: input.byte()?,
        });
    }
    let reply = HouseSaleReply {
        now_unix_ms,
        rows,
        has_more,
    };
    if !input.finished() {
        return Err(Error::Malformed);
    }
    encode_reply(&reply)?;
    Ok(reply)
}
