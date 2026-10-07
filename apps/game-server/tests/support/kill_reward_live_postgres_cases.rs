// KILL-REWARD-COMP-1 Part B (CP D929 option b): the real rat reward row,
// settled live. The rat's Creature profile (XP, corpse `i5964`) and the
// admission facts are read from the production pin (`spell-native-profiles`
// and the `loot_tables` section); the settled table is the rat's pinned
// table narrowed to its admitted entries (gold `i3031`). The real table also
// names cheese `i3607`, which is not materializable, so the content row for
// the rat refuses with `loot_item_inadmissible`
// (`content::creature_reward` tests); its admission is
// OTV2-20261007-d3-8-cheese.
use crate::combat::{
    CreatureDeathRewardInput, DurabilitySession, LootDefinitionRef, LootSelectionAlgorithm,
    LootTableDefinition, LootTableEntry, settle_creature_death_rewards,
};
use crate::combat_death_reward_postgres_cases::{
    Harness, TestResult, capture, configured_admin, death_fixture, debug, ground, id,
    progression_binding, reward_principal, runtime, uuid_text,
};
use crate::domain::CharacterId;
use crate::durability::character_progression::ExperienceCommitOutcome;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::foundation::CombatDeathFixture;
use oteryn_simulation_determinism::ExactI64;
use serde_json::Value;

const RAT: &str = "oteryn:creature.rat";
const GOLD: &str = "oteryn:item.tibia.i3031";
const CHEESE: &str = "oteryn:item.tibia.i3607";

fn pinned(path: &str) -> TestResult<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn reference(value: &Value) -> TestResult<LootDefinitionRef> {
    let field = |name: &str| {
        value[name]
            .as_str()
            .ok_or_else(|| format!("reference without {name}"))
    };
    Ok(LootDefinitionRef::new(
        field("family")?,
        field("key")?,
        field("revision")?,
    ))
}

/// The rat's pinned reward facts, as the content builder reads them.
struct RatPin {
    xp: i64,
    corpse: LootDefinitionRef,
    table_ref: LootDefinitionRef,
    admitted: LootTableDefinition,
    cheese_refused: bool,
}

fn rat_pin() -> TestResult<RatPin> {
    let profiles = pinned("content/creatures/definitions/spell-native-profiles.json")?;
    let profile = profiles["records"]
        .as_array()
        .ok_or("profile records")?
        .iter()
        .find(|record| record["profile"]["target"]["key"] == RAT)
        .ok_or("rat profile pinned")?;
    let data = &profile["profile"]["data"]["profile"];
    let xp = data["experience"].as_i64().ok_or("rat experience")?;
    let corpse = &data["details"]["corpse_item"];

    let section = pinned("tools/content-schema/native-gameplay/loot-tables.json")?;
    let binding = section["creature_loot"]
        .as_array()
        .ok_or("creature_loot")?
        .iter()
        .find(|row| row["creature"]["key"] == RAT)
        .ok_or("rat binding pinned")?;
    let table = section["tables"]
        .as_array()
        .ok_or("tables")?
        .iter()
        .find(|table| table["identity"] == binding["loot"])
        .ok_or("rat table pinned")?;
    let facts = |item: &Value| {
        section["items"]
            .as_array()
            .and_then(|items| items.iter().find(|facts| &facts["item"] == item))
    };
    let corpse_facts = facts(corpse).ok_or("corpse admission facts")?;
    assert_eq!(corpse_facts["materializable"], true);
    assert_eq!(corpse_facts["container_capacity"], 16);

    let mut entries = Vec::new();
    let mut cheese_refused = false;
    for entry in table["entries"].as_array().ok_or("entries")? {
        let item_facts = facts(&entry["item"]).ok_or("entry admission facts")?;
        if item_facts["materializable"] != true {
            cheese_refused |= entry["item"]["key"] == CHEESE;
            continue;
        }
        let count = |name: &str| -> TestResult<u32> {
            Ok(u32::try_from(entry[name].as_u64().ok_or("count")?)?)
        };
        entries.push(LootTableEntry {
            item: reference(&entry["item"])?,
            min_count: count("min_count")?,
            max_count: count("max_count")?,
            // A guaranteed draw keeps the minted plan exact.
            probability_ppm: Some(1_000_000),
        });
    }
    assert_eq!(table["algorithm"], "IndependentBernoulliPpm");
    Ok(RatPin {
        xp,
        corpse: reference(corpse)?,
        table_ref: reference(&binding["loot"])?,
        admitted: LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries,
        },
        cheese_refused,
    })
}

/// The real rat (corpse `i5964`, XP 5) dies once: its corpse holds the
/// admitted gold stack and its killer gains the rat's XP exactly once.
#[test]
fn the_pinned_rat_row_settles_its_corpse_gold_and_xp() -> TestResult {
    let pin = rat_pin()?;
    assert!(
        pin.cheese_refused,
        "the pinned rat table still names cheese"
    );
    assert_eq!(pin.xp, 5);
    assert_eq!(pin.corpse.production_key, "oteryn:item.tibia.i5964");
    assert_eq!(pin.admitted.entries.len(), 1);
    assert_eq!(pin.admitted.entries[0].item.production_key, GOLD);
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "liverat").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        let mut fixture = death_fixture()?;
        fixture
            .strike_by(
                "fixture:reward.strike.lethal",
                CombatDeathFixture::HEALTH,
                crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
            )
            .map_err(debug)?;
        fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();
        let mut slot = CharacterRevisionSequencer::new()
            .acquire(CharacterId::from_bytes(id(41)).map_err(debug)?)
            .await;
        let input = CreatureDeathRewardInput {
            corpse_item: pin.corpse,
            loot_table_ref: pin.table_ref,
            loot_table: pin.admitted,
            ground: ground(),
            inflight_loot_mints_before_this_death: 0,
            reward_principals: vec![reward_principal(1, 1)?],
            xp_amount: ExactI64::new(pin.xp),
            progression: Some(progression_binding()),
        };
        let outcome = settle_creature_death_rewards(
            capture(&mut fixture, actor)?,
            &session,
            &mut slot,
            input,
        )
        .await
        .map_err(debug)?;
        let minted = outcome.loot.map_err(debug)?;
        assert_eq!(minted.entries.len(), 1);
        let ExperienceCommitOutcome::Committed(award) = outcome.xp.map_err(debug)? else {
            return Err("the rat's XP award must be freshly committed".into());
        };
        assert_eq!(award.experience_before.get(), 0);
        assert_eq!(award.experience_after.get(), pin.xp);
        assert_eq!(harness.ground_items().await?, 1);
        assert_eq!(harness.corpse_entries().await?, 1);
        let item_key: String = sqlx::query_scalar(
            "SELECT definition_production_key FROM game_item_instances \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(minted.entries[0].item_instance_id.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(item_key, GOLD);
        let (winner, materialized) = harness
            .corpse_receipt(minted.corpse.item_instance_id)
            .await?;
        assert_eq!(winner, uuid_text(id(41)));
        assert!(materialized);
        assert_eq!(harness.count("game_character_xp_receipts").await?, 1);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
