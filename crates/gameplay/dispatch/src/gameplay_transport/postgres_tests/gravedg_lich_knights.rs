//! The Darashia knights use the existing native lever and exact room party.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, MovementLocalPosition, OwnerDamageCommand,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase, Position};

const NICTROS: &str = "oteryn:creature.sir_nictros";
const BAELOC: &str = "oteryn:creature.sir_baeloc";
const QUEST: &str = "oteryn:quest.grave_danger_quest";
const CREDIT: &str = "oteryn:quest-transition/native-kills/gravedg-source-sirs";

pub(super) fn credit_catalogue(
    callback: &Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    encounter_credit_catalogue(callback, BAELOC, CREDIT)
}

fn encounter_credit_catalogue(
    callback: &Callback,
    boss: &str,
    credit: &str,
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
                    "key": credit, "creature": boss, "track": kills,
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
    let cooldown_keys: Vec<String> = sqlx::query_scalar(
        "SELECT boss_key FROM game_character_boss_cooldowns WHERE boss_key IN ($1, $2)",
    )
    .bind(NICTROS)
    .bind(BAELOC)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        cooldown_keys,
        [NICTROS, NICTROS],
        "the shared room has Nictros's lever cooldown and Baeloc's death credit"
    );
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
    partner: &AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let second = partner.runtime_actor.ok_or("second admitted player")?;
    let earl = {
        let runtime = owner.runtime.lock().await;
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&actor| runtime.matches_live_creature_identity(actor, EARL.as_bytes()))
            .ok_or("Earl admitted through USE")?
    };
    let (step, _) = owner
        .ordinary_paced_step(
            second,
            partner.game_session_id,
            StepDirection::North,
            None,
            None,
        )
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    assert_position(owner, second, 116, 116).await?;
    hit(owner, admitted, earl, EARL, 101).await?;
    let now = owner.owner_now().get();
    super::gravedg_zelos::tick_at(owner, now).await;
    super::gravedg_zelos::tick_at(owner, now + 100_000).await;
    assert_position(owner, earl, 116, 116).await?;
    assert_position(owner, second, 116, 118).await?;
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
    for actor in [player, second] {
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

const COUNT: &str = "oteryn:creature.count_vlarkorth";

#[test]
fn gravedg_count_two_players_shield_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::CountVlarkorth)
}

pub(super) fn configure_count(callback: &mut Callback) -> TestResult {
    configure_duke(callback)?;
    callback.key = "oteryn:quest-callback/grave-danger/count-vlarkorth".into();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerCountVlarkorth { exit: at(103, 100) });
    encounter.monsters[0].creature = COUNT.into();
    encounter.monsters.push(
        oteryn_gameplay_quests::quest::world_callbacks::EncounterMonster {
            creature: "oteryn:creature.soulless_minion".into(),
            position: at(119, 116),
        },
    );
    Ok(())
}

pub(super) async fn exercise_count(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: &AdmittedSession,
) -> TestResult {
    use super::gravedg_zelos::tick_at;
    use oteryn_gameplay_spell::spell::Vocation;
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let peer = partner.runtime_actor.ok_or("admitted partner")?;
    assert_position(owner, player, 116, 117).await?;
    assert_position(owner, peer, 110, 117).await?;
    let boss = {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        for (actor, admission, vocation) in [
            (player, admitted, Vocation::EliteKnight),
            (peer, partner, Vocation::ElderDruid),
        ] {
            states
                .get_mut(&runtime, actor, admission.game_session_id)
                .ok_or("admitted spell state")?
                .facts
                .vocation = vocation;
        }
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&actor| runtime.matches_live_creature_identity(actor, COUNT.as_bytes()))
            .ok_or("admitted Count Vlarkorth")?
    };
    hit(owner, admitted, boss, COUNT, 101).await?;
    let now = owner.owner_now().get();
    tick_at(owner, now).await;
    tick_at(owner, now + 100_000).await;
    {
        let runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        for key in ["oteryn:creature.dark_knight", "oteryn:creature.dark_druid"] {
            assert!(
                census
                    .iter()
                    .any(|&actor| runtime.matches_live_creature_identity(actor, key.as_bytes())),
                "{key}"
            );
        }
    }
    assert!(
        hit_by_amount(owner, admitted, boss, COUNT, 102, 1)
            .await
            .is_err()
    );
    let read_health = || async {
        owner
            .runtime
            .lock()
            .await
            .companion_snapshot(boss)
            .map(|snapshot| snapshot.health)
            .map_err(debug)
    };
    let wounded = read_health().await?;
    tick_at(owner, now + 1_999_998).await;
    assert_eq!(
        read_health().await?,
        wounded,
        "the minion waits two seconds"
    );
    tick_at(owner, now + 2_000_000).await;
    assert!((300..=545).contains(&(read_health().await? - wounded)));
    use_count_remains(owner, admitted, partner, boss).await?;
    tick_at(owner, now + 120_100_001).await;
    hit_by_amount(owner, admitted, boss, COUNT, 103, 1).await?;
    tick_at(owner, now + 20 * 60 * 1_000_000).await;
    for actor in [player, peer] {
        assert_position(owner, actor, 103, 100).await?;
    }
    Ok(())
}

async fn use_count_remains(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: &AdmittedSession,
    boss: ExactActorRef,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::gameplay_transport::item_view::ItemKey;
    use crate::map::view::{EntrySource, MapFacts};
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

    const DARK: &str = "oteryn:creature.dark_knight";
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let (dark, dark_position) = {
        let runtime = owner.runtime.lock().await;
        let dark = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&actor| runtime.matches_live_creature_identity(actor, DARK.as_bytes()))
            .ok_or("Dark Knight")?;
        (
            dark,
            runtime.read_actor_position(dark).map_err(debug)?.position(),
        )
    };
    super::engenc2_cults::kill(owner, admitted, dark, DARK, 110).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    let map = owner.bundle_map().ok_or("admitted map")?;
    let tile = crate::map::overlay::TilePos {
        x: u16::try_from(dark_position.x)?,
        y: u16::try_from(dark_position.y)?,
        floor: -i8::try_from(dark_position.floor)?,
    };
    let remains = map
        .facts
        .movement_items(tile)
        .into_iter()
        .find_map(|entry| match entry.source {
            EntrySource::Added(id) if entry.facts.appearance_id == 31203 => Some(id.get()),
            _ => None,
        })
        .ok_or("Dark Knight remains on its death tile")?;
    let intent = {
        let mut runtime = owner.runtime.lock().await;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let before = runtime.read_actor_position(player).map_err(debug)?;
        let proof = crate::map::floor::prepare_quest_encounter_admission(
            &runtime,
            owner.movement_cells(),
            player,
            admitted.game_session_id,
            before,
            MovementLocalPosition {
                x: 118,
                y: 115,
                floor: 7,
            },
            false,
            &closed,
            false,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
        let visible = runtime.visible_entities();
        let target = visible
            .creatures
            .iter()
            .find(|entry| entry.actor == boss)
            .ok_or("visible Count")?;
        UseIntent {
            target: UseTarget::Item(ItemHandle::new(1).ok_or("remains handle")?),
            use_with: Some(EntityRef {
                identity: boss.placement_identity(),
                generation: target.generation,
            }),
            use_with_map_item: None,
            use_with_position: None,
        }
    };
    for (member, expected, id) in [
        (partner, UseDisposition::RequirementNotMet, 111),
        (admitted, UseDisposition::Committed, 112),
    ] {
        let command = UseCommand {
            game_session_id: member.game_session_id,
            command_id: id,
            item_fence: member.item_fence,
        };
        let outcome = owner
            .use_count_remains(
                member.runtime_actor.ok_or("actor")?,
                command,
                intent.clone(),
                ItemKey::MapAdded(remains),
            )
            .await
            .map_err(|()| "native remains use")?
            .ok_or("selected remains use")?;
        assert_eq!(outcome.disposition, expected);
    }
    assert!(map.facts.movement_item(remains).is_none());
    assert!(
        hit_by_amount(owner, admitted, boss, COUNT, 113, 1)
            .await
            .is_err(),
        "one matched remains leaves the other shield charge active"
    );
    Ok(())
}

const LORD: &str = "oteryn:creature.lord_azaram";
const SOUL: &str = "oteryn:creature.azaram_s_soul";

#[test]
fn gravedg_azaram_two_players_soul_phase_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::LordAzaram)
}

pub(super) fn configure_azaram(callback: &mut Callback) -> TestResult {
    configure_duke(callback)?;
    callback.key = "oteryn:quest-callback/grave-danger/lord-azaram".into();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerLordAzaram {
        exit: at(103, 100),
        splinters: [at(115, 113), at(121, 113), at(115, 121), at(121, 121)],
    });
    encounter.monsters[0].creature = LORD.into();
    encounter.monsters.push(
        oteryn_gameplay_quests::quest::world_callbacks::EncounterMonster {
            creature: "oteryn:creature.condensed_sins".into(),
            position: at(117, 117),
        },
    );
    Ok(())
}

pub(super) async fn exercise_azaram(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    peer: ExactActorRef,
) -> TestResult {
    use super::gravedg_zelos::tick_at;
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    assert_position(owner, player, 116, 117).await?;
    assert_position(owner, peer, 110, 117).await?;
    let boss = {
        let runtime = owner.runtime.lock().await;
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&actor| runtime.matches_live_creature_identity(actor, LORD.as_bytes()))
            .ok_or("admitted Lord Azaram")?
    };
    exercise_condensed_sins(owner, admitted).await?;
    hit(owner, admitted, boss, LORD, 101).await?;
    let now = owner.owner_now().get();
    let saved = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(boss)
        .map_err(debug)?
        .health;
    tick_at(owner, now).await;
    tick_at(owner, now + 100_000).await;
    assert!(
        hit_by_amount(owner, admitted, boss, SOUL, 102, 1)
            .await
            .is_err()
    );
    let (splinter, soul) = {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.matches_live_creature_identity(boss, SOUL.as_bytes()));
        let soul = runtime.companion_snapshot(boss).map_err(debug)?;
        assert_eq!(soul.health, soul.maximum_health * 3 / 10);
        let splinters: Vec<_> = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|&actor| {
                runtime
                    .matches_live_creature_identity(actor, b"oteryn:creature.tainted_soul_splinter")
            })
            .collect();
        assert_eq!(splinters.len(), 4);
        (splinters[0], soul)
    };
    super::engenc2_cults::kill(
        owner,
        admitted,
        splinter,
        "oteryn:creature.tainted_soul_splinter",
        103,
    )
    .await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let (_, to) = runtime
            .borrow_combat_death()
            .projected_death(splinter)
            .map_err(debug)?;
        let from = crate::foundation::MovementLocalPosition { y: to.y - 1, ..to };
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(boss, before, from)
            .map_err(debug)?;
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(boss, before, to)
            .map_err(debug)?;
        let map = owner.bundle_map().ok_or("admitted bundle map")?;
        let remains_count = || {
            map.facts
                .movement_item_entries()
                .into_iter()
                .filter(|(_, entry)| entry.facts.appearance_id == 31160)
                .count()
        };
        assert_eq!(
            remains_count(),
            1,
            "death publishes a physical Cleansed Soul Splinter"
        );

        let step = crate::map::facts::MonsterStep {
            actor: boss,
            from,
            to,
        };
        for _ in 0..2 {
            assert!(map.facts.record_monster_step(step));
            crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
                &mut runtime,
                &states,
                map,
                now + 150_000,
            );
            assert_eq!(
                runtime.companion_snapshot(boss).map_err(debug)?.health,
                soul.health + soul.maximum_health / 3
            );
            assert_eq!(
                remains_count(),
                0,
                "committed monster step consumes the physical remains"
            );
        }
        let maximum = u64::try_from(soul.maximum_health)?;
        let digest = runtime.content_pin().server_artifact_digest();
        runtime
            .commit_source_creature_heal_batch(digest, &[(boss, SOUL.to_owned(), maximum, maximum)])
            .map_err(debug)?;
    }
    tick_at(owner, now + 200_000).await;
    tick_at(owner, now + 700_000).await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.matches_live_creature_identity(boss, LORD.as_bytes()));
        assert_eq!(
            runtime.companion_snapshot(boss).map_err(debug)?.health,
            saved
        );
        assert!(
            !runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .any(|actor| runtime.matches_live_creature_identity(
                    actor,
                    b"oteryn:creature.tainted_soul_splinter"
                ))
        );
    }
    tick_at(owner, now + 20 * 60 * 1_000_000).await;
    for actor in [player, peer] {
        assert_position(owner, actor, 103, 100).await?;
    }
    Ok(())
}

async fn exercise_condensed_sins(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::actor_spell::{
        commit_owner_batch, stage_player_batch_with_magnitude,
    };
    use crate::spell::combat_batch::{
        LiveActorBinding, OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect,
    };
    use crate::spell::magnitude_owner::{
        MagnitudeOwnedAttributes, MagnitudeOwnerBinding, MagnitudePolicy, PreparedMagnitudeOwner,
    };
    use crate::spell::native_combat::{CombatPlan, Element, MagnitudePlan, NativeCombatPlan};

    const SINS: &str = "oteryn:creature.condensed_sins";
    let actor = admitted.runtime_actor.ok_or("admitted player")?;
    let session = admitted.game_session_id;
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let sins = runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .find(|&target| runtime.matches_live_creature_identity(target, SINS.as_bytes()))
        .ok_or("admitted Condensed Sins")?;
    for (element, command, expected_reflection) in
        [(Element::Fire, 97, 0), (Element::Death, 98, 20)]
    {
        let state = states
            .get(&runtime, actor, session)
            .ok_or("admitted spell state")?;
        let previous = state.vitals().health;
        let now = owner.owner_now().get() / 1000;
        let attrs = MagnitudeOwnedAttributes {
            policy: MagnitudePolicy::BaselineTest,
            item_augments: vec![],
            owner_revisions: Some((1, 1, None)),
            binding: MagnitudeOwnerBinding {
                actor,
                session,
                character: lease.character_id(),
                lease_generation: lease.generation(),
                player_revision: state.revision(),
                content_digest: runtime.content_pin().server_artifact_digest(),
            },
            base_critical_chance_permyriad: Some(0),
            base_critical_extra_permyriad: Some(0),
            equipment_critical_chance_permyriad: Some(0),
            equipment_critical_extra_permyriad: Some(0),
            fatal_chance_permyriad: None,
            armor_penetration_permyriad: None,
            elemental_pierce_permyriad: None,
            wheel_effects: None,
            wheel_flat_damage: None,
            wheel_flat_healing: None,
            source_damage_multiplier_percent: None,
            source_healing_multiplier_percent: None,
            healing_dealt_percent: None,
            monster_armor_disabled: None,
            inactive_extra_stages: None,
        };
        let plan = NativeCombatPlan::Combat(CombatPlan {
            element,
            area: None,
            hits: vec![MagnitudePlan {
                target: 1,
                magnitude: 20,
                bonus_percent: 0,
                side_percent: None,
                delay_ms: 0,
            }],
            cooldown_ms: None,
            shared_cooldown: None,
            reduce_all_spell_cooldowns_ms: 0,
            block_armor: false,
            use_weapon_charges: false,
            weapon_missile: false,
            dispel_paralysis_targets: vec![],
            resolve_critical_and_fatal_once: false,
        });
        let bindings = [LiveActorBinding {
            source_id: 1,
            actor: sins,
            target_atom: SINS.into(),
        }];
        let magnitude = PreparedMagnitudeOwner::qualify(
            &runtime,
            state,
            actor,
            session,
            Some(&attrs),
            &plan,
            &bindings,
            &[],
            now,
        )
        .map_err(debug)?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: lease.character_id(),
            command: CommandRef::new(session, CommandId::new(command).map_err(debug)?),
            current_lease_generation: lease.generation(),
            occurrence: crate::spell::combat_batch::SpellOccurrenceBinding {
                id: format!("gravedg:condensed-sins:{command}"),
                revisions: [
                    "ruleset-1",
                    "content-1",
                    "world-1",
                    "formula-1",
                    "simulation-1",
                ]
                .map(str::to_owned),
            },
            binding: b"gravedg-condensed-sins".to_vec(),
            anchor: None,
            now_us: None,
            now_ms: now,
            effects: vec![OwnerCombatEffect {
                target: sins,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: SINS.into(),
                    magnitude: 20,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let preflight =
            stage_player_batch_with_magnitude(&runtime, &states, &batch, None, Some(&magnitude))
                .map_err(debug)?;
        assert!(
            commit_owner_batch(&mut runtime, &mut states, staged, Some(preflight))
                .map_err(debug)?
                .applied
        );
        let current = states
            .get(&runtime, actor, session)
            .ok_or("admitted spell state")?
            .vitals()
            .health;
        assert_eq!(previous - current, expected_reflection);
    }
    Ok(())
}

const FIVE_CREDIT: &str = "oteryn:quest-transition/native-kills/gravedg-five-player";

#[test]
fn gravedg_azaram_five_player_lever_and_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::FivePlayerCredit {
        azaram: true,
    })
}

#[test]
fn gravedg_count_five_player_lever_and_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::FivePlayerCredit {
        azaram: false,
    })
}

pub(super) fn configure_five(callback: &mut Callback, azaram: bool) -> TestResult {
    if azaram {
        configure_azaram(callback)?;
    } else {
        configure_count(callback)?;
    }
    callback.quest = QUEST.into();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.players = (0..5)
        .map(
            |index| oteryn_gameplay_quests::quest::world_callbacks::EncounterPlayer {
                from: at(100, 100 + index),
                to: at(116, 117),
            },
        )
        .collect();
    Ok(())
}

pub(super) fn five_credit_catalogue(
    callback: &Callback,
    azaram: bool,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    encounter_credit_catalogue(callback, if azaram { LORD } else { COUNT }, FIVE_CREDIT)
}

pub(super) async fn exercise_five(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    party: &[AdmittedSession],
    azaram: bool,
) -> TestResult {
    let key = if azaram { LORD } else { COUNT };
    let boss = {
        let runtime = owner.runtime.lock().await;
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|&target| runtime.matches_live_creature_identity(target, key.as_bytes()))
            .ok_or("boss spawned by native lever USE")?
    };
    assert_eq!(party.len(), 5);
    for (index, member) in party.iter().enumerate() {
        assert_position(owner, member.runtime_actor.ok_or("party actor")?, 116, 117).await?;
        hit_by_amount(owner, member, boss, key, 100 + u64::try_from(index)?, 1).await?;
    }
    super::engenc2_cults::kill(owner, &party[0], boss, key, 110).await?;
    for member in party {
        assert_eq!(
            owner.drain_kill_rewards(member.game_session_id).await,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        );
    }
    let credited: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT DISTINCT uuid_send(character_id) AS character_bytes
         FROM game_character_quest_receipts WHERE transition_key = $1 ORDER BY character_bytes",
    )
    .bind(FIVE_CREDIT)
    .fetch_all(&h.pool)
    .await?;
    let mut entrants = {
        let runtime = owner.runtime.lock().await;
        party
            .iter()
            .map(|member| {
                let actor = member.runtime_actor.ok_or("party actor")?;
                let lease = runtime
                    .bound_attacker_lease(actor, member.game_session_id)
                    .map_err(debug)?;
                Ok(lease.character_id().as_bytes().to_vec())
            })
            .collect::<TestResult<Vec<_>>>()?
    };
    entrants.sort_unstable();
    assert_eq!(
        credited, entrants,
        "all five admitted damage contributors earn durable credit"
    );
    super::gravedg_zelos::tick_at(owner, owner.owner_now().get() + 60_000_000).await;
    for member in party {
        assert_position(owner, member.runtime_actor.ok_or("party actor")?, 103, 100).await?;
    }
    repeat_after_cooldown(h, owner, party, key).await
}

async fn repeat_after_cooldown(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    party: &[AdmittedSession],
    boss: &str,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

    let last = party.last().ok_or("last staged player")?;
    let last_character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(
            last.runtime_actor.ok_or("last actor")?,
            last.game_session_id,
        )
        .map_err(debug)?
        .character_id();
    let expired = sqlx::query(
        "UPDATE game_character_boss_cooldowns SET available_at = 0
         WHERE boss_key = $1 AND character_id <> encode($2,'hex')::uuid",
    )
    .bind(boss)
    .bind(last_character.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(expired.rows_affected(), 4);
    for (index, member) in party.iter().enumerate() {
        let actor = member.runtime_actor.ok_or("party actor")?;
        owner
            .runtime
            .lock()
            .await
            .place_respawned_player(
                actor,
                member.game_session_id,
                MovementLocalPosition {
                    x: 100,
                    y: 100 + i32::try_from(index)?,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        owner.admit_quest_session(member).await;
    }
    let leader = party.first().ok_or("party leader")?;
    let map = owner.bundle_map().ok_or("encounter map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-7, 100, 100, 1).ok_or("lever key")?;
    let expected_revision = map.facts.object_revision(key);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let leader_actor = leader.runtime_actor.ok_or("leader actor")?;
    let use_lever = async |command_id| {
        owner
            .use_object(
                leader_actor,
                UseCommand {
                    game_session_id: leader.game_session_id,
                    command_id,
                    item_fence: leader.item_fence,
                },
                WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision,
                },
            )
            .await
    };
    assert_eq!(
        use_lever(111).await.disposition,
        UseDisposition::RequirementNotMet,
        "one staged player on cooldown blocks the entire party"
    );
    assert_eq!(map.facts.object_revision(key), expected_revision);
    for (index, member) in party.iter().enumerate() {
        assert_position(
            owner,
            member.runtime_actor.ok_or("party actor")?,
            100,
            100 + i32::try_from(index)?,
        )
        .await?;
    }
    let expired = sqlx::query(
        "UPDATE game_character_boss_cooldowns SET available_at = 0
         WHERE boss_key = $1 AND available_at > 0",
    )
    .bind(boss)
    .execute(&h.pool)
    .await?;
    assert_eq!(
        expired.rows_affected(),
        1,
        "refusal does not renew any cooldown"
    );
    let retry = use_lever(112).await;
    assert_eq!(
        retry.disposition,
        UseDisposition::Committed,
        "credited entrants can fight again when their cooldown expires"
    );
    for member in party {
        assert_position(owner, member.runtime_actor.ok_or("party actor")?, 116, 117).await?;
    }
    Ok(())
}
