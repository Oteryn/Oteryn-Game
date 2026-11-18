//! Global Paw and Fur reward and objective regressions through command 7.
use super::TestResult;
use super::fixture::Harness;
use super::grizzly_npc::{holy_icon, scenario, seed, value, wire::talk};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_entities::carrier::soul_war_player_source::{
    CurrentSoulWarPlayerSourceProof, SoulWarPlayerSourceFacts, SoulWarPlayerSourceOrigin,
    source_proof_seal,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::grizzly_adams as quest;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

fn run_case(case: u8, name: &str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || scenario(case))?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn mammoths() -> TestResult {
    run_case(7, "mammoths")
}
#[test]
fn terramites() -> TestResult {
    run_case(8, "terramites")
}
#[test]
fn gargoyles() -> TestResult {
    run_case(9, "gargoyles")
}
#[test]
fn mutated_tigers() -> TestResult {
    run_case(10, "mutated_tigers")
}
#[test]
fn hellspawns() -> TestResult {
    run_case(11, "hellspawns")
}
#[test]
fn demons() -> TestResult {
    run_case(12, "demons")
}

#[test]
fn infernal_demon_credit() -> TestResult {
    run_case(13, "infernal_demon_credit")
}

#[test]
fn tiquandas_revenge_level_gate() -> TestResult {
    run_case(14, "tiquandas_revenge_level_gate")
}

#[test]
fn demodras_level_gate() -> TestResult {
    run_case(15, "demodras_level_gate")
}

#[test]
fn rank_promotions() -> TestResult {
    run_case(16, "rank_promotions")
}

#[test]
fn elite_hunter_points_cap() -> TestResult {
    run_case(17, "elite_hunter_points_cap")
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(super::fixture::id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

// The fixture room is outside Soul War. The ordinary combat commit still needs
// the current player facts for Infernal Demon's lethal Fourth Taint callback.
struct OutsideSoulWarSource {
    actor: ExactActorRef,
    command: CommandRef,
    now: u64,
    facts: SoulWarPlayerSourceFacts,
}

impl source_proof_seal::Sealed for OutsideSoulWarSource {}

impl CurrentSoulWarPlayerSourceProof for OutsideSoulWarSource {
    fn current_facts(
        &self,
        _: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        command: Option<CommandRef>,
        now: u64,
    ) -> Option<SoulWarPlayerSourceFacts> {
        (actor == self.actor
            && session == self.command.game_session_id()
            && command == Some(self.command)
            && now == self.now)
            .then_some(self.facts)
    }
}

async fn prepare_infernal_demon_killer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command: u64,
) -> TestResult {
    use super::fixture::debug;

    let actor = admitted.runtime_actor.ok_or("hunter")?;
    let session = admitted.game_session_id;
    let command = CommandRef::new(session, CommandId::new(command).map_err(debug)?);
    let now = owner.owner_now().get();
    owner.states.test_owner_time.store(
        now.checked_add(1).ok_or("fixture owner time")?,
        std::sync::atomic::Ordering::Relaxed,
    );

    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, actor, session)
        .ok_or("hunter casting state")?;
    let proof = OutsideSoulWarSource {
        actor,
        command,
        now,
        facts: SoulWarPlayerSourceFacts {
            vocation: state.facts.vocation,
            taints: 0,
            realm: None,
            before_revision: state.revision(),
            origin: SoulWarPlayerSourceOrigin::Combat,
        },
    };
    runtime
        .install_current_soul_war_player_source(actor, session, Some(command), now, &proof)
        .map_err(|error| format!("Infernal Demon lethal source: {error:?}"))?;
    Ok(())
}

// TibiaWiki Spoiler, curid 36696, oldid 1206121, ranked task tables.
#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    case: u8,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    initial_xp: i64,
) -> TestResult {
    if case == 13 {
        seed(
            h,
            &[
                (quest::QUEST_STORAGE_BASE + 42, 1),
                (quest::KILL_COUNTER_BASE + 42, 0),
            ],
        )
        .await?;
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native creature content")?;
        {
            let mut runtime = owner.runtime.lock().await;
            // This fixture has a classic map room with no qualified Soul War death owner.
            runtime
                .use_classic_creature_deaths()
                .map_err(super::fixture::debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(super::fixture::debug)?;
        }
        let attack_command = 100;
        prepare_infernal_demon_killer(owner, admitted, attack_command).await?;
        super::grizzly_npc::questpc8_campaign::hunt(
            owner,
            admitted,
            "oteryn:creature.infernal_demon",
            1,
            attack_command,
        )
        .await?;
        assert_eq!(
            value(owner, authority, quest::KILL_COUNTER_BASE + 42).await?,
            1
        );
        return Ok(());
    }
    if matches!(case, 14 | 15) {
        let tiquanda = quest::boss(29).ok_or("Tiquandas Revenge")?;
        let demodras = quest::boss(30).ok_or("Demodras")?;
        seed(
            h,
            &[(quest::POINTS_STORAGE, 70), (quest::PROMOTION_STORAGE, 6)],
        )
        .await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        if case == 15 {
            talk(owner, admitted, npc, &[(3, "hi"), (4, "special")]).await?;
            assert_eq!(value(owner, authority, tiquanda.storage).await?, 1);
            seed(h, &[(tiquanda.storage, 3)]).await?;
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
        }
        let boss = if case == 14 { tiquanda } else { demodras };
        let command = if case == 14 { 3 } else { 5 };
        super::grizzly_npc::wire::talk_refused(
            owner,
            admitted,
            npc,
            &[(command, "hi"), (command + 1, "special")],
        )
        .await?;
        assert_eq!(value(owner, authority, boss.storage).await?, -1);
        // An already accepted assignment can be turned in after a level loss.
        seed(h, &[(boss.storage, 2), (boss.kill_storage, 1)]).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        talk(
            owner,
            admitted,
            npc,
            &[(command + 2, "hi"), (command + 3, "special")],
        )
        .await?;
        assert_eq!(value(owner, authority, boss.storage).await?, 3);
        return Ok(());
    }
    if case == 16 {
        seed(h, &[(quest::POINTS_STORAGE, 100)]).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let mut promotions = Vec::new();
        let mut previous_xp = initial_xp;
        for command in [3, 5, 7, 9, 11] {
            talk(
                owner,
                admitted,
                npc,
                &[
                    (command, "hi"),
                    (command + 1, "promotion"),
                    (command + 1, "promotion"),
                ],
            )
            .await?;
            let xp = experience(h).await?;
            promotions.push((
                value(owner, authority, quest::PROMOTION_STORAGE).await?,
                xp - previous_xp,
            ));
            previous_xp = xp;
        }
        let achievements: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key IN ('oteryn:achievement/huntsman','oteryn:achievement/elite_hunter')",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            (promotions.as_slice(), achievements),
            (
                &[
                    (0, 5_000),
                    (2, 8_000),
                    (4, 10_000),
                    (6, 15_000),
                    (7, 30_000)
                ][..],
                2
            ),
        );
        return Ok(());
    }
    if case == 17 {
        seed(
            h,
            &[(quest::POINTS_STORAGE, 100), (quest::PROMOTION_STORAGE, 7)],
        )
        .await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    let expected_xp = if case == 12 {
        holy_icon(owner, admitted, npc, h, authority).await?;
        assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 100);
        100_000
    } else {
        let (id, name, kills, xp, points) = match case {
            7 => (6, "mammoths", 300, 5_000, 3),
            8 => (8, "terramites", 300, 2_500, 2),
            9 => (11, "gargoyles", 200, 1_500, 2),
            10 => (22, "mutated tigers", 250, 12_000, 3),
            11 => (27, "hellspawns", 600, 25_000, 4),
            17 => (32, "hydras", 650, 30_000, 100),
            _ => return Err("unknown Paw and Fur regression case".into()),
        };
        let task = quest::QUEST_STORAGE_BASE + id;
        talk(owner, admitted, npc, &[(3, "hi"), (4, name), (5, "yes")]).await?;
        assert_eq!(value(owner, authority, task).await?, 1);
        seed(h, &[(quest::KILL_COUNTER_BASE + id, kills)]).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        talk(owner, admitted, npc, &[(6, "hi"), (7, "tasks")]).await?;
        assert_eq!(value(owner, authority, task).await?, 0);
        assert_eq!(
            value(owner, authority, quest::POINTS_STORAGE).await?,
            points
        );
        xp
    };
    assert_eq!(experience(h).await?, initial_xp + expected_xp);
    Ok(())
}
