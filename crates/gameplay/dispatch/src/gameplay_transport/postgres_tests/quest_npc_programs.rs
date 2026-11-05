//! The real NPC Talk caller must execute the captured program's topic branch.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

#[test]
fn captured_npc_program_drives_real_topics_and_refuses_unowned_effects_whole() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quest-npc-programs", || {
        super::quest_world_callbacks::scenario(
            "quest-npc-programs",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    _h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [tereban, demeron] = super::classic_quests::install(
        owner,
        admitted,
        ["oteryn:npc.tereban", "oteryn:npc.second_in_command_demeron"],
    )
    .await?;
    super::classic_quests::talk(owner, admitted, tereban, 9100, "hi").await?;
    let reply = super::classic_quests::talk(owner, admitted, tereban, 9101, "mission").await?;
    assert!(
        reply
            .iter()
            .any(|line| line.starts_with("I have four sons")),
        "source mission offer"
    );
    let before = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 9102,
                item_fence: Some(before.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: tereban,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(result.disposition, NpcIntentDisposition::Rejected);
    assert!(!result.unknown);
    assert!(
        result.conversation.is_none(),
        "no source success text before complete admission"
    );
    let after = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(before, after, "refusal advances no Character revision");
    let copy = owner
        .root
        .read_character_quest_state(authority, after.character_id)
        .await
        .map_err(debug)?;
    assert!(
        !copy
            .tracks()
            .contains_key("oteryn:quest-progress/quest/u8_6/afathers_burden/status")
    );
    // The failed confirmation retains the offer topic, so a later source 'no'
    // still follows its exact branch rather than an unrelated ordinary reply.
    let reply = super::classic_quests::talk(owner, admitted, tereban, 9103, "no").await?;
    assert!(
        reply.iter().any(|line| line.contains("change your mind")),
        "source refusal follows retained topic: {reply:?}"
    );
    super::classic_quests::talk(owner, admitted, demeron, 9110, "hi").await?;
    super::classic_quests::talk(owner, admitted, demeron, 9111, "spares").await?;
    let track = "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/demeron";
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, track).await?,
        1,
        "the actual NPC Talk caller must commit the complete bounded storage branch"
    );
    Ok(())
}
