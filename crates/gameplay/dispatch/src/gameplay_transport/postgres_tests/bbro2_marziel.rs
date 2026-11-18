//! The actual Marziel STEP tile gates the durable vial burn and native spawn.
use super::annihilator_route::step;
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintContent, RewardClaimMintRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::map::{boot::BundleMap, overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.blood_brothers_quest";
const TRACK: &str = "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/marziel_blood";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/movements_marziel_blood.lua:10-37";
const TRIGGER: Position = Position {
    x: 32940,
    y: 31458,
    floor: -2,
};
const STATUE: Position = Position {
    x: 32940,
    y: 31457,
    floor: -2,
};
pub(super) const STARTS: [Position; 3] = [
    Position {
        x: 32939,
        ..TRIGGER
    },
    Position {
        y: 31459,
        ..TRIGGER
    },
    Position {
        x: 32941,
        ..TRIGGER
    },
];

#[test]
fn marziel_step_debits_only_female_carried_blood_and_spawns_brides() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::blood_brothers_vial()
}

pub(super) fn catalogue(revision: &str) -> TestResult<QuestStateCatalogue> {
    let document = serde_json::json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"177ccbdc1380ca10ff2076b10e18d3925aae1ce7a24ce499523cf536d929ce20",
            "sources":[SOURCE],
            "tracks":[{"key":TRACK,"initial":0,"minimum":0,"maximum":1}],
            "stages":[{"key":"blood-vial","completes":false,"trigger":{
                "kind":"step_in",
                "callback":{
                    "key":"oteryn:quest-callback/bbro2-marziel-blood",
                    "quest":QUEST,"trigger":"step_in",
                    "selector":{"native":{"position":TRIGGER,"ordinal":1,"appearance":8695}},
                    "reversible":false,"return_on_failure":false,"requires_world_admission":true,
                    "passages":[],"offerings":[],
                    "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1}],
                    "item_on_target":{"item":"oteryn:item.tibia.i2874","count":1,"on_step":true},
                    "effects":{
                        "required_sex":"female",
                        "required_fluid_subtype":5,
                        "target_transform":{"position":STATUE,"from":8325,"to":8326},
                        "encounter":{
                            "minimum_level":0,"players":[],"spawn_in_place":true,
                            "room_min":{"x":32936,"y":31456,"floor":-2},
                            "room_max":{"x":32944,"y":31464,"floor":-2},
                            "monsters":[
                                {"position":{"x":32938,"y":31458,"floor":-2},"creature":"oteryn:creature.vampire_bride"},
                                {"position":{"x":32942,"y":31458,"floor":-2},"creature":"oteryn:creature.vampire_bride"}
                            ]
                        }
                    },
                    "sources":[SOURCE]
                }
            }}]
        }]
    });
    let base = QuestStateCatalogue::empty(revision).ok_or("quest catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&document)?)?.attach(&base)?)
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(
                "../../../../../../content/world/placements/region-z02-x128-y122.b3"
            ),
            region: (2, 128, 122),
            bounds: (32938, 31457, 32942, 31459),
            start: (STARTS[0].x, STARTS[0].y),
            expected: vec![
                ExpectedTile {
                    position: (TRIGGER.x, TRIGGER.y),
                    ids: vec![8308, 8695],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (STATUE.x, STATUE.y),
                    ids: vec![8308, 8698, 8325],
                    counted_entry: None,
                },
            ],
        }],
        &[],
        &[8326],
    )
}

async fn mint_vial(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
    subtype: u8,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("session")?
        .item_fence();
    let facts = |id| {
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)
    };
    owner
        .root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence,
            &NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate {
                definition: facts(2854)?.definition,
                ..crate::durability::starter_grant::StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command: CommandRef::new(
            member.game_session_id,
            CommandId::new(9000 + u64::from(subtype)).map_err(debug)?,
        ),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "fixture:reward.bbro2-marziel-vial".into(),
            revision_ref: "definition-r1".into(),
        },
        source_placement: "fixture:placement.bbro2-marziel-vial".into(),
        item: facts(2853)?,
        quantity: 1,
        contents: vec![RewardClaimMintContent {
            charges_remaining: None,
            item: facts(2874)?,
            quantity: 1,
            fluid_subtype: Some(subtype),
            key_number: None,
            text: None,
        }],
        additional_items: Vec::new(),
        root_text: None,
        backpack: facts(2854)?,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let weights = NativeItemWeights(native);
    let mut frozen = owner
        .root
        .freeze_reward_claim_mint_weighted(owner.character, owner.holder, fence, request, &weights)
        .await
        .map_err(debug)?;
    owner
        .root
        .commit_reward_claim_mint_weighted_noticed(
            owner.character,
            owner.holder,
            fence,
            &mut frozen,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
    pool: &sqlx::PgPool,
) -> TestResult {
    let [male, wrong_fluid, blood] = members else {
        return Err("three durable admissions".into());
    };
    let statue = crate::map::view::placement_key(
        TilePos {
            x: STATUE.x,
            y: STATUE.y,
            floor: STATUE.floor,
        },
        2,
    )
    .ok_or("statue placement")?;
    for (member, subtype) in [(&*male, 5), (&*wrong_fluid, 1), (&*blood, 5)] {
        mint_vial(owner, member, subtype).await?;
        owner.admit_quest_session(member).await;
    }
    step(owner, male, StepDirection::East).await?;
    assert_eq!(map.facts.object_revision(statue), 0);
    step(owner, male, StepDirection::West).await?;
    step(owner, wrong_fluid, StepDirection::North).await?;
    assert_eq!(map.facts.object_revision(statue), 0);
    step(owner, wrong_fluid, StepDirection::South).await?;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .is_empty()
    );

    step(owner, blood, StepDirection::West).await?;
    assert_eq!(map.facts.object_revision(statue), 1);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len(),
        2
    );
    let retained:Vec<(i16,i64)> = sqlx::query_as("SELECT f.fluid_subtype,count(*) FROM game_item_fluid_states f JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2874' GROUP BY f.fluid_subtype ORDER BY f.fluid_subtype")
        .fetch_all(pool).await?;
    assert_eq!(retained, vec![(1, 1), (5, 1)]);
    let spent:i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-callback/bbro2-marziel-blood'")
        .fetch_one(pool).await?;
    assert_eq!(spent, 1);
    eprintln!("POSTGRES_CASE_COMPLETED bbro2_marziel_step_sex_subtype_custody");
    Ok(())
}
