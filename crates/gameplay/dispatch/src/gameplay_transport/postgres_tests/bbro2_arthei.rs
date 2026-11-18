//! Crystal's one-use tear ritual, retained arena and native two-player exit.
use super::annihilator_route::{position, staged_step as step};
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::map::{boot::BundleMap, overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.blood_brothers_quest";
const MISSION: &str = "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mission10";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/actions_ghost_tear.lua:3-39";
const TEAR: &str = "oteryn:item.tibia.i8746";
const STATUE: Position = Position {
    x: 32953,
    y: 31440,
    floor: -2,
};
pub(super) const START: Position = Position { y: 31441, ..STATUE };

#[test]
fn arthei_one_selected_tear_admits_two_players_and_native_exit_returns_both() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::blood_brothers_tear()
}

pub(super) fn catalogue(revision: &str) -> TestResult<QuestStateCatalogue> {
    let encounter = serde_json::json!({
        "minimum_level":0,"players":[],
        "staging_area":{
            "minimum":{"x":32941,"y":31431,"floor":-2},
            "maximum":{"x":32957,"y":31447,"floor":-2},
            "destination":{"x":32953,"y":31444,"floor":-1}
        },
        "room_min":{"x":32951,"y":31440,"floor":-1},
        "room_max":{"x":32955,"y":31446,"floor":-1},
        "monsters":[
            {"position":{"x":32953,"y":31441,"floor":-1},"creature":"oteryn:creature.arthei"},
            {"position":{"x":32952,"y":31441,"floor":-1},"creature":"oteryn:creature.vampire"},
            {"position":{"x":32954,"y":31441,"floor":-1},"creature":"oteryn:creature.vampire"}
        ]
    });
    let callback = serde_json::json!({
        "key":"oteryn:quest-callback/bbro2-arthei-tear",
        "quest":QUEST,
        "trigger":"use",
        "selector":{"native":{"position":STATUE,"ordinal":2,"appearance":8325}},
        "reversible":false,"return_on_failure":false,"requires_world_admission":true,
        "passages":[],"offerings":[],
        "storage":[{"track":MISSION,"minimum":1,"maximum":1,"set":1}],
        "item_on_target":{"item":TEAR,"count":1,"selected_instance":true},
        "effects":{"transform":8326,"encounter":encounter},
        "sources":[SOURCE]
    });
    let document = serde_json::json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":"5ac1cc7901b843ea147d8dba5de3959d1331c9813cb6143c89ef9fc099c04e9c",
        "sources":[SOURCE],
        "tracks":[{"key":MISSION,"initial":1,"minimum":1,"maximum":2}],
        "stages":[{"key":"ghost-tear","completes":false,"trigger":{"kind":"map_use","callback":callback}}]
    }]});
    let base = QuestStateCatalogue::empty(revision).ok_or("quest catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&document)?)?.attach(&base)?)
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{
        ExpectedTeleport, ExpectedTile, NativeCrop, native_scene_with_appearances,
    };
    const ROOM: &[u8] =
        include_bytes!("../../../../../../content/world/placements/region-z01-x128-y122.b3");
    native_scene_with_appearances(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(
                    "../../../../../../content/world/placements/region-z02-x128-y122.b3"
                ),
                region: (2, 128, 122),
                bounds: (32951, 31440, 32955, 31445),
                start: (START.x, START.y),
                expected: vec![ExpectedTile {
                    position: (STATUE.x, STATUE.y),
                    ids: vec![8308, 8698, 8325],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: ROOM,
                region: (1, 128, 122),
                bounds: (32951, 31440, 32956, 31447),
                start: (32953, 31444),
                expected: Vec::new(),
            },
            NativeCrop {
                bytes: ROOM,
                region: (1, 128, 122),
                bounds: (32966, 31479, 32969, 31482),
                start: (32967, 31480),
                expected: Vec::new(),
            },
        ],
        &[ExpectedTeleport {
            position: (32953, 31446, 1),
            ordinal: 1,
            destination: (32967, 31480, 1),
        }],
        &[8326],
    )
}

// Crystal data/items/items.xml:23374-23376, ghost's tear weight in centi-oz.
const TEAR_WEIGHT: u32 = 1_000;

fn tear_definition() -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: TEAR.into(),
        revision_ref: "definition-r1".into(),
    }
}

struct TearFixtureWeights<'a>(NativeItemWeights<'a>);

impl ItemWeightLookup for TearFixtureWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.generation_digest()
    }

    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        if definition == &tear_definition() {
            Ok(TEAR_WEIGHT)
        } else {
            self.0.weight_of(definition)
        }
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
    pool: &sqlx::PgPool,
) -> TestResult {
    let [first, other] = members else {
        return Err("two durable admissions".into());
    };
    let fence = owner
        .current_quest_fence(first.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("session")?;
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("native item index")?;
    let reference = index
        .definition_ref("Item", TEAR, "definition-r1")
        .ok_or("ghost tear")?;
    owner
        .root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..crate::durability::starter_grant::StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    // The canonical Item index has the tear, but the active item policy does not
    // yet materialize it. Seed custody with the exact source facts for this test.
    let weights = TearFixtureWeights(NativeItemWeights(native));
    let backpack = generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
    )
    .map_err(debug)?;
    let mint = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command: CommandRef::new(first.game_session_id, CommandId::new(951).map_err(debug)?),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "fixture:reward.bbro2-arthei-tear".into(),
            revision_ref: "definition-r1".into(),
        },
        source_placement: "fixture:placement.bbro2-arthei-tear".into(),
        item: crate::domain::item_definition::ItemDefinitionFacts {
            definition: tear_definition(),
            stack: crate::domain::item_definition::ItemStackClass::NonStackable,
            weight: Some(TEAR_WEIGHT),
            container_capacity: None,
            container_slot_equip_pattern: false,
        },
        quantity: 1,
        contents: Vec::new(),
        additional_items: Vec::new(),
        root_text: None,
        backpack,
        content_revision: format!(
            "sha256:{}",
            native.source_digest().iter().map(|byte| format!("{byte:02x}")).collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let mut frozen = owner
        .root
        .freeze_reward_claim_mint_weighted(owner.character, owner.holder, fence.item_fence(), mint, &weights)
        .await
        .map_err(debug)?;
    owner
        .root
        .commit_reward_claim_mint_weighted_noticed(owner.character, owner.holder, fence.item_fence(), &mut frozen, None, &weights)
        .await
        .map_err(debug)?;
    owner.admit_quest_session(first).await;
    owner.admit_quest_session(other).await;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    first.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("tool capability")?;

    let key = crate::map::view::placement_key(
        TilePos {
            x: STATUE.x,
            y: STATUE.y,
            floor: STATUE.floor,
        },
        2,
    )
    .ok_or("statue placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("tear handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (admitted, result, _) = super::item_craft::exchange(
        owner,
        *first,
        first.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *first = admitted;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.next_movement_decay_time(), None);
    let spent:i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-callback/bbro2-arthei-tear' AND before_item->>'definition_production_key'=$1")
        .bind(TEAR).fetch_one(pool).await?;
    assert_eq!(spent, 1);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?
            .len(),
        3
    );

    for (member, clear_landing) in [(first, true), (other, false)] {
        assert_eq!(position(owner, member).await?, (32953, 31444, 1));
        for direction in [StepDirection::South; 2] {
            step(owner, member, direction).await?;
        }
        assert_eq!(position(owner, member).await?, (32967, 31480, 1));
        if clear_landing {
            step(owner, member, StepDirection::East).await?;
        }
    }
    eprintln!("POSTGRES_CASE_COMPLETED bbro2_arthei_single_selected_tear_and_exit");
    Ok(())
}
