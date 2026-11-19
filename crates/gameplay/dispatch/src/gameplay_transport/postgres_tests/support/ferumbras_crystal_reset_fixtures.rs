//! Crystal 201189f8, ferumbras_ascension/actions_ferumbras_lever.lua:1-62.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const CASE: &str = "ferumbras-crystal-reset";
pub(super) const LEVER: Position = at(33270, 31476);
const ARRIVAL: Position = at(33392, 31479);
const BOSS_AT: Position = at(33392, 31473);
const ROOM_MIN: Position = at(33379, 31460);
const ROOM_MAX: Position = at(33405, 31485);
const OUTSIDE: Position = at(33392, 31486);
const VORTEX_TILE: Position = at(33388, 31471);
const VORTEX_APPROACH: Position = at(33388, 31470);
const CRYSTALS: [Position; 8] = [
    at(33390, 31468),
    at(33394, 31468),
    at(33397, 31471),
    at(33397, 31475),
    at(33394, 31478),
    at(33390, 31478),
    at(33387, 31475),
    at(33387, 31471),
];
const BOSS: &str = "oteryn:creature.ascending_ferumbras";
const DEMON: &str = "oteryn:creature.enthralled_demon";
const DESTABILIZED: &str = "oteryn:creature.destabilized_ferumbras";
const ESSENCE: &str = "oteryn:creature.ferumbras_essence";
const RIFT_INVADER: &str = "oteryn:creature.rift_invader";
const SHELL: &str = "oteryn:creature.ferumbras_mortal_shell";
const SPLINTER: &str = "oteryn:creature.ferumbras_soul_splinter";
const DORMANT_CRYSTAL: u16 = 14955;
const CHARGED_CRYSTAL: u16 = 14961;
const CHARGE_COUNT: u16 = 10;
const CRYSTAL_EFFECT: u16 = 74; // Crystal src/utils/utils_definitions.hpp:129.
const VORTEX: u16 = 20121;
const VORTEX_MS: u32 = 10_000;
const VORTEX_THRESHOLD: usize = 7;
const POOF: u16 = 3; // Crystal CONST_ME_POFF.

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

pub(super) fn callback() -> TestResult<Callback> {
    let players: Vec<_> = (33269..=33271)
        .flat_map(|x| {
            (31477..=31481).map(move |y| serde_json::json!({"from": at(x, y), "to": ARRIVAL}))
        })
        .collect();
    let mut monsters = vec![serde_json::json!({"creature": BOSS, "position": BOSS_AT})];
    monsters.extend(
        [
            at(33385, 31466),
            at(33396, 31466),
            at(33392, 31480),
            at(33392, 31468),
        ]
        .map(|position| serde_json::json!({"creature": DEMON, "position": position})),
    );
    let edits = CRYSTALS.map(|position| serde_json::json!({
        "position": position, "remove": CHARGED_CRYSTAL, "create": DORMANT_CRYSTAL, "passable": false, "allow_missing": true
    }));
    // Crystal creaturescripts_rift_invader_death.lua:2-9,26-32,45: count starts at 0 and
    // the transform occurs when the previous count is > 8, on the tenth death.
    let crystals = CRYSTALS.map(|position| {
        serde_json::json!({
            "position":position,"area":{
                "minimum":at(position.x-1,position.y-1),"maximum":at(position.x+1,position.y+1)
            }
        })
    });
    let late_area = serde_json::json!({"minimum":ROOM_MIN,"maximum":ROOM_MAX});
    let document = serde_json::json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/ferumbras/final-fight",
        "quest":"oteryn:quest.ferumbras_ascension_quest",
        "trigger":"use","selector":{"position":LEVER,"appearance":2772},
        "reversible":false,"passages":[],"storage":[],"offerings":[],
        "return_on_failure":false,
        "effects":{
            "scene":{"open":edits,"close":[]},
            "encounter":{
                "minimum_level":0,"players":players,"monsters":monsters,
                "room_min":ROOM_MIN,"room_max":ROOM_MAX,"shared_staging":true,
                "death_phases":{"terminal":SHELL,"crystal_circuit":{
                    "defeated":RIFT_INVADER,"charges_per_crystal":CHARGE_COUNT,
                    "dormant_appearance":DORMANT_CRYSTAL,"charged_appearance":CHARGED_CRYSTAL,
                    "crystals":crystals,"retire":BOSS,"spawn":SPLINTER,
                    "boss_position":BOSS_AT,"charged_effect":CRYSTAL_EFFECT,
                    "vortex":{"appearance":VORTEX,"duration_ms":VORTEX_MS,
                        "consume_creature":ESSENCE,"consume_effect":POOF,"threshold":VORTEX_THRESHOLD,"spawn":DESTABILIZED}
                },"replacements":[
                    {"defeated":SPLINTER,"observe_deaths_in":late_area,"spawn":[{"creature":ESSENCE}]},
                    {"defeated":DESTABILIZED,"observe_deaths_in":late_area,"spawn":[{"creature":SHELL,"position":BOSS_AT}]}
                ]}
            }
        },
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/actions_ferumbras_lever.lua:1-62",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/creaturescripts_rift_invader_death.lua:1-10,23-45",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/creaturescripts_ferumbras_soul_splinter.lua:3-9",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/creaturescripts_ferumbras_mortal_shell_death.lua:5-16",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/movements_vortex.lua:5-18",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/lib/register_lever_tables.lua:1-14"
        ]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into()
        .map_err(|_| "one Ferumbras callback")?;
    Ok(callback)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let mut cells: Vec<_> = CRYSTALS
        .iter()
        .map(|position| at(position.x + 1, position.y))
        .collect();
    cells.extend([OUTSIDE, VORTEX_APPROACH]);
    super::quest_world_callbacks_fixtures::world_with_scene_cells(
        callback,
        LEVER,
        &CRYSTALS.map(|position| (position, CHARGED_CRYSTAL)),
        &[DORMANT_CRYSTAL, VORTEX],
        &cells,
    )
}

pub(super) async fn admit_peer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    at: Position,
) -> TestResult<crate::foundation::ExactActorRef> {
    admit_peer_with_index(owner, h, at, 0).await
}

pub(super) async fn admit_peer_with_index(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    at: Position,
    index: u8,
) -> TestResult<crate::foundation::ExactActorRef> {
    use crate::foundation::{CharacterId, CharacterLease};
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

    let evidence = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    super::maintain_ready(&h.root).await?;
    let character = super::desert_party_rite_fixtures::create_peer(
        h,
        owner.character,
        &evidence.evidence,
        index,
    )
    .await?;
    let session = super::activity_deadline_paging_fixtures::admit_created_member(
        h,
        &evidence.evidence,
        index,
        character,
    )
    .await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "peer fence")?
        .ok_or("peer fence")?;
    let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("peer cast facts".into());
    };
    let mut runtime = owner.runtime.lock().await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reserve = runtime.reserve_fresh_session(session).map_err(debug)?;
    let peer = runtime.commit_fresh_session(reserve).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(peer, super::quest_world_callbacks_fixtures::local(at))
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(
            peer,
            session,
            CharacterLease::new(
                CharacterId::decode(&character).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .initialize(&runtime, peer, session, facts, (0, 0), owner.owner_now())
        .ok_or("peer spell state")?;
    Ok(peer)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: Callback,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_spatial::StepDirection;

    let primary = admitted.runtime_actor.ok_or("player")?;
    let peer = admit_peer(owner, h, at(33270, 31477)).await?;
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback already installed")?;
    super::questpc7_ferumbras_fixtures::step(owner, admitted, StepDirection::South).await?;
    assert_eq!(
        super::questpc7_ferumbras_fixtures::use_at(owner, admitted, LEVER, 2772).await?,
        UseDisposition::Committed,
    );
    let runtime = owner.runtime.lock().await;
    for actor in [primary, peer] {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks_fixtures::local(ARRIVAL)
        );
    }
    let census = runtime.current_live_creature_census().map_err(debug)?;
    for (key, count) in [(BOSS, 1), (DEMON, 4)] {
        assert_eq!(
            census
                .iter()
                .filter(|actor| runtime.matches_live_creature_identity(**actor, key.as_bytes()))
                .count(),
            count
        );
    }
    drop(runtime);
    let map = owner.bundle_map().ok_or("bundle map")?;
    let has_crystal = |pos: Position, appearance| {
        let pos = crate::map::overlay::TilePos {
            x: pos.x,
            y: pos.y,
            floor: pos.floor,
        };
        map.overlay
            .base()
            .tile(pos.x, pos.y, pos.floor)
            .is_some_and(|tile| {
                tile.ids()
                    .iter()
                    .zip(tile.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .enumerate()
                    .any(|(ordinal, (id, _))| {
                        u8::try_from(ordinal)
                            .ok()
                            .and_then(|ordinal| map.facts.base_entry(pos, ordinal, *id))
                            .is_some_and(|facts| facts.appearance_id == appearance)
                    })
            })
    };
    assert!(
        CRYSTALS
            .into_iter()
            .all(|pos| has_crystal(pos, DORMANT_CRYSTAL))
    );
    let mut command = 100;
    let mut sites = vec![ARRIVAL];
    sites.extend(
        CRYSTALS
            .into_iter()
            .flat_map(|pos| std::iter::repeat_n(at(pos.x + 1, pos.y), usize::from(CHARGE_COUNT))),
    );
    for site in sites {
        let invader = spawn_late(owner, RIFT_INVADER, site).await?;
        super::engenc2_cults_fixtures::kill(owner, admitted, invader, RIFT_INVADER, command)
            .await?;
        let credited = command - 100;
        let charged = CRYSTALS
            .into_iter()
            .filter(|&pos| has_crystal(pos, CHARGED_CRYSTAL))
            .count();
        assert_eq!(charged as u64, credited / u64::from(CHARGE_COUNT));
        if charged != CRYSTALS.len() {
            assert_eq!(actors(owner, BOSS).await?.len(), 1);
            assert!(actors(owner, SPLINTER).await?.is_empty());
        }
        command += 1;
    }
    assert!(actors(owner, BOSS).await?.is_empty());
    let splinters = actors(owner, SPLINTER).await?;
    assert_eq!(splinters.len(), 8);
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .filter(|(_, _, effect)| *effect == CRYSTAL_EFFECT)
            .count(),
        8
    );
    let local = super::quest_world_callbacks_fixtures::local;
    let outside = spawn_late(owner, SPLINTER, OUTSIDE).await?;
    super::engenc2_cults_fixtures::kill(owner, admitted, outside, SPLINTER, command).await?;
    assert!(actors(owner, ESSENCE).await?.is_empty());
    let vortex_pos = crate::map::overlay::TilePos {
        x: VORTEX_TILE.x,
        y: VORTEX_TILE.y,
        floor: VORTEX_TILE.floor,
    };
    assert_eq!(
        map.facts
            .movement_items(vortex_pos)
            .iter()
            .filter(|entry| entry.facts.appearance_id == VORTEX)
            .count(),
        usize::from(CHARGE_COUNT),
        "a replay of a Rift Invader death must not add a second vortex",
    );
    let mut commands = (command + 1)..;
    for (used, &splinter) in splinters.iter().take(VORTEX_THRESHOLD).enumerate() {
        assert_eq!(actors(owner, SPLINTER).await?.len(), splinters.len() - used);
        super::engenc2_cults_fixtures::kill(
            owner,
            admitted,
            splinter,
            SPLINTER,
            commands.next().ok_or("kill command")?,
        )
        .await?;
        let [essence] = actors(owner, ESSENCE)
            .await?
            .try_into()
            .map_err(|_| "one Essence from the slain Splinter")?;
        {
            let mut runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(essence).map_err(debug)?;
            assert_eq!(expected.position(), local(BOSS_AT));
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(essence, expected, local(VORTEX_APPROACH))
                .map_err(debug)?;
        }
        super::zamulosh_monster_step_fixtures::step_monster(owner, h, essence, VORTEX_TILE).await?;
        assert!(actors(owner, ESSENCE).await?.is_empty());
        assert_eq!(
            actors(owner, DESTABILIZED).await?.len(),
            usize::from(used + 1 == VORTEX_THRESHOLD)
        );
    }
    let [destabilized] = actors(owner, DESTABILIZED)
        .await?
        .try_into()
        .map_err(|_| "one Destabilized Ferumbras")?;
    super::engenc2_cults_fixtures::kill(
        owner,
        admitted,
        destabilized,
        DESTABILIZED,
        commands.next().ok_or("kill command")?,
    )
    .await?;
    let [shell] = actors(owner, SHELL)
        .await?
        .try_into()
        .map_err(|_| "one Mortal Shell")?;
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(shell)
        .map_err(debug)?;
    assert_eq!(position.position(), local(BOSS_AT));
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .filter(|(_, pos, fx)| *pos == vortex_pos && *fx == POOF)
            .count(),
        VORTEX_THRESHOLD
    );
    map.facts
        .expire_movement_items(owner.owner_now().get() + u64::from(VORTEX_MS) * 1000);
    assert!(map.facts.movement_items(vortex_pos).is_empty());
    Ok(())
}

async fn actors(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
) -> TestResult<Vec<crate::foundation::ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .filter(|actor| runtime.matches_live_creature_identity(*actor, creature.as_bytes()))
        .collect())
}

async fn spawn_late(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
    position: Position,
) -> TestResult<crate::foundation::ExactActorRef> {
    let mut runtime = owner.runtime.lock().await;
    let health = runtime
        .companion_policy(creature)
        .map_err(debug)?
        .maximum_health;
    let [actor] = runtime
        .commit_quest_creature_spawn(&[(
            creature,
            health,
            super::quest_world_callbacks_fixtures::local(position),
        )])
        .map_err(debug)?
        .try_into()
        .map_err(|_| "one late Ferumbras actor")?;
    Ok(actor)
}
