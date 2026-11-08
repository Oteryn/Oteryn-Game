//! Maeryn's source helmet preference; no helmet or boss credit is invented.
use super::classic_quests::{install, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;

const TRACK: &str = "oteryn:quest-progress/quest/u10_80/grimvale_quest/were_helmet_enchant";

#[test]
fn questpc2_maeryn_helmet_preferences_commit_zero_and_all_knight_choices() -> TestResult {
    super::qengine_rewards::scenario("questpc2-maeryn-helmet")
}

async fn choice(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<Option<i64>> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let copy = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(super::fixture::debug)?;
    Ok(copy.tracks().get(TRACK).copied())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC2_MAERYN_POSTGRES_SERVER_VERSION={version}");
    let [npc] = install(owner, admitted, ["oteryn:npc.maeryn"]).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    talk(owner, admitted, npc, 36000, "hi").await?;
    talk(owner, admitted, npc, 36001, "yes").await?;
    talk(owner, admitted, npc, 36002, "axe").await?;
    assert_eq!(choice(owner, admitted, authority).await?, None);
    let offer = talk(owner, admitted, npc, 36003, "werewolf helmet yes").await?;
    assert!(
        offer
            .iter()
            .any(|s| s
                == "You brought the wolven helmet, as i see. Do you want to change something?"),
        "missing source helmet offer: {offer:?}"
    );
    // Source elseif offers only; a compound keyword cannot also confirm.
    talk(owner, admitted, npc, 36004, "sorcerer").await?;
    assert_eq!(choice(owner, admitted, authority).await?, None);
    talk(owner, admitted, npc, 36005, "yes time").await?;
    let menu = talk(owner, admitted, npc, 36006, "yes").await?;
    assert!(
        menu.iter().any(|s| s.starts_with("So, which profession")),
        "earlier informational branch must not change topic: {menu:?}"
    );
    talk(owner, admitted, npc, 36007, "sorcerer yes").await?;
    assert_eq!(choice(owner, admitted, authority).await?, None);
    let zero = talk(owner, admitted, npc, 36008, "sorcerer").await?;
    assert!(
        zero.iter()
            .any(|s| s.starts_with("So this is your choice."))
    );
    assert_eq!(
        choice(owner, admitted, authority).await?,
        Some(0),
        "zero is an earned sorcerer choice, not an absent flag"
    );
    for (index, (profession, weapon, value)) in [
        ("druid", None, 1),
        ("paladin", None, 2),
        ("knight", Some("club"), 3),
        ("knight", Some("axe"), 4),
        ("knight", Some("sword"), 5),
    ]
    .into_iter()
    .enumerate()
    {
        let base = 36100 + u64::try_from(index)? * 10;
        talk(owner, admitted, npc, base, "werewolf helmet").await?;
        talk(owner, admitted, npc, base + 1, "no").await?; // Source does not cancel this offer.
        talk(owner, admitted, npc, base + 2, "yes").await?;
        let selected = talk(owner, admitted, npc, base + 3, profession).await?;
        if let Some(weapon) = weapon {
            assert!(
                selected
                    .iter()
                    .any(|s| s.starts_with("And what would be your preferred weapon?"))
            );
            assert_eq!(choice(owner, admitted, authority).await?, Some(value - 1));
            talk(owner, admitted, npc, base + 4, "axe sword").await?;
            assert_eq!(choice(owner, admitted, authority).await?, Some(value - 1));
            talk(owner, admitted, npc, base + 5, weapon).await?;
        }
        assert_eq!(choice(owner, admitted, authority).await?, Some(value));
        talk(owner, admitted, npc, base + 6, "yes").await?;
        assert_eq!(choice(owner, admitted, authority).await?, Some(value));
    }
    talk(owner, admitted, npc, 36200, "werewolf helmet").await?;
    talk(owner, admitted, npc, 36201, "yes").await?;
    talk(owner, admitted, npc, 36202, "knight").await?;
    talk(owner, admitted, npc, 36203, "bye").await?;
    talk(owner, admitted, npc, 36204, "hi").await?;
    talk(owner, admitted, npc, 36205, "club").await?;
    assert_eq!(choice(owner, admitted, authority).await?, Some(5));
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after - before,
        6,
        "only six exact live selections commit progress"
    );
    Ok(())
}
