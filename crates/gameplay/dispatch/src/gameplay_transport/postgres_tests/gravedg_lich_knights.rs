//! The Darashia knights use the existing native lever and exact room party.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, OwnerDamageCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase, Position};

const NICTROS: &str = "oteryn:creature.sir_nictros";
const BAELOC: &str = "oteryn:creature.sir_baeloc";
const QUEST: &str = "oteryn:quest.grave_danger_quest";
const CREDIT: &str = "oteryn:quest-transition/native-kills/gravedg-source-sirs";

pub(super) fn credit_catalogue(
    callback: &Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
    use serde_json::json;
    let phase = "oteryn:quest-progress/gravedg-fixture/sirs-phase";
    let kills = "oteryn:quest-progress/gravedg-fixture/sirs-kills";
    let source = json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": "fc16ee20de6f8c89059692b754094eb4f4e9bae653a4eb51c68827b9af08057a",
            "sources": ["Crystal 201189f actions_bosses_killed.lua"],
            "tracks": [
                {"key": phase, "initial": 0, "minimum": 0, "maximum": 1},
                {"key": kills, "initial": 0, "minimum": 0, "maximum": 1}
            ],
            "stages": [
                {"key": "entry", "completes": false, "trigger": {"kind": "map_use", "callback": callback}},
                {"key": "credit", "completes": false, "trigger": {"kind": "kill", "counter": {
                    "key": CREDIT, "creature": BAELOC, "track": kills,
                    "gate": phase, "gate_value": 0, "maximum": 1, "encounter_source": callback.key
                }}}
            ]
        }]
    });
    let bindings = Bindings::parse(&serde_json::to_vec(&source)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("empty fixture catalogue")?;
    bindings.attach(&base).map_err(Into::into)
}

#[test]
fn gravedg_lich_knights_two_players_sirs_switch_credit_and_exit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::Sirs)
}

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -7 }
}

pub(super) fn configure(callback: &mut Callback) -> TestResult {
    callback.key = "oteryn:quest-callback/grave-danger/sirs".into();
    callback.quest = QUEST.into();
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerSirs {
        fight: [at(114, 114), at(115, 114), at(116, 114)],
        exit: at(103, 100),
    });
    encounter.minimum_level = 1;
    encounter.monsters.truncate(2);
    for (monster, (creature, position)) in encounter
        .monsters
        .iter_mut()
        .zip([(NICTROS, at(109, 109)), (BAELOC, at(111, 109))])
    {
        monster.creature = creature.into();
        monster.position = position;
    }
    Ok(())
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: &AdmittedSession,
) -> TestResult {
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let partner_actor = partner.runtime_actor.ok_or("admitted partner")?;
    let (nictros, baeloc) = {
        let runtime = owner.runtime.lock().await;
        for actor in [player, partner_actor] {
            let at = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            assert_eq!((at.x, at.y), (116, 117));
        }
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        let find = |name: &str| {
            actors
                .iter()
                .copied()
                .find(|&actor| runtime.matches_live_creature_identity(actor, name.as_bytes()))
                .ok_or("admitted knight")
        };
        let (nictros, baeloc) = (find(NICTROS)?, find(BAELOC)?);
        for actor in [nictros, baeloc] {
            assert!(
                runtime
                    .encounter_hooks()
                    .is_some_and(|hooks| hooks.contains(actor))
            );
        }
        (nictros, baeloc)
    };
    let now = owner.owner_now().get();
    super::gravedg_zelos::tick_at(owner, now + 5_000_000).await;
    assert_position(owner, nictros, 114, 114).await?;
    hit(owner, admitted, nictros, NICTROS, 101).await?;
    super::gravedg_zelos::tick_at(owner, now + 5_001_000).await;
    assert_position(owner, nictros, 109, 109).await?;
    assert_position(owner, baeloc, 115, 114).await?;
    hit(owner, admitted, baeloc, BAELOC, 102).await?;
    super::gravedg_zelos::tick_at(owner, now + 5_002_000).await;
    assert_position(owner, nictros, 116, 114).await?;
    // The partner earns an exact damage share without taking the last hit.
    hit_by_amount(owner, partner, baeloc, BAELOC, 103, 1).await?;
    super::engenc2_cults::kill(owner, admitted, baeloc, BAELOC, 104).await?;
    super::engenc2_cults::kill(owner, admitted, nictros, NICTROS, 105).await?;
    for session in [admitted.game_session_id, partner.game_session_id] {
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained,
        );
    }
    let credited: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT character_id) FROM game_character_quest_receipts WHERE transition_key=$1",
    ).bind(CREDIT).fetch_one(&h.pool).await?;
    assert_eq!(
        credited, 2,
        "both admitted damage dealers earn the Baeloc grave credit"
    );
    super::gravedg_zelos::tick_at(owner, now + 5_002_000 + 60_000_000).await;
    for actor in [player, partner_actor] {
        assert_position(owner, actor, 103, 100).await?;
    }
    let runtime = owner.runtime.lock().await;
    assert!(!runtime.contains_live_creature(nictros));
    assert!(!runtime.contains_live_creature(baeloc));
    Ok(())
}

async fn assert_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    x: i32,
    y: i32,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (x, y, 7));
    Ok(())
}

async fn hit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    defender: ExactActorRef,
    key: &str,
    command: u64,
) -> TestResult {
    let health = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(defender)
        .map_err(debug)?
        .maximum_health;
    hit_by_amount(owner, admitted, defender, key, command, health / 2).await
}

async fn hit_by_amount(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    defender: ExactActorRef,
    key: &str,
    command: u64,
    damage: i64,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    runtime
        .borrow_exact_actor_commit_at(owner.owner_now().get() / 1_000)
        .commit_damage_for_bound_attacker(
            defender,
            player,
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            0,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: b"gravedg-lich-knights",
                binding: b"gravedg-sirs-phase",
                damage,
            },
        )
        .map_err(debug)?;
    Ok(())
}

#[test]
fn gravedg_lich_knights_two_players_osam_channel_and_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::EarlOsam)
}

const EARL: &str = "oteryn:creature.earl_osam";
const SPHERE: &str = "oteryn:creature.magical_sphere";

pub(super) fn configure_osam(callback: &mut Callback) -> TestResult {
    callback.key = "oteryn:quest-callback/grave-danger/earl-osam".into();
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerEarlOsam {
        exit: at(103, 100),
        centre: at(116, 116),
    });
    encounter.minimum_level = 1;
    encounter.monsters.truncate(1);
    let monster = encounter.monsters.first_mut().ok_or("Earl Osam")?;
    monster.creature = EARL.into();
    monster.position = at(118, 116);
    Ok(())
}

pub(super) async fn exercise_osam(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: ExactActorRef,
) -> TestResult {
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let earl = {
        let runtime = owner.runtime.lock().await;
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&actor| runtime.matches_live_creature_identity(actor, EARL.as_bytes()))
            .ok_or("Earl admitted through USE")?
    };
    hit(owner, admitted, earl, EARL, 101).await?;
    let now = owner.owner_now().get();
    super::gravedg_zelos::tick_at(owner, now).await;
    super::gravedg_zelos::tick_at(owner, now + 100_000).await;
    assert_position(owner, earl, 116, 116).await?;
    {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.matches_live_creature_identity(earl, EARL.as_bytes()));
        assert_eq!(
            runtime
                .companion_snapshot(earl)
                .map_err(debug)?
                .state
                .current_speed(now + 100_000)
                .map_err(debug)?,
            0,
            "Earl channels in place without losing his physical incarnation",
        );
        assert!(
            runtime
                .encounter_hooks()
                .is_some_and(|hooks| hooks.contains(earl))
        );
        let spheres = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|&actor| runtime.matches_live_creature_identity(actor, SPHERE.as_bytes()))
            .count();
        assert_eq!(spheres, 4);
    }
    super::gravedg_zelos::tick_at(owner, now + 20 * 60 * 1_000_000).await;
    for actor in [player, partner] {
        assert_position(owner, actor, 103, 100).await?;
    }
    let runtime = owner.runtime.lock().await;
    assert!(!runtime.contains_live_creature(earl));
    assert!(
        !runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .any(|actor| runtime.matches_live_creature_identity(actor, SPHERE.as_bytes()))
    );
    Ok(())
}

#[test]
fn gravedg_lich_knights_two_players_krule_cycles_forms_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::DukeKrule)
}

const DUKE: &str = "oteryn:creature.duke_krule";

pub(super) fn configure_duke(callback: &mut Callback) -> TestResult {
    callback.key = "oteryn:quest-callback/grave-danger/duke-krule".into();
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerDukeKrule { exit: at(103, 100) });
    encounter.minimum_level = 1;
    encounter.monsters.truncate(1);
    let monster = encounter.monsters.first_mut().ok_or("Duke Krule")?;
    monster.creature = DUKE.into();
    monster.position = at(118, 116);
    encounter
        .players
        .get_mut(1)
        .ok_or("second staging tile")?
        .to = at(110, 117);
    Ok(())
}

pub(super) async fn exercise_duke(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: ExactActorRef,
) -> TestResult {
    use crate::domain::condition::ConflictKey;
    use crate::foundation::ConditionValues;
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let duke = {
        let runtime = owner.runtime.lock().await;
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&actor| runtime.matches_live_creature_identity(actor, DUKE.as_bytes()))
            .ok_or("Duke admitted through USE")?
    };
    assert_position(owner, player, 116, 117).await?;
    assert_position(owner, partner, 110, 117).await?;
    hit(owner, admitted, duke, DUKE, 101).await?;
    let now = owner.owner_now().get();
    super::gravedg_zelos::tick_at(owner, now).await;
    super::gravedg_zelos::tick_at(owner, now + 3_000_000).await;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let current = states
            .get(&runtime, player, admitted.game_session_id)
            .ok_or("admitted state")?;
        let outfit = current
            .owned_conditions()
            .active_at(ConflictKey::Outfit, now + 3_000_000)
            .ok_or("Duke Krule elemental outfit")?;
        assert!(matches!(
            outfit.definition().values(),
            ConditionValues::Outfit {
                look_type: 49 | 286,
                duration_ms: 6000
            }
        ));
        assert!(
            current
                .owned_conditions()
                .active_at(ConflictKey::Outfit, now + 9_000_000)
                .is_none()
        );
    }
    super::gravedg_zelos::tick_at(owner, now + 20 * 60 * 1_000_000).await;
    for actor in [player, partner] {
        assert_position(owner, actor, 103, 100).await?;
    }
    Ok(())
}
