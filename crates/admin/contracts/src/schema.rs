use crate::{Operation, Request, Response, WireError};
use schemars::{JsonSchema, schema_for};
use std::collections::BTreeMap;

type Schemas = BTreeMap<String, String>;

fn insert<T: JsonSchema>(out: &mut Schemas, name: String) -> Result<(), WireError> {
    out.insert(name, serde_json::to_string_pretty(&schema_for!(T))? + "\n");
    Ok(())
}

pub(crate) fn insert_pair<P: JsonSchema, R: JsonSchema>(
    out: &mut Schemas,
    name: &str,
) -> Result<(), WireError> {
    insert::<P>(out, format!("{name}.params.schema.json"))?;
    insert::<R>(out, format!("{name}.result.schema.json"))
}

/// Stable schema names and formatting form the shared repository golden contract.
pub fn schemas() -> Result<BTreeMap<String, String>, WireError> {
    let mut out = BTreeMap::new();
    insert::<Request>(&mut out, "request.schema.json".into())?;
    insert::<Response>(&mut out, "result.schema.json".into())?;
    Operation::insert_schemas(&mut out)?;
    Ok(out)
}
