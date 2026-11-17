//! Albinius's five-tome contribution through the ordinary NPC/item/quest owners.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const TRACK: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/tomes";
const TOME: &str = "oteryn:item.tibia.i23986";
const OTHER_BOOK: &str = "oteryn:item.tibia.i2848";

#[test]
fn questpc2_albinius_four_tomes_and_another_book_cannot_pay_the_contribution() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-four-tomes")
}

#[test]
fn questpc2_albinius_five_tomes_commit_with_progress_and_preserve_other_books() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-five-tomes")
}

#[test]
fn questpc2_albinius_combined_offer_and_confirmation_preserve_source_if_order() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-combined")
}

async fn carried(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.lifecycle=1")
        .bind(id(41).as_slice()).bind(key).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC2_ALBINIUS_POSTGRES_SERVER_VERSION={version} case={name}");
    let enough = name != "questpc2-albinius-four-tomes";
    let initial = if enough { 5 } else { 4 };
    // Fixture items use the existing fenced mint/transfer owners; no quest seed.
    let mut entries = vec![(TOME, 1); initial];
    entries.push((OTHER_BOOK, 1));
    let at = owner.runtime.lock().await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?).map_err(debug)?.position();
    let at = Position {
        x: u16::try_from(at.x)?, y: u16::try_from(at.y)?, floor: i8::try_from(at.floor)?,
    };
    for (index, (item, _)) in entries.iter().enumerate() {
        let fence = owner.current_quest_fence(admitted.game_session_id).await
            .map_err(|()| "fence")?.ok_or("fence")?;
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
            position: at,
            item: (*item).into(), count: 1,
        }, 9100 + u64::try_from(index)?, true).await?;
    }
    let [npc] = install(owner, admitted, ["oteryn:npc.albinius"]).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    talk(owner, admitted, npc, 33000, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 33001).await?;
    talk(owner, admitted, npc, 33002, "temple").await?;
    talk(owner, admitted, npc, 33003, "no").await?;
    talk(owner, admitted, npc, 33004, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 33005).await?;
    talk(owner, admitted, npc, 33006, "temple").await?;
    talk(owner, admitted, npc, 33007, "tomes").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 33008).await?;
    talk(owner, admitted, npc, 33009, "temple").await?;
    talk(owner, admitted, npc, 33010, "bye").await?;
    talk(owner, admitted, npc, 33011, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 33012).await?;
    assert_eq!(progress(owner, admitted, authority, TRACK).await?, 0);
    assert_eq!(carried(h, TOME).await?, i64::try_from(initial)?);
    let reply = if name == "questpc2-albinius-combined" {
        let result = talk(owner, admitted, npc, 33013, "temple yes").await?;
        assert_eq!(result.len(), 2, "both independent source if replies must be retained");
        result
    } else {
        talk(owner, admitted, npc, 33013, "temple").await?;
        talk(owner, admitted, npc, 33014, "yes").await?
    };
    if enough {
        assert!(reply.iter().any(|s| s.contains("Your first step in the ways of the {Shapers}")), "missing source contribution: {reply:?}");
        assert_eq!(progress(owner, admitted, authority, TRACK).await?, 1);
        assert_eq!(carried(h, TOME).await?, 0);
        let active: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(TOME).fetch_one(&h.pool).await?;
        assert_eq!(active, 0, "all five exact source tomes must be consumed");
        // Source success keeps topic 1; the next yes gives the already-donated reply.
        let repeated = talk(owner, admitted, npc, 33015, "yes").await?;
        assert!(repeated.iter().any(|s| s.starts_with("You already offered enough tomes")));
        super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 33016).await?;
    } else {
        assert!(reply.iter().any(|s| s == "You need 5 heavy old tome."));
        let repeated = talk(owner, admitted, npc, 33015, "yes").await?;
        assert!(repeated.iter().any(|s| s == "You need 5 heavy old tome."));
        assert_eq!(progress(owner, admitted, authority, TRACK).await?, 0);
        assert_eq!(carried(h, TOME).await?, 4);
    }
    assert_eq!(carried(h, OTHER_BOOK).await?, 1, "another book is never payment");
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(after - before, i64::from(enough), "only one exact paid contribution may earn a receipt");
    Ok(())
}
