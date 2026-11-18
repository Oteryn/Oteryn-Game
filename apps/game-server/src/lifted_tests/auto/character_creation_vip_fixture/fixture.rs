use crate::bestiary_postgres_harness::{CHARACTER, Harness, TestResult, debug, id};

pub(super) async fn fixture_character_world(
    h: &Harness,
    seed: u8,
    owner: u8,
    world: u8,
) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(id(owner).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex,creation_vocation) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($5,'hex')::uuid,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3,2,'knight' FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(owner).as_slice()).bind(format!("Vip Fixture {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26))).bind(id(CHARACTER).as_slice()).bind(id(world).as_slice()).execute(&h.pool).await?;
    // Complete historical creation fixture through the existing native decoder/binder.
    // This is not an actor authority constructor: the real sealed recovery and full
    // integrity checks below must still succeed after these historical rows exist.
    let distinct = |tag| {
        let mut value = id(seed);
        value[15] ^= tag;
        value
    };
    let operation = distinct(0x80);
    let decision = distinct(0x81);
    let uuid = |bytes: [u8; 16]| {
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    };
    let raw = serde_json::to_vec(&serde_json::json!({
        "contract_version":4,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),
        "source_revision":seed.to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":uuid(id(owner)),"target_world_id":uuid(id(world)),
        "requested_sex":"Male","requested_vocation":"Knight","requested_name":format!("Vip Fixture {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26)),
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"
    }))?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(id(owner).as_slice()).bind(id(seed).as_slice()).bind(id(world).as_slice())
        .bind(distinct(0x82).as_slice()).bind(distinct(0x83).as_slice()).bind(decision.as_slice()).bind(i64::from(seed)).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding")
        .execute(&h.pool).await?;
    Ok(())
}
