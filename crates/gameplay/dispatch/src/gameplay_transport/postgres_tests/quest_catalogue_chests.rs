//! Catalogue-wide reward qualification through the production USE and durable owners.
//! This proves the reward at its source selector, not the quest's access path.
use super::TestResult;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) type CatalogueActivation = (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
);

pub(super) struct CatalogueChest {
    pub claim: String,
    pub position: (u16, u16, i8),
    pub unique: u16,
    pub servable: bool,
    pub appearance: u16,
    pub root: String,
    pub quantity: u32,
    pub contents: Vec<(String, i64, i32)>,
    pub texts: Vec<(String, String)>,
    pub key_number: Option<i32>,
    pub achievement: bool,
}

fn string(value: &Value) -> TestResult<String> {
    Ok(value.as_str().ok_or("catalogue string")?.into())
}

fn cases() -> TestResult<Vec<CatalogueChest>> {
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let supported: BTreeSet<_> = coverage["quests"]
        .as_array()
        .ok_or("coverage quests")?
        .iter()
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .filter(|chest| chest["status"] != "not_playable")
        .map(|chest| string(&chest["claim"]))
        .collect::<TestResult<_>>()?;
    let mut found = BTreeSet::new();
    let mut served = BTreeSet::new();
    let mut cases = Vec::new();
    for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("claim records")? {
            let definition = &row["definition"];
            let claim = string(&definition["identity"]["key"])?;
            if !supported.contains(&claim) {
                continue;
            }
            assert!(found.insert(claim.clone()), "duplicate claim {claim}");
            for (index, placement) in definition["placements"]
                .as_array()
                .ok_or("claim placements")?
                .iter()
                .enumerate()
            {
                let binding = &placement["source_binding"];
                let position = &binding["project_position"];
                let ids = binding["legacy_unique_ids"]
                    .as_array()
                    .ok_or("source unique ids")?;
                let crystal = ids
                    .iter()
                    .find(|source| source["server"] == "crystalserver");
                let unique = crystal.or_else(|| ids.first()).ok_or("source selector")?["unique_id"]
                    .as_u64()
                    .ok_or("source unique id")?;
                if crystal.is_some() {
                    served.insert(claim.clone());
                }
                let reward = &placement["reward"];
                let native = definition["native_contents"]["placements"].get(index);
                let (root, quantity, contents) = if let Some(native) = native {
                    let children = native["contents"]
                        .as_array()
                        .ok_or("native contents")?
                        .iter()
                        .map(|child| {
                            Ok((
                                string(&child["item"]["key"])?,
                                child["count"].as_i64().ok_or("child count")?,
                                i32::try_from(child["key_number"].as_i64().unwrap_or(0))?,
                            ))
                        })
                        .collect::<TestResult<Vec<_>>>()?;
                    (string(&native["root"]["key"])?, 1, children)
                } else {
                    let items = reward["items"].as_array().ok_or("leaf items")?;
                    assert_eq!(items.len(), 1, "unsupported leaf shape {claim}");
                    (
                        string(&items[0]["item"]["key"])?,
                        u32::try_from(items[0]["count"].as_u64().ok_or("leaf count")?)?,
                        Vec::new(),
                    )
                };
                let mut texts = Vec::new();
                if let Some(native) = native {
                    if let Some(text) = native["root_text"].as_str() {
                        texts.push((root.clone(), text.into()));
                    }
                    for child in native["contents"].as_array().ok_or("native contents")? {
                        if let Some(text) = child["text"].as_str() {
                            texts.push((string(&child["item"]["key"])?, text.into()));
                        }
                    }
                }
                let key_number = reward["key_binding"]
                    .as_str()
                    .map(|binding| -> TestResult<i32> {
                        Ok(binding
                            .rsplit('/')
                            .next()
                            .ok_or("key binding")?
                            .parse::<i32>()?)
                    })
                    .transpose()?;
                cases.push(CatalogueChest {
                    claim: claim.clone(),
                    position: (
                        u16::try_from(position["x"].as_u64().ok_or("source x")?)?,
                        u16::try_from(position["y"].as_u64().ok_or("source y")?)?,
                        -i8::try_from(position["z"].as_u64().ok_or("source z")?)?,
                    ),
                    unique: u16::try_from(unique)?,
                    servable: crystal.is_some(),
                    appearance: u16::try_from(
                        placement["appearance_tibia_id"]
                            .as_u64()
                            .ok_or("source appearance")?,
                    )?,
                    root,
                    quantity,
                    contents,
                    texts,
                    key_number,
                    achievement: native.is_some_and(|native| !native["achievement"].is_null()),
                });
            }
        }
    }
    assert_eq!(found, supported, "every supported claim needs a scenario");
    assert_eq!(
        served, supported,
        "every supported claim needs a native-source placement"
    );
    Ok(cases)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal() -> TestResult {
    execute(cases()?)
}

#[test]
fn quest_catalogue_chests_medusa_exact_crystal_reward_use_and_replay() -> TestResult {
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| case.claim == "oteryn:reward-claim.quest.u7_3.the_medusa_quest")
        .collect();
    assert_eq!(selected.len(), 1);
    execute(selected)
}

fn execute(cases: Vec<CatalogueChest>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            // Every case has an isolated durable world/session, using the same
            // immutable production content generation. Qualify that content once;
            // no mutable character, command or receipt state is shared.
            let world = crate::foundation::WorldId::decode(&super::fixture::id(42))
                .map_err(|error| format!("{error:?}"))?;
            let activation =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")
                    .map_err(|error| error.to_string())?;
            for case in cases {
                let started = std::time::Instant::now();
                super::quest_container_chests::catalogue_scenario(&case, &activation)
                    .map_err(|error| format!("{} {:?}: {error}", case.claim, case.position))?;
                println!(
                    "QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED {} {:?} native_source={} duration={:?}",
                    case.claim,
                    case.position,
                    case.servable,
                    started.elapsed()
                );
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "catalogue chest scenario panicked")??;
    Ok(())
}
