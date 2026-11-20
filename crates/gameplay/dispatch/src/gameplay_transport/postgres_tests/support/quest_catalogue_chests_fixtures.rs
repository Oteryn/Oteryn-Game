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
    pub charges: Vec<(String, i32)>,
    pub fluids: Vec<(String, i32)>,
}

pub(super) fn string(value: &Value) -> TestResult<String> {
    Ok(value.as_str().ok_or("catalogue string")?.into())
}

pub(super) fn cases() -> TestResult<Vec<CatalogueChest>> {
    cases_including(|_| false)
}

pub(super) fn cases_including(include: impl Fn(&str) -> bool) -> TestResult<Vec<CatalogueChest>> {
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let mut supported: BTreeSet<_> = coverage["quests"]
        .as_array()
        .ok_or("coverage quests")?
        .iter()
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .filter(|chest| chest["status"] != "not_playable")
        .map(|chest| string(&chest["claim"]))
        .collect::<TestResult<_>>()?;
    // Random repeat sources have their own complete production USE scenarios;
    // the generic once-only recipe harness cannot represent their cycle owner.
    for chest in oteryn_gameplay_quests::quest::falcon::CHESTS {
        assert!(
            supported.remove(&chest.key()),
            "Falcon source needs cycle coverage"
        );
    }
    let mut found = BTreeSet::new();
    let mut served = BTreeSet::new();
    let mut cases = Vec::new();
    for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("claim records")? {
            let definition = &row["definition"];
            let claim = string(&definition["identity"]["key"])?;
            // A new source slice must reach the production binder before its
            // coverage report can claim qualification. The broad sweep still
            // requires every previously supported claim exactly as before.
            if include(&claim) {
                supported.insert(claim.clone());
            }
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
                    .filter(|_| reward.get("container").is_none())
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
                    charges: expected_charges(&claim),
                    fluids: expected_fluids(&claim),
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

// Small cohorts use the same production dispatch scenario as the catalogue
// sweep. Keep every placement and every already-qualified sibling so an early
// checkpoint describes a whole reward set, rather than a partial failed sweep.
// Access puzzles, NPC exchanges and outfits remain separate qualification work.
pub(super) fn phase_a_cohort(quests: &[&str]) -> TestResult {
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let mut found = BTreeSet::new();
    let mut claims = BTreeSet::new();
    let mut placements = 0;
    for quest in coverage["quests"].as_array().ok_or("coverage quests")? {
        let title = quest["quest"]
            .as_str()
            .ok_or("quest title")?
            .rsplit('/')
            .next()
            .ok_or("quest slug")?;
        if !quests.contains(&title) {
            continue;
        }
        assert!(found.insert(title), "duplicate quest {title}");
        for chest in quest["chests"].as_array().ok_or("quest chests")? {
            assert_ne!(chest["status"], "not_playable", "held reward in {title}");
            assert!(
                claims.insert(string(&chest["claim"])?),
                "duplicate cohort claim"
            );
            placements += chest["placements"]
                .as_array()
                .ok_or("coverage placements")?
                .len();
        }
    }
    assert_eq!(
        found,
        quests.iter().copied().collect(),
        "every cohort quest must be authored"
    );
    assert!(!claims.is_empty(), "a cohort must exercise actual rewards");
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| claims.contains(&case.claim))
        .collect();
    assert_eq!(
        selected.len(),
        placements,
        "every source placement must execute"
    );
    assert_eq!(
        selected
            .iter()
            .map(|case| &case.claim)
            .collect::<BTreeSet<_>>(),
        claims.iter().collect(),
        "every reward must execute"
    );
    execute(selected)
}

// Historical SOURCE readiness retains lowering holds even after the native
// claim owner has been implemented. Qualify those exact standalone contracts
// against their original claim inventory; never infer a storyline from a chest.
pub(super) fn standalone_claim_cohort(quests: &[&str], test: &str) -> TestResult {
    let mut found = BTreeSet::new();
    for shard in [
        include_str!("../../../../../../../content/quests/definitions/quests-00000-00099.json"),
        include_str!("../../../../../../../content/quests/definitions/quests-00100-00199.json"),
        include_str!("../../../../../../../content/quests/definitions/quests-00200-00299.json"),
        include_str!("../../../../../../../content/quests/definitions/quests-00300-00351.json"),
    ] {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("quest records")? {
            let definition = &row["definition"];
            let slug = definition["source_refs"]["quest"]["key"]
                .as_str()
                .and_then(|key| key.rsplit('/').next());
            let Some(slug) = slug.filter(|slug| quests.contains(slug)) else {
                continue;
            };
            assert!(found.insert(slug.to_owned()), "duplicate source quest");
            assert_eq!(definition["kind"], "reward_only", "{slug}");
            assert_eq!(definition["shown_in_quest_log"], false, "{slug}");
            assert!(
                definition["source_refs"]["gates"]
                    .as_array()
                    .ok_or("quest gates")?
                    .is_empty(),
                "{slug}: an access gate needs its own dispatch path"
            );
            let features = definition["reported_source_readiness"]["features"]
                .as_array()
                .ok_or("source features")?
                .iter()
                .map(string)
                .collect::<TestResult<BTreeSet<_>>>()?;
            assert_eq!(
                features,
                BTreeSet::from(["reward_claim".to_owned(), "trigger_use".to_owned()]),
                "{slug}: non-claim obligations cannot use this cohort"
            );
            let expected = definition["claims"]
                .as_array()
                .ok_or("quest claims")?
                .iter()
                .map(|claim| string(&claim["key"]))
                .collect::<TestResult<BTreeSet<_>>>()?;
            let coverage: Value = serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
            )))?;
            let actual = coverage["quests"]
                .as_array()
                .ok_or("coverage quests")?
                .iter()
                .find(|quest| quest["quest"] == definition["source_refs"]["quest"]["key"])
                .ok_or("source coverage")?["chests"]
                .as_array()
                .ok_or("coverage chests")?
                .iter()
                .map(|claim| string(&claim["claim"]))
                .collect::<TestResult<BTreeSet<_>>>()?;
            assert_eq!(actual, expected, "{slug}: every source claim must execute");
        }
    }
    assert_eq!(found, quests.iter().map(|q| (*q).to_owned()).collect());
    let started = std::time::Instant::now();
    phase_a_cohort(quests)?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        println!(
            "POSTGRES_CASE_COMPLETED {test} duration={:?}",
            started.elapsed()
        );
    }
    Ok(())
}

pub(super) fn expected_charges(claim: &str) -> Vec<(String, i32)> {
    let ids: &[u16] = match claim.strip_prefix("oteryn:reward-claim.") {
        Some("kv.deeperfibula4") => &[3082],
        Some("quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet") => &[3081],
        Some("quest.u15_24.targuna.silver_amulet_chest") => &[3054],
        Some("quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield") => &[3048, 3085],
        Some("kv.ornamentedshield") => &[3083],
        _ => &[],
    };
    ids.iter()
        .map(|id| (format!("oteryn:item.tibia.i{id}"), 1))
        .collect()
}

pub(super) fn expected_fluids(claim: &str) -> Vec<(String, i32)> {
    match claim.strip_prefix("oteryn:reward-claim.") {
        Some("kv.goblintemple2") => vec![("oteryn:item.tibia.i2874".into(), 9)],
        Some("quest.u7_9.dreamers_challenge.chests_wine") => {
            vec![("oteryn:item.tibia.i2874".into(), 2)]
        }
        Some("kv.present_box") => vec![
            ("oteryn:item.tibia.i2881".into(), 1),
            ("oteryn:item.tibia.i2882".into(), 1),
        ],
        _ => Vec::new(),
    }
}

pub(super) async fn assert_charges(
    pool: &sqlx::PgPool,
    root: [u8; 16],
    chest: &CatalogueChest,
) -> TestResult {
    let actual: Vec<(String, i32, i64, bool, bool, bool)> = sqlx::query_as(
        "SELECT i.definition_production_key,t.charges,t.state_revision::bigint, \
            t.remaining_ms IS NULL,t.deadline_at IS NULL,t.xmin=r.xmin \
         FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) \
         JOIN game_reward_claim_mint_receipts r ON r.item_instance_id=encode($1,'hex')::uuid \
         WHERE i.item_instance_id=r.item_instance_id OR i.item_instance_id IN \
            (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=r.item_instance_id) \
         ORDER BY i.definition_production_key",
    )
    .bind(root.as_slice())
    .fetch_all(pool)
    .await?;
    let expected: Vec<_> = chest
        .charges
        .iter()
        .map(|(key, count)| (key.clone(), *count, 1, true, true, true))
        .collect();
    assert_eq!(
        actual, expected,
        "exact source charge state, minted atomically and stable on replay"
    );
    let (reserved, committed): (Value, Value) = sqlx::query_as(
        "SELECT v.charges_plan,r.charges_plan FROM game_reward_claim_mint_receipts r \
         JOIN game_reward_claim_mint_reservations v USING(game_session_id,command_id) \
         WHERE r.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(root.as_slice())
    .fetch_one(pool)
    .await?;
    let mut expected_plan = Vec::new();
    for (key, charges) in &chest.charges {
        let ordinal = if *key == chest.root {
            0
        } else {
            chest
                .contents
                .iter()
                .position(|(child, _, _)| child == key)
                .ok_or("charged child absent")?
                + 1
        };
        expected_plan.push(serde_json::json!({"ordinal": ordinal, "charges_remaining": charges}));
    }
    expected_plan.sort_by_key(|value| value["ordinal"].as_u64());
    assert_eq!(committed, Value::Array(expected_plan));
    assert_eq!(
        reserved, committed,
        "reservation binds the exact complete charge plan"
    );
    if !chest.charges.is_empty() {
        let shape: (bool, bool, bool, bool, bool) = sqlx::query_as(
            "SELECT game_reward_claim_charges_plan_valid(r.charges_plan,r.contents_plan), \
                game_reward_claim_charges_plan_valid(r.charges_plan#-'{0,charges_remaining}',r.contents_plan), \
                game_reward_claim_charges_plan_valid(jsonb_set(r.charges_plan,'{0,charges_remaining}','2'),r.contents_plan), \
                game_reward_claim_charges_plan_valid(jsonb_set(r.charges_plan,'{0,ordinal}','-1'),r.contents_plan), \
                game_reward_claim_charges_plan_valid(r.charges_plan||r.charges_plan,r.contents_plan) \
             FROM game_reward_claim_mint_receipts r WHERE r.item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(root.as_slice())
        .fetch_one(pool)
        .await?;
        assert_eq!(shape, (true, false, false, false, false));
        for mutation in [
            "UPDATE game_reward_claim_mint_reservations SET charges_plan='[]'::jsonb WHERE item_instance_id=encode($1,'hex')::uuid",
            "UPDATE game_item_timed_states SET charges=2,state_revision=state_revision+1 WHERE item_instance_id=encode($1,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)",
        ] {
            let changed = sqlx::query(mutation)
                .bind(root.as_slice())
                .execute(pool)
                .await;
            assert!(
                matches!(changed, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("23514")),
                "charge initialization never permits later plan substitution or unfenced inflation"
            );
        }
    }
    Ok(())
}

/// The binder admits a claim only when every source placement is present.
/// Preserve all sibling selectors even while exercising one clicked chest.
pub(super) fn catalogue_source_world(
    chest: &CatalogueChest,
) -> TestResult<crate::map::boot::BundleWorld> {
    catalogue_source_worlds(std::slice::from_ref(chest), &[], &[], false)
}

pub(super) fn fibula_cases() -> TestResult<Vec<CatalogueChest>> {
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|c| {
            c.claim.starts_with("oteryn:reward-claim.kv.deeperfibula")
                || c.claim == "oteryn:reward-claim.quest.key.id3980"
        })
        .collect();
    assert_eq!(selected.len(), 6);
    Ok(selected)
}

/// Combine exact source selectors for one continuous qualification path. Extra
/// props retain raw source attributes; teleport z is lowered to the native signed floor byte.
pub(super) fn catalogue_source_worlds(
    chests: &[CatalogueChest],
    props: &[(
        oteryn_gameplay_quests::quest::world_callbacks::Position,
        u16,
        oteryn_world_bundle::sector::Attrs,
    )],
    cells: &[oteryn_gameplay_quests::quest::world_callbacks::Position],
    corridors: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    let chest = chests.first().ok_or("source chest cohort")?;
    use oteryn_world_bundle::{bundle, sector};
    use std::collections::BTreeMap;
    let mut placements = Vec::new();
    for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("claim records")? {
            let definition = &row["definition"];
            if !chests
                .iter()
                .any(|c| definition["identity"]["key"] == c.claim)
            {
                continue;
            }
            for source in definition["placements"]
                .as_array()
                .ok_or("claim placements")?
            {
                let binding = &source["source_binding"];
                let position = &binding["project_position"];
                let position = (
                    u16::try_from(position["x"].as_u64().ok_or("source x")?)?,
                    u16::try_from(position["y"].as_u64().ok_or("source y")?)?,
                    -i8::try_from(position["z"].as_u64().ok_or("source z")?)?,
                );
                if !chests.iter().any(|chest| {
                    definition["identity"]["key"] == chest.claim
                        && (chest.servable || chest.position == position)
                }) {
                    continue;
                }
                let ids = binding["legacy_unique_ids"].as_array().ok_or("source ids")?;
                // Preserve a Canary-only selector as a negative source-alias
                // fixture. The production binder must still refuse to serve it.
                let unique = ids
                    .iter()
                    .find(|id| id["server"] == "crystalserver")
                    .or_else(|| ids.first())
                    .ok_or("source selector")?;
                placements.push((
                    position.0,
                    position.1,
                    position.2,
                    u16::try_from(unique["unique_id"].as_u64().ok_or("source uid")?)?,
                    u16::try_from(
                        source["appearance_tibia_id"]
                            .as_u64()
                            .ok_or("source appearance")?,
                    )?,
                ));
            }
        }
    }
    assert!(
        !placements.is_empty(),
        "every catalogue claim has placed sources"
    );
    for chest in chests {
        assert_eq!(
            placements
                .iter()
                .filter(|p| (p.0, p.1, p.2) == chest.position)
                .count(),
            1,
            "every requested source placement must be present exactly once"
        );
    }
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut appearances = BTreeMap::new();
    let mut tiles = BTreeMap::<(i8, u16, u16), sector::Tile>::new();
    for (x, y, floor, unique, appearance) in placements {
        let palette = if let Some(index) = appearances.get(&appearance) {
            *index
        } else {
            let index = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                id: index,
                terrain: Some(bundle::Terrain {
                    blocks_projectile: None,
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                }),
            });
            appearances.insert(appearance, index);
            index
        };
        for tile_x in x.saturating_sub(2)..=x {
            tiles
                .entry((floor, y, tile_x))
                .or_insert_with(|| sector::Tile {
                    x: tile_x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: sector::Attrs::default(),
                    }],
                });
        }
        tiles
            .get_mut(&(floor, y, x))
            .ok_or("source tile")?
            .items
            .push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs {
                    // Zero requests the actual native placement without a retained source tag.
                    unique: (chest.unique != 0).then_some(unique),
                    ..Default::default()
                },
            });
    }
    let ground = |p: oteryn_gameplay_quests::quest::world_callbacks::Position| sector::Tile {
        x: p.x,
        y: p.y,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items: vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }],
    };
    for p in cells {
        tiles
            .entry((p.floor, p.y, p.x))
            .or_insert_with(|| ground(*p));
    }
    for (p, appearance, attrs) in props {
        let palette = if let Some(index) = appearances.get(appearance) {
            *index
        } else {
            let index = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                id: index,
                terrain: Some(bundle::Terrain {
                    blocks_projectile: None,
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                }),
            });
            appearances.insert(*appearance, index);
            index
        };
        tiles
            .entry((p.floor, p.y, p.x))
            .or_insert_with(|| ground(*p))
            .items
            .push(sector::Item {
                palette,
                depth: 0,
                attrs: attrs.clone(),
            });
    }
    if corridors {
        let mut bounds = BTreeMap::<i8, (u16, u16, u16, u16)>::new();
        for &(floor, y, x) in tiles.keys() {
            bounds
                .entry(floor)
                .and_modify(|b| {
                    b.0 = b.0.min(x);
                    b.1 = b.1.min(y);
                    b.2 = b.2.max(x);
                    b.3 = b.3.max(y);
                })
                .or_insert((x, y, x, y));
        }
        for (floor, (min_x, min_y, max_x, max_y)) in bounds {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    tiles.entry((floor, y, x)).or_insert_with(|| {
                        ground(oteryn_gameplay_quests::quest::world_callbacks::Position {
                            x,
                            y,
                            floor,
                        })
                    });
                }
            }
        }
    }
    let coordinates: Vec<_> = tiles.keys().copied().collect();
    data.manifest.world = bundle::Extent {
        min_x: coordinates
            .iter()
            .map(|(_, _, x)| *x)
            .min()
            .ok_or("source minimum x")?,
        min_y: coordinates
            .iter()
            .map(|(_, y, _)| *y)
            .min()
            .ok_or("source minimum y")?,
        max_x: coordinates
            .iter()
            .map(|(_, _, x)| *x)
            .max()
            .ok_or("source maximum x")?
            + 1,
        max_y: coordinates
            .iter()
            .map(|(_, y, _)| *y)
            .max()
            .ok_or("source maximum y")?
            + 1,
        floors: coordinates
            .iter()
            .map(|(floor, _, _)| *floor)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    for tile in tiles.values_mut() {
        for item in &mut tile.items {
            if let Some((_, _, z)) = &mut item.attrs.teleport {
                let floor = -i8::try_from(*z)?;
                if !data.manifest.world.floors.contains(&floor) {
                    return Err("source teleport floor".into());
                }
                *z = floor.to_ne_bytes()[0];
            }
        }
    }
    let mut grouped = BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for ((floor, y, x), tile) in tiles {
        grouped
            .entry((floor, y / 32, x / 32))
            .or_default()
            .push(tile);
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(super::fixture::debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: chest.position.0 - 1,
                y: chest.position.1,
                floor: chest.position.2,
            },
        },
        crate::foundation::WorldId::decode(&super::fixture::id(42))
            .map_err(super::fixture::debug)?,
        crate::foundation::ChannelId::decode(&super::fixture::id(43))
            .map_err(super::fixture::debug)?,
        |_| None,
    )
    .map_err(super::fixture::debug)
}

/// Set OTERYN_PROFILE_PG_CHESTS when running a catalogue test with --nocapture.
pub(super) fn profile_phases(case: &str) -> impl FnMut(&str) + '_ {
    let mut last = std::env::var_os("OTERYN_PROFILE_PG_CHESTS")
        .is_some()
        .then(std::time::Instant::now);
    move |name| {
        if let Some(start) = &mut last {
            let now = std::time::Instant::now();
            eprintln!("PG_CHEST_PHASE case={case} phase={name} us={}", (now - *start).as_micros());
            *start = now;
        }
    }
}

pub(super) fn execute(cases: Vec<CatalogueChest>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            // Every case has an isolated durable world/session, using the same
            // immutable production content generation. Qualify that content once;
            // no mutable character, command or receipt state is shared.
            let mut slice_phase = profile_phases("slice");
            let world = crate::foundation::WorldId::decode(&super::fixture::id(42))
                .map_err(|error| format!("{error:?}"))?;
            let activation =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")
                    .map_err(|error| error.to_string())?;
            let generation = activation.0.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native generation")?;
            slice_phase("activation");
            let quests = super::quest_container_chests_fixtures::embedded_catalogue()
                .map_err(|error| error.to_string())?;
            slice_phase("quest_catalogue");
            let mut template = None;
            super::fixture::run(async |admin| {
                template = Some(super::fixture::MigratedDatabaseTemplate::create(admin).await?);
                Ok(())
            }).map_err(|error| error.to_string())?;
            slice_phase("migration_template");
            let result = (|| -> Result<(), String> {
            let batch = super::quest_container_chests_fixtures::CatalogueBatch {
                template: template.as_ref().ok_or("migration template")?,
                quests,
            };
            for case in cases {
                for key in std::iter::once(case.root.as_str())
                    .chain(case.contents.iter().map(|(key, _, _)| key.as_str()))
                {
                    let definition = crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(), production_key: key.into(), revision_ref: "definition-r1".into(),
                    };
                    let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(native, &definition)
                        .map_err(|error| format!("{} {key}: {error:?}", case.claim))?;
                    if matches!(facts.stack, crate::durability::item_transfer::ItemStackClass::Unknown) {
                        return Err(format!("{} {key}: native Item admission missing", case.claim));
                    }
                }
                let started = std::time::Instant::now();
                super::quest_container_chests_fixtures::catalogue_scenario_in_batch(&case, &activation, &batch)
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
            })();
            super::fixture::run(async |_| {
                template.ok_or("migration template")?.cleanup().await
            }).map_err(|error| error.to_string())?;
            result
        })?
        .join()
        .map_err(|_| "catalogue chest scenario panicked")??;
    Ok(())
}
