//! Crystal 201189f8, ferumbras_ascension/actions_ferumbras_lever.lua:1-62.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const CASE: &str = "ferumbras-crystal-reset";
const LEVER: Position = at(33270, 31476);
const ARRIVAL: Position = at(33392, 31479);
const CRYSTALS: [Position; 8] = [
    at(33390, 31468), at(33394, 31468), at(33397, 31471), at(33397, 31475),
    at(33394, 31478), at(33390, 31478), at(33387, 31475), at(33387, 31471),
];
const BOSS: &str = "oteryn:creature.ascending_ferumbras";
const DEMON: &str = "oteryn:creature.enthralled_demon";

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

#[test]
fn ferumbras_lever_stacks_two_staged_players_and_resets_the_eight_crystals() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, LEVER, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    let players: Vec<_> = (33269..=33271)
        .flat_map(|x| (31477..=31481).map(move |y| serde_json::json!({"from": at(x, y), "to": ARRIVAL})))
        .collect();
    let mut monsters = vec![serde_json::json!({"creature": BOSS, "position": at(33392,31473)})];
    monsters.extend([at(33385,31466), at(33396,31466), at(33392,31480), at(33392,31468)]
        .map(|position| serde_json::json!({"creature": DEMON, "position": position})));
    let edits = CRYSTALS.map(|position| serde_json::json!({
        "position": position, "remove": 14961, "create": 14955, "passable": false, "allow_missing": true
    }));
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
                "room_min":at(33379,31460),"room_max":at(33405,31485),"shared_staging":true
            }
        },
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/actions_ferumbras_lever.lua:1-62"]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into().map_err(|_| "one Ferumbras callback")?;
    Ok(callback)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_objects(
        callback, LEVER, &CRYSTALS.map(|position| (position, 14961)), &[14955],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: Callback,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    use crate::map::view::MapFacts;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_protocol_oteryn::world_spatial::StepDirection;

    let evidence = super::house_transfer::RecoverySource::start(h).await?;
    super::maintain_ready(&h.root).await?;
    let character = super::desert_party_rite::create_peer(h, owner.character, &evidence.evidence, 0).await?;
    let session = super::activity_deadline_paging::admit_created_member(h, &evidence.evidence, 0, character).await?;
    let fence = owner.current_quest_fence(session).await.map_err(|()| "peer fence")?.ok_or("peer fence")?;
    let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root, owner.character, fence.character_id,
        ).await
    else { return Err("peer cast facts".into()); };
    let primary = admitted.runtime_actor.ok_or("player")?;
    let peer = {
        let mut runtime = owner.runtime.lock().await;
        let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native policies")?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let reserve = runtime.reserve_fresh_session(session).map_err(debug)?;
        let peer = runtime.commit_fresh_session(reserve).map_err(debug)?;
        runtime.initialize_pinned_test_position(peer, super::quest_world_callbacks::local(at(33270,31477))).map_err(debug)?;
        runtime.bind_attacker_lease(peer, session,
            CharacterLease::new(CharacterId::decode(&character).map_err(debug)?, fence.character_lease_generation).map_err(debug)?
        ).map_err(debug)?;
        owner.states.spell_states.lock().await.initialize(
            &runtime, peer, session, facts, (0, 0), owner.owner_now(),
        ).ok_or("peer spell state")?;
        peer
    };
    owner.states.quest_callbacks.fixture.set(vec![callback]).map_err(|_| "callback already installed")?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::South).await?;
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, LEVER, 2772).await?,
        UseDisposition::Committed,
    );
    let runtime = owner.runtime.lock().await;
    for actor in [primary, peer] {
        assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(),
            super::quest_world_callbacks::local(ARRIVAL));
    }
    let census = runtime.current_live_creature_census().map_err(debug)?;
    for (key, count) in [(BOSS, 1), (DEMON, 4)] {
        assert_eq!(census.iter()
            .filter(|actor| runtime.matches_live_creature_identity(**actor, key.as_bytes())).count(), count);
    }
    drop(runtime);
    let map = owner.bundle_map().ok_or("bundle map")?;
    for pos in CRYSTALS {
        let pos = crate::map::overlay::TilePos {x:pos.x,y:pos.y,floor:pos.floor};
        let tile = map.overlay.base().tile(pos.x,pos.y,pos.floor).ok_or("crystal tile")?;
        assert!(tile.ids().iter().zip(tile.depths()).filter(|(_, depth)| **depth == 0)
            .enumerate().any(|(ordinal,(id,_))| u8::try_from(ordinal).ok()
                .and_then(|ordinal| map.facts.base_entry(pos,ordinal,*id))
                .is_some_and(|facts| facts.appearance_id == 14955)));
    }
    Ok(())
}
