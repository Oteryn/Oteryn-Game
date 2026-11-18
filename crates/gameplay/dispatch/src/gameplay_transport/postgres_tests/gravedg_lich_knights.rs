//! The Darashia knights use the existing native lever and exact room party.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, OwnerDamageCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase, Position};

const NICTROS: &str = "oteryn:creature.sir_nictros";
const BAELOC: &str = "oteryn:creature.sir_baeloc";

#[test]
fn gravedg_lich_knights_two_players_sirs_switch_and_timeout_postgres() -> TestResult {
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
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerSirs {
        fight: [at(114, 114), at(115, 114), at(116, 114)],
        exit: at(103, 100),
    });
    encounter.minimum_level = 1;
    encounter.monsters.truncate(2);
    for (monster, (creature, position)) in
        encounter.monsters.iter_mut().zip([(NICTROS, at(109, 109)), (BAELOC, at(111, 109))])
    {
        monster.creature = creature.into();
        monster.position = position;
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: ExactActorRef,
) -> TestResult {
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let (nictros, baeloc) = {
        let runtime = owner.runtime.lock().await;
        for actor in [player, partner] {
            let at = runtime.read_actor_position(actor).map_err(debug)?.position();
            assert_eq!((at.x, at.y), (116, 117));
        }
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        let find = |name: &str| actors.iter().copied().find(|&actor| {
            runtime.matches_live_creature_identity(actor, name.as_bytes())
        }).ok_or("admitted knight");
        let (nictros, baeloc) = (find(NICTROS)?, find(BAELOC)?);
        for actor in [nictros, baeloc] {
            assert!(runtime.encounter_hooks().is_some_and(|hooks| hooks.contains(actor)));
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
    super::gravedg_zelos::tick_at(owner, now + 20 * 60 * 1_000_000).await;
    for actor in [player, partner] {
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
    let at = runtime.read_actor_position(actor).map_err(debug)?.position();
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
    let mut runtime = owner.runtime.lock().await;
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let health = runtime.companion_snapshot(defender).map_err(debug)?.maximum_health;
    runtime.borrow_exact_actor_commit_at(owner.owner_now().get() / 1_000)
        .commit_damage_for_bound_attacker(
            defender, player, CommandRef::new(admitted.game_session_id, CommandId::new(command).map_err(debug)?),
            0,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: b"gravedg-lich-knights",
                binding: b"gravedg-sirs-phase",
                damage: health / 2,
            },
        ).map_err(debug)?;
    Ok(())
}
