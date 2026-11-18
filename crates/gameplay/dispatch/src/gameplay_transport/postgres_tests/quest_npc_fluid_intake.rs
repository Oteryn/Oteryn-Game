//! Crystal 201189f8 data-global/npc/rapanaio.lua:90-100,206-235 and lazaran.lua:100-103,153-162.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    isle_of_evil, npc_rules, unnatural_selection, world_callbacks::Position,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) const NAME: &str = "quest-npc-exact-fluid";
const RAPANAIO: &str = "oteryn:npc.rapanaio";
const LAZARAN: &str = "oteryn:npc.lazaran";
const RUM_SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rapanaio.lua:90-100";
const RAPANAIO_INTAKE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rapanaio.lua:206-235";
const LAZARAN_INTAKE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/lazaran.lua:100-103";
const LAZARAN_BEER: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/lazaran.lua:153-162";

pub(super) enum FixtureItem {
    ChargedCrystal,
    GhostTear,
    Pot,
    Skull,
    HeatedHammer,
}

pub(super) fn generation_with_item(
    world: crate::foundation::WorldId,
    item: FixtureItem,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use sha2::{Digest, Sha256};
    let mut input = super::death_ml_loss_composed::production_input_with_source("content-1", None)?;
    let profiles = input.item_profiles.as_mut().ok_or("native item profiles")?;
    let mut packet: Value = serde_json::from_slice(&profiles.bytes)?;
    let records = packet["records"].as_array_mut().ok_or("profile records")?;
    let (id, template, name, weight) = match item {
        // Crystal 201189f8 data/items/items.xml:22924-22930.
        FixtureItem::ChargedCrystal => (8225, "oteryn:item.tibia.i10198", "blood crystal", 890),
        // Crystal 201189f8 data/items/items.xml:23374-23376.
        FixtureItem::GhostTear => (8746, "oteryn:item.tibia.i10198", "ghost's tear", 1000),
        // Crystal 201189f8 data/items/items.xml:26533-26536.
        FixtureItem::Skull => (
            10159,
            "oteryn:item.tibia.i10198",
            "skull of a caveman",
            2180,
        ),
        // Crystal 201189f8 data/items/items.xml:10874-10878.
        FixtureItem::Pot => (3465, "oteryn:item.tibia.i2874", "pot", 5250),
        // Crystal 201189f8 data/items/items.xml:24803-24809.
        FixtureItem::HeatedHammer => (
            9201,
            "oteryn:item.tibia.i10198",
            "heated worm punisher",
            6450,
        ),
    };
    let mut definitions = vec![(id, template, name, weight)];
    if id == 9201 {
        // Crystal 201189f8 data/items/items.xml:24975-24979.
        definitions.push((9234, template, "worm punisher", 6450));
    }
    for (id, template, name, weight) in definitions {
        let key = format!("oteryn:item.tibia.i{id}");
        if records
            .iter()
            .any(|row| row["production_definition"]["production_key"] == key)
        {
            continue;
        }
        let mut record = records
            .iter()
            .find(|row| row["production_definition"]["production_key"] == template)
            .ok_or("admitted item shape")?
            .clone();
        record["production_definition"]["production_key"] = json!(key);
        record["authoring"]["item"]["key"] = json!(key);
        record["authoring"]["item"]["revision"] = json!("definition-r1");
        record["production_binding"]["external_id"] = json!(id.to_string());
        record["production_binding"]["target"]["key"] = json!(key);
        record["semantics"]["physical"]["value"]["weight"]["value"] = json!(weight);
        record["semantics"]["presentation"]["value"]["name"]["value"] = json!(name);
        record["semantics"]["presentation"]["value"]["description"] = json!({"state":"UNKNOWN"});
        records.push(record);
    }
    profiles.bytes = serde_json::to_vec(&packet)?;
    profiles.sha256 = Sha256::digest(&profiles.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::content::native_gameplay::test_support::try_activate_input(world, &input)
        .map_err(super::fixture::debug)
}

#[test]
fn quest_npc_rapanaio_consumes_only_rum_and_fruit_juice_at_their_topics() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run("quest-npc-exact-fluid-rapanaio")
}

#[test]
fn quest_npc_lazaran_exchanges_only_beer_for_the_tribal_crest() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run("quest-npc-exact-fluid-lazaran")
}

fn run(name: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

struct Intake {
    mission: String,
    topic: i32,
    stage: i64,
    item: u16,
    subtype: u8,
    quantity: u8,
    start_at: i64,
    complete_at: i64,
    reward: Option<u16>,
    prompt: &'static str,
    missing: &'static str,
    accepted: &'static [&'static str],
}

impl Intake {
    fn rules(&self, quest: &str, npc: &str, questline: &str) -> [Value; 3] {
        let (prompt_source, intake_source) = if npc == RAPANAIO {
            (RUM_SOURCE, RAPANAIO_INTAKE)
        } else {
            (LAZARAN_INTAKE, LAZARAN_BEER)
        };
        let item = format!("oteryn:item.tibia.i{}", self.item);
        let topic = self.topic;
        let group =
            format!("oteryn:quest-npc/fixture-fluid/{npc}/{topic}").replace("oteryn:npc.", "");
        let mut effects = vec![
            json!({"kind":"take_item","item":item,"count":self.quantity,"subtype":self.subtype}),
            json!({"kind":"set","track":questline,"value":self.stage+1}),
            json!({"kind":"set","track":self.mission,"value":self.complete_at}),
            json!({"kind":"set_topic","value":0}),
        ];
        if let Some(id) = self.reward {
            effects.push(
                json!({"kind":"give_item","item":format!("oteryn:item.tibia.i{id}"),"count":1}),
            );
        }
        [
            json!({
                "key":format!("{group}/prompt"),"quest":quest,"npc":npc,"trigger":"say",
                "keywords":["mission"],"keyword_mode":"exact",
                "guards":[{"kind":"storage","track":questline,"op":"eq","value":self.stage}],
                "reply":[self.prompt],"effects":[{"kind":"set_topic","value":topic}],
                "sources":[prompt_source]
            }),
            json!({
                "key":format!("{group}/accept"),"quest":quest,"npc":npc,"trigger":"say",
                "keywords":["yes"],"keyword_mode":"exact","topics":[topic],
                "guards":[{"kind":"item","item":item,"count":self.quantity,"subtype":self.subtype}],
                "reply":self.accepted,"effects":effects,"sources":[intake_source]
            }),
            json!({
                "key":format!("{group}/refuse"),"quest":quest,"npc":npc,"trigger":"say",
                "keywords":["yes"],"keyword_mode":"exact","topics":[topic],
                "guards":[{"kind":"item","item":item,"count":self.quantity,"subtype":self.subtype,"op":"lt"}],
                "reply":[self.missing],"effects":[{"kind":"set_topic","value":0}],"sources":[intake_source]
            }),
        ]
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let (quest, npc, questline, intakes) = if name.ends_with("rapanaio") {
        (
            isle_of_evil::QUEST,
            RAPANAIO,
            format!("{}questline", isle_of_evil::PREFIX),
            vec![
                Intake {
                    mission: format!("{}mission02", isle_of_evil::PREFIX),
                    topic: 10,
                    stage: 4,
                    item: 2874,
                    subtype: 13,
                    quantity: 10,
                    start_at: 1,
                    complete_at: 2,
                    reward: None,
                    prompt: "Do you have the rum .. erm the fuel with you?",
                    missing: "You dont have what I ask for.",
                    accepted: &[
                        "You did it! Ye Gods, what a temptation! But I will not give in! I will put it into the machine as soon as possible. Oh and if you happen to see a dwarf wearing a funnel, please ask him to return it to me. ...",
                        "But now let's talk about your next mission.",
                    ],
                },
                Intake {
                    mission: format!("{}mission03", isle_of_evil::PREFIX),
                    topic: 11,
                    stage: 6,
                    item: 2874,
                    subtype: 14,
                    quantity: 1,
                    start_at: 1,
                    complete_at: 2,
                    reward: None,
                    prompt: "So you have brought me a vial of fruit juice?",
                    missing: "You dont have what I ask for.",
                    accepted: &[
                        "Ahhhh that juice was just what I needed. I haven't had any fruit juice for ages! You can't fight evil the whole day without allowing yourself a small treat! ...",
                        "Though I bet it would have tasted better in a glass with one of those amusing umbrellas on top! But I don't blame you. ...",
                        "I fear we hunters of evil must be content with the small pleasures life grants us. Now where were we? Oh, yes you are probably eager to learn about your next mission as part of our big plan!",
                    ],
                },
            ],
        )
    } else {
        (
            unnatural_selection::QUEST,
            LAZARAN,
            unnatural_selection::QUESTLINE_TRACK.into(),
            vec![Intake {
                mission: "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission06".into(),
                topic: 6,
                stage: 14,
                item: 3465,
                subtype: 3,
                quantity: 1,
                start_at: 2,
                complete_at: 3,
                reward: Some(10198),
                prompt: "You bring us big pot of strange water from little men?",
                missing: "You do not have the brown strange water!",
                accepted: &[
                    "We make big ritual soon and learn much about world outside. Me thank you many times for teaching us world. Very wise and adventurous you are!",
                ],
            }],
        )
    };
    let base = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let rules = intakes
        .iter()
        .flat_map(|intake| intake.rules(quest, npc, &questline))
        .collect::<Vec<_>>();
    let source = if npc == RAPANAIO {
        RAPANAIO_INTAKE
    } else {
        LAZARAN_BEER
    };
    let tracks = std::iter::once(&questline).chain(intakes.iter().map(|intake| &intake.mission))
        .map(|key| {
            let track = base.track(key).ok_or("canonical track")?;
            Ok(json!({"key":key,"quest":quest,"initial":track.initial,"min":track.min,"max":track.max,"sources":[source]}))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let document = json!({"schema":1,"quest":quest,"tracks":tracks,"rules":rules}).to_string();
    owner.quest_catalogue = Some(Arc::new(npc_rules::with_fixture_programs(
        (**base).clone(),
        &[&document],
    )?));
    let [actor] = super::classic_quests::install(owner, admitted, [npc]).await?;
    let mut command_id = 8100;
    let mut occurrence = 1;
    for intake in intakes {
        super::qengine_outfit_stories::seed_quest_prerequisites(
            h,
            owner,
            admitted,
            &[
                (questline.clone(), intake.stage),
                (intake.mission.clone(), intake.start_at),
            ],
            120 + intake.topic as u8,
        )
        .await?;
        super::m42_hot_cuisine::typed_fluids(
            owner,
            admitted,
            h,
            authority,
            (intake.item, 1, 1),
            occurrence,
        )
        .await?;
        occurrence += 1;
        let snapshot = live_fluids(h, intake.item).await?;

        for accept in [false, true] {
            if accept {
                for count in (0..intake.quantity)
                    .step_by(5)
                    .map(|n| (intake.quantity - n).min(5))
                {
                    super::m42_hot_cuisine::typed_fluids(
                        owner,
                        admitted,
                        h,
                        authority,
                        (intake.item, intake.subtype, count),
                        occurrence,
                    )
                    .await?;
                    occurrence += 1;
                }
            }
            let mut reply = Vec::new();
            for word in ["hi", "mission", "yes"] {
                reply =
                    super::classic_quests::talk(owner, admitted, actor, command_id, word).await?;
                command_id += 1;
            }
            if accept {
                assert_eq!(reply, intake.accepted);
            } else {
                assert_eq!(reply, [intake.missing]);
            }
            assert_eq!(live_fluids(h, intake.item).await?, snapshot);
            let mission = super::m39_bone_journey::stage(h, &intake.mission).await?;
            let stage = super::m39_bone_journey::stage(h, &questline).await?;
            if accept {
                assert_eq!(
                    (mission, stage),
                    (Some(intake.complete_at), Some(intake.stage + 1))
                );
            } else {
                assert_eq!(
                    (mission, stage),
                    (Some(intake.start_at), Some(intake.stage))
                );
            }
        }
        if let Some(id) = intake.reward {
            let key = format!("oteryn:item.tibia.i{id}");
            let gifts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
                .bind(key).fetch_one(&h.pool).await?;
            assert_eq!(gifts, 1);
        }
    }
    Ok(())
}

async fn live_fluids(h: &Harness, item: u16) -> TestResult<Vec<(i16, i64)>> {
    Ok(sqlx::query_as("SELECT f.fluid_subtype,count(*) FROM game_item_fluid_states f JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$1 GROUP BY f.fluid_subtype ORDER BY f.fluid_subtype")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_all(&h.pool).await?)
}
