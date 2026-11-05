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
closed!(Inventory { items: Vec<Item>, next_cursor: Option<String> });
closed!(Sanction { sanction_id: String, character_id: Option<String>, account_ref: Option<String>, kind: String, starts_at: String, expires_at: Option<String>, active: bool });
closed!(SanctionPage { sanctions: Vec<Sanction>, next_cursor: Option<String> });
closed!(LedgerEntry { ledger_id: String, request_id: String, admin_operation_id: String, operation: String, admin_id: String, risk_class: RiskClass, target: crate::Target, status: crate::Status, error_code: Option<crate::ErrorCode>, started_at: String, finished_at: String });
closed!(LedgerPage { entries: Vec<LedgerEntry>, next_cursor: Option<String> });
closed!(PolicyOperation { operation: String, min_risk_class: RiskClass, allowed_permissions: Vec<String>, worlds: Vec<String> });
closed!(PolicyView { operations: Vec<PolicyOperation> });
closed!(AgentPage { agents: Vec<Agent>, next_cursor: Option<String> });
closed!(AnalyticsMetric { query_id: String, metric_revision: String, watermark: Option<String> });
closed!(AnalyticsCatalog { metrics: Vec<AnalyticsMetric> });

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "ops.overview.get")]
    OpsOverviewGet,
    #[serde(rename = "world.list")]
    WorldList,
    #[serde(rename = "world.get")]
    WorldGet,
    #[serde(rename = "node.list")]
    NodeList,
    #[serde(rename = "node.get")]
    NodeGet,
    #[serde(rename = "db.status")]
    DbStatus,
    #[serde(rename = "db.migrations.list")]
    DbMigrationsList,
    #[serde(rename = "player.search")]
    PlayerSearch,
    #[serde(rename = "player.character.get")]
    PlayerCharacterGet,
    #[serde(rename = "player.inventory.get")]
    PlayerInventoryGet,
    #[serde(rename = "sanction.list")]
    SanctionList,
    #[serde(rename = "admin.ledger.query")]
    AdminLedgerQuery,
    #[serde(rename = "admin.policy.get")]
    AdminPolicyGet,
    #[serde(rename = "admin.agent.list")]
    AdminAgentList,
    #[serde(rename = "analytics.catalog")]
    AnalyticsCatalog,
}
impl Operation {
    pub const ALL: [Self; 15] = [
        Self::OpsOverviewGet,
        Self::WorldList,
        Self::WorldGet,
        Self::NodeList,
        Self::NodeGet,
        Self::DbStatus,
        Self::DbMigrationsList,
        Self::PlayerSearch,
        Self::PlayerCharacterGet,
        Self::PlayerInventoryGet,
        Self::SanctionList,
        Self::AdminLedgerQuery,
        Self::AdminPolicyGet,
        Self::AdminAgentList,
        Self::AnalyticsCatalog,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::OpsOverviewGet => "ops.overview.get",
            Self::WorldList => "world.list",
            Self::WorldGet => "world.get",
            Self::NodeList => "node.list",
            Self::NodeGet => "node.get",
            Self::DbStatus => "db.status",
            Self::DbMigrationsList => "db.migrations.list",
            Self::PlayerSearch => "player.search",
            Self::PlayerCharacterGet => "player.character.get",
            Self::PlayerInventoryGet => "player.inventory.get",
            Self::SanctionList => "sanction.list",
            Self::AdminLedgerQuery => "admin.ledger.query",
            Self::AdminPolicyGet => "admin.policy.get",
            Self::AdminAgentList => "admin.agent.list",
            Self::AnalyticsCatalog => "analytics.catalog",
        }
    }
    pub fn minimum(self) -> RiskClass {
        match self {
            Self::OpsOverviewGet => RiskClass::R0,
            Self::WorldList => RiskClass::R0,
            Self::WorldGet => RiskClass::R0,
            Self::NodeList => RiskClass::R0,
            Self::NodeGet => RiskClass::R0,
            Self::DbStatus => RiskClass::R0,
            Self::DbMigrationsList => RiskClass::R0,
            Self::PlayerSearch => RiskClass::R1,
            Self::PlayerCharacterGet => RiskClass::R1,
            Self::PlayerInventoryGet => RiskClass::R1,
            Self::SanctionList => RiskClass::R1,
            Self::AdminLedgerQuery => RiskClass::R1,
            Self::AdminPolicyGet => RiskClass::R0,
            Self::AdminAgentList => RiskClass::R0,
            Self::AnalyticsCatalog => RiskClass::R0,
        }
    }
    pub fn permission(self) -> &'static str {
        match self {
            Self::OpsOverviewGet => "game.ops.read",
            Self::WorldList => "game.world.read",
            Self::WorldGet => "game.world.read",
            Self::NodeList => "game.node.read",
            Self::NodeGet => "game.node.read",
            Self::DbStatus => "game.db.read",
            Self::DbMigrationsList => "game.db.read",
            Self::PlayerSearch => "game.player.read",
            Self::PlayerCharacterGet => "game.player.read",
            Self::PlayerInventoryGet => "game.player.read",
            Self::SanctionList => "game.sanction.read",
            Self::AdminLedgerQuery => "game.audit.read",
            Self::AdminPolicyGet => "game.ops.read",
            Self::AdminAgentList => "game.ops.read",
            Self::AnalyticsCatalog => "game.analytics.read",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.name() == name)
    }
    pub fn validate_target(self, target: &crate::Target) -> Result<(), WireError> {
        let present = |id: &Option<String>| id.as_deref().is_some_and(|s| !s.trim().is_empty());
        let valid = match self {
            Self::WorldGet => present(&target.world_id),
            Self::NodeGet => present(&target.node_id),
            Self::PlayerCharacterGet | Self::PlayerInventoryGet => {
                present(&target.character_id) && present(&target.world_id)
            }
            Self::SanctionList => present(&target.character_id) || present(&target.account_ref),
            _ => true,
        };
        if !valid {
            return Err("required operation target missing".into());
        }
        Ok(())
    }
    pub fn validate_params(self, value: &Value) -> Result<(), WireError> {
        match self {
            Self::OpsOverviewGet => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
            Self::WorldList => {
                let _: Page = serde_json::from_value(value.clone())?;
            }
            Self::WorldGet => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
            Self::NodeList => {
                let _: Page = serde_json::from_value(value.clone())?;
            }
            Self::NodeGet => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
            Self::DbStatus => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
            Self::DbMigrationsList => {
                let _: Page = serde_json::from_value(value.clone())?;
            }
            Self::PlayerSearch => {
                let _: PlayerSearch = serde_json::from_value(value.clone())?;
            }
            Self::PlayerCharacterGet => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
            Self::PlayerInventoryGet => {
                let _: Page = serde_json::from_value(value.clone())?;
            }
            Self::SanctionList => {
                let _: Page = serde_json::from_value(value.clone())?;
            }
            Self::AdminLedgerQuery => {
                let _: LedgerQuery = serde_json::from_value(value.clone())?;
            }
            Self::AdminPolicyGet => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
            Self::AdminAgentList => {
                let _: Page = serde_json::from_value(value.clone())?;
            }
            Self::AnalyticsCatalog => {
                let _: ReadParams = serde_json::from_value(value.clone())?;
            }
        }
        if let Some(limit) = value.get("limit")
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OperationResult {
    OpsOverviewGet(Overview),
    WorldList(WorldPage),
    WorldGet(World),
    NodeList(NodePage),
    NodeGet(Node),
    DbStatus(DatabaseStatus),
    DbMigrationsList(MigrationPage),
    PlayerSearch(PlayerPage),
    PlayerCharacterGet(Character),
    PlayerInventoryGet(Inventory),
    SanctionList(SanctionPage),
    AdminLedgerQuery(LedgerPage),
    AdminPolicyGet(PolicyView),
    AdminAgentList(AgentPage),
    AnalyticsCatalog(AnalyticsCatalog),
}
