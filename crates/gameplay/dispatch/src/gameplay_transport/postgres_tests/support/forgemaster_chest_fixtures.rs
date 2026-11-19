use super::fixture::{Harness, debug};
use super::m07_realm_trials_fixtures::use_source;
use super::quest_world_callbacks_fixtures::{local, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, NativeSelector, Position},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use std::sync::Arc;

pub(super) const LOCKED: &str = "forgemaster-chest-locked";
pub(super) const NATIVE: &str = "forgemaster-chest-native";
pub(super) const OPEN: &str = "forgemaster-chest-open";
pub(super) const UNADMITTED: &str = "forgemaster-chest-unadmitted-item";
// Crystal startup/tables/item.lua:5117-5160 binds the UIDs in this row.
// Retained B3 region z14-x133-y126 has these objects above the ground.
pub(super) const CHEST: Position = Position {
    x: 34087,
    y: 32346,
    floor: -14,
};
pub(super) const BETWEEN_CHESTS: Position = Position { x: 34088, ..CHEST };
const UID: u16 = 57237;
const CHEST_APPEARANCE: u16 = 2472;
// Crystal no_rest_for_the_wicked_rewards.lua:1-13 pairs each UID with a prize.
const CHESTS: [(u16, u16, &[u16], u16); 11] = [
    (57237, 34087, &[2187], 49520),
    (57238, 34089, &[2187], 49522),
    (57239, 34091, &[2187], 49523),
    (57240, 34093, &[2187], 49524),
    (57241, 34095, &[2187], 49525),
    (57242, 34097, &[2187], 49526),
    (57243, 34099, &[2187], 49527),
    (57244, 34101, &[2187], 49530),
    (57245, 34103, &[2187], 49528),
    (57246, 34105, &[2187], 49529),
    (57247, 34107, &[6381, 6385], 50250),
];
const FIXTURE_REWARD_ID: u16 = 3031;
const QUEST: &str = "oteryn:quest.no_rest_for_the_wicked_quest";
const ACCESS: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forge_master/access_door";
const CLAIM: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/reward";
const PHASE: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let item = reward_id(name);
    let selector = if name == NATIVE {
        json!({"native": native_selector(CHESTS[0].1, CHESTS[0].2)?})
    } else {
        json!({"unique":UID,"appearance":CHEST_APPEARANCE})
    };
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/fixture/forgemaster-chest-57237",
        "quest":QUEST,"trigger":"use","actor_kind":"player",
        "selector":selector,
        "storage":[
            {"track":ACCESS,"minimum":1,"maximum":1,"set":1,"preserve":true},
            {"track":CLAIM,"minimum":0,"maximum":0,"set":1},
            {"track":PHASE,"minimum":0,"maximum":3,"set":3}
        ],
        "effects":{"item_reward":{"item":format!("oteryn:item.tibia.i{item}"),"count":1,"contents":[]}},
        "passages":[],"offerings":[],"return_on_failure":false,"reversible":false,
        "requires_world_admission":true,
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/the_forgemaster.lua:176-180",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:5117-5160",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/no_rest_for_the_wicked/no_rest_for_the_wicked_rewards.lua:1-32"
        ]
    }))?)
}

fn native_selector(x: u16, under_chest: &[u16]) -> TestResult<NativeSelector> {
    Ok(NativeSelector {
        position: Position { x, ..CHEST },
        ordinal: u8::try_from(under_chest.len() + 1)?,
        appearance: CHEST_APPEARANCE,
    })
}

pub(super) fn native_world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let first = native_selector(CHESTS[0].1, CHESTS[0].2)?;
    let mut objects = Vec::new();
    let mut approach = Vec::new();
    for &(_, x, under_chest, _) in &CHESTS[1..] {
        let position = Position { x, ..CHEST };
        objects.extend(
            under_chest
                .iter()
                .chain(&[CHEST_APPEARANCE])
                .map(|&id| (position, id)),
        );
        approach.push(Position {
            y: position.y + 1,
            ..position
        });
    }
    world_cells(
        callback,
        first.position,
        first.appearance,
        first.ordinal,
        &objects,
        &[],
        None,
        &approach,
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    actor: ExactActorRef,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let has_access = name != LOCKED;
    let can_claim = matches!(name, NATIVE | OPEN);
    let ordinal = callback
        .selector
        .native
        .as_ref()
        .map_or(1, |native| native.ordinal);
    let item = reward_id(name);
    let mut stages = vec![json!({"key":"chest-57237","completes":false,
        "trigger":{"kind":"map_use","callback":callback}})];
    let empty_message = "The chest is empty.";
    if name == NATIVE {
        for &(uid, x, under_chest, _) in &CHESTS[1..] {
            let mut other = callback.clone();
            other.key = format!("oteryn:quest-callback/fixture/forgemaster-chest-{uid}");
            other.selector.native = Some(native_selector(x, under_chest)?);
            other.failure_message = Some(empty_message.into());
            stages.push(json!({"key":format!("chest-{uid}"),"completes":false,
                "trigger":{"kind":"map_use","callback":other}}));
        }
        owner.states.chat.admit(
            admitted.game_session_id,
            "Forge claimant",
            owner.owner_now(),
        );
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
            "sources":callback.sources,
            "tracks":[
                {"key":ACCESS,"initial":i64::from(has_access),"minimum":0,"maximum":1},
                {"key":CLAIM,"initial":0,"minimum":0,"maximum":1},
                {"key":PHASE,"initial":if has_access { 3 } else { 2 },"minimum":0,"maximum":3}
            ],
            "stages":stages
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let before = super::qengine_voucher_stories_fixtures::quantity(harness, item).await?;
    if name == NATIVE {
        // The interaction targets the chest entry, not the tile's other object.
        assert_ne!(
            use_source(owner, admitted, 1, CHEST, ordinal - 1, 0).await?,
            UseDisposition::Committed,
        );
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(harness, item).await?,
            before
        );
    }
    let result = use_source(owner, admitted, 2, CHEST, ordinal, 0).await?;
    assert_eq!(
        result,
        if can_claim {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        }
    );
    if can_claim {
        assert_eq!(
            use_source(owner, admitted, 2, CHEST, ordinal, 0).await?,
            UseDisposition::Committed,
            "the same USE replays its original result",
        );
        assert_eq!(
            use_source(owner, admitted, 3, CHEST, ordinal, 0).await?,
            UseDisposition::Rejected
        );
    }
    if name == NATIVE {
        // Crystal uses one Reward storage for every chest UID. Use each bare
        // placement afresh and require its own empty message from the owner.
        let session = admitted.game_session_id;
        owner.states.chat.drain(session);
        for (index, &(uid, x, under_chest, _)) in CHESTS[1..].iter().enumerate() {
            let source = native_selector(x, under_chest)?;
            let approach = Position {
                y: source.position.y + 1,
                ..source.position
            };
            {
                let mut runtime = owner.runtime.lock().await;
                let expected = runtime.read_actor_position(actor).map_err(debug)?;
                let doors =
                    crate::map::floor::BundleDoors(&owner.bundle_map().ok_or("chest map")?.facts);
                let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(approach),
                    false,
                    &doors,
                    None,
                )
                .map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)?;
            }
            let command = u64::try_from(index)? + 4;
            assert_eq!(
                use_source(owner, admitted, command, source.position, source.ordinal, 0).await?,
                UseDisposition::Rejected,
                "chest {uid}"
            );
            assert!(owner.states.chat.drain(session).iter().any(|line|
                matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == empty_message)),
                "chest {uid} must resolve its native selector and report the shared claim");
        }
    }
    assert_eq!(
        super::qengine_voucher_stories_fixtures::quantity(harness, item).await?,
        before + i64::from(can_claim),
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(CLAIM).copied(), can_claim.then_some(1));
    Ok(())
}

fn reward_id(name: &str) -> u16 {
    if name == UNADMITTED {
        CHESTS[0].3
    } else {
        FIXTURE_REWARD_ID
    }
}
