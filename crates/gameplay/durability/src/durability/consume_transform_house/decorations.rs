//! W18 source-backed House decoration recipes. The existing consume/transform
//! owner supplies the locked source, actor fence, custody, receipt and item audit.
use super::*;
use serde::Deserialize;
use std::sync::OnceLock;

type RankedTileTop = ((u8, bool, u64), Option<bool>, Option<bool>);

#[derive(Debug, Deserialize)]
struct Pair {
    item_id: u32,
    target: u32,
    donor: String,
}
#[derive(Debug, Deserialize)]
struct Mirror {
    item_ids: Vec<u32>,
    speech: Vec<String>,
    cooldown_ms: u64,
    exhausted_speech: String,
    donor: String,
}
#[derive(Debug, Deserialize)]
struct Effect {
    item_id: u32,
    effect_id: u16,
    effect_binding: String,
    donor: String,
    achievement_progress: Option<Value>,
}
#[derive(Debug, Deserialize)]
struct Birdcage {
    item_id: u32,
    target: u32,
    success_out_of: u64,
    effect_id: u16,
    effect_binding: String,
    donor: String,
}
#[derive(Debug, Deserialize)]
struct Tree {
    item_id: u32,
    target: u32,
    storage: u32,
    cooldown_seconds: u64,
    donor: String,
}
#[derive(Debug, Deserialize)]
struct NativeTransform {
    item_id: u32,
    target: u32,
    kind: String,
    donor: String,
    #[serde(default)]
    noop: bool,
}
#[derive(Debug, Deserialize)]
struct BedKit {
    item_id: u32,
    south: [u32; 2],
    east: [u32; 2],
    donor: String,
    effect_id: u16,
    effect_binding: String,
}
#[derive(Debug, Deserialize)]
struct Catalogue {
    schema: String,
    crystal_commit: String,
    carpets: Vec<Pair>,
    windows: Vec<Pair>,
    mirror: Mirror,
    bathtub: Pair,
    music: Vec<Effect>,
    birdcage: Birdcage,
    fish_tank: Effect,
    magic_trees: Vec<Tree>,
    bed_kits: Vec<BedKit>,
    native_transforms: Vec<NativeTransform>,
}

fn catalogue() -> Result<&'static Catalogue, DurabilityError> {
    static RULES: OnceLock<Option<Catalogue>> = OnceLock::new();
    RULES.get_or_init(|| {
        let rules: Catalogue = serde_json::from_str(include_str!("../../../../../../tools/content-schema/native-gameplay/house-decoration-actions.json")).ok()?;
        (rules.schema == "OTERYN_HOUSE_DECORATION_ACTIONS/v1"
            && rules.crystal_commit == "201189f871d23597b82caf88fd611454a2ddefb8"
            && !rules.mirror.speech.is_empty()).then_some(rules)
    }).as_ref().ok_or(DurabilityError::InvalidStoredState)
}

fn item_id(key: &str) -> Option<u32> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}

pub(super) fn is_bed_kit(key: &str) -> Result<bool, DurabilityError> {
    let rules = catalogue()?;
    Ok(item_id(key).is_some_and(|id| rules.bed_kits.iter().any(|kit| kit.item_id == id)))
}

async fn cooldown(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    context: &HouseActionContext,
    key: &str,
) -> Result<Option<i64>, DurabilityError> {
    sqlx::query_scalar("SELECT until_ms FROM game_house_decoration_cooldowns WHERE character_id=encode($1,'hex')::uuid AND cooldown_key=$2 FOR UPDATE")
        .bind(context.fence.character_id.as_bytes().as_slice()).bind(key).fetch_optional(&mut ***tx).await.map_err(Into::into)
}

// Keep the source recipe and its same-transaction prerequisites in one closed dispatch.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn plan(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    context: &HouseActionContext,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
    at: [i32; 3],
    placed: bool,
) -> Result<Option<Result<ActionPlan, UseDisposition>>, DurabilityError> {
    let Some(id) = item_id(&definition.production_key) else {
        return Ok(None);
    };
    let rules = catalogue()?;
    if context.rotation.is_some() && !placed {
        return Ok(Some(Err(UseDisposition::NotPossible)));
    }
    let mut target = id;
    let mut remove = false;
    let mut cues = Vec::<Value>::new();
    let mut extra_cues = Vec::<Value>::new();
    let mut base_changes: Option<Value> = None;
    let mut house_container_capacity: Option<u16> = None;
    let mut timed_decoration: Option<Value> = None;
    let mut speech: Option<String> = None;
    let mut achievement_progress = Vec::<Value>::new();
    let mut cooldown_effect: Option<Value> = None;
    let family;
    let donor;
    let draw = |domain, upper| {
        oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw(
            *context.fence.game_session_id.as_bytes(),
            request.command.command_id().get(),
            request.source,
            domain,
            upper,
        )
        .ok_or(DurabilityError::InvalidStoredState)
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut ***tx)
            .await?;
    if context.bed_target.is_some() && !rules.bed_kits.iter().any(|kit| kit.item_id == id) {
        return Ok(Some(Err(UseDisposition::NothingToUse)));
    }
    if let Some(kit) = rules
        .bed_kits
        .iter()
        .find(|kit| kit.item_id == id && context.rotation.is_none())
    {
        family = "bed_kit";
        donor = &kit.donor;
        let Some(bed) = &context.bed_target else {
            return Ok(Some(Err(UseDisposition::NothingToUse)));
        };
        let Some(first) = bed.get("target") else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let Some(before) = first
            .get("before_key")
            .and_then(Value::as_str)
            .and_then(item_id)
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if before == kit.south[0] || before == kit.east[0] {
            return Ok(Some(Err(UseDisposition::NothingToUse)));
        }
        let south = [2491, 5501, 15506].contains(&before)
            || rules.bed_kits.iter().any(|entry| entry.south[0] == before);
        let east = [2489, 5499, 15508].contains(&before)
            || rules.bed_kits.iter().any(|entry| entry.east[0] == before);
        let (name, pair) = if south {
            ("south", kit.south)
        } else if east {
            ("east", kit.east)
        } else {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        };
        let Some(partner) = bed.get(name).filter(|value| !value.is_null()) else {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        };
        let mut items = vec![first.clone(), partner.clone()];
        // Crystal bed_modification_kits.lua:10-16,37-46 transforms the two
        // adjacent native bed parts atomically, then removes exactly one kit.
        for (item, after) in items.iter_mut().zip(pair) {
            item.as_object_mut()
                .ok_or(DurabilityError::InvalidStoredState)?
                .insert(
                    "after_key".into(),
                    json!(format!("oteryn:item.tibia.i{after}")),
                );
        }
        for item in [&items[0], &items[0], &items[1]] {
            extra_cues.push(
                json!({"position":item.get("position").ok_or(DurabilityError::InvalidStoredState)?,
                "effect_id":kit.effect_id,"binding":kit.effect_binding}),
            );
        }
        base_changes = Some(
            json!({"bundle_digest":bed.get("bundle_digest").ok_or(DurabilityError::InvalidStoredState)?,"items":items}),
        );
        remove = true;
    } else if let Some(rule) = rules
        .carpets
        .iter()
        .find(|rule| rule.item_id == id && context.rotation.is_none())
    {
        family = "carpet";
        donor = &rule.donor;
        target = rule.target;
        // Crystal carpets.lua:172-201 counts the actual source tile, even for a
        // carried item whose getTile resolves through its owner (item.cpp:469-495).
        let position: Vec<u8> = at.iter().flat_map(|v| v.to_be_bytes()).collect();
        let rows = sqlx::query("SELECT i.definition_production_key,i.definition_revision_ref,h.stack_ordinal::text FROM game_item_house_interior_locations h JOIN game_item_instances i USING(item_instance_id,world_id) WHERE h.world_id=encode($1,'hex')::uuid AND h.house_key=$2 AND h.spatial_position=$3 AND i.lifecycle=1 ORDER BY h.stack_ordinal LIMIT 513")
            .bind(context.house.world_id().as_bytes().as_slice()).bind(context.house.house_key()).bind(position).fetch_all(&mut ***tx).await?;
        if rows.len() > 512 {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        }
        let mut tile_items = context
            .base_items
            .iter()
            .filter(|item| item.position == at)
            .map(|item| (item.definition.clone(), (false, item.ordinal)))
            .collect::<Vec<_>>();
        for row in rows {
            let ordinal = row
                .try_get::<String, _>("stack_ordinal")?
                .parse::<u64>()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            tile_items.push((
                TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: row.try_get("definition_production_key")?,
                    revision_ref: row.try_get("definition_revision_ref")?,
                },
                (true, ordinal),
            ));
        }
        let count = tile_items
            .iter()
            .filter(|(item, _)| item_id(&item.production_key) == Some(id))
            .count();
        if count == 1 {
            let mut top: Option<RankedTileTop> = None;
            for (item, ordinal) in &tile_items {
                let Some(policy) = native.item_policy(&item.production_key, &item.revision_ref)
                else {
                    return Ok(Some(Err(UseDisposition::NotPossible)));
                };
                let attributes = &policy.record().attributes;
                match attributes.always_on_top {
                    Some(false) => {}
                    Some(true) => {
                        let Some(order) = attributes.always_on_top_order else {
                            return Ok(Some(Err(UseDisposition::NotPossible)));
                        };
                        if top
                            .as_ref()
                            .is_none_or(|(prior, _, _)| (order, ordinal.0, ordinal.1) > *prior)
                        {
                            top = Some((
                                (order, ordinal.0, ordinal.1),
                                attributes.blocks_movement,
                                attributes.has_height,
                            ));
                        }
                    }
                    None => return Ok(Some(Err(UseDisposition::NotPossible))),
                }
            }
            // Crystal item.hpp:553-555: blocking && alwaysOnTop && !hasHeight.
            if let Some((_, blocking, height)) = top {
                match (blocking, height) {
                    (Some(true), Some(false)) => {
                        remove = true;
                        target = id;
                    }
                    (Some(false), _) | (_, Some(true)) => {}
                    _ => return Ok(Some(Err(UseDisposition::NotPossible))),
                }
            }
            if !remove
                && tile_items.iter().any(|(item, _)| {
                    item_id(&item.production_key).is_some_and(|other| {
                        other != id && rules.carpets.iter().any(|carpet| carpet.item_id == other)
                    })
                })
            {
                return Ok(Some(Err(UseDisposition::NotPossible)));
            }
        }
    } else if rules.mirror.item_ids.contains(&id) && context.rotation.is_none() {
        family = "mirror";
        donor = &rules.mirror.donor;
        let key = "wall-mirror";
        let previous = cooldown(tx, context, key).await?;
        // Crystal wall_mirror.lua:17-24: one shared per-player 20-hour exhaustion.
        if previous.is_some_and(|until| until > now) {
            speech = Some(rules.mirror.exhausted_speech.clone());
        } else {
            let choice = usize::try_from(draw(0xd8, rules.mirror.speech.len() as u64)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            speech = rules.mirror.speech.get(choice).cloned();
            let until = now
                .checked_add(
                    i64::try_from(rules.mirror.cooldown_ms)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                )
                .ok_or(DurabilityError::InvalidStoredState)?;
            cooldown_effect = Some(json!({"key":key,"before":previous,"until":until}));
        }
    } else if id == rules.bathtub.item_id && context.rotation.is_none() {
        family = "bathtub";
        donor = &rules.bathtub.donor;
        // Crystal bath_tub_drain.lua:3-9 tests every creature, including the user.
        if !placed || context.occupied.contains(&at) {
            return Ok(Some(Err(UseDisposition::Occupied)));
        }
        target = rules.bathtub.target;
    } else if let Some(rule) = rules
        .magic_trees
        .iter()
        .find(|rule| rule.item_id == id && context.rotation.is_none())
    {
        family = "magic_tree";
        donor = &rule.donor;
        // Crystal magic_tree.lua:18-36: placed in a House; storage >= os.time refuses.
        if !placed {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        }
        let key = format!("tree.{}", rule.storage);
        let previous = cooldown(tx, context, &key).await?;
        if previous.is_some_and(|until| until / 1000 >= now / 1000) {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        }
        let until = (now / 1000)
            .checked_add(
                i64::try_from(rule.cooldown_seconds)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .and_then(|seconds| seconds.checked_mul(1000))
            .ok_or(DurabilityError::InvalidStoredState)?;
        cooldown_effect = Some(json!({"key":key,"before":previous,"until":until}));
        target = rule.target;
    } else if let Some(rule) = rules
        .music
        .iter()
        .find(|rule| rule.item_id == id && context.rotation.is_none())
    {
        family = "music";
        donor = &rule.donor;
        cues.push(json!({"effect_id":rule.effect_id,"binding":rule.effect_binding}));
        // Crystal music.lua:65-66: every instrument advances Rockstar and emits notes.
        if let Some(progress) = &rule.achievement_progress {
            achievement_progress.push(progress.clone());
        }
    } else if id == rules.birdcage.item_id && context.rotation.is_none() {
        family = "birdcage";
        donor = &rules.birdcage.donor;
        // Crystal birdcage.lua:3-9: one in 100 frees the bird; other draws emit notes.
        if draw(0xd9, rules.birdcage.success_out_of)? == 0 {
            target = rules.birdcage.target;
        } else {
            cues.push(json!({"effect_id":rules.birdcage.effect_id,"binding":rules.birdcage.effect_binding}));
        }
    } else if id == rules.fish_tank.item_id && context.rotation.is_none() {
        family = "fish_tank";
        donor = &rules.fish_tank.donor;
        cues.push(
            json!({"effect_id":rules.fish_tank.effect_id,"binding":rules.fish_tank.effect_binding}),
        );
    } else if let Some(rule) = rules
        .windows
        .iter()
        .find(|rule| rule.item_id == id && context.rotation.is_none())
    {
        family = "window";
        donor = &rule.donor;
        target = rule.target;
    } else if let Some(rule) = rules.native_transforms.iter().find(|rule| {
        rule.item_id == id
            && rule.kind
                == if context.rotation.is_some() {
                    "ROTATE"
                } else {
                    "USE"
                }
    }) {
        // Crystal Game::transformItem game.cpp:3054-3057 returns the original
        // item for an unavailable target. Do not invent a canonical identity,
        // receipt mutation, revision bump or item audit for that source NOOP.
        if rule.noop {
            return Ok(Some(Err(UseDisposition::Committed)));
        }
        family = "native_transform";
        donor = &rule.donor;
        target = rule.target;
        let key = format!("oteryn:item.tibia.i{target}");
        let Some(before) = native
            .item_policy(&definition.production_key, &definition.revision_ref)
            .and_then(|policy| {
                crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(
                    policy,
                )
                .ok()
            })
        else {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        };
        let Some(after) = native
            .item_policy(&key, &definition.revision_ref)
            .and_then(|policy| {
                crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(
                    policy,
                )
                .ok()
            })
        else {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        };
        house_container_capacity = before.container_capacity;
        // Native rotations retain their existing container graph and metadata.
        if before.container_capacity != after.container_capacity {
            return Ok(Some(Err(UseDisposition::NotPossible)));
        }
        if before.decay.is_some() || after.decay.is_some() {
            let after_ref = TypedDefinitionRef {
                family: definition.family.clone(),
                production_key: key,
                revision_ref: definition.revision_ref.clone(),
            };
            let Some(metadata) = super::timed_decorations::prepare(
                tx, context, request, native, definition, &after_ref,
            )
            .await?
            else {
                return Ok(Some(Err(UseDisposition::NotPossible)));
            };
            target = metadata
                .get("selected_key")
                .and_then(Value::as_str)
                .and_then(item_id)
                .ok_or(DurabilityError::InvalidStoredState)?;
            timed_decoration = Some(metadata);
        }
    } else {
        return Ok(None);
    }
    let key = format!("oteryn:item.tibia.i{target}");
    let Some(policy) = native.item_policy(&key, &definition.revision_ref) else {
        return Ok(Some(Err(UseDisposition::NotPossible)));
    };
    if !policy.record().admission.materializable {
        return Ok(Some(Err(UseDisposition::NotPossible)));
    }
    let effects = json!({"kind":"house_decoration","source_item_id":id,"donor":donor,"decoration_family":family,
        "house_after":{"key":key,"revision":definition.revision_ref},"house_remove":remove,"house_mutate":target != id || remove,
        "ground":{"x":at[0],"y":at[1],"floor":at[2]},"cues":cues,"speech":speech,
        "private_speech":false,"require_head":false,"require_player_target":false,
        "achievement_progress":achievement_progress,"decoration_cooldown":cooldown_effect,
        "base_changes":base_changes,"extra_cues":extra_cues,
        "house_container_capacity":house_container_capacity,"timed_decoration":timed_decoration});
    Ok(Some(Ok(ActionPlan {
        recipe: format!("oteryn:house-decoration.{family}.i{id}.v1"),
        outputs: Vec::new(),
        consume_source: remove,
        source_count: u32::from(remove),
        effects,
        extra_inputs: Vec::new(),
    })))
}
