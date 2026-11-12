pub fn request(definitions: &[u32]) -> Option<Vec<u8>> {
    oteryn_client_runtime::action_bar_cooldowns::encode_query(definitions)
}
