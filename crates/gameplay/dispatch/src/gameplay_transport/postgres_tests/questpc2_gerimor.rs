//! Gerimor's seven initial assignments; no boss completion or reward seeds.
use super::classic_quests::{install, progress, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const PREFIX: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/";
const CULTS: [(&str, &str); 7] = [
    ("minotaurs", "minotaurs"),
    ("prosperity", "mot_a"),
    ("barkless", "barkless"),
    ("orcs", "orcs"),
    ("life", "life"),
    ("misguided", "misguided"),
    ("humans", "humans"),
];

#[test]
fn questpc2_gerimor_seven_assignments_require_exact_selection_and_live_confirmation() -> TestResult {
    super::qengine_rewards::scenario("questpc2-gerimor-assignments")
}

async fn reject_nonexact_selection(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
) -> TestResult {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    let outcome = owner.npc_talk(admitted.runtime_actor.ok_or("actor")?, UseCommand {
        game_session_id: admitted.game_session_id, command_id: 32003,
        item_fence: Some(fence.item_fence()),
    }, NpcTalkIntent { npc_actor: npc, text: "prefix minotaurs".into() }).await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(!outcome.conversation.into_iter().flat_map(|c| c.lines)
        .any(|line| line.contains("animal-like cult")), "a substring cannot select the cult");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: String = sqlx::query_scalar("SHOW server_version_num").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC2_GERIMOR_POSTGRES_SERVER_VERSION={version}");
    let [npc] = install(owner, admitted, ["oteryn:npc.gerimor"]).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    talk(owner, admitted, npc, 32000, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 32001).await?;
    let menu = talk(owner, admitted, npc, 32002, "mission").await?;
    assert!(menu.iter().any(|s| s.contains("Cult of {Life}")), "missing source cult menu: {menu:?}");
    // The source indexes a table by the entire lowercased message, not MsgContains.
    reject_nonexact_selection(owner, admitted, npc).await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 32004).await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}minotaurs/mission")).await?, 0);
    let offer = talk(owner, admitted, npc, 32005, "MiNoTaUrS").await?;
    assert_eq!(offer.len(), 2);
    let declined = talk(owner, admitted, npc, 32006, "no").await?;
    assert!(declined.iter().any(|s| s.contains("What a pitty!")));
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 32007).await?;
    talk(owner, admitted, npc, 32008, "mission").await?;
    talk(owner, admitted, npc, 32009, "minotaurs").await?;
    talk(owner, admitted, npc, 32010, "bye").await?;
    talk(owner, admitted, npc, 32011, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 32012).await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 0);
    for (index, (word, _)) in CULTS.iter().enumerate() {
        let command = 32100 + u64::try_from(index)? * 10;
        talk(owner, admitted, npc, command, "mission").await?;
        talk(owner, admitted, npc, command + 1, word).await?;
        let accepted = talk(owner, admitted, npc, command + 2, "yes").await?;
        assert!(accepted.iter().any(|s| s.starts_with("Very nice!")), "cult {word}: {accepted:?}");
        for (other_index, (_, track)) in CULTS.iter().enumerate() {
            assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}{track}/mission")).await?,
                       i64::from(other_index <= index), "only confirmed cults may progress");
        }
        assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 1);
        super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, command + 3).await?;
    }
    talk(owner, admitted, npc, 32200, "mission").await?;
    talk(owner, admitted, npc, 32201, "minotaurs").await?;
    let pending = talk(owner, admitted, npc, 32202, "yes").await?;
    assert!(pending.iter().any(|s| s.starts_with("You have not finished your work yet.")));
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}final_boss/mission")).await?, 0);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(after - before, 7, "one atomic receipt per earned assignment");
    eprintln!("QUESTPC2_GERIMOR_POSTGRES_COMPLETED duration={:?}", started.elapsed());
    Ok(())
}
