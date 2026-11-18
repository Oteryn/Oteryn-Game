//! Crystal 201189f8 movements_unlock_entrace.lua:4-16,52-119.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::starter_grant::StarterTemplate;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Offering, Position};
use oteryn_protocol_oteryn::world_spatial::{self, StepDirection};
use serde_json::{Value, json};

pub(super) const NAME: &str = "castle-crystal-rite";
const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission05";
const ENTRY: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/castle_entrance_stg";
const CRYSTAL: &str = "oteryn:item.tibia.i8225";
const STATIONS: [Position; 4] = [
    position(32928, 31460, -7),
    position(32941, 31510, -7),
    position(32966, 31498, -6),
    position(32946, 31499, -6),
];
const DESTINATIONS: [Position; 4] = [
    position(32951, 31479, -6),
    position(32954, 31479, -6),
    position(32951, 31482, -6),
    position(32954, 31482, -6),
];

const fn position(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

#[test]
fn castle_crystal_step_commits_four_station_rite_only_when_every_member_qualifies() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, STATIONS[0], false)
    })
}

fn binding() -> Value {
    let stations: Vec<_> = STATIONS
        .into_iter()
        .zip(DESTINATIONS)
        .map(|(from, to)| json!({"from":from,"to":to}))
        .collect();
    json!({
        "key":"oteryn:quest-callback/fixture-castle-crystal-rite",
        "quest":"oteryn:quest.blood_brothers_quest","trigger":"step_in",
        "selector":{"position":STATIONS[0],"position_aliases":STATIONS[1..],"ordinal":0},
        "lever":null,"reversible":false,"teleport":null,"offerings":[],"passages":[],
        "item_on_target":null,"return_on_failure":false,"failure_message":null,
        "requires_world_admission":true,
        "storage":[
            {"track":MISSION,"minimum":3,"maximum":4,"set":3,"preserve":true},
            {"track":ENTRY,"minimum":0,"maximum":1,"set":1}
        ],
        "effects":{"party_rite":{"stations":stations,"admission_item":CRYSTAL,"entry_track":ENTRY}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/movements_unlock_entrace.lua:4-119"]
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(binding())?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":[binding()]}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let template = StarterTemplate {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..StarterTemplate::main()
    };
    let mut members = vec![*admitted];
    for (index, at) in STATIONS.into_iter().enumerate().skip(1) {
        let seed = u8::try_from(index - 1)?;
        let character =
            super::desert_party_rite::create_peer(h, authority, owner.evidence, seed).await?;
        let session = super::activity_deadline_paging::admit_created_member(
            h,
            owner.evidence,
            seed,
            character,
        )
        .await?;
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(actor, super::quest_world_callbacks::local(at))
                .map_err(debug)?;
            actor
        };
        let mut peer = *admitted;
        peer.game_session_id = session;
        peer.runtime_actor = Some(actor);
        owner.admit_progression_session(&mut peer).await;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("peer fence")?;
        h.root
            .grant_starter_items(
                authority,
                &h.node,
                &fence.item_fence(),
                &NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        members.push(peer);
    }
    for (index, member) in members.iter().enumerate() {
        let entered = i64::from(index == 3);
        super::qengine_outfit_stories::seed_quest_prerequisites(
            h,
            owner,
            member,
            &[(MISSION.into(), 3), (ENTRY.into(), entered)],
            210 + u8::try_from(index)?,
        )
        .await?;
        if index < 2 {
            give_crystal(owner, member, index).await?;
        }
    }

    step(owner, admitted, StepDirection::East).await?;
    verify_positions(owner, &members, &STATIONS).await?;
    assert_eq!(entry_flags(h).await?, vec![0, 0, 0, 1]);

    step(owner, admitted, StepDirection::West).await?;
    give_crystal(owner, &members[2], 2).await?;
    step(owner, admitted, StepDirection::East).await?;
    verify_positions(owner, &members, &DESTINATIONS).await?;
    assert_eq!(entry_flags(h).await?, vec![1, 1, 1, 1]);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/fixture-castle-crystal-rite'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 3);
    assert_eq!(super::m39_bone_journey::carried(h, 8225).await?, 3);
    Ok(())
}

async fn give_crystal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
    index: usize,
) -> TestResult {
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("crystal holder fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: STATIONS[index],
            item: CRYSTAL.into(),
            count: 1,
        },
        78 + u64::try_from(index)?,
        true,
    )
    .await
}

async fn entry_flags(h: &Harness) -> TestResult<Vec<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1 ORDER BY value",
    )
    .bind(ENTRY)
    .fetch_all(&h.pool)
    .await?)
}

async fn verify_positions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    members: &[AdmittedSession],
    positions: &[Position],
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    for (member, &at) in members.iter().zip(positions) {
        let actor = member.runtime_actor.ok_or("station actor")?;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(at)
        );
    }
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(5))
        .ok_or("owner clock")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        world_spatial::decode_step_result(&reply).map_err(debug)?,
        world_spatial::StepDisposition::Moved
    );
    Ok(())
}
