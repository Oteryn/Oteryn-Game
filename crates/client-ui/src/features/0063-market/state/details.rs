//! Catalogue and completed prices are server projections, separate from loaded quotes.
use oteryn_client_runtime::market::CommandStatus;
pub use oteryn_client_runtime::market::market_details::{Filter, Query, Reply, Statistics, Ware};
use oteryn_client_runtime::raw_feature::FeatureCommand;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Details {
    pub filter: Filter,
    pub categories: Vec<String>,
    pub items: Vec<Ware>,
    pub page: u32,
    pub has_next: bool,
    pub book_has_next: bool,
    pub statistics: Option<Statistics>,
    pub pending: Option<Query>,
    pub feedback: Option<&'static str>,
}
pub fn request(panel: &mut super::MarketPanel, query: Query) -> Option<FeatureCommand> {
    if !panel.selected || panel.details.pending.is_some() {
        return None;
    }
    let payload = oteryn_client_runtime::market::market_details::encode_query(&query).ok()?;
    match &query {
        Query::Catalogue { filter, .. } => panel.details.filter = filter.clone(),
        Query::Book { .. } => {
            panel.selected_offer = None;
            panel.rows.clear();
            panel.details.statistics = None;
        }
        Query::Statistics { definition } => {
            if panel
                .details
                .statistics
                .as_ref()
                .is_some_and(|s| s.definition != *definition)
            {
                panel.details.statistics = None;
            }
        }
    }
    panel.details.pending = Some(query);
    panel.details.feedback = None;
    Some(FeatureCommand {
        capability: super::market::CAPABILITY_MARKET_V1,
        kind: oteryn_client_runtime::market::market_details::COMMAND_TYPE_MARKET_DETAILS,
        payload,
    })
}
pub fn apply(
    panel: &mut super::MarketPanel,
    command: &FeatureCommand,
    status: CommandStatus,
    bytes: &[u8],
) -> bool {
    let Ok(query) = oteryn_client_runtime::market::market_details::decode_query(&command.payload)
    else {
        return false;
    };
    if panel.details.pending.as_ref() != Some(&query) {
        return false;
    }
    panel.details.pending = None;
    if !matches!(
        status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.details.feedback = Some("Market query refused");
        return false;
    }
    let Ok(reply) = oteryn_client_runtime::market::market_details::decode_reply(bytes) else {
        panel.details.feedback = Some("Invalid market projection");
        return false;
    };
    match (query, reply) {
        (
            Query::Book {
                definition: expected,
                page: requested,
            },
            Reply::Book {
                definition,
                page,
                rows,
                statistics,
                has_next,
            },
        ) if definition == expected && page == requested => {
            panel.view = super::MarketView::Browse;
            panel.definition = definition;
            panel.page = page;
            panel.rows = rows;
            panel.details.statistics = Some(statistics);
            panel.details.book_has_next = has_next;
        }
        (
            Query::Catalogue { page: expected, .. },
            Reply::Catalogue {
                items,
                categories,
                page,
                has_next,
            },
        ) if page == expected => {
            panel.details.items = items;
            panel.details.categories = categories;
            panel.details.page = page;
            panel.details.has_next = has_next;
        }
        (Query::Statistics { definition }, Reply::Statistics(statistics))
            if definition == statistics.definition =>
        {
            panel.details.statistics = Some(statistics)
        }
        _ => {
            panel.details.feedback = Some("Market query refused or mismatched");
            return false;
        }
    }
    true
}
pub fn statistics_text(panel: &super::MarketPanel) -> String {
    let Some(stats) = panel
        .details
        .statistics
        .as_ref()
        .filter(|s| s.definition == panel.definition)
    else {
        return "30-day completed trades: not loaded".into();
    };
    if stats.trades == 0 {
        return "30-day completed trades: none; prices unavailable".into();
    }
    let side = |name, value: &oteryn_client_runtime::market::market_details::PriceSummary| {
        let price = |v: Option<u64>| v.map_or_else(|| "—".into(), |v| v.to_string());
        format!(
            "{name}: {} trades · {} units · min {} · max {} · average {}",
            value.trades,
            value.units,
            price(value.minimum),
            price(value.maximum),
            price(value.average)
        )
    };
    format!(
        "30-day completed trades (server snapshot): {} · {} units · min {} · max {} · average {}\r\n{}\r\n{}",
        stats.trades,
        stats.units,
        stats.minimum.map_or_else(|| "—".into(), |v| v.to_string()),
        stats.maximum.map_or_else(|| "—".into(), |v| v.to_string()),
        stats.average.map_or_else(|| "—".into(), |v| v.to_string()),
        side("Buy offers", &stats.buy),
        side("Sell offers", &stats.sell)
    )
}
