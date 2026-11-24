use crate::{RiskClass, WireError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! closed {
    ($name:ident { $($(#[$meta:meta])* $field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $($(#[$meta])* pub $field: $ty),* }
    };
}
#[path = "phase2.rs"]
mod phase2;
pub use phase2::*;

closed!(ReadParams { preview_digest: Option<String> });
closed!(Page { #[schemars(range(min = 1, max = 100))] limit: u16, cursor: Option<String> });
closed!(PlayerSearch { #[schemars(range(min = 1, max = 100))] limit: u16, cursor: Option<String>, name_prefix: Option<String>, character_id: Option<String>, account_ref: Option<String>, ip_hash: Option<String> });
closed!(LedgerQuery { #[schemars(range(min = 1, max = 100))] limit: u16, cursor: Option<String>, admin_id: Option<String>, operation: Option<String>, world_id: Option<String>, character_id: Option<String>, from: String, to: String });
closed!(World { world_id: String, name: String, state: String, channels: Vec<String>, ruleset_revision: String, content_revision: String, map_revision: String, policy_revision: String });
closed!(Node { node_id: String, host: String, build: String, uptime_seconds: u32, worlds: Vec<String>, ready: bool, cpu_percent: f64, memory_bytes: String });
closed!(Agent {
    agent_id: String,
    build: String,
    catalog_digest: String,
    last_heartbeat: String,
    connected: bool
});
closed!(Overview { agents: Vec<Agent>, worlds: Vec<World>, nodes: Vec<Node>, players_online: u32, db_available: bool, backup_age_seconds: Option<u32>, alerts: Vec<String> });
closed!(WorldPage { worlds: Vec<World>, next_cursor: Option<String> });
closed!(NodePage { nodes: Vec<Node>, next_cursor: Option<String> });
closed!(TableSize {
    schema: String,
    table: String,
    bytes: String
});
closed!(RoleConnections {
    role: String,
    connections: u32
});
closed!(DatabaseStatus { tables: Vec<TableSize>, connections: Vec<RoleConnections>, locks_waiting: u32, replication_lag_seconds: Option<f64>, bloat_estimate_bytes: Option<String>, oldest_transaction_seconds: Option<u32>, cache_hit_ratio: f64 });
closed!(Migration {
    version: String,
    description: String,
    applied: bool,
    checksum_drift: bool
});
closed!(MigrationPage { migrations: Vec<Migration>, next_cursor: Option<String> });
closed!(PlayerSummary {
    character_id: String,
    name: String,
    world_id: String,
    level: u32,
    online: bool
});
closed!(PlayerPage { players: Vec<PlayerSummary>, next_cursor: Option<String> });
closed!(Skill {
    name: String,
    level: u32,
    progress: u16
});
closed!(Position {
    x: i32,
    y: i32,
    z: i32
});
closed!(Character { character_id: String, name: String, world_id: String, level: u32, vocation: String, skills: Vec<Skill>, position: Position, online: bool, premium: bool, guild: Option<String>, house: Option<String>, bank_balance: String, sanctions: Vec<Sanction>, last_logins: Vec<String> });
closed!(Item { instance_id: String, type_id: u32, count: String, location: String, parent_id: Option<String> });
closed!(StashBalance {
    type_id: u32,
    count: String
});
closed!(Inventory { items: Vec<Item>, stash_balances: Vec<StashBalance>, next_cursor: Option<String> });
closed!(Sanction { sanction_id: String, character_id: Option<String>, account_ref: Option<String>, kind: String, starts_at: String, expires_at: Option<String>, active: bool });
closed!(SanctionPage { sanctions: Vec<Sanction>, next_cursor: Option<String> });
closed!(LedgerEntry { ledger_id: String, request_id: String, admin_operation_id: String, operation: String, admin_id: String, risk_class: RiskClass, target: crate::Target, status: crate::Status, error_code: Option<crate::ErrorCode>, started_at: String, finished_at: String });
closed!(LedgerPage { entries: Vec<LedgerEntry>, next_cursor: Option<String> });
closed!(PolicyOperation { operation: String, min_risk_class: RiskClass, allowed_permissions: Vec<String>, worlds: Vec<String> });
closed!(PolicyView { operations: Vec<PolicyOperation> });
closed!(AgentPage { agents: Vec<Agent>, next_cursor: Option<String> });
closed!(AnalyticsMetric { query_id: String, metric_revision: String, watermark: Option<String> });
closed!(AnalyticsCatalog { metrics: Vec<AnalyticsMetric> });

// Keep the wire name, risk, and schema pair together so every catalog entry is exported.
macro_rules! operations {
    ($($variant:ident => ($name:literal, $phase:literal, $kind:ident, $risk:ident, $perm:literal, $params:ty, $result:ty)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub enum Operation {
            $(#[serde(rename = $name)] $variant),+
        }

        impl Operation {
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            pub fn name(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }

            pub fn phase(self) -> u8 {
                match self { $(Self::$variant => $phase),+ }
            }

            pub fn kind(self) -> crate::Kind {
                match self { $(Self::$variant => crate::Kind::$kind),+ }
            }

            /// Base minimum. Use minimum_for_params for named queries.
            pub fn minimum(self) -> RiskClass {
                match self { $(Self::$variant => RiskClass::$risk),+ }
            }

            pub fn permission(self) -> &'static str {
                match self { $(Self::$variant => $perm),+ }
            }

            fn decode_params(self, value: &Value) -> Result<(), WireError> {
                match self { $(Self::$variant => decode::<$params>(value)),+ }
            }

            pub fn validate_result(self, value: &Value) -> Result<(), WireError> {
                match self { $(Self::$variant => decode::<$result>(value)),+ }
            }

            pub(crate) fn insert_schemas(out: &mut std::collections::BTreeMap<String, String>) -> Result<(), WireError> {
                $(crate::schema::insert_pair::<$params, $result>(out, $name)?;)+
                economy_schemas(out)
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[serde(untagged)]
        pub enum OperationResult {
            $($variant($result)),+
        }
    };
}

operations! {
    OpsOverviewGet => ("ops.overview.get", 1, Query, R0, "game.ops.read", ReadParams, Overview),
    WorldList => ("world.list", 1, Query, R0, "game.world.read", Page, WorldPage),
    WorldGet => ("world.get", 1, Query, R0, "game.world.read", ReadParams, World),
    NodeList => ("node.list", 1, Query, R0, "game.node.read", Page, NodePage),
    NodeGet => ("node.get", 1, Query, R0, "game.node.read", ReadParams, Node),
    DbStatus => ("db.status", 1, Query, R0, "game.db.read", ReadParams, DatabaseStatus),
    DbMigrationsList => ("db.migrations.list", 1, Query, R0, "game.db.read", Page, MigrationPage),
    PlayerSearch => ("player.search", 1, Query, R1, "game.player.read", PlayerSearch, PlayerPage),
    PlayerCharacterGet => ("player.character.get", 1, Query, R1, "game.player.read", ReadParams, Character),
    PlayerInventoryGet => ("player.inventory.get", 1, Query, R1, "game.player.read", Page, Inventory),
    SanctionList => ("sanction.list", 1, Query, R1, "game.sanction.read", Page, SanctionPage),
    AdminLedgerQuery => ("admin.ledger.query", 1, Query, R1, "game.audit.read", LedgerQuery, LedgerPage),
    AdminPolicyGet => ("admin.policy.get", 1, Query, R0, "game.ops.read", ReadParams, PolicyView),
    AdminAgentList => ("admin.agent.list", 1, Query, R0, "game.ops.read", Page, AgentPage),
    AnalyticsCatalog => ("analytics.catalog", 1, Query, R0, "game.analytics.read", ReadParams, AnalyticsCatalog),
    AnalyticsQuery => ("analytics.query", 2, Query, R0, "game.analytics.read", AnalyticsQueryParams, AnalyticsQueryResult),
    NodeDrain => ("node.drain", 2, Command, R2, "game.node.ops", ReadParams, DrainResult),
    OpsAlertsAck => ("ops.alerts.ack", 2, Command, R2, "game.ops.write", AlertAckParams, AlertAckResult),
    OpsAlertsList => ("ops.alerts.list", 2, Query, R0, "game.ops.read", AlertsParams, AlertsResult),
    OpsLogsTail => ("ops.logs.tail", 2, Query, R1, "game.ops.logs.read", LogsParams, LogsResult),
    OpsMetricsSeries => ("ops.metrics.series", 2, Query, R0, "game.ops.read", MetricsParams, MetricsResult),
    PlayerKick => ("player.kick", 2, Command, R2, "game.player.moderate", KickParams, KickResult),
    PlayerMessage => ("player.message", 2, Command, R2, "game.player.moderate", MessageParams, MessageResult),
    PlayerTeleport => ("player.teleport", 2, Command, R2, "game.player.moderate", TeleportParams, TeleportResult),
    SanctionBanAccount => ("sanction.ban.account", 2, Command, R3, "game.sanction.ban", BanParams, BanAccountResult),
    SanctionBanCharacter => ("sanction.ban.character", 2, Command, R3, "game.sanction.ban", BanParams, BanCharacterResult),
    SanctionMute => ("sanction.mute", 2, Command, R2, "game.sanction.write", MuteParams, MuteResult),
    SanctionNotationAdd => ("sanction.notation.add", 2, Command, R2, "game.sanction.write", NotationParams, NotationResult),
    SanctionRevoke => ("sanction.revoke", 2, Command, R3, "game.sanction.ban", RevokeParams, RevokeResult),
    SanctionUnmute => ("sanction.unmute", 2, Command, R2, "game.sanction.write", UnmuteParams, UnmuteResult),
    WorldBroadcast => ("world.broadcast", 2, Command, R2, "game.world.ops", MessageParams, BroadcastResult),
    WorldMotdSet => ("world.motd.set", 2, Command, R2, "game.world.ops", MotdParams, MotdResult),
    WorldRestart => ("world.restart", 2, Command, R3, "game.world.ops", RestartParams, RestartResult),
    WorldSaveNow => ("world.save_now", 2, Command, R2, "game.world.ops", ReadParams, SaveReceipt),
    WorldStateSet => ("world.state.set", 2, Command, R3, "game.world.config", WorldStateParams, WorldStateResult),
}

fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<(), WireError> {
    serde_json::from_value::<T>(value.clone())?;
    Ok(())
}

impl Operation {
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.name() == name)
    }

    /// Phase 2 is published for form generation. The agent advertises only implemented executors.
    pub fn minimum_for_params(self, params: &Value) -> Result<RiskClass, WireError> {
        if self == Self::AnalyticsQuery {
            return Ok(serde_json::from_value::<AnalyticsQueryParams>(params.clone())?.minimum());
        }
        Ok(self.minimum())
    }

    pub fn is_global_read(self) -> bool {
        matches!(
            self,
            Self::AdminPolicyGet | Self::DbMigrationsList | Self::DbStatus
        )
    }

    pub fn validate_target(self, target: &crate::Target) -> Result<(), WireError> {
        let present = |id: &Option<String>| id.as_deref().is_some_and(|s| !s.trim().is_empty());
        let valid = match self {
            Self::AnalyticsQuery
            | Self::WorldGet
            | Self::WorldMotdSet
            | Self::WorldRestart
            | Self::WorldSaveNow
            | Self::WorldStateSet => present(&target.world_id),
            Self::NodeDrain | Self::NodeGet => present(&target.node_id),
            Self::PlayerCharacterGet
            | Self::PlayerInventoryGet
            | Self::PlayerKick
            | Self::PlayerMessage
            | Self::PlayerTeleport
            | Self::SanctionBanCharacter
            | Self::SanctionMute
            | Self::SanctionUnmute => present(&target.character_id) && present(&target.world_id),
            Self::SanctionBanAccount => present(&target.account_ref),
            Self::SanctionList | Self::SanctionNotationAdd | Self::SanctionRevoke => {
                present(&target.character_id) || present(&target.account_ref)
            }
            _ => true,
        };
        if !valid {
            return Err("required operation target missing".into());
        }
        let incompatible = match self {
            Self::AdminPolicyGet | Self::DbMigrationsList | Self::DbStatus => {
                target.world_id.is_some()
                    || target.node_id.is_some()
                    || target.character_id.is_some()
                    || target.account_ref.is_some()
            }
            Self::NodeGet
            | Self::NodeList
            | Self::OpsOverviewGet
            | Self::WorldGet
            | Self::WorldList => {
                target.character_id.is_some()
                    || target.account_ref.is_some()
                    || (matches!(self, Self::WorldGet | Self::WorldList)
                        && target.node_id.is_some())
            }
            Self::PlayerCharacterGet
            | Self::PlayerInventoryGet
            | Self::PlayerSearch
            | Self::SanctionList => target.node_id.is_some(),
            _ => false,
        };
        if incompatible {
            return Err("incompatible operation target".into());
        }
        Ok(())
    }

    pub fn validate_params(self, value: &Value) -> Result<(), WireError> {
        self.decode_params(value)?;
        let integer = |key| value.get(key).and_then(Value::as_u64);
        if integer("effective_at_ms")
            .zip(integer("expires_at_ms"))
            .is_some_and(|(start, end)| start >= end)
        {
            return Err("sanction expiry must follow its effective time".into());
        }
        if integer("prior_decision_revision")
            .zip(integer("decision_revision"))
            .is_some_and(|(prior, next)| prior >= next)
        {
            return Err("sanction decision revision must advance".into());
        }
        if self.phase() == 1
            && let Some(limit) = value.get("limit")
            && limit.as_u64().is_none_or(|n| n == 0 || n > 100)
        {
            return Err("limit must be 1..=100".into());
        }
        if self == Self::PlayerSearch
            && !["name_prefix", "character_id", "account_ref", "ip_hash"]
                .iter()
                .any(|k| {
                    value
                        .get(k)
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                })
        {
            return Err("search filter required".into());
        }
        Ok(())
    }
}
