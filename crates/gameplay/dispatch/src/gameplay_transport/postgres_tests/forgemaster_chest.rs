//! Crystal 201189f8, the_forgemaster.lua:176-180 and
//! no_rest_for_the_wicked_rewards.lua:1-32. The positive fixture uses an admitted
//! item to prove the shared claim owner while the donor item awaits admission.
use super::fixture::{Harness, debug};
use super::m07_realm_trials::use_source;
use super::quest_world_callbacks::scenario;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use std::sync::Arc;

pub(super) const LOCKED: &str = "forgemaster-chest-locked";
pub(super) const NATIVE: &str = "forgemaster-chest-native";
pub(super) const OPEN: &str = "forgemaster-chest-open";
pub(super) const UNADMITTED: &str = "forgemaster-chest-unadmitted-item";
// Crystal startup/tables/item.lua:5117-5120 binds UID 57237 at startup.
// The pinned B3 tile retains bare item 2472 above two top-level entries.
const CHEST: Position = Position { x: 34087, y: 32346, floor: -14 };
const UID: u16 = 57237;
const CHEST_APPEARANCE: u16 = 2472;
const NATIVE_ORDINAL: u8 = 2;
const DONOR_REWARD_ID: u16 = 49520;
const FIXTURE_REWARD_ID: u16 = 3031;
const QUEST: &str = "oteryn:quest.no_rest_for_the_wicked_quest";
const ACCESS: &str =
    "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forge_master/access_door";
const CLAIM: &str =
    "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/reward";
const PHASE: &str =
    "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";

#[test]
fn forgemaster_chest_requires_door_access_and_claims_the_retained_reward_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    for case in [LOCKED, OPEN, UNADMITTED] {
        super::kill_reward_e2e::scenario_thread::run(case, move || scenario(case, CHEST, false))?;
    }
    Ok(())
}

#[test]
fn forgemaster_chest_use_matches_the_untagged_native_placement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(NATIVE, || scenario(NATIVE, CHEST, false))
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let item = reward_id(name);
    let selector = if name == NATIVE {
        json!({"position":CHEST,"ordinal":NATIVE_ORDINAL,"appearance":CHEST_APPEARANCE})
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
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:5117-5120",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/no_rest_for_the_wicked/no_rest_for_the_wicked_rewards.lua:1-32"
        ]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let has_access = name != LOCKED;
    let can_claim = matches!(name, NATIVE | OPEN);
    let ordinal = if name == NATIVE { NATIVE_ORDINAL } else { 1 };
    let item = reward_id(name);
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
            "stages":[{"key":"chest-57237","completes":false,
                "trigger":{"kind":"map_use","callback":callback}}]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let before = super::qengine_voucher_stories::quantity(harness, item).await?;
    if name == NATIVE {
        // The interaction targets the chest entry, not the tile's other object.
        assert_ne!(
            use_source(owner, admitted, 1, CHEST, NATIVE_ORDINAL - 1, 0).await?,
            UseDisposition::Committed,
        );
        assert_eq!(super::qengine_voucher_stories::quantity(harness, item).await?, before);
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
            use_source(owner, admitted, 3, CHEST, ordinal, 0).await?,
            UseDisposition::Rejected
        );
    }
    assert_eq!(
        super::qengine_voucher_stories::quantity(harness, item).await?,
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
        DONOR_REWARD_ID
    } else {
        FIXTURE_REWARD_ID
    }
}
