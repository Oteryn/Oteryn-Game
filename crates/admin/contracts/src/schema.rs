use crate::*;
use schemars::schema_for;
use std::collections::BTreeMap;

/// Stable schema names and formatting form the shared repository golden contract.
pub fn schemas() -> Result<BTreeMap<String, String>, WireError> {
    let mut out = BTreeMap::new();
    out.insert(
        "request.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(crate::Request))? + "\n",
    );
    out.insert(
        "result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(crate::Response))? + "\n",
    );
    out.insert(
        "ops.overview.get.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "ops.overview.get.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Overview))? + "\n",
    );
    out.insert(
        "world.list.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Page))? + "\n",
    );
    out.insert(
        "world.list.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(WorldPage))? + "\n",
    );
    out.insert(
        "world.get.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "world.get.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(World))? + "\n",
    );
    out.insert(
        "node.list.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Page))? + "\n",
    );
    out.insert(
        "node.list.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(NodePage))? + "\n",
    );
    out.insert(
        "node.get.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "node.get.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Node))? + "\n",
    );
    out.insert(
        "db.status.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "db.status.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(DatabaseStatus))? + "\n",
    );
    out.insert(
        "db.migrations.list.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Page))? + "\n",
    );
    out.insert(
        "db.migrations.list.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(MigrationPage))? + "\n",
    );
    out.insert(
        "player.search.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(PlayerSearch))? + "\n",
    );
    out.insert(
        "player.search.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(PlayerPage))? + "\n",
    );
    out.insert(
        "player.character.get.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "player.character.get.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Character))? + "\n",
    );
    out.insert(
        "player.inventory.get.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Page))? + "\n",
    );
    out.insert(
        "player.inventory.get.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Inventory))? + "\n",
    );
    out.insert(
        "sanction.list.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Page))? + "\n",
    );
    out.insert(
        "sanction.list.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(SanctionPage))? + "\n",
    );
    out.insert(
        "admin.ledger.query.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(LedgerQuery))? + "\n",
    );
    out.insert(
        "admin.ledger.query.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(LedgerPage))? + "\n",
    );
    out.insert(
        "admin.policy.get.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "admin.policy.get.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(PolicyView))? + "\n",
    );
    out.insert(
        "admin.agent.list.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(Page))? + "\n",
    );
    out.insert(
        "admin.agent.list.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(AgentPage))? + "\n",
    );
    out.insert(
        "analytics.catalog.params.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(ReadParams))? + "\n",
    );
    out.insert(
        "analytics.catalog.result.schema.json".into(),
        serde_json::to_string_pretty(&schema_for!(AnalyticsCatalog))? + "\n",
    );
    Ok(out)
}
