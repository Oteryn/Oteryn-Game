//! Rathleton citizen ranks gate Marvin's voting topics (TibiaWiki Rathleton_Quest/Spoiler oldid 1141625).
use super::*;

const PROGRAM: &str =
    include_str!("../../../../../../content/dialogues/quest-rules/quest-authored-rathleton.json");
const RANK: &str = "oteryn:quest-progress/chosen/rathleton-quest/rank";
const VOTES: &str = "oteryn:quest-progress/chosen/rathleton-quest/votes_cast";

fn plan(
    message: &str,
    topic: i32,
    tracks: &BTreeMap<String, i64>,
) -> Result<Option<NpcRulePlan>, NpcRuleError> {
    let catalog = NpcRuleCatalog::from_json_documents(&[PROGRAM])?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    catalog.plan_storage(&NpcRuleInput {
        player_name: None,
        npc: "oteryn:npc.marvin",
        trigger: NpcRuleTrigger::Say,
        message,
        topic,
        tracks,
        inventory: &inventory,
        now_epoch_seconds: 0,
        calendar_month: 1,
        level: 150,
        vocation: "",
        premium_current: true,
        sex: 0,
        outfits: &outfits,
    })
}

#[test]
fn rathleton_rank_gates_voting_topics_below_and_admits_at_rank() -> Result<(), NpcRuleError> {
    for (category, rank, topic) in [("dungeon", 1, 20), ("boss", 2, 30), ("raid", 3, 40)] {
        let below = BTreeMap::from([(RANK.to_owned(), rank - 1)]);
        assert_eq!(plan(category, 1, &below)?, None, "{category} below rank");
        let at = BTreeMap::from([(RANK.to_owned(), rank)]);
        assert_eq!(plan(category, 1, &at)?.map(|p| p.next_topic), Some(topic));
    }
    let none = BTreeMap::new();
    assert_eq!(plan("shortcut", 1, &none)?.map(|p| p.next_topic), Some(10));
    // A forged topic cannot bypass the gate at the confirmation step either.
    assert_eq!(plan("yes", 121, &none)?, None);
    let cast = plan("yes", 121, &BTreeMap::from([(RANK.to_owned(), 1)]))?
        .ok_or(NpcRuleError::Invalid("dungeon cast".into()))?;
    assert_eq!(
        cast.selected_rule_keys,
        ["oteryn:quest-npc/authored-rathleton/marvin/dungeon/golem/cast"]
    );
    assert!(
        cast.track_changes
            .iter()
            .any(|c| c.key == VOTES && c.before == 0 && c.after == 1)
    );
    Ok(())
}

#[test]
fn rathleton_promotion_needs_lifetime_votes_and_grants_the_rank_achievement()
-> Result<(), NpcRuleError> {
    for (rank, threshold, achievement) in [
        (0, 50, "Rathleton Commoner"),
        (1, 150, "Rathleton Inhabitant"),
        (2, 300, "Rathleton Citizen"),
        (3, 500, "Rathleton Squire"),
    ] {
        let short = BTreeMap::from([(RANK.to_owned(), rank), (VOTES.to_owned(), threshold - 1)]);
        assert_eq!(plan("promotion", 0, &short)?, None);
        let enough = BTreeMap::from([(RANK.to_owned(), rank), (VOTES.to_owned(), threshold)]);
        let promoted =
            plan("promotion", 0, &enough)?.ok_or(NpcRuleError::Invalid("promotion".into()))?;
        assert!(
            promoted
                .track_changes
                .iter()
                .any(|c| c.key == RANK && c.after == rank + 1)
        );
        assert!(
            promoted
                .effects
                .iter()
                .any(|e| matches!(e, NpcRuleEffect::Achievement { name } if name == achievement))
        );
    }
    let squire = BTreeMap::from([(RANK.to_owned(), 4), (VOTES.to_owned(), 9999)]);
    assert_eq!(plan("promotion", 0, &squire)?, None);
    Ok(())
}
