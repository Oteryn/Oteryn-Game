//! Lane qc: Twenty Miles Beneath the Sea. The NPC chain runs through native wire
//! conversations; every world step is its own scenario named after its callback key.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CharacterId, CharacterLease};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const START: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const P: &str = "oteryn:quest-progress/qc/twenty_miles/";
const QUEST: &str = "oteryn:quest.twenty_miles_beneath_the_sea_quest";

pub(super) fn matches_chain(name: &str) -> bool {
    name == "qc-twenty-miles"
}

pub(super) fn matches_world(name: &str) -> bool {
    name.starts_with("qc-twenty-miles-")
}

#[test]
fn qc_twenty_miles_npc_chain_from_brandon_to_the_time_compass() -> TestResult {
    run("qc-twenty-miles")
}

#[test]
fn qc_twenty_miles_pile_lays_one_skull_once() -> TestResult {
    run("qc-twenty-miles-pile-1")
}

#[test]
fn qc_twenty_miles_altar_takes_a_skull_and_counts_it() -> TestResult {
    run("qc-twenty-miles-altar-1")
}

#[test]
fn qc_twenty_miles_hoard_takes_cursed_gold_and_opens_the_way() -> TestResult {
    run("qc-twenty-miles-hoard-1")
}

#[test]
fn qc_twenty_miles_ring_chest_grants_the_wedding_ring_once() -> TestResult {
    run("qc-twenty-miles-ring-chest")
}

#[test]
fn qc_twenty_miles_shipwreck_chest_grants_the_first_slab_once() -> TestResult {
    run("qc-twenty-miles-shipwreck-chest")
}

#[test]
fn qc_twenty_miles_shrine_chest_grants_the_second_slab_once() -> TestResult {
    run("qc-twenty-miles-shrine-chest")
}

#[test]
fn qc_twenty_miles_broken_altar_takes_its_slab() -> TestResult {
    run("qc-twenty-miles-broken-altar-1")
}

#[test]
fn qc_twenty_miles_sundial_trades_the_sea_ear_for_the_compass() -> TestResult {
    run("qc-twenty-miles-sundial")
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let trigger = if matches_chain(name) {
        START
    } else {
        oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == format!("oteryn:quest-callback/{name}"))
            .and_then(|c| c.selector.position)
            .ok_or("qc twenty miles callback")?
    };
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, trigger, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn track(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
    )
    .bind(key)
    .fetch_optional(&h.pool)
    .await?
    .unwrap_or(-1))
}

async fn held(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances \
         WHERE lifecycle=1 AND definition_production_key=$1",
    )
    .bind(item)
    .fetch_one(&h.pool)
    .await?)
}

async fn completed(h: &Harness, quest: &str) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE \
         character_id=encode($1,'hex')::uuid AND quest_key=$2 \
         AND completed_character_revision IS NOT NULL)",
    )
    .bind(id(41).as_slice())
    .bind(quest)
    .fetch_one(&h.pool)
    .await?)
}

async fn mint(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: &str,
    nonce: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: START.x - 1,
                ..START
            },
            item: item.into(),
            count: 1,
        },
        nonce,
        true,
    )
    .await
}

fn item(number: u32) -> String {
    format!("oteryn:item.tibia.i{number}")
}

fn callback<'a>(callbacks: &'a [RewardCallback], name: &str) -> TestResult<&'a RewardCallback> {
    callbacks
        .iter()
        .find(|c| c.key == format!("oteryn:quest-reward/{name}"))
        .ok_or_else(|| format!("missing callback {name}").into())
}

/// One state-seeding call per occurrence number; the harness owner needs unique ones.
struct Seeder(u8);

impl Seeder {
    async fn set(
        &mut self,
        h: &Harness,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        admitted: &AdmittedSession,
        values: &[(String, i64)],
    ) -> TestResult {
        self.0 += 1;
        super::qengine_outfit_stories::seed_quest_prerequisites(h, owner, admitted, values, self.0)
            .await
    }
}

fn select_npc_capabilities(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    Ok(())
}

pub(super) async fn exercise_chain(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    select_npc_capabilities(owner, admitted)?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
    let [brandon, guardian, yeo] = super::classic_quests::install(
        owner,
        admitted,
        [
            "oteryn:npc.mr_brandon",
            "oteryn:npc.guardian_of_the_deep_seas",
            "oteryn:npc.yeo",
        ],
    )
    .await?;
    let stage_key = format!("{P}stage");
    let mut seeder = Seeder(100);
    let mut nonce = 21_000_u64;
    macro_rules! say {
        ($npc:expr, $name:expr, $words:expr) => {{
            let cb = callback(&callbacks, $name)?;
            let (result, _, _) = super::qengine_rewards::wire_conversation(
                owner,
                admitted,
                $npc,
                cb,
                false,
                $words.to_vec(),
            )
            .await?;
            result
        }};
    }
    macro_rules! refused {
        ($npc:expr, $name:expr, $words:expr, $stage:expr) => {{
            assert_ne!(say!($npc, $name, $words), NpcIntentDisposition::Ok, $name);
            assert_eq!(track(h, &stage_key).await?, $stage, "refused: {}", $name);
        }};
    }
    macro_rules! accepted {
        ($npc:expr, $name:expr, $words:expr, $stage:expr) => {{
            assert_eq!(say!($npc, $name, $words), NpcIntentDisposition::Ok, $name);
            assert_eq!(track(h, &stage_key).await?, $stage, "accepted: {}", $name);
            assert!(!completed(h, QUEST).await?, "complete before the compass: {}", $name);
        }};
    }
    macro_rules! stage {
        ($value:expr) => {
            seeder
                .set(h, owner, admitted, &[(stage_key.clone(), $value)])
                .await?
        };
    }
    macro_rules! give {
        ($number:expr) => {{
            nonce += 1;
            mint(owner, admitted, &item($number), nonce).await?
        }};
    }

    // Nothing starts without Brandon: the Guardian refuses an unstarted quest.
    refused!(guardian, "qc-twenty-miles-guardian-yes-start", ["hi", "assist", "yes"], -1);
    accepted!(brandon, "qc-twenty-miles-brandon-yes", ["hi", "assist", "yes"], 1);
    // The start is once only.
    refused!(brandon, "qc-twenty-miles-brandon-yes", ["hi", "assist", "yes"], 1);
    accepted!(guardian, "qc-twenty-miles-guardian-yes-start", ["hi", "assist", "yes"], 2);

    // Five skulls laid: four are not enough, five earn the cursed gold once.
    let skulls = format!("{P}skulls");
    for laid in [0_i64, 4] {
        if laid > 0 {
            seeder.set(h, owner, admitted, &[(skulls.clone(), laid)]).await?;
        }
        refused!(guardian, "qc-twenty-miles-guardian-skulls-laid", ["hi", "yes"], 2);
        assert_eq!(held(h, &item(21746)).await?, 0);
    }
    seeder.set(h, owner, admitted, &[(skulls.clone(), 5)]).await?;
    accepted!(guardian, "qc-twenty-miles-guardian-skulls-laid", ["hi", "yes"], 3);
    assert_eq!(held(h, &item(21746)).await?, 1);
    refused!(guardian, "qc-twenty-miles-guardian-skulls-laid", ["hi", "yes"], 3);
    assert_eq!(held(h, &item(21746)).await?, 1, "the gold is granted once");

    // The hoard and the ring chest are world scenarios; Brandon takes the ring.
    stage!(5);
    refused!(brandon, "qc-twenty-miles-brandon-ring", ["hi", "wedding ring"], 5);
    give!(21745);
    accepted!(brandon, "qc-twenty-miles-brandon-ring", ["hi", "wedding ring"], 6);
    assert_eq!(held(h, &item(21745)).await?, 0, "Brandon keeps the ring");
    assert_eq!(track(h, &format!("{P}access_west")).await?, 1);
    refused!(brandon, "qc-twenty-miles-brandon-ring", ["hi", "wedding ring"], 6);

    // The slab mission.
    accepted!(brandon, "qc-twenty-miles-brandon-mission-slabs", ["hi", "mission"], 7);
    accepted!(guardian, "qc-twenty-miles-guardian-mission-slabs", ["hi", "mission"], 8);
    stage!(9);
    refused!(guardian, "qc-twenty-miles-guardian-first-slab", ["hi", "mission"], 9);
    give!(21753);
    accepted!(guardian, "qc-twenty-miles-guardian-first-slab", ["hi", "mission"], 10);
    assert_eq!(held(h, &item(21753)).await?, 0);
    stage!(11);
    refused!(guardian, "qc-twenty-miles-guardian-second-slab", ["hi", "mission"], 11);
    give!(21861);
    accepted!(guardian, "qc-twenty-miles-guardian-second-slab", ["hi", "mission"], 12);
    assert_eq!(held(h, &item(21861)).await?, 0);
    accepted!(brandon, "qc-twenty-miles-brandon-yeo", ["hi", "yeo"], 13);
    // Yeo exchanges the pearl for the third slab only against the pearl.
    refused!(yeo, "qc-twenty-miles-yeo-slab", ["hi", "nacre slab", "yes"], 13);
    assert_eq!(held(h, &item(21862)).await?, 0);
    give!(21747);
    accepted!(yeo, "qc-twenty-miles-yeo-slab", ["hi", "nacre slab", "yes"], 14);
    assert_eq!(held(h, &item(21747)).await?, 0);
    assert_eq!(held(h, &item(21862)).await?, 1);
    accepted!(guardian, "qc-twenty-miles-guardian-third-slab", ["hi", "mission"], 15);
    assert_eq!(held(h, &item(21862)).await?, 0);
    assert_eq!(held(h, &item(21753)).await?, 1);
    accepted!(guardian, "qc-twenty-miles-guardian-second-slab-back", ["hi", "mission"], 16);
    assert_eq!(held(h, &item(21861)).await?, 1);
    // The broken altars are world scenarios; the mended counter gates the reward.
    refused!(guardian, "qc-twenty-miles-guardian-altars-mended", ["hi", "mission"], 16);
    seeder
        .set(h, owner, admitted, &[(format!("{P}slabs"), 3)])
        .await?;
    accepted!(guardian, "qc-twenty-miles-guardian-altars-mended", ["hi", "mission"], 17);
    assert_eq!(track(h, &format!("{P}access_sunken_city")).await?, 1);

    // Sea Ear, statues and sundial.
    accepted!(guardian, "qc-twenty-miles-guardian-quest-sea-ear", ["hi", "quest"], 18);
    accepted!(brandon, "qc-twenty-miles-brandon-sea-ear", ["hi", "mission"], 19);
    refused!(guardian, "qc-twenty-miles-guardian-sea-ear", ["hi", "mission"], 19);
    give!(21754);
    accepted!(guardian, "qc-twenty-miles-guardian-sea-ear", ["hi", "mission"], 20);
    assert_eq!(held(h, &item(21754)).await?, 1, "showing the Sea Ear keeps it");
    accepted!(guardian, "qc-twenty-miles-guardian-compass-place", ["hi", "mission"], 21);

    // The compass turn completes only against the compass.
    stage!(22);
    refused!(guardian, "qc-twenty-miles-guardian-compass", ["hi", "compass"], 22);
    assert!(!completed(h, QUEST).await?);
    give!(21795);
    assert_eq!(
        say!(guardian, "qc-twenty-miles-guardian-compass", ["hi", "compass"]),
        NpcIntentDisposition::Ok
    );
    assert_eq!(track(h, &stage_key).await?, 23);
    assert_eq!(held(h, &item(21795)).await?, 0);
    assert!(completed(h, QUEST).await?);
    assert_eq!(track(h, &format!("{P}access_birthing")).await?, 1);
    give!(21795);
    refused!(guardian, "qc-twenty-miles-guardian-compass", ["hi", "compass"], 23);
    assert_eq!(held(h, &item(21795)).await?, 1, "the second compass is kept");
    Ok(())
}

pub(super) async fn exercise_world(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("placement key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reward = callback.effects.item_reward.as_ref();
    let tool = callback.item_on_target.as_ref();
    // The counter or chain track that this step advances is the first one that is written.
    let written = callback
        .storage
        .iter()
        .find(|s| !s.preserve)
        .ok_or("written track")?;
    let seeds: Vec<(String, i64)> = callback
        .storage
        .iter()
        .filter(|s| s.minimum >= 0)
        .map(|s| (s.track.clone(), s.minimum))
        .collect();
    let guarded = callback.storage.iter().any(|s| s.preserve) || written.minimum >= 0;
    let before_value = track(h, &written.track).await?;
    let audits = async || -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
                .fetch_one(&h.pool)
                .await?,
        )
    };
    let try_use = async |owner: &mut ComposedFreshAdmission<'_, '_, '_>,
                         admitted: &mut AdmittedSession,
                         command: u64|
           -> TestResult<UseDisposition> {
        let bytes =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        oteryn_protocol_oteryn::world_object::decode_use_result(&bytes).map_err(debug)
    };
    // A step whose stage is not reached is refused and leaves everything untouched.
    if guarded {
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            try_use(owner, admitted, command).await?,
            UseDisposition::Rejected,
            "stage guard"
        );
        assert_eq!(track(h, &written.track).await?, before_value);
        if let Some(reward) = reward {
            assert_eq!(held(h, &reward.item).await?, 0);
        }
    }
    if !seeds.is_empty() {
        Seeder(110).set(h, owner, admitted, &seeds).await?;
    }
    let seeded = track(h, &written.track).await?;
    let mut nonce = 22_000_u64;
    if let Some(tool) = tool {
        // Neither a missing nor a wrong item moves the quest.
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            try_use(owner, admitted, command).await?,
            UseDisposition::Rejected,
            "missing item"
        );
        nonce += 1;
        mint(owner, admitted, &item(3031), nonce).await?;
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            try_use(owner, admitted, command).await?,
            UseDisposition::Rejected,
            "wrong item"
        );
        assert_eq!(track(h, &written.track).await?, seeded);
        nonce += 1;
        mint(owner, admitted, &tool.item, nonce).await?;
        assert_eq!(held(h, &tool.item).await?, i64::from(tool.count));
    }
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        try_use(owner, admitted, command).await?,
        UseDisposition::Committed
    );
    let expected = if written.increment {
        seeded + written.set
    } else {
        written.set
    };
    assert_eq!(track(h, &written.track).await?, expected);
    if let Some(tool) = tool {
        assert_eq!(held(h, &tool.item).await?, 0, "the offered item is spent");
    }
    if let Some(reward) = reward {
        assert_eq!(held(h, &reward.item).await?, i64::from(reward.count));
    }
    assert_eq!(completed(h, &callback.quest).await?, callback.completes);
    let after = audits().await?;
    // The same command id replays the committed result without a second effect.
    assert_eq!(
        try_use(owner, admitted, command).await?,
        UseDisposition::Committed
    );
    assert_eq!(audits().await?, after);
    assert_eq!(track(h, &written.track).await?, expected);
    // A new use is refused: the stage moved, the counter is spent or the item is gone.
    let next = admitted.continuity.next_command_id;
    assert_eq!(
        try_use(owner, admitted, next).await?,
        UseDisposition::Rejected,
        "second use"
    );
    assert_eq!(track(h, &written.track).await?, expected);
    if let Some(reward) = reward {
        assert_eq!(held(h, &reward.item).await?, i64::from(reward.count));
    }
    Ok(())
}
