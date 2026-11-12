//! Closed items record encoding. Source ownership and snapshot-wide graph checks belong to the durable producer.
use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsBegin {
    pub snapshot_id: String,
    pub character_revision: String,
    pub page_count: u32,
    pub item_count: u32,
    pub depots: Vec<u16>,
    pub houses: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsEnd {
    pub snapshot_id: String,
    pub page_count: u32,
    pub item_count: u32,
    pub pages_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsPage {
    pub snapshot_id: String,
    pub page_index: u32,
    pub items: Vec<ItemRow>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRow {
    pub item_instance_id: String,
    pub definition_family: String,
    pub definition_production_key: String,
    pub definition_revision_ref: String,
    pub quantity: u32,
    pub root: String,
    pub town_id: Option<u16>,
    pub depot_box: Option<u8>,
    pub house_id: Option<String>,
    pub spatial_position_hex: Option<String>,
    pub equipment_slot: Option<u8>,
    pub parent_item_instance_id: Option<String>,
    pub placement_ordinal: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLine {
    pub item_instance_id: String,
    pub definition_family: String,
    pub definition_production_key: String,
    pub definition_revision_ref: String,
    pub quantity: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankEntry {
    pub operation_occurrence_id: String,
    pub event_id: String,
    pub entry_id: String,
    pub previous_entry_id: Option<String>,
    pub kind: String,
    pub direction: String,
    pub amount: String,
    pub balance_before: String,
    pub balance_after: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketEvent {
    pub event_id: String,
    pub offer_id: String,
    pub side: String,
    pub state: String,
    pub end_state: Option<String>,
    pub definition_family: String,
    pub definition_production_key: String,
    pub definition_revision_ref: String,
    pub amount: u32,
    pub remaining: u32,
    pub piece_price: String,
    pub fee_gold: String,
    pub held_credit_gold: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerTradeEvent {
    pub occurrence_id: String,
    pub transaction_id: String,
    pub event_id: String,
    pub given: Vec<ItemLine>,
    pub received: Vec<ItemLine>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishGameJournalRecordV1 {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub world_id: String,
    pub character_id: Option<String>,
    pub character_revision: Option<String>,
    pub from_projection_revision: Option<String>,
    pub to_projection_revision: Option<String>,
    pub projection_epoch: Option<String>,
    pub family: ItemsFamily,
    pub recovery_generation: String,
    pub source_revision: String,
    pub record_id: String,
    pub occurred_at: String,
    pub coverage_start_at: String,
    pub kind: String,
    pub payload: ItemsPayload,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ItemsPayload {
    Begin(ItemsBegin),
    Page(ItemsPage),
    End(ItemsEnd),
    Bank(BankEntry),
    Market(MarketEvent),
    Trade(PlayerTradeEvent),
}
impl PublishGameJournalRecordV1 {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        let cursor = ReadGameJournalCursorV1 {
            contract_version: self.contract_version,
            operation: "ReadGameJournalCursorV1".into(),
            source_authority: self.source_authority.clone(),
            account_id: self.account_id.clone(),
            world_id: self.world_id.clone(),
            character_id: self.character_id.clone(),
            character_revision: self.character_revision.clone(),
            from_projection_revision: self.from_projection_revision.clone(),
            to_projection_revision: self.to_projection_revision.clone(),
            projection_epoch: self.projection_epoch.clone(),
            family: self.family,
            recovery_generation: self.recovery_generation.clone(),
        };
        cursor.encode()?;
        let valid_payload = match &self.payload {
            ItemsPayload::Begin(p) => {
                self.family == ItemsFamily::Items
                    && self.kind == "ITEMS_BEGIN"
                    && uuid_v7(&p.snapshot_id)
                    && Some(&p.character_revision) == self.character_revision.as_ref()
                    && counts(p.page_count, p.item_count)
                    && p.depots.len() <= 256
                    && p.houses.len() <= 256
                    && p.depots.windows(2).all(|v| v[0] < v[1])
                    && p.houses.windows(2).all(|v| v[0] < v[1])
                    && p.houses.iter().all(|v| house(v))
            }
            ItemsPayload::End(p) => {
                self.family == ItemsFamily::Items
                    && self.kind == "ITEMS_END"
                    && uuid_v7(&p.snapshot_id)
                    && counts(p.page_count, p.item_count)
                    && hex(&p.pages_sha256, 64)
                    && (p.page_count != 0
                        || p.pages_sha256
                            == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
            }
            ItemsPayload::Page(p) => {
                self.family == ItemsFamily::Items
                    && self.kind == "ITEMS_PAGE"
                    && uuid_v7(&p.snapshot_id)
                    && p.page_index < 16384
                    && !p.items.is_empty()
                    && p.items.len() <= 32
                    && p.items
                        .windows(2)
                        .all(|v| v[0].item_instance_id < v[1].item_instance_id)
                    && p.items.iter().all(ItemRow::valid)
            }
            ItemsPayload::Bank(p) => {
                self.family == ItemsFamily::Bank && self.kind == "BANK_ENTRY" && p.valid()
            }
            ItemsPayload::Market(p) => {
                self.family == ItemsFamily::Market && self.kind == "MARKET_EVENT" && p.valid()
            }
            ItemsPayload::Trade(p) => {
                self.family == ItemsFamily::PlayerTrade
                    && self.kind == "PLAYER_TRADE_EVENT"
                    && uuid_v7(&p.occurrence_id)
                    && uuid_v7(&p.transaction_id)
                    && uuid_v7(&p.event_id)
                    && p.given.len() == 1
                    && p.received.len() == 1
                    && p.given
                        .iter()
                        .chain(&p.received)
                        .all(|v| v.valid() && v.quantity <= 100)
                    && p.given.first().map(|v| &v.item_instance_id)
                        != p.received.first().map(|v| &v.item_instance_id)
            }
        };
        if self.operation != "PublishGameJournalRecordV1"
            || !uuid_v7(&self.record_id)
            || counter(&self.source_revision).is_none()
            || !valid_payload
            || !matches!((counter(&self.coverage_start_at),counter(&self.occurred_at)),(Some(a),Some(b)) if a<=b)
            || (self.family == ItemsFamily::Items && self.coverage_start_at != self.occurred_at)
        {
            return Err(SourceError::InvalidInput);
        }
        let bytes = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        if bytes.len() > 65536 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 65536 {
            return Err(SourceError::CapacityExceeded);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if value.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(value)
    }
}
fn counts(p: u32, i: u32) -> bool {
    p <= 16384
        && i <= 524288
        && if p == 0 {
            i == 0
        } else {
            p <= i && i <= 32 * p
        }
}
fn reference(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 512
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn house(s: &str) -> bool {
    s.len() <= 512
        && s.strip_prefix("oteryn:content.house.").is_some_and(|v| {
            !v.is_empty()
                && v.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}
fn hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn gold(s: &str) -> Option<u64> {
    if s == "0" {
        Some(0)
    } else {
        counter(s).filter(|v| *v <= 999999999999)
    }
}
impl ItemLine {
    fn valid(&self) -> bool {
        uuid_v7(&self.item_instance_id)
            && source_authority_valid(&self.definition_family)
            && reference(&self.definition_production_key)
            && reference(&self.definition_revision_ref)
            && self.quantity > 0
    }
}
impl ItemRow {
    fn valid(&self) -> bool {
        let base = uuid_v7(&self.item_instance_id)
            && source_authority_valid(&self.definition_family)
            && reference(&self.definition_production_key)
            && reference(&self.definition_revision_ref)
            && self.quantity > 0
            && counter(&self.placement_ordinal).is_some()
            && self.parent_item_instance_id.as_deref().is_none_or(uuid_v7);
        base && match self.root.as_str() {
            "EQUIPMENT" => {
                self.equipment_slot.is_some_and(|v| v > 0)
                    && self.town_id.is_none()
                    && self.depot_box.is_none()
                    && self.house_id.is_none()
                    && self.spatial_position_hex.is_none()
            }
            "DEPOT" => {
                self.town_id.is_some()
                    && self.depot_box.is_some_and(|v| (1..=17).contains(&v))
                    && self.equipment_slot.is_none()
                    && self.house_id.is_none()
                    && self.spatial_position_hex.is_none()
            }
            "INBOX" => {
                self.town_id.is_none()
                    && self.depot_box.is_none()
                    && self.equipment_slot.is_none()
                    && self.house_id.is_none()
                    && self.spatial_position_hex.is_none()
            }
            "HOUSE" => {
                self.house_id.as_deref().is_some_and(house)
                    && self.spatial_position_hex.as_deref().is_some_and(|v| {
                        (2..=256).contains(&v.len()) && v.len() % 2 == 0 && hex(v, v.len())
                    })
                    && self.parent_item_instance_id.is_none()
                    && self.town_id.is_none()
                    && self.depot_box.is_none()
                    && self.equipment_slot.is_none()
            }
            _ => false,
        }
    }
}
impl BankEntry {
    fn valid(&self) -> bool {
        let (Some(a), Some(b), Some(c)) = (
            gold(&self.amount),
            gold(&self.balance_before),
            gold(&self.balance_after),
        ) else {
            return false;
        };
        uuid_v7(&self.operation_occurrence_id)
            && uuid_v7(&self.event_id)
            && uuid_v7(&self.entry_id)
            && self.previous_entry_id.as_deref().is_none_or(uuid_v7)
            && a > 0
            && match (self.kind.as_str(), self.direction.as_str()) {
                ("DEPOSIT", "CREDIT") => a <= 20000000 && b.checked_add(a) == Some(c),
                ("WITHDRAW", "DEBIT") => a <= 1009999 && b.checked_sub(a) == Some(c),
                ("TRANSFER", "CREDIT") => b.checked_add(a) == Some(c),
                ("TRANSFER", "DEBIT") => b.checked_sub(a) == Some(c),
                _ => false,
            }
    }
}
impl MarketEvent {
    fn valid(&self) -> bool {
        let (Some(price), Some(fee), Some(held)) = (
            gold(&self.piece_price),
            gold(&self.fee_gold),
            gold(&self.held_credit_gold),
        ) else {
            return false;
        };
        uuid_v7(&self.event_id)
            && uuid_v7(&self.offer_id)
            && matches!(self.side.as_str(), "BUY" | "SELL")
            && source_authority_valid(&self.definition_family)
            && reference(&self.definition_production_key)
            && reference(&self.definition_revision_ref)
            && (1..=64000).contains(&self.amount)
            && self.remaining <= self.amount
            && price > 0
            && price
                .checked_mul(u64::from(self.amount))
                .is_some_and(|v| v <= 999999999999)
            && fee <= 2500000
            && if self.state == "CREDIT_HELD" {
                matches!(
                    self.end_state.as_deref(),
                    Some("FILLED" | "CANCELLED" | "EXPIRED")
                ) && (self.end_state.as_deref() != Some("FILLED") || self.remaining == 0)
            } else {
                matches!(
                    self.state.as_str(),
                    "OPEN" | "FILLED" | "CANCELLED" | "EXPIRED"
                ) && self.end_state.is_none()
                    && held == 0
                    && (self.state != "FILLED" || self.remaining == 0)
            }
    }
}

#[cfg(test)]
mod publish_tests {
    use super::*;
    fn assert_closed<T>(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>>
    where
        T: serde::de::DeserializeOwned + Serialize,
    {
        fn canonical<T: serde::de::DeserializeOwned + Serialize>(b: &[u8]) -> bool {
            serde_json::from_slice::<T>(b)
                .is_ok_and(|v| serde_json::to_vec(&v).is_ok_and(|encoded| encoded == b))
        }
        assert!(canonical::<T>(bytes));
        let text = std::str::from_utf8(bytes)?;
        let first = text.find(',').ok_or("first member")?;
        let second = first + 1 + text[first + 1..].find(',').ok_or("second member")?;
        let reordered = format!(
            "{{{},{},{}",
            &text[first + 1..second],
            &text[1..first],
            &text[second + 1..]
        );
        assert!(!canonical::<T>(reordered.as_bytes()));

        assert!(!canonical::<T>(
            text.replacen("{", "{\"extra\":0,", 1).as_bytes()
        ));
        assert!(!canonical::<T>(text.replacen(",", ", ", 1).as_bytes()));
        let end = text.find(',').ok_or("missing member separator")?;
        assert!(!canonical::<T>(
            format!("{{{}", &text[end + 1..]).as_bytes()
        ));
        Ok(())
    }
    #[test]
    fn journal_items_payload_golden() -> Result<(), Box<dyn std::error::Error>> {
        assert_closed::<PublishGameJournalRecordV1>(br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","character_revision":"1","from_projection_revision":"1","to_projection_revision":"1","projection_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000004","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_BEGIN","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","character_revision":"1","page_count":0,"item_count":0,"depots":[],"houses":[]}}"#)?;
        assert_closed::<PublishGameJournalRecordV1>(br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","character_revision":"1","from_projection_revision":"1","to_projection_revision":"1","projection_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"2","record_id":"019a0000-0000-7000-8000-000000000006","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_END","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","page_count":0,"item_count":0,"pages_sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}"#)?;
        assert_closed::<PublishGameJournalRecordV1>(br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000007","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"BANK_ENTRY","payload":{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":"100","balance_before":"0","balance_after":"100"}}"#)?;
        assert_closed::<ItemsPage>(br#"{"snapshot_id":"019a0000-0000-7000-8000-000000000005","page_index":0,"items":[{"item_instance_id":"019a0000-0000-7000-8000-00000000000b","definition_family":"ITEM","definition_production_key":"oteryn:content.item.backpack","definition_revision_ref":"rev-1","quantity":1,"root":"EQUIPMENT","town_id":null,"depot_box":null,"house_id":null,"spatial_position_hex":null,"equipment_slot":3,"parent_item_instance_id":null,"placement_ordinal":"1"}]}"#)?;
        assert_closed::<MarketEvent>(br#"{"event_id":"019a0000-0000-7000-8000-00000000000c","offer_id":"019a0000-0000-7000-8000-00000000000d","side":"SELL","state":"FILLED","end_state":null,"definition_family":"ITEM","definition_production_key":"oteryn:content.item.gold_coin","definition_revision_ref":"rev-1","amount":10,"remaining":0,"piece_price":"2","fee_gold":"0","held_credit_gold":"0"}"#)?;
        assert_closed::<PlayerTradeEvent>(br#"{"occurrence_id":"019a0000-0000-7000-8000-00000000000e","transaction_id":"019a0000-0000-7000-8000-00000000000f","event_id":"019a0000-0000-7000-8000-000000000010","given":[{"item_instance_id":"019a0000-0000-7000-8000-000000000011","definition_family":"ITEM","definition_production_key":"oteryn:content.item.gold_coin","definition_revision_ref":"rev-1","quantity":10}],"received":[{"item_instance_id":"019a0000-0000-7000-8000-000000000012","definition_family":"ITEM","definition_production_key":"oteryn:content.item.platinum_coin","definition_revision_ref":"rev-1","quantity":1}]}"#)?;
        Ok(())
    }
    #[test]
    fn journal_items_publish_golden() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","character_revision":"1","from_projection_revision":"1","to_projection_revision":"1","projection_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000004","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_BEGIN","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","character_revision":"1","page_count":0,"item_count":0,"depots":[],"houses":[]}}"#;
        assert_eq!(PublishGameJournalRecordV1::decode(bytes)?.encode()?, bytes);
        assert!(PublishGameJournalRecordV1::decode(&[bytes.as_slice(), b"\n"].concat()).is_err());
        let bytes = br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","character_revision":"1","from_projection_revision":"1","to_projection_revision":"1","projection_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"2","record_id":"019a0000-0000-7000-8000-000000000006","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_END","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","page_count":0,"item_count":0,"pages_sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}"#;
        assert_eq!(PublishGameJournalRecordV1::decode(bytes)?.encode()?, bytes);
        assert!(PublishGameJournalRecordV1::decode(&[bytes.as_slice(), b"\n"].concat()).is_err());
        let bytes = br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000007","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"BANK_ENTRY","payload":{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":"100","balance_before":"0","balance_after":"100"}}"#;
        assert_eq!(PublishGameJournalRecordV1::decode(bytes)?.encode()?, bytes);
        assert!(PublishGameJournalRecordV1::decode(&[bytes.as_slice(), b"\n"].concat()).is_err());
        assert!(PublishGameJournalRecordV1::decode(&[b' '; 65537]).is_err());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn journal_items_negative_golden_vectors() {
    assert!(serde_json::from_slice::<BankEntry>(br#"{"event_id":"019a0000-0000-7000-8000-00000000000c","offer_id":"019a0000-0000-7000-8000-00000000000d","side":"SELL","state":"FILLED","end_state":null,"definition_family":"ITEM","definition_production_key":"oteryn:content.item.gold_coin","definition_revision_ref":"rev-1","amount":10,"remaining":0,"piece_price":"2","fee_gold":"0","held_credit_gold":"0"}"#).is_err());
    assert!(ReadGameJournalCursorV1::decode(br#"{"contract_version":1,"operation":"ReadGameJournalCursorV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","character_revision":"1","from_projection_revision":"1","to_projection_revision":"1","projection_epoch":"1","family":"BANK","recovery_generation":"1"}"#).is_err());
    assert!(serde_json::from_slice::<BankEntry>(br#"{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":100,"balance_before":"0","balance_after":"99"}"#).is_err());
}
