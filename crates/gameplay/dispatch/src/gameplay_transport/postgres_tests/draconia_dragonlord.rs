//! Source greet callback: mushroom debit, fetish gift and exact flag share one commit.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

const NPC: &str = "oteryn:npc.an_old_dragonlord";
const TRACK: &str = "oteryn:quest-progress/dragonfetish";
const RULE: &str = "oteryn:quest-npc/draconia_quest/an_old_dragonlord/exchange";
#[test]
fn draconia_dragonlord_missing_mushroom_has_no_side_effects() -> TestResult {
    scenario("draconia-dragonlord-missing")
}
#[test]
fn draconia_dragonlord_exchange_replay_and_exact_flag_refusal() -> TestResult {
    scenario("draconia-dragonlord-exchange")
}
#[test]
fn draconia_dragonlord_final_track_failure_rolls_back_items() -> TestResult {
    scenario("draconia-dragonlord-rollback")
}
fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    println!(
        "POSTGRES_CASE_{} {name} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT COALESCE(sum(quantity) FILTER(WHERE definition_production_key='oteryn:item.tibia.i3723'),0)::bigint,COALESCE(sum(quantity) FILTER(WHERE definition_production_key='oteryn:item.tibia.i3206'),0)::bigint FROM game_item_instances WHERE lifecycle=1").fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert!(!state.tracks().contains_key(TRACK));
    if name.ends_with("missing") {
        let before = counts(h).await?;
        let reply = super::classic_quests::talk(owner, admitted, npc, 3300, "hi").await?;
        assert!(
            reply
                .iter()
                .any(|line| line.contains("BRRRING ME MUSHRRROOOMSSS")),
            "source missing-mushroom greet"
        );
        assert_eq!(counts(h).await?, before);
        assert!(
            !owner
                .root
                .read_character_quest_state(authority, fence.character_id)
                .await
                .map_err(debug)?
                .tracks()
                .contains_key(TRACK)
        );
        return Ok(());
    }
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i3723".into(),
            count: 2,
        },
        450,
        true,
    )
    .await?;
    let before = counts(h).await?;
    assert_eq!(before.0, 2);
    if name.ends_with("rollback") {
        sqlx::query("CREATE SEQUENCE fixture_dragonlord_failure_seen")
            .execute(&h.pool)
            .await?;
        sqlx::query("GRANT USAGE, SELECT ON SEQUENCE fixture_dragonlord_failure_seen TO oteryn_game_runtime").execute(&h.pool).await?;
        sqlx::query("CREATE FUNCTION fixture_dragonlord_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.track_key='oteryn:quest-progress/dragonfetish' THEN PERFORM nextval('fixture_dragonlord_failure_seen'); RAISE EXCEPTION 'injected final Dragonlord flag failure'; END IF; RETURN NEW; END $$").execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER fixture_dragonlord_fail BEFORE INSERT OR UPDATE ON game_character_quest_tracks FOR EACH ROW EXECUTE FUNCTION fixture_dragonlord_fail()").execute(&h.pool).await?;
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let failed_command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 3300,
            item_fence: Some(current.item_fence()),
        };
        let failed = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                failed_command,
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "hi".into(),
                },
            )
            .await;
        assert!(
            failed.unknown,
            "injected final write failure must not acknowledge an exchange"
        );
        let seen: bool =
            sqlx::query_scalar("SELECT is_called FROM fixture_dragonlord_failure_seen")
                .fetch_one(&h.pool)
                .await?;
        assert!(seen, "real final flag write reached");
        assert_eq!(counts(h).await?, before);
        assert!(
            !owner
                .root
                .read_character_quest_state(authority, fence.character_id)
                .await
                .map_err(debug)?
                .tracks()
                .contains_key(TRACK)
        );
        sqlx::query("DROP TRIGGER fixture_dragonlord_fail ON game_character_quest_tracks")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP FUNCTION fixture_dragonlord_fail()")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP SEQUENCE fixture_dragonlord_failure_seen")
            .execute(&h.pool)
            .await?;
        let recovered = owner
            .resume_npc_bank(admitted.runtime_actor.ok_or("actor")?, failed_command)
            .await
            .ok_or("retained failed exchange")?;
        assert!(!recovered.unknown);
        assert_eq!(recovered.disposition, NpcIntentDisposition::Rejected);
    }
    let reply = super::classic_quests::talk(owner, admitted, npc, 3301, "hi").await?;
    assert!(
        reply
            .iter()
            .any(|line| line.contains("NOW MY PAIN WILL BE EASSSED")),
        "source successful exchange greet"
    );
    assert_eq!(counts(h).await?, (before.0 - 1, before.1 + 1));
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(TRACK), Some(&1));
    let replay = owner
        .root
        .reconcile_npc_program(
            authority,
            fence.character_id,
            crate::durability::quest_state::npc_program::NpcProgramRequest {
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(3301).map_err(debug)?,
                ),
                npc: NPC.into(),
                trigger: oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger::Greet,
                message: "hi".into(),
                topic: 0,
                transition_key: RULE.into(),
            },
        )
        .await
        .map_err(debug)?
        .ok_or("exchange receipt")?;
    assert_eq!(replay.reply, reply);
    assert_eq!(counts(h).await?, (before.0 - 1, before.1 + 1));
    owner.admit_quest_session(admitted).await;
    let refused = super::classic_quests::talk(owner, admitted, npc, 3302, "hi").await?;
    assert!(
        refused
            .iter()
            .any(|line| line.contains("LEAVE THE DRAGONS' CEMETERY")),
        "exact flag1 refuses another debit/gift"
    );
    assert_eq!(counts(h).await?, (before.0 - 1, before.1 + 1));
    println!(
        "DRACONIA_DRAGONLORD_ATOMIC source_greet=true debit=1 gift=1 flag=1 replay=true exact_flag_refusal=true"
    );
    Ok(())
}
