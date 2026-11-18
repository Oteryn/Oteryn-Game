use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::content::{ReferenceRationalPercent, ReferenceResistanceKind};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::{FreshAdmissionAuthority, connection::UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "qengine-rewards-rahemos-hat";
const TARGET: Position = Position { x: 101, y: 100, floor: -7 };

#[test]
fn rahemos_lever_roll_leaves_hat_closed_on_damage_and_opens_on_success() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback(at: Position) -> TestResult<Callback> {
    // Crystal 201189f8 actions_oasis_lever_door.lua:22-44.
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/rahemos-hat-fixture",
        "quest":"oteryn:quest.the_ancient_tombs_quest","trigger":"use",
        "selector":{"position":at,"ordinal":1,"appearance":2772},
        "lever":[2772,2773],"reversible":false,"passages":[],"storage":[],"offerings":[],
        "return_on_failure":false,
        "effects":{"reset_after_ms":4000,"use_chance":{
            "faces":3,"winning_faces":1,
            "failure_hit":{"kind":"physical","amount":200,"effect":3},
            "failure_message":"You guessed wrong! Take this! Carrot changed now the Hat!"
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/actions_oasis_lever_door.lua:22-45"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let row = callback(TARGET)?;
    assert!(owner.states.quest_callbacks.fixture.set(vec![row.clone()]).is_ok());
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 1)
        .ok_or("source key")?;
    let (loss, win) = {
        let runtime = owner.runtime.lock().await;
        owner.states.spell_states.lock().await
            .get_mut(&runtime, actor, session).ok_or("state")?
            .install_equipped_element_protection(ReferenceResistanceKind::Physical,
                Some(ReferenceRationalPercent::new(95,1).map_err(debug)?));
        let chance = row.effects.use_chance.as_ref().ok_or("chance")?;
        let mut loss = None;
        let mut win = None;
        for id in 20..256 {
            let command = CommandRef::new(session, CommandId::new(id).map_err(debug)?);
            let wins = crate::gameplay_transport::quest_callbacks::use_chance::wins(
                &runtime, actor, command, &row.key, key, chance
            ).ok_or("decision")?;
            if wins && loss.is_some() {
                win = Some(id);
                break;
            }
            if !wins {
                loss.get_or_insert(id);
            }
        }
        (loss.ok_or("losing occurrence")?, win.ok_or("winning occurrence")?)
    };
    let appearance = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<u16> {
        let map = owner.bundle_map().ok_or("map")?;
        let tile = crate::map::overlay::TilePos { x: TARGET.x, y: TARGET.y, floor: TARGET.floor };
        let id = *map.overlay.base().tile(tile.x,tile.y,tile.floor).ok_or("tile")?
            .ids().get(1).ok_or("lever")?;
        Ok(map.facts.base_entry(tile,1,id).ok_or("entry")?.appearance_id)
    };
    let before = super::underwater_drowning::health(owner, actor, session).await?;
    for (command_id, expected) in [(loss,2772),(win,2773)] {
        owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        let mut placement=map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let result = owner.use_object(actor, UseCommand {
            game_session_id:session, command_id, item_fence:admitted.item_fence,
        }, WorldObjectTarget {
            placement, expected_revision:map.facts.object_revision(key),
        }).await;
        assert_eq!(result.disposition, UseDisposition::Committed);
        assert_eq!(appearance(owner)?, expected);
        let remaining = super::underwater_drowning::health(owner, actor, session).await?;
        assert!((1..=10).contains(&before.saturating_sub(remaining)));
    }
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(5)).ok_or("clock")?;
    owner.tick_timed_items().await;
    assert_eq!(appearance(owner)?,2772);
    Ok(())
}
