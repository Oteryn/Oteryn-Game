//! Wire types for Phase 2. The catalog exposes these before executors exist.
use crate::{Position, RiskClass, WireError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}

macro_rules! bounded_integer {
    ($name:ident, $ty:ty, $source:literal, $min:literal, $max:literal) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
        #[schemars(!try_from)]
        #[serde(try_from = $source)]
        pub struct $name(#[schemars(range(min = $min, max = $max))] $ty);

        impl TryFrom<$ty> for $name {
            type Error = &'static str;

            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                ($min..=$max)
                    .contains(&value)
                    .then_some(Self(value))
                    .ok_or(concat!(stringify!($name), " out of range"))
            }
        }
    };
}

macro_rules! bounded_text {
    ($name:ident, $min:literal, $max:literal) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[schemars(!try_from)]
        #[serde(try_from = "String")]
        pub struct $name(#[schemars(length(min = $min, max = $max))] String);

        impl TryFrom<String> for $name {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                ($min..=$max)
                    .contains(&value.chars().count())
                    .then_some(Self(value))
                    .ok_or(concat!(stringify!($name), " length out of range"))
            }
        }
    };
}

macro_rules! bounded_rows {
    ($name:ident, $max:literal) => {
        #[derive(Debug, Clone, Serialize, JsonSchema)]
        pub struct $name<T>(#[schemars(length(max = $max))] Vec<T>);

        impl<'de, T: Deserialize<'de>> Deserialize<'de> for $name<T> {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let rows = Vec::<T>::deserialize(d)?;
                if rows.len() > $max {
                    return Err(serde::de::Error::custom("too many rows"));
                }
                Ok(Self(rows))
            }
        }
    };
}

bounded_integer!(CountdownSeconds, u16, "u16", 1, 3600);
bounded_integer!(DecisionRevision, u64, "u64", 1, 9_007_199_254_740_991u64);
bounded_integer!(LogLimit, u16, "u16", 1, 2000);
bounded_integer!(PageLimit, u16, "u16", 1, 100);
bounded_integer!(UnixMillis, u64, "u64", 0, 9_007_199_254_740_991u64);
bounded_rows!(AnalyticsRows, 10000);
bounded_rows!(LogLines, 2000);
bounded_rows!(PageRows, 100);
bounded_text!(MessageText, 1, 255);
bounded_text!(MotdText, 0, 255);
bounded_text!(StaffNote, 1, 1024);

/// Exact nonnegative amount or count, including values outside JSON's safe integer range.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(!try_from)]
#[serde(try_from = "String")]
pub struct Decimal(#[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))] String);

impl TryFrom<String> for Decimal {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("expected a canonical unsigned decimal string");
        }
        Ok(Self(value))
    }
}

/// A positive session generation, represented as a string to cover the full u64 range.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(!try_from)]
#[serde(try_from = "String")]
pub struct SessionGeneration(
    #[schemars(length(max = 20), regex(pattern = "^[1-9][0-9]*$"))] String,
);

impl TryFrom<String> for SessionGeneration {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !value.starts_with(['0', '+']) && value.parse::<u64>().is_ok_and(|n| n > 0) {
            return Ok(Self(value));
        }
        Err("expected a positive decimal session generation")
    }
}

closed!(KickParams { expected_session_generation: SessionGeneration, preview_digest: Option<String> });
closed!(KickResult {
    fenced_session_generation: SessionGeneration
});
wire_enum!(Temple { Temple });
closed!(TempleDestination { kind: Temple });
wire_enum!(Coordinates { Coordinates });
closed!(CoordinateDestination {
    kind: Coordinates,
    position: Position
});

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TeleportDestination {
    Temple(TempleDestination),
    Coordinates(CoordinateDestination),
}

closed!(TeleportParams { destination: TeleportDestination, preview_digest: Option<String> });
closed!(TeleportResult {
    from_position: Position,
    to_position: Position
});
closed!(MessageParams { text: MessageText, preview_digest: Option<String> });
closed!(MessageResult { delivered: bool });
closed!(BroadcastResult { recipients: u32 });
closed!(MotdParams { text: MotdText, preview_digest: Option<String> });
closed!(MotdResult { text: MotdText });
closed!(SaveReceipt {
    save_receipt_id: String,
    saved_at: String
});
closed!(DrainResult {
    admission_closed: bool,
    gameplay_sealed: bool,
    sessions_remaining: u32
});
wire_enum!(WorldState {
    Open,
    Closed,
    Maintenance
});
closed!(WorldStateParams { state: WorldState, kick_sessions: Option<bool>, expected_revision: Option<String>, preview_digest: Option<String> });
closed!(WorldStateResult {
    previous_state: WorldState,
    state: WorldState,
    sessions_kicked: u32
});
closed!(RestartParams { countdown_seconds: CountdownSeconds, preview_digest: Option<String> });
closed!(RestartResult { restarted_node_ids: Vec<String>, save_receipt: SaveReceipt, state: WorldState });

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(!try_from)]
#[serde(try_from = "String")]
pub struct EnforcementId(
    #[schemars(regex(
        pattern = "^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
    ))]
    String,
);

impl TryFrom<String> for EnforcementId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                14 => byte == b'7',
                19 => b"89ab".contains(&byte),
                _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
            })
        {
            return Ok(Self(value));
        }
        Err("expected a canonical UUIDv7")
    }
}

bounded_integer!(EnforcementVersion, u16, "u16", 1, 1);
bounded_integer!(EnforcementReason, u16, "u16", 1, 65535);
bounded_integer!(EnforcementMillis, u64, "u64", 1, 9_007_199_254_740_991u64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplyIntent {
    Apply,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevokeIntent {
    Revoke,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnforcementMuteProfile {
    Mute,
}

wire_enum!(BanProfile {
    BanCharacter,
    BanAccount
});

macro_rules! sanction_decision {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        closed!($name {
            operation_id: EnforcementId,
            sanction_stream_id: EnforcementId,
            decision_revision: DecisionRevision,
            $($field: $ty,)*
            preview_digest: Option<String>
        });
    };
}

// Preserve the fields and values of Enforcement Command v1 for the existing MUTE profile.
macro_rules! mute_decision {
    ($name:ident, $intent:ty, $prior:ty, $timestamp:ty) => {
        sanction_decision!($name {
            command_version: EnforcementVersion,
            intent: $intent,
            profile: EnforcementMuteProfile,
            prior_decision_revision: $prior,
            policy_revision: DecisionRevision,
            account_id: EnforcementId,
            character_id: EnforcementId,
            effective_at_ms: $timestamp,
            expires_at_ms: $timestamp,
            reason_code: EnforcementReason,
            decided_at_ms: EnforcementMillis,
            audit_ref: EnforcementId,
            correlation_id: EnforcementId
        });
    };
}

mute_decision!(MuteParams, ApplyIntent, (), EnforcementMillis);
mute_decision!(UnmuteParams, RevokeIntent, DecisionRevision, ());
sanction_decision!(BanParams { effective_at_ms: EnforcementMillis, expires_at_ms: Option<EnforcementMillis> });
sanction_decision!(RevokeParams {
    profile: BanProfile,
    prior_decision_revision: DecisionRevision
});
wire_enum!(ActiveState { Active });
wire_enum!(RevokedState { Revoked });
wire_enum!(MuteProfile { Mute });
wire_enum!(BanCharacterProfile { BanCharacter });
wire_enum!(BanAccountProfile { BanAccount });
wire_enum!(RestrictionEffect {
    OnlineApplied,
    OnNextAdmission,
    None
});

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SanctionDecisionResult<P, S> {
    pub operation_id: EnforcementId,
    pub sanction_stream_id: EnforcementId,
    pub decision_revision: DecisionRevision,
    pub profile: P,
    pub sanction_state: S,
    pub runtime_effect: RestrictionEffect,
}

pub type MuteResult = SanctionDecisionResult<MuteProfile, ActiveState>;
pub type UnmuteResult = SanctionDecisionResult<MuteProfile, RevokedState>;
pub type BanCharacterResult = SanctionDecisionResult<BanCharacterProfile, ActiveState>;
pub type BanAccountResult = SanctionDecisionResult<BanAccountProfile, ActiveState>;
pub type RevokeResult = SanctionDecisionResult<BanProfile, RevokedState>;
closed!(NotationParams { note: StaffNote, preview_digest: Option<String> });
closed!(NotationResult {
    notation_id: String
});

wire_enum!(Metric {
    TickTimeMs,
    PlayersOnline,
    CpuPercent,
    MemoryBytes,
    DiskUsedBytes,
    PgPoolWaitMs,
    QueueDepth,
    ErrorRatePerSecond
});
closed!(MetricsParams {
    metric: Metric,
    from: String,
    to: String,
    step_seconds: CountdownSeconds
});
closed!(MetricPoint { at: String, value: Option<f64> });
closed!(MetricsResult { metric: Metric, points: AnalyticsRows<MetricPoint> });
wire_enum!(AlertKind {
    NodeDown,
    Lag,
    Disk,
    BackupAge,
    ErrorSpike
});
closed!(AlertsParams { limit: PageLimit, cursor: Option<String> });
closed!(Alert { alert_id: String, kind: AlertKind, opened_at: String, world_id: Option<String>, node_id: Option<String>, acknowledged_at: Option<String> });
closed!(AlertsResult { alerts: PageRows<Alert>, next_cursor: Option<String> });
closed!(AlertAckParams { alert_id: String, preview_digest: Option<String> });
closed!(AlertAckResult {
    alert_id: String,
    acknowledged_at: String
});

wire_enum!(LogLevel {
    Error,
    Warn,
    Info,
    Debug
});
closed!(LogsParams { limit: LogLimit, from: String, to: String, channel_id: Option<String>, level: Option<LogLevel> });
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(!try_from)]
#[serde(try_from = "String")]
pub struct DiagnosticToken(
    #[schemars(length(min = 1, max = 128), regex(pattern = "^[A-Za-z0-9._:/+-]+$"))] String,
);

impl TryFrom<String> for DiagnosticToken {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if (1..=128).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:/+-".contains(&byte))
        {
            return Ok(Self(value));
        }
        Err("expected a diagnostic token")
    }
}

// Safe subset of the structured diagnostic Line fields. Source redaction is mandatory.
closed!(DiagnosticLine {
    process: DiagnosticToken,
    ts: UnixMillis,
    level: LogLevel,
    module: DiagnosticToken,
    event: DiagnosticToken,
    build: Option<DiagnosticToken>,
    code: Option<DiagnosticToken>,
    wire: Option<u32>,
    world: Option<DiagnosticToken>,
    channel: Option<DiagnosticToken>
});
closed!(LogsResult { lines: LogLines<DiagnosticLine> });

wire_enum!(Grain { Minute, Hour, Day });
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Completeness {
    ProvenCompleteForDeclaredDurableScope,
    NoKnownGapBestEffort,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Watermark {
    Known(String),
    Unavailable(()),
}

closed!(EconomyContext {
    world_id: String,
    world_age_days: u32,
    active_characters: Option<Decimal>,
    ruleset_revision: String,
    content_revision: String
});
closed!(GoldSupplyRow {
    bucket_start: String,
    gold: Decimal
});
closed!(GoldFlowRow {
    bucket_start: String,
    source: String,
    minted_gold: Decimal,
    burned_gold: Decimal
});
closed!(ItemSupplyRow {
    bucket_start: String,
    item_type_id: u32,
    quantity: Decimal
});
closed!(MarketVolumeRow {
    bucket_start: String,
    item_type_id: u32,
    trade_count: Decimal,
    quantity: Decimal,
    gold_volume: Decimal
});
closed!(MarketPriceRow {
    bucket_start: String,
    item_type_id: u32,
    min_unit_price_gold: Decimal,
    median_unit_price_gold: Decimal,
    max_unit_price_gold: Decimal
});
closed!(NpcTradeRow {
    bucket_start: String,
    npc_type_id: String,
    item_type_id: u32,
    items_bought_from_npc: Decimal,
    items_sold_to_npc: Decimal,
    gold_paid_to_npc: Decimal,
    gold_received_from_npc: Decimal
});
closed!(WealthRow {
    rank: PageLimit,
    analytics_actor_id: String,
    gold: Decimal
});

macro_rules! economy_queries {
    ($($variant:ident, $id:literal, $tag:ident, $params:ident, $filters:ident, $result:ident, $row:ty, $risk:ident {
        $($field:ident: $ty:ty),* $(,)?
    }),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
            pub enum $tag {
                #[serde(rename = $id)]
                $variant,
            }
            closed!($filters { $($field: $ty),* });
            closed!($params { query_id: $tag, params: $filters, from: String, to: String, grain: Grain });
            closed!($result {
                query_id: $tag,
                filters: $params,
                metric_revision: String,
                watermark: Watermark,
                completeness: Completeness,
                context: EconomyContext,
                rows: AnalyticsRows<$row>
            });
        )+

        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[serde(untagged)]
        pub enum AnalyticsQueryParams { $($variant($params)),+ }

        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[serde(untagged)]
        pub enum AnalyticsQueryResult { $($variant($result)),+ }

        impl AnalyticsQueryParams {
            pub fn minimum(&self) -> RiskClass {
                match self { $(Self::$variant(_) => RiskClass::$risk),+ }
            }
        }

        pub const ECONOMY_QUERY_IDS: &[&str] = &[$($id),+];

        pub(crate) fn economy_schemas(out: &mut std::collections::BTreeMap<String, String>) -> Result<(), WireError> {
            $(
                crate::schema::insert_pair::<$params, $result>(out, concat!("analytics.query.", $id))?;
            )+
            Ok(())
        }
    };
}

economy_queries! {
    GoldSourcesSinks, "gold_sources_sinks", GoldSourcesSinksId, GoldSourcesSinksParams, GoldSourcesSinksFilters, GoldSourcesSinksResult, GoldFlowRow, R0 {
        source: Option<String>
    },
    GoldSupply, "gold_supply", GoldSupplyId, GoldSupplyParams, GoldSupplyFilters, GoldSupplyResult, GoldSupplyRow, R0 {},
    ItemSupplyByType, "item_supply_by_type", ItemSupplyByTypeId, ItemSupplyByTypeParams, ItemSupplyByTypeFilters, ItemSupplyByTypeResult, ItemSupplyRow, R0 {
        item_type_id: Option<u32>
    },
    MarketPriceSeries, "market_price_series", MarketPriceSeriesId, MarketPriceSeriesParams, MarketPriceSeriesFilters, MarketPriceSeriesResult, MarketPriceRow, R0 {
        item_type_id: u32
    },
    MarketVolume, "market_volume", MarketVolumeId, MarketVolumeParams, MarketVolumeFilters, MarketVolumeResult, MarketVolumeRow, R0 {
        item_type_id: Option<u32>
    },
    NpcTradeFlow, "npc_trade_flow", NpcTradeFlowId, NpcTradeFlowParams, NpcTradeFlowFilters, NpcTradeFlowResult, NpcTradeRow, R0 {
        item_type_id: Option<u32>, npc_type_id: Option<String>
    },
    TopWealth, "top_wealth", TopWealthId, TopWealthParams, TopWealthFilters, TopWealthResult, WealthRow, R1 {
        limit: PageLimit
    },
}
