//! Receipt-backed historical XP/build setup, matching the Wheel writer qualification.
use super::*;
const CHARACTER: u8 = 41;
const LEVEL: i64 = 60;
const EXPERIENCE: i64 = 5000;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}
fn advance(original: u64, level: i64, experience: i64) -> String {
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state \
            SET character_revision = {committed}, level = {level}, total_experience = {experience} \
          WHERE character_id = {character} AND character_revision = {original};",
        committed = original + 1,
        character = uuid(CHARACTER),
    )
}

/// An XP award transaction at `original` from (`before`) to (`after`) level and experience.
fn award(occurrence: u8, original: u64, before: (i64, i64), after: (i64, i64)) -> String {
    format!(
        "{advance}INSERT INTO game_character_xp_receipts(\
           reward_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, level_after, \
           experience_before, experience_after, experience_awarded, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           {original}, {committed}, {level_before}, {level_after}, {experience_before}, \
           {experience_after}, {awarded}, 'profile-1', 'ruleset-1', 'content-1', 'simulation-1', \
           'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 1);",
        advance = advance(original, after.0, after.1),
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 8]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
        level_before = before.0,
        level_after = after.0,
        experience_before = before.1,
        experience_after = after.1,
        awarded = after.1 - before.1,
    )
}

/// The vocation choice `none` -> `vocation` at r3 (level 60), with its build row.
fn choose_vocation(occurrence: u8, vocation: &str) -> String {
    let columns = "magic_level{s}, mana_spent{s}, fist_level{s}, fist_tries{s}, club_level{s}, \
                   club_tries{s}, sword_level{s}, sword_tries{s}, axe_level{s}, axe_tries{s}, \
                   distance_level{s}, distance_tries{s}, shielding_level{s}, shielding_tries{s}, \
                   fishing_level{s}, fishing_tries{s}";
    let seed = "0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0";
    format!(
        "{advance}INSERT INTO game_character_build_receipts(\
           build_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, cause, level_before, \
           level_after, experience_before, experience_after, vocation_before, {before}, \
           vocation_after, {after}, stance_before, stance_after, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, 2, 3, \
           'vocation_choice', {LEVEL}, {LEVEL}, {EXPERIENCE}, {EXPERIENCE}, 'none', {seed}, \
           '{vocation}', {seed}, NULL, NULL, 'profile-1', 'ruleset-1', 'content-1', \
           'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 2); \
         INSERT INTO game_character_build_state(character_id, vocation, {plain}, \
           committed_character_revision, last_build_occurrence_id) \
         VALUES ({character}, '{vocation}', {seed}, 3, {occurrence});",
        advance = advance(2, LEVEL, EXPERIENCE),
        before = columns.replace("{s}", "_before"),
        after = columns.replace("{s}", "_after"),
        plain = columns.replace("{s}", ""),
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 33]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
    )
}

pub(in crate::gameplay_transport) async fn eligible(pool: &sqlx::PgPool) -> TestResult {
    for statement in [
        award(90, 1, (50, 1000), (LEVEL, EXPERIENCE)),
        choose_vocation(91, "sorcerer"),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(statement))
            .execute(pool)
            .await?;
    }
    Ok(())
}

use crate::durability::item_fee_burn::test_support::{item, request};
use crate::durability::npc_purchase::{NpcPurchaseRequest, PurchaseService};
use crate::foundation::{CommandId, CommandRef};
pub(in crate::gameplay_transport) async fn prepare(h: &Harness) -> TestResult<NpcPurchaseRequest> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "{}{}",
        item(210, "oteryn:item.tibia.i2854", "rev-1", 1),
        item(212, "oteryn:item.tibia.i3043", "rev-1", 2)
    )))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(214).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(212).as_slice()).bind(id(42).as_slice()).bind(id(215).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let fence = crate::bestiary_postgres_harness::fence(1)?;
    Ok(NpcPurchaseRequest {
        command: CommandRef::new(fence.game_session_id, CommandId::new(99).map_err(debug)?),
        occurrence: id(200),
        transaction_id: id(201),
        event_id: id(202),
        account: id(40),
        npc_key: "oteryn:npc.eremo".into(),
        npc_identity: id(203),
        npc_generation: 1,
        service: PurchaseService::Promotion("oteryn:service.promotion.eremo".into()),
        coins: request(1, 20_000)?.change,
        expected_backpack: Some(id(210)),
        occurred_at_unix_ms: 1_790_000_000_000,
    })
}

pub(super) struct MechanicContent;
impl crate::durability::item_transfer::ItemWeightLookup for MechanicContent {
    fn generation_digest(&self) -> [u8; 32] {
        [0; 32]
    }
    fn weight_of(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        Ok(1)
    }
}
impl crate::durability::item_loop_move::ItemLoopContent for MechanicContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        [0; 32]
    }
    fn loop_facts(
        &self,
        d: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
        use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
        if d.family != "Item" || d.revision_ref != "rev-1" {
            return None;
        }
        let bag = d.production_key == "oteryn:item.tibia.i2854";
        Some(ItemDefinitionFacts {
            definition: d.clone(),
            weight: Some(1),
            stack: if bag {
                ItemStackClass::NonStackable
            } else {
                ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                }
            },
            container_capacity: bag.then_some(20),
            container_slot_equip_pattern: bag,
        })
    }
}

pub(super) async fn paid_assets(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, count, ordinal) in [
        (220, "oteryn:item.tibia.i44608", 2, 2),
        (223, "oteryn:item.tibia.i44609", 2, 3),
        (226, "oteryn:item.tibia.i44610", 2, 4),
        (229, "oteryn:item.tibia.i46625", 100, 5),
        (232, "oteryn:item.tibia.i46626", 100, 6),
        (235, "oteryn:item.tibia.i3043", 100, 7),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("{} INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES({},{},{},{},{ordinal},{});",item(seed,key,"rev-1",count),uuid(seed),uuid(42),uuid(41),uuid(210),uuid(seed+2)))).execute(&mut *tx).await?;
    }
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({occ},{operation},{event},1,sha256('seed'),{character},{account},{world},{channel},1,1,0,1,sha256('seed'),0,{p},{g},{bag}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({entry},{operation},{account},{world},1,1000000000,0,1000000000,{character}); INSERT INTO game_account_bank_balances VALUES({account},{world},1000000000,{entry});",
        occ=uuid(240),operation=uuid(241),event=uuid(242),character=uuid(41),account=uuid(40),world=uuid(42),channel=uuid(43),p=uuid(243),g=uuid(244),bag=uuid(210),entry=uuid(245)
    ))).execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING")
        .execute(&h.pool)
        .await?;
    Ok(())
}

pub(in crate::gameplay_transport) async fn higher_level(h: &Harness) -> TestResult {
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(award(
        246,
        revision.parse()?,
        (LEVEL, EXPERIENCE),
        (2000, 6000),
    )))
    .execute(&h.pool)
    .await?;
    Ok(())
}

pub(super) async fn small_fragment_stacks(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, ordinal) in [(148, 100), (151, 101), (154, 102)] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("{} INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES({},{},{},{},{ordinal},{});",item(seed,"oteryn:item.tibia.i46625","rev-1",2),uuid(seed),uuid(42),uuid(41),uuid(210),uuid(seed+2)))).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
