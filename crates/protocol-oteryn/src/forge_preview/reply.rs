use super::*;
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let out = self.0.get(..N)?.try_into().ok()?;
        self.0 = &self.0[N..];
        Some(out)
    }
    fn byte(&mut self) -> Option<u8> {
        Some(self.take::<1>()?[0])
    }
    fn boolean(&mut self) -> Option<bool> {
        match self.byte()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take()?))
    }
    fn text(&mut self) -> Option<String> {
        let n = usize::from(u16::from_le_bytes(self.take()?));
        if n == 0 || n > MAX_NAME_BYTES {
            return None;
        }
        let text = std::str::from_utf8(self.0.get(..n)?).ok()?.to_owned();
        self.0 = &self.0[n..];
        Some(text)
    }
    fn resources(&mut self) -> Option<Resources> {
        Some(Resources {
            carried_gold: self.u64()?,
            bank_gold: self.u64()?,
            bank_spendable: self.boolean()?,
            dust: self.u32()?,
            dust_limit: self.u32()?,
            cores: self.u64()?,
            slivers: self.u64()?,
        })
    }
    fn item(&mut self) -> Option<Item> {
        Some(Item {
            handle: ItemHandle::new(self.u64()?)?,
            definition_ref: self.u32()?,
            appearance: self.u32()?,
            classification: self.byte()?,
            tier: self.byte()?,
            maximum_tier: self.byte()?,
            container: self.boolean()?,
            name: self.text()?,
        })
    }
}
fn item_valid(item: &Item) -> bool {
    item.definition_ref > 0
        && item.appearance <= 65535
        && !item.name.is_empty()
        && item.name.len() <= MAX_NAME_BYTES
        && !item.name.chars().any(char::is_control)
        && if item.container {
            item.classification == 0 && item.tier == 0 && item.maximum_tier == 0
        } else {
            (1..=4).contains(&item.classification)
                && (1..=10).contains(&item.maximum_tier)
                && item.tier <= item.maximum_tier
        }
}
fn resources_valid(r: &Resources) -> bool {
    r.dust <= r.dust_limit
}
fn write_resources(out: &mut Vec<u8>, r: Resources) {
    out.extend(r.carried_gold.to_le_bytes());
    out.extend(r.bank_gold.to_le_bytes());
    out.push(u8::from(r.bank_spendable));
    out.extend(r.dust.to_le_bytes());
    out.extend(r.dust_limit.to_le_bytes());
    out.extend(r.cores.to_le_bytes());
    out.extend(r.slivers.to_le_bytes());
}
fn write_item(out: &mut Vec<u8>, i: &Item) {
    out.extend(i.handle.get().to_le_bytes());
    out.extend(i.definition_ref.to_le_bytes());
    out.extend(i.appearance.to_le_bytes());
    out.extend([
        i.classification,
        i.tier,
        i.maximum_tier,
        u8::from(i.container),
    ]);
    out.extend((i.name.len() as u16).to_le_bytes());
    out.extend(i.name.as_bytes());
}
impl Reply {
    fn valid(&self) -> bool {
        if self.nonce == 0 {
            return false;
        }
        match &self.payload {
            Payload::Refused => true,
            Payload::Resource(q) => q.valid(),
            Payload::Page(p) => {
                resources_valid(&p.resources)
                    && p.items.len() <= PAGE_SIZE
                    && p.items.iter().all(item_valid)
                    && p.items.first().is_none_or(|i| i.handle.get() > p.after)
                    && p.items.windows(2).all(|r| r[0].handle < r[1].handle)
                    && p.next.is_none_or(|next| {
                        p.items.len() == PAGE_SIZE
                            && p.items.last().is_some_and(|i| i.handle.get() == next)
                    })
            }
            Payload::Quote(q) => {
                resources_valid(&q.resources)
                    && item_valid(&q.first)
                    && item_valid(&q.second)
                    && !q.first.container
                    && !q.second.container
                    && q.selection.canonical([0; 16]).is_some()
                    && matches!(q.selection, Selection::Pair {first,second,..} if first == q.first.handle && second == q.second.handle)
                    && (1..=100).contains(&q.success_percent)
                    && q.protection_percent <= 100
                    && (1..=q.first.maximum_tier).contains(&q.target_tier)
            }
        }
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let mut out = self.nonce.to_le_bytes().to_vec();
        match &self.payload {
            Payload::Refused => out.push(0),
            Payload::Resource(q) => {
                out.extend([3, q.operation as u8, u8::from(q.full_resources)]);
                write_resources(&mut out, q.before);
                out.extend(q.after_dust.to_le_bytes());
                out.extend(q.after_limit.to_le_bytes());
                out.extend(q.after_slivers.to_le_bytes());
                out.extend(q.after_cores.to_le_bytes());
            }
            Payload::Page(p) => {
                out.push(1);
                out.extend(p.after.to_le_bytes());
                out.extend(p.next.unwrap_or(0).to_le_bytes());
                write_resources(&mut out, p.resources);
                out.push(p.items.len() as u8);
                for i in &p.items {
                    write_item(&mut out, i);
                }
            }
            Payload::Quote(q) => {
                out.push(2);
                let request = Request {
                    nonce: self.nonce,
                    selection: q.selection,
                }
                .encode()?;
                out.extend(&request[9..]);
                write_item(&mut out, &q.first);
                write_item(&mut out, &q.second);
                write_resources(&mut out, q.resources);
                out.extend(q.cost.gold.to_le_bytes());
                out.extend(q.cost.dust.to_le_bytes());
                out.extend([
                    q.cost.cores,
                    q.target_tier,
                    q.success_percent,
                    q.protection_percent,
                ]);
            }
        }
        (out.len() <= MAX_REPLY_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let mut r = Reader(bytes);
        let nonce = r.u64()?;
        let payload = match r.byte()? {
            0 => Payload::Refused,
            3 => {
                let operation = match r.byte()? {
                    5 => Operation::DustToSlivers,
                    6 => Operation::SliversToCores,
                    7 => Operation::DustLimit,
                    _ => return None,
                };
                Payload::Resource(ResourceQuote {
                    operation,
                    full_resources: r.boolean()?,
                    before: r.resources()?,
                    after_dust: r.u32()?,
                    after_limit: r.u32()?,
                    after_slivers: r.u64()?,
                    after_cores: r.u64()?,
                })
            }
            1 => {
                let after = r.u64()?;
                let next = match r.u64()? {
                    0 => None,
                    n => Some(n),
                };
                let resources = r.resources()?;
                let n = usize::from(r.byte()?);
                if n > PAGE_SIZE {
                    return None;
                }
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(r.item()?);
                }
                Payload::Page(Page {
                    after,
                    next,
                    resources,
                    items,
                })
            }
            2 => {
                let raw = r.0.get(..51)?;
                r.0 = &r.0[51..];
                let mut request = nonce.to_le_bytes().to_vec();
                request.push(1);
                request.extend(raw);
                let selection = Request::decode(&request)?.selection;
                let first = r.item()?;
                let second = r.item()?;
                let resources = r.resources()?;
                let cost = Cost {
                    gold: r.u64()?,
                    dust: r.u32()?,
                    cores: r.byte()?,
                };
                Payload::Quote(Quote {
                    selection,
                    first,
                    second,
                    resources,
                    cost,
                    target_tier: r.byte()?,
                    success_percent: r.byte()?,
                    protection_percent: r.byte()?,
                })
            }
            _ => return None,
        };
        let reply = Self { nonce, payload };
        (r.0.is_empty() && reply.valid()).then_some(reply)
    }
}
