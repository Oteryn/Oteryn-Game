//! Read-only MARKET_V1 catalogue and completed-trade projections, command 6126.
use crate::charm_wire::{push_message_field, read_bytes, read_varint};
use crate::world_interaction::WorldInteractionWireError as Error;
use serde::{Deserialize, Serialize};
pub const COMMAND_TYPE_MARKET_DETAILS: u32 = 6126;
pub const PAGE_SIZE: usize = 32;
pub const MAX_REPLY: usize = 32 * 1024;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    pub search: String,
    pub category: Option<String>,
    pub vocation: Option<u8>,
    pub minimum_level: Option<u16>,
    pub maximum_level: Option<u16>,
    pub forge_class: Option<u8>,
    pub depot_only: bool,
}
impl Filter {
    pub fn valid(&self) -> bool {
        self.search.len() <= 80
            && !self.search.chars().any(char::is_control)
            && self
                .category
                .as_ref()
                .is_none_or(|s| !s.is_empty() && s.len() <= 120 && !s.chars().any(char::is_control))
            && self.vocation.is_none_or(|v| (1..=6).contains(&v))
            && self.forge_class.is_none_or(|v| v <= 4)
            && self
                .minimum_level
                .zip(self.maximum_level)
                .is_none_or(|(a, b)| a <= b)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "query", deny_unknown_fields)]
pub enum Query {
    Catalogue { filter: Filter, page: u32 },
    Statistics { definition: u32 },
    Book { definition: u32, page: u32 },
}
impl Query {
    pub fn valid(&self) -> bool {
        match self {
            Self::Catalogue { filter, page } => filter.valid() && *page <= 4096,
            Self::Statistics { definition } => *definition > 0,
            Self::Book { definition, page } => *definition > 0 && *page <= 21,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ware {
    pub definition: u32,
    pub name: String,
    pub categories: Vec<String>,
    /// None preserves unqualified requirements; it never means no restriction.
    pub vocations: Option<Vec<u8>>,
    pub level: Option<u16>,
    pub forge_class: Option<u8>,
    pub depot_units: u64,
}
impl Filter {
    pub fn matches(&self, item: &Ware) -> bool {
        item.name
            .to_lowercase()
            .contains(&self.search.to_lowercase())
            && self
                .category
                .as_ref()
                .is_none_or(|c| item.categories.contains(c))
            && self.vocation.is_none_or(|v| {
                item.vocations
                    .as_ref()
                    .is_some_and(|vs| vs.is_empty() || vs.contains(&v))
            })
            && self
                .minimum_level
                .is_none_or(|v| item.level.is_some_and(|n| n >= v))
            && self
                .maximum_level
                .is_none_or(|v| item.level.is_some_and(|n| n <= v))
            && self.forge_class.is_none_or(|v| item.forge_class == Some(v))
            && (!self.depot_only || item.depot_units > 0)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriceSummary {
    pub trades: u64,
    pub units: u64,
    pub minimum: Option<u64>,
    pub maximum: Option<u64>,
    pub average: Option<u64>,
}
impl PriceSummary {
    fn valid(&self) -> bool {
        if self.trades == 0 {
            self.units == 0
                && self.minimum.is_none()
                && self.maximum.is_none()
                && self.average.is_none()
        } else {
            self.units >= self.trades
                && matches!((self.minimum,self.maximum,self.average),
                (Some(a),Some(b),Some(mean)) if a>0 && a<=mean && mean<=b && b<=999999999999)
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statistics {
    pub buy: PriceSummary,
    pub sell: PriceSummary,
    pub definition: u32,
    pub as_of_us: i64,
    pub since_us: i64,
    pub trades: u64,
    pub units: u64,
    pub minimum: Option<u64>,
    pub maximum: Option<u64>,
    pub average: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", deny_unknown_fields)]
pub enum Reply {
    Catalogue {
        items: Vec<Ware>,
        categories: Vec<String>,
        page: u32,
        has_next: bool,
    },
    Book {
        definition: u32,
        page: u32,
        rows: Vec<crate::market::MarketRow>,
        statistics: Statistics,
        has_next: bool,
    },
    Statistics(Statistics),
    Refused {
        code: u8,
    },
}
pub fn is_query(bytes: &[u8]) -> bool {
    bytes.starts_with(&[8, 9])
}
pub fn encode_query(query: &Query) -> Result<Vec<u8>, Error> {
    if !query.valid() {
        return Err(Error::Malformed);
    }
    let mut out = vec![8, 9];
    let bytes = serde_json::to_vec(query).map_err(|_| Error::Malformed)?;
    push_message_field(&mut out, 8, &bytes);
    if out.len() > 512 {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
pub fn decode_query(bytes: &[u8]) -> Result<Query, Error> {
    if !is_query(bytes) || bytes.len() > 512 {
        return Err(Error::Malformed);
    }
    let mut at = 2;
    if read_varint(bytes, &mut at)? != 66 {
        return Err(Error::Malformed);
    }
    let query: Query =
        serde_json::from_slice(read_bytes(bytes, &mut at)?).map_err(|_| Error::Malformed)?;
    if at != bytes.len() || !query.valid() {
        return Err(Error::Malformed);
    }
    Ok(query)
}
fn valid_statistics(s: &Statistics) -> bool {
    s.buy.valid()
        && s.sell.valid()
        && s.buy.trades.checked_add(s.sell.trades) == Some(s.trades)
        && s.buy.units.checked_add(s.sell.units) == Some(s.units)
        && s.definition > 0
        && s.as_of_us >= s.since_us
        && s.as_of_us.saturating_sub(s.since_us) == 2592000000000
        && if s.trades == 0 {
            s.units == 0 && s.minimum.is_none() && s.maximum.is_none() && s.average.is_none()
        } else {
            s.units >= s.trades
                && matches!((s.minimum,s.maximum,s.average),(Some(a),Some(b),Some(mean)) if a>0&&a<=mean&&mean<=b&&b<=999999999999)
        }
}
fn valid_reply(reply: &Reply) -> bool {
    match reply {
        Reply::Refused { code } => (2..=15).contains(code),
        Reply::Statistics(s) => valid_statistics(s),
        Reply::Book {
            definition,
            page,
            rows,
            statistics,
            has_next,
        } => {
            *definition > 0
                && *page <= 21
                && rows.len() <= 64
                && statistics.definition == *definition
                && valid_statistics(statistics)
                && (!*has_next || *page < 21)
                && rows.iter().enumerate().all(|(i, r)| {
                    r.definition == *definition
                        && matches!(r.side, 1 | 2)
                        && r.state == 1
                        && (1..=64000).contains(&r.amount)
                        && r.price > 0
                        && u128::from(r.price) * u128::from(r.amount) <= 999999999999
                        && r.offer[6] >> 4 == 7
                        && r.offer[8] >> 6 == 2
                        && !rows[..i].iter().any(|p| p.offer == r.offer)
                })
        }
        Reply::Catalogue {
            items,
            categories,
            page,
            ..
        } => {
            items.len() <= PAGE_SIZE
                && categories.len() <= 256
                && *page <= 4096
                && categories
                    .iter()
                    .all(|c| !c.is_empty() && c.len() <= 120 && !c.chars().any(char::is_control))
                && items.iter().all(|r| {
                    r.definition > 0
                        && !r.name.is_empty()
                        && r.name.len() <= 160
                        && r.categories.len() <= 4
                        && r.categories.iter().all(|c| categories.contains(c))
                        && r.vocations.as_ref().is_none_or(|vs| {
                            vs.len() <= 6
                                && vs
                                    .iter()
                                    .enumerate()
                                    .all(|(i, v)| (1..=6).contains(v) && !vs[..i].contains(v))
                        })
                        && r.forge_class.is_none_or(|c| c <= 4)
                })
        }
    }
}
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    if !valid_reply(reply) {
        return Err(Error::Malformed);
    }
    let bytes = serde_json::to_vec(reply).map_err(|_| Error::Malformed)?;
    if bytes.len() > MAX_REPLY {
        return Err(Error::LimitExceeded);
    }
    Ok(bytes)
}
pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > MAX_REPLY {
        return Err(Error::LimitExceeded);
    }
    let reply: Reply = serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
    if !valid_reply(&reply) {
        return Err(Error::Malformed);
    }
    Ok(reply)
}
/// Navigation labels for the pinned 15.30 categories; not the outdated Canary enum.
pub fn category_label(category: u8) -> &'static str {
    const NAMES: [&str; 28] = [
        "Unclassified",
        "Armour",
        "Neckwear",
        "Boots",
        "Containers",
        "Decorations",
        "Food",
        "Headwear",
        "Legwear",
        "Other items",
        "Potions",
        "Rings",
        "Runes",
        "Shields",
        "Tools",
        "Valuables",
        "Ammunition",
        "Axes",
        "Clubs",
        "Distance weapons",
        "Swords",
        "Wands and rods",
        "Premium items",
        "Coin items",
        "Creature products",
        "Quivers",
        "Soul cores",
        "Fist weapons",
    ];
    NAMES
        .get(usize::from(category))
        .copied()
        .unwrap_or("Unclassified")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t124_all_catalogue_filters_compose_and_prices_reject_impossible_data()
    -> Result<(), Box<dyn std::error::Error>> {
        let ware = Ware {
            definition: 42,
            name: "Test blade".into(),
            categories: vec!["Swords".into()],
            vocations: Some(vec![2]),
            level: Some(50),
            forge_class: Some(4),
            depot_units: 7,
        };
        let filter = Filter {
            search: "BLADE".into(),
            category: Some("Swords".into()),
            vocation: Some(2),
            minimum_level: Some(50),
            maximum_level: Some(50),
            forge_class: Some(4),
            depot_only: true,
        };
        assert!(filter.matches(&ware));
        for other in [
            Filter {
                vocation: Some(1),
                ..filter.clone()
            },
            Filter {
                category: Some("Potions".into()),
                ..filter.clone()
            },
            Filter {
                minimum_level: Some(51),
                maximum_level: None,
                ..filter.clone()
            },
            Filter {
                minimum_level: None,
                maximum_level: Some(49),
                ..filter.clone()
            },
            Filter {
                forge_class: Some(3),
                ..filter.clone()
            },
        ] {
            assert!(!other.matches(&ware));
        }
        assert!(!filter.matches(&Ware {
            depot_units: 0,
            ..ware
        }));
        let stats = Statistics {
            buy: PriceSummary {
                trades: 1,
                units: 4,
                minimum: Some(100),
                maximum: Some(100),
                average: Some(100),
            },
            sell: PriceSummary {
                trades: 1,
                units: 1,
                minimum: Some(201),
                maximum: Some(201),
                average: Some(201),
            },
            definition: 42,
            as_of_us: 2592000000000,
            since_us: 0,
            trades: 2,
            units: 5,
            minimum: Some(100),
            maximum: Some(201),
            average: Some(150),
        };
        assert_eq!(
            decode_reply(
                &encode_reply(&Reply::Statistics(stats.clone())).map_err(|e| format!("{e:?}"))?
            )
            .map_err(|e| format!("{e:?}"))?,
            Reply::Statistics(stats.clone())
        );
        let mut mismatched = stats.clone();
        mismatched.buy.trades = 2;
        assert!(encode_reply(&Reply::Statistics(mismatched)).is_err());
        assert!(
            encode_reply(&Reply::Statistics(Statistics {
                average: Some(99),
                ..stats
            }))
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn t124_queries_are_bounded_and_keep_unknown_requirements()
    -> Result<(), Box<dyn std::error::Error>> {
        let q = Query::Catalogue {
            filter: Filter::default(),
            page: 0,
        };
        assert_eq!(
            decode_query(&encode_query(&q).map_err(|e| format!("{e:?}"))?)
                .map_err(|e| format!("{e:?}"))?,
            q
        );
        let mut bytes = encode_query(&q).map_err(|e| format!("{e:?}"))?;
        bytes.push(0);
        assert!(decode_query(&bytes).is_err());
        let unknown = Ware {
            definition: 1,
            name: "Sword".into(),
            categories: vec!["Weapons".into()],
            vocations: None,
            level: None,
            forge_class: None,
            depot_units: 0,
        };
        assert!(
            !Filter {
                vocation: Some(1),
                ..Default::default()
            }
            .matches(&unknown)
        );
        Ok(())
    }
}
