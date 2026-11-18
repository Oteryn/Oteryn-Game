//! Reuses genuine Characters in the party-rite fixture to test the carried
//! branch of Crystal 201189f8 elemental_spheres/actions_lever.lua:35-68.
use super::desert_party_rite::{current, positions, pull_next, relocate};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    reward_callbacks::ItemAmount,
    world_callbacks::{Callback, Offering, StationProgress, Storage},
};
use oteryn_protocol_oteryn::{
    world_object::{UseDisposition, WorldObjectTarget},
    world_spatial::{StepDirection, StepDisposition},
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const TRACK: &str = "oteryn:quest-progress/elemental_spheres/quest_line";

#[test]
fn lord_chamber_checks_each_station_holders_carried_item_before_group_travel() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("qespike-lord-stations", || {
        super::desert_party_rite::scenario_with_station_custody(false, Some(4))
    })
}

#[test]
fn world_party_custody_transfers_two_players_and_dispatches_their_exits() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("qespike-two-station-round-trip", || {
        super::desert_party_rite::scenario_with_station_custody(false, Some(2))
    })
}

pub(super) fn configure(callback: &mut Callback) -> TestResult {
    // This fixture borrows the active map and native items from the Desert rite.
    // The production Elemental Spheres binding supplies the four actual soils.
    callback.key = "oteryn:quest-callback/qespike/lord-carried-stations".into();
    callback.quest = QUEST.into();
    callback.requires_world_admission = true;
    callback.storage = vec![Storage {
        track: TRACK.into(),
        minimum: 1,
        maximum: 3,
        set: 1,
        preserve: true,
        read_only: false,
        set_if: None,
        add: None,
        increment: false,
        at_least: false,
    }];
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_lever.lua:35-68".into()];
    let party = callback.effects.party_transfer.as_mut().ok_or("party")?;
    party.activation_appearance = 2773;
    party.initiator_position = Some(party.stations[2].from);
    for station in &mut party.stations {
        let input = callback
            .offerings
            .iter()
            .find(|item| Some(item.position) == station.offering)
            .ok_or("fixture offering")?;
        station.offering = None;
        station.carried = vec![ItemAmount {
            item: input.item.clone(),
            count: 1,
            fluid_subtype: None,
        }];
        station.progress = Some(StationProgress {
            track: TRACK.into(),
            minimum: 1,
        });
    }
    callback.offerings.clear();
    Ok(())
}

pub(super) fn install(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    callback: &Callback,
) -> TestResult {
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: 1,
                min: -1,
                max: 3,
            }],
            Vec::new(),
        )
        .map_err(debug)?
        .with_world_callback_fixture(callback.clone())?,
    ));
    let party = callback
        .effects
        .party_transfer
        .as_ref()
        .ok_or("party transfer")?;
    let mut callbacks = vec![callback.clone()];
    if party.stations.len() == 2 {
        // Both Desert arrivals have an unoccupied walkable tile immediately
        // east. Fixture exits use the ordinary authored StepOut dispatch.
        for (index, station) in party.stations.iter().enumerate() {
            callbacks.push(serde_json::from_value(serde_json::json!({
                "key":format!("oteryn:quest-callback/qespike/station-exit-{index}"),
                "quest":QUEST,
                "trigger":"step_out",
                "selector":{"position":station.to,"ordinal":0},
                "teleport":station.from,
                "reversible":false,
                "passages":[],"storage":[],"offerings":[],"achievements":[],
                "return_on_failure":false,
                "sources":callback.sources,
            }))?);
        }
    }
    owner
        .states
        .quest_callbacks
        .fixture
        .set(callbacks)
        .map_err(|_| "station fixture")?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    members: &[(ExactActorRef, GameSessionId)],
    h: &Harness,
    key: u64,
    target: &WorldObjectTarget,
    callback: &Callback,
) -> TestResult {
    let party = callback.effects.party_transfer.as_ref().ok_or("party")?;
    let initiator = members
        .iter()
        .position(|(_, session)| *session == admitted.game_session_id)
        .ok_or("initiator member")?;
    let last = members.len().checked_sub(1).ok_or("station members")?;
    relocate(
        owner,
        members[initiator].0,
        members[initiator].1,
        party.stations[initiator].from,
    )
    .await?;
    let before = positions(owner, members).await?;
    for (index, station) in party.stations.iter().enumerate() {
        // Put the last station's required item in the initiator's bag first.
        let holder = if index == last { initiator } else { index };
        let fence = current(owner, members[holder].1).await?.item_fence();
        super::quest_world_callbacks::input_item(
            owner,
            fence,
            &Offering {
                position: party.stations[holder].from,
                item: station.carried[0].item.clone(),
                count: 1,
            },
            7400 + u64::try_from(index)?,
            true,
        )
        .await?;
    }
    assert_eq!(
        pull_next(owner, admitted, target).await?,
        UseDisposition::Rejected
    );
    assert_eq!(positions(owner, members).await?, before);
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0
    );
    let fence = current(owner, members[last].1).await?.item_fence();
    super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: party.stations[last].from,
            item: party.stations[last].carried[0].item.clone(),
            count: 1,
        },
        7404,
        true,
    )
    .await?;
    assert_eq!(
        pull_next(owner, admitted, target).await?,
        UseDisposition::Committed
    );

    let arrivals = positions(owner, members).await?;
    assert_eq!(
        arrivals,
        party
            .stations
            .iter()
            .map(|station| super::quest_world_callbacks::local(station.to))
            .collect::<Vec<_>>()
    );
    let burns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1",
    )
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(burns, 0);
    if members.len() == 2 {
        let mut expected = arrivals;
        for (index, (actor, session)) in members.iter().enumerate() {
            let (outcome, _) = owner
                .paced_step(*actor, *session, StepDirection::East)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
            expected[index] = super::quest_world_callbacks::local(party.stations[index].from);
            assert_eq!(positions(owner, members).await?, expected);
        }
    }
    Ok(())
}
