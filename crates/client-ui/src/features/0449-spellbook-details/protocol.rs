use oteryn_client_runtime::{
    raw_feature::FeatureCommand,
    spell_catalog::spellbook_details::{
        CAPABILITY_SPELLBOOK_DETAILS_V1, COMMAND_TYPE_SPELLBOOK_QUERY, Query,
    },
};
pub fn command(query: &Query) -> Option<FeatureCommand> {
    Some(FeatureCommand {
        capability: CAPABILITY_SPELLBOOK_DETAILS_V1,
        kind: COMMAND_TYPE_SPELLBOOK_QUERY,
        payload: query.encode()?,
    })
}
