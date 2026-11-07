//! Hidden Threats source dialogue uses the production NPC and durable quest owners.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const LINE: &str = "oteryn:quest-progress/quest/u11_50/hidden_threats/quest_line";
const COUNTER: &str = "oteryn:quest-progress/quest/u11_50/hidden_threats/corym_rescue_mission";

#[test]
fn hidden_threats_source_npc_acceptance_initializes_mission_once() -> TestResult {
    scenario(0)
}
#[test]
fn hidden_threats_source_npc_eight_rescues_grant_each_reward_once() -> TestResult {
    scenario(1)
}
#[test]
fn hidden_threats_source_npc_rare_earth_exchange_is_atomic_and_replays() -> TestResult {
    scenario(2)
}

fn scenario(phase: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("hidden-threats-npcs", move || {
        super::quest_source_objects::hidden_npc_scenario(phase)
    });
    println!(
        "POSTGRES_CASE_{} hidden_threats_source_npcs phase={phase} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}

const NPCS: [&str; 10] = [
    "oteryn:npc.corym_ratter",
    "oteryn:npc.corym_servant",
    "oteryn:npc.corym_worker_1",
    "oteryn:npc.corym_slave",
    "oteryn:npc.corym_worker_2",
    "oteryn:npc.corym_worker_3",
    "oteryn:npc.corym_butler",
    "oteryn:npc.corym_worker_4",
    "oteryn:npc.corym_worker_5",
    "oteryn:npc.corym_footman",
];

pub(super) struct Dialogue {
    command: u64,
    npcs: [EntityRef; 10],
}
impl Dialogue {
    async fn npc(
        &mut self,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        admitted: &AdmittedSession,
        key: &str,
    ) -> TestResult<EntityRef> {
        let index = NPCS
            .iter()
            .position(|candidate| *candidate == key)
            .ok_or("source NPC")?;
        let npc = self.npcs[index];
        super::quest_source_objects::fixture_relocate(
            owner,
            admitted,
            admitted.runtime_actor.ok_or("actor")?,
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33100 + u16::try_from(index)?,
                y: 32000,
                floor: -13,
            },
        )
        .await?;
        self.say(owner, admitted, npc, "hi").await?;
        Ok(npc)
    }
    async fn say(
        &mut self,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        admitted: &AdmittedSession,
        npc: EntityRef,
        message: &str,
    ) -> TestResult<Vec<String>> {
        let command = self.command;
        self.command += 1;
        super::classic_quests::talk(owner, admitted, npc, command, message).await
    }
}

pub(super) async fn start(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<Dialogue> {
    super::quest_source_objects::fixture_relocate(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        oteryn_gameplay_quests::quest::world_callbacks::Position {
            x: 33100,
            y: 32000,
            floor: -13,
        },
    )
    .await?;
    let npcs = super::classic_quests::install(owner, admitted, NPCS).await?;
    let mut dialogue = Dialogue {
        command: 4000,
        npcs,
    };
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(version, 180006);
    println!("POSTGRES_CASE_STARTED hidden_threats_source_npc_acceptance version={version}");
    let ratter = dialogue
        .npc(owner, admitted, "oteryn:npc.corym_ratter")
        .await?;
    dialogue.say(owner, admitted, ratter, "help").await?;
    dialogue.say(owner, admitted, ratter, "yes").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, LINE).await?,
        1,
        "Ratter's accepted help must commit the source quest start"
    );
    for track in [
        "oteryn:quest-progress/quest/u8_1/tibia_tales/default_start",
        "oteryn:quest-progress/quest/u11_50/hidden_threats/ratter_door",
    ] {
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, track).await?,
            1
        );
    }
    let servant = dialogue
        .npc(owner, admitted, "oteryn:npc.corym_servant")
        .await?;
    dialogue
        .say(owner, admitted, servant, "decreasing resources")
        .await?;
    dialogue.say(owner, admitted, servant, "defy").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, LINE).await?,
        2
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, COUNTER).await?,
        0
    );
    assert_eq!(
        super::classic_quests::progress(
            owner,
            admitted,
            authority,
            "oteryn:quest-progress/quest/u11_50/hidden_threats/servant_door"
        )
        .await?,
        1
    );
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
    assert_eq!(
        state.tracks().get(COUNTER),
        Some(&0),
        "counter must be written, not merely absent"
    );
    for command in [4002_i64, 4005] {
        let count: i32 = sqlx::query_scalar("SELECT cardinality(track_keys) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_ordinal=$2::bigint::numeric")
            .bind(fence.character_id.as_bytes().as_slice()).bind(command).fetch_one(&h.pool).await?;
        assert_eq!(
            count, 3,
            "each source start group is one three-track receipt"
        );
    }
    Ok(dialogue)
}

pub(super) async fn rescues(
    dialogue: &mut Dialogue,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let before = [
        count(h, 3030).await?,
        count(h, 3032).await?,
        count(h, 3029).await?,
    ];
    for (index, (npc, keyword)) in [
        ("corym_worker_1", None),
        ("corym_slave", Some("pay back")),
        ("corym_worker_2", Some("repression")),
        ("corym_worker_3", None),
        ("corym_butler", Some("repression")),
        ("corym_worker_4", None),
        ("corym_worker_5", None),
        ("corym_footman", Some("hunger")),
    ]
    .into_iter()
    .enumerate()
    {
        let npc = dialogue
            .npc(owner, admitted, &format!("oteryn:npc.{npc}"))
            .await?;
        if let Some(keyword) = keyword {
            dialogue.say(owner, admitted, npc, keyword).await?;
        }
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, COUNTER).await?,
            (index + 1) as i64
        );
        let flag = format!(
            "oteryn:quest-progress/quest/u11_50/hidden_threats/corym_rescued{:02}",
            index + 1
        );
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, &flag).await?,
            1
        );
        dialogue.say(owner, admitted, npc, "hi").await?;
        if let Some(keyword) = keyword {
            dialogue.say(owner, admitted, npc, keyword).await?;
        }
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, COUNTER).await?,
            (index + 1) as i64,
            "repeat rescue cannot increment counter"
        );
    }
    assert_eq!(
        [
            count(h, 3030).await?,
            count(h, 3032).await?,
            count(h, 3029).await?
        ],
        before.map(|count| count + 1)
    );
    dialogue
        .npc(owner, admitted, "oteryn:npc.corym_servant")
        .await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, LINE).await?,
        4
    );
    Ok(())
}

async fn count(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_family='Item' AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

pub(super) async fn delivery(
    dialogue: &mut Dialogue,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let before = count(h, 3040).await?;
    let servant = dialogue
        .npc(owner, admitted, "oteryn:npc.corym_servant")
        .await?;
    let missing = dialogue.say(owner, admitted, servant, "yes").await?;
    assert!(
        missing
            .iter()
            .any(|line| line.contains("don't have enough"))
    );
    assert!(
        owner
            .states
            .chat
            .engaged(admitted.game_session_id)
            .is_none(),
        "source failure ends interaction"
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, LINE).await?,
        4
    );
    for (quantity, instance) in [(19, 610), (1, 611)] {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 33079,
                    y: 32014,
                    floor: -13,
                },
                item: "oteryn:item.tibia.i27301".into(),
                count: quantity,
            },
            instance,
            true,
        )
        .await?;
        if quantity == 19 {
            let npc = dialogue
                .npc(owner, admitted, "oteryn:npc.corym_servant")
                .await?;
            dialogue.say(owner, admitted, npc, "yes").await?;
            assert_eq!(count(h, 27301).await?, 19);
            assert_eq!(count(h, 3040).await?, before);
            assert_eq!(
                super::classic_quests::progress(owner, admitted, authority, LINE).await?,
                4
            );
        }
    }
    let servant = dialogue
        .npc(owner, admitted, "oteryn:npc.corym_servant")
        .await?;
    rollback_after_values(dialogue, owner, admitted, h, authority, servant, before).await?;
    let servant = dialogue
        .npc(owner, admitted, "oteryn:npc.corym_servant")
        .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let command = dialogue.command;
    let delivered = dialogue.say(owner, admitted, servant, "yes").await?;
    assert!(
        delivered
            .iter()
            .any(|line| line.contains("Thank you very much"))
    );
    assert_eq!(count(h, 27301).await?, 0);
    assert_eq!(count(h, 3040).await?, before + 2);
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, LINE).await?,
        5
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, COUNTER).await?,
        9
    );
    let replay = owner
        .root
        .reconcile_npc_program(
            authority,
            fence.character_id,
            crate::durability::quest_state::npc_program::NpcProgramRequest {
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(command).map_err(debug)?,
                ),
                npc: "oteryn:npc.corym_servant".into(),
                trigger: oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger::Say,
                message: "yes".into(),
                topic: 0,
                transition_key: "oteryn:quest-npc/tibia_tales/corym_servant/rare-earth".into(),
            },
        )
        .await
        .map_err(debug)?
        .ok_or("NPC replay")?;
    assert_eq!(replay.reply, delivered);
    assert!(replay.inventory_changed && !replay.end_conversation);
    dialogue.say(owner, admitted, servant, "yes").await?;
    assert_eq!(count(h, 3040).await?, before + 2);
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "one controlled source NPC rollback case"
)]
async fn rollback_after_values(
    dialogue: &mut Dialogue,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    servant: EntityRef,
    gems: i64,
) -> TestResult {
    sqlx::query("CREATE SEQUENCE fixture_corym_failure_seen")
        .execute(&h.pool)
        .await?;
    sqlx::query("GRANT USAGE,SELECT ON SEQUENCE fixture_corym_failure_seen TO oteryn_game_runtime")
        .execute(&h.pool)
        .await?;
    sqlx::query("CREATE FUNCTION fixture_corym_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.track_key='oteryn:quest-progress/quest/u11_50/hidden_threats/quest_line' AND NEW.value=5 THEN PERFORM nextval('fixture_corym_failure_seen'); RAISE EXCEPTION 'injected final Corym track failure'; END IF; RETURN NEW; END $$").execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER fixture_corym_fail BEFORE INSERT OR UPDATE ON game_character_quest_tracks FOR EACH ROW EXECUTE FUNCTION fixture_corym_fail()").execute(&h.pool).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: dialogue.command,
        item_fence: Some(fence.item_fence()),
    };
    dialogue.command += 1;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let failed = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            command,
            NpcTalkIntent {
                npc_actor: servant,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(failed.disposition, NpcIntentDisposition::Rejected);
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT is_called FROM fixture_corym_failure_seen")
            .fetch_one(&h.pool)
            .await?,
        "failure must occur after both debits and fresh reward mint"
    );
    assert_eq!(count(h, 27301).await?, 20);
    assert_eq!(count(h, 3040).await?, gems);
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, LINE).await?,
        4
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, COUNTER).await?,
        8
    );
    sqlx::query("DROP TRIGGER fixture_corym_fail ON game_character_quest_tracks")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION fixture_corym_fail()")
        .execute(&h.pool)
        .await?;
    let recovery = owner
        .resume_npc_bank(admitted.runtime_actor.ok_or("actor")?, command)
        .await
        .ok_or("retained failed NPC command")?;
    assert!(!recovery.unknown);
    assert_eq!(recovery.disposition, NpcIntentDisposition::Rejected);
    Ok(())
}
