//! INSPECT-1 read-only composition. Handles, current custody and the content pin are
//! revalidated after metadata I/O; instance identities never leave the server.
use super::item_ref_admission::{definition_ref, pinned_index};
use super::{
    ComposedFreshAdmission,
    connection::AdmittedSession,
    item_view::{ItemKey, SessionItemView},
};
use crate::content::{ReferenceItemField::Known, native_gameplay::ItemProfileRecord};
use crate::durability::{inspection::ItemMetadata, item_mint::TypedDefinitionRef};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::inspection::{InspectedItem, Reply, Request};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::{collections::HashMap, sync::Mutex};
#[path = "inspection_imbuements.rs"]
mod imbuement_text;

macro_rules! authority_methods {
    () => {
        fn inspection_command(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _request: oteryn_protocol_oteryn::inspection::Request,
            _views: Option<&$crate::gameplay_transport::item_view::SessionItemView>,
        ) -> impl std::future::Future<Output = oteryn_protocol_oteryn::inspection::Reply> {
            async { oteryn_protocol_oteryn::inspection::Reply::refused(2) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn inspection_command(
            &self,
            admitted: AdmittedSession,
            id: u64,
            request: oteryn_protocol_oteryn::inspection::Request,
            views: Option<&$crate::gameplay_transport::item_view::SessionItemView>,
        ) -> oteryn_protocol_oteryn::inspection::Reply {
            inspection::inspect(self, admitted, id, request, views)
                .await
                .unwrap_or_else(|| oteryn_protocol_oteryn::inspection::Reply::refused(2))
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn inspect(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    request: Request,
    views: Option<&SessionItemView>,
) -> Option<Reply> {
    let actor = admitted.runtime_actor?;
    {
        let runtime = owner.runtime.lock().await;
        runtime.owner_fence().ok()?;
        if runtime
            .player_control_facts(actor, admitted.game_session_id)
            .ok()?
            .control_loss
            .is_some()
        {
            return None;
        }
        pinned_index(&runtime, owner.active_generation)?;
    }
    match request {
        Request::Item(handle) => {
            let views = views?;
            let (definition, count, _) = super::item_look::observe_item(
                owner,
                actor,
                admitted.game_session_id,
                admitted.item_fence,
                views,
                handle,
            )
            .await
            .ok()?;
            let key = views.resolve(handle)?;
            let metadata = metadata(owner, admitted, key, &definition, count).await?;
            // Unmaterialized map entries have no durable instance attributes. Their
            // first MINT has tier zero and no applied imbuements; this is an actual
            // object inspection rather than a catalogue template. Corpse entities
            // retain their separate owner and are not assigned synthetic contents.
            let metadata = metadata.or_else(|| {
                matches!(key, ItemKey::MapBase { .. } | ItemKey::MapAdded(_)).then(|| {
                    ItemMetadata {
                        definition: definition.clone(),
                        quantity: count,
                        tier: 0,
                        imbuements: Vec::new(),
                        contents: vec![(definition.clone(), count)],
                        charges: None,
                        remaining_ms: None,
                    }
                })
            });

            let (after, after_count, _) = super::item_look::observe_item(
                owner,
                actor,
                admitted.game_session_id,
                admitted.item_fence,
                views,
                handle,
            )
            .await
            .ok()?;
            if after != definition || after_count != count {
                return None;
            }
            let item = project(owner, 0, &definition, count, metadata.as_ref()).await?;
            Some(Reply {
                status: 1,
                requests: Vec::new(),
                items: vec![item],
            })
        }
        Request::Catalogue(reference) => {
            let definition = {
                let runtime = owner.runtime.lock().await;
                super::item_look::typed_definition(
                    pinned_index(&runtime, owner.active_generation)?,
                    std::num::NonZeroU32::new(reference)?,
                )?
            };
            Some(Reply {
                status: 1,
                requests: Vec::new(),
                items: vec![project(owner, 0, &definition, 1, None).await?],
            })
        }
        Request::Trade { counter, index } => {
            let (instance, definition, count) = owner
                .inspection_trade_item(admitted, id, counter, index)
                .await?;
            let metadata = metadata(
                owner,
                admitted,
                ItemKey::Instance(instance),
                &definition,
                count,
            )
            .await??;
            let after = owner
                .inspection_trade_item(admitted, id, counter, index)
                .await?;
            if after != (instance, definition.clone(), count) {
                return None;
            }
            Some(Reply {
                status: 1,
                requests: Vec::new(),
                items: vec![project(owner, 0, &definition, count, Some(&metadata)).await?],
            })
        }
        Request::Permission { action, target } => permission(owner, admitted, action, target).await,
        Request::Player(target) => {
            let (target_actor, session) = player(owner, actor, target).await?;
            if target_actor != actor
                && !owner.states.inspection_permissions.allowed(
                    session,
                    admitted.game_session_id,
                    own_entity(owner, actor).await?,
                    target,
                )
            {
                return Some(Reply::refused(6));
            }
            let equipment = super::equipment_move::read_current_equipment(owner, session).await?;
            let character = crate::domain::CharacterId::from_bytes(equipment.character).ok()?;
            let backpack = owner
                .root
                .read_character_backpack(owner.character, character)
                .await
                .ok()?;
            let mut items = Vec::new();
            if let Some(backpack) = &backpack {
                let item = &backpack.backpack;
                let metadata = metadata(
                    owner,
                    admitted,
                    ItemKey::Instance(item.item_instance_id),
                    &item.definition,
                    item.quantity,
                )
                .await??;
                items.push(
                    project(owner, 10, &item.definition, item.quantity, Some(&metadata)).await?,
                );
            }
            for item in &equipment.items {
                let slot = oteryn_protocol_oteryn::item_view::wire_slot(item.slot)? as u8;
                let metadata = metadata(
                    owner,
                    admitted,
                    ItemKey::Instance(item.item_instance_id),
                    &item.definition,
                    item.quantity,
                )
                .await??;
                items.push(
                    project(
                        owner,
                        slot,
                        &item.definition,
                        item.quantity,
                        Some(&metadata),
                    )
                    .await?,
                );
            }
            items.sort_by_key(|item| item.slot);
            if owner
                .root
                .read_character_backpack(owner.character, character)
                .await
                .ok()?
                != backpack
                || player(owner, actor, target).await? != (target_actor, session)
                || super::equipment_move::read_current_equipment(owner, session).await? != equipment
            {
                return None;
            }
            if target_actor != actor
                && !owner.states.inspection_permissions.allowed(
                    session,
                    admitted.game_session_id,
                    own_entity(owner, actor).await?,
                    target,
                )
            {
                return Some(Reply::refused(6));
            }
            Some(Reply {
                status: 1,
                items,
                requests: Vec::new(),
            })
        }
    }
}
async fn player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> Option<(
    crate::foundation::ExactActorRef,
    crate::foundation::GameSessionId,
)> {
    let mut runtime = owner.runtime.lock().await;
    match super::item_look::runtime_look_creature(
        &mut runtime,
        owner.states.chat.npc_talk(),
        actor,
        target,
    )
    .ok()?
    {
        super::item_look::RuntimeLook::Player { actor, session, .. } => Some((actor, session)),
        _ => None,
    }
}
async fn metadata(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    key: ItemKey,
    definition: &TypedDefinitionRef,
    count: u32,
) -> Option<Option<ItemMetadata>> {
    let ItemKey::Instance(instance) = key else {
        return Some(None);
    };
    let metadata = owner
        .root
        .read_inspection_metadata(owner.character, admitted.world_id, instance)
        .await
        .ok()??;
    if &metadata.definition != definition || metadata.quantity != count {
        return None;
    }
    Some(Some(metadata))
}
async fn project(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    slot: u8,
    definition: &TypedDefinitionRef,
    count: u32,
    metadata: Option<&ItemMetadata>,
) -> Option<InspectedItem> {
    let runtime = owner.runtime.lock().await;
    runtime.owner_fence().ok()?;
    let reference = definition_ref(pinned_index(&runtime, owner.active_generation)?, definition)?;
    let policy = owner
        .active_generation?
        .native_gameplay()?
        .item_policy(&definition.production_key, &definition.revision_ref)?;
    if policy.source_digest() != runtime.content_pin().server_artifact_digest()
        || policy.record().production_definition.family != definition.family
    {
        return None;
    }
    let weight = if let Some(metadata) = metadata {
        let mut total = 0u64;
        for (definition, count) in &metadata.contents {
            let policy = owner
                .active_generation?
                .native_gameplay()?
                .item_policy(&definition.production_key, &definition.revision_ref)?;
            let Known(physical) = &policy.record().semantics.physical else {
                return None;
            };
            let Known(weight) = physical.weight else {
                return None;
            };
            total = total.checked_add(u64::from(weight).checked_mul(u64::from(*count))?)?;
        }
        Some(total)
    } else {
        None
    };
    details(
        policy.record(),
        slot,
        reference.get(),
        count,
        metadata,
        weight,
    )
}
fn details(
    record: &ItemProfileRecord,
    slot: u8,
    reference: u32,
    count: u32,
    metadata: Option<&ItemMetadata>,
    total_weight: Option<u64>,
) -> Option<InspectedItem> {
    let s = &record.semantics;
    let Known(presentation) = &s.presentation else {
        return None;
    };
    let Known(name) = &presentation.name else {
        return None;
    };
    let two_handed = matches!(&s.equipment,Known(equipment) if matches!(&equipment.patterns,Known(patterns) if !patterns.is_empty() && patterns.iter().all(|pattern| pattern.primary_slot==Known(crate::content::ReferenceEquipmentSlot::Weapon) && matches!(&pattern.additional_reserved_slots,Known(slots) if slots.contains(&crate::content::ReferenceEquipmentSlot::Shield)))));
    let mut attributes = Vec::new();
    let mut add = |label: &str, value: String| {
        if !value.is_empty() {
            attributes.push((label.to_owned(), value));
        }
    };
    if let Known(value) = &presentation.description {
        add("Description", value.clone());
    }
    if let Known(value) = &s.container
        && let Known(capacity) = value.capacity
    {
        add("Capacity", capacity.to_string());
    }
    if let Known(weapon) = &s.weapon {
        let elemental = match &weapon.elemental {
            Known(elements) => elements
                .iter()
                .filter_map(|e| match e.points {
                    Known(points) if points.0 != 0 => Some(format!(
                        "{} {}",
                        points.0,
                        format!("{:?}", e.element).to_lowercase()
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" + "),
            _ => String::new(),
        };
        let attack = match weapon.attack {
            Known(v) => v.0,
            _ => 0,
        };
        let ranged = matches!(
            weapon.weapon_type,
            Known(
                crate::content::ReferenceWeaponType::Distance
                    | crate::content::ReferenceWeaponType::Ammunition
            )
        );
        if ranged {
            let mut parts = Vec::new();
            if attack != 0 {
                parts.push(format!("attack +{attack}"));
            }
            if let Known(hit) = weapon.hit_chance
                && hit.numerator != 0
            {
                parts.push(format!(
                    "chance to hit +{}%",
                    ratio(hit.numerator, hit.denominator)
                ));
            }
            if let Known(range) = weapon.range
                && range.0 != 0
            {
                parts.push(format!("{} fields", range.0));
            }
            add("Attack", parts.join(", "));
        } else if attack != 0 || !elemental.is_empty() {
            add(
                "Attack",
                if elemental.is_empty() {
                    attack.to_string()
                } else if attack == 0 {
                    elemental
                } else {
                    format!("{attack} physical + {elemental}")
                },
            );
        }
        if let Known(kind) = weapon.weapon_type {
            add(
                "Weapon Type",
                format!(
                    "{}{}",
                    match kind {
                        crate::content::ReferenceWeaponType::Ammunition => "ammunition",
                        crate::content::ReferenceWeaponType::Axe => "axe",
                        crate::content::ReferenceWeaponType::Club => "club",
                        crate::content::ReferenceWeaponType::Distance => "distance",
                        crate::content::ReferenceWeaponType::Fist => "fist",
                        crate::content::ReferenceWeaponType::Shield => "shield",
                        crate::content::ReferenceWeaponType::Spellbook => "shield",
                        crate::content::ReferenceWeaponType::Sword => "sword",
                        crate::content::ReferenceWeaponType::Wand => "wand",
                    },
                    if two_handed { ", two-handed" } else { "" }
                ),
            );
        }
        if let Known(defense) = weapon.defense {
            let extra = match weapon.extra_defense {
                Known(v) => v.0,
                _ => 0,
            };
            if defense.0 != 0 || extra != 0 {
                add(
                    "Defence",
                    if extra == 0 {
                        defense.0.to_string()
                    } else {
                        format!("{} {extra:+}", defense.0)
                    },
                );
            }
        }
        if let Known(hit) = weapon.hit_chance
            && hit.numerator != 0
        {
            add("HitChance", ratio(hit.numerator, hit.denominator));
        }
    }
    if let Known(protection) = &s.protection {
        if let Known(armor) = protection.armor
            && armor.0 != 0
        {
            add("Armor", armor.0.to_string());
        }
        if let Known(resistances) = &protection.resistances {
            let values: Vec<_> = resistances
                .iter()
                .filter_map(|r| {
                    if r.kind == crate::content::ReferenceResistanceKind::FireField {
                        return None;
                    }
                    let Known(p) = r.percent else {
                        return None;
                    };
                    (p.numerator != 0).then(|| {
                        format!(
                            "{} {}%",
                            format!("{:?}", r.kind).to_lowercase(),
                            signed_ratio(p.numerator, p.denominator)
                        )
                    })
                })
                .collect();
            if !values.is_empty() {
                add("Protection", values.join(", "));
            }
            for resistance in resistances {
                if resistance.kind == crate::content::ReferenceResistanceKind::FireField
                    && let Known(p) = resistance.percent
                    && p.numerator != 0
                {
                    add(
                        "Field Protection",
                        format!("fire {}%", signed_ratio(p.numerator, p.denominator)),
                    );
                }
            }
        }
    }
    if let Known(modifiers) = &s.skill_modifiers
        && let Known(modifiers) = &modifiers.modifiers
    {
        use crate::content::{
            ReferenceModifierParameter as Parameter, ReferenceSkillModifierKind as Kind,
        };
        let mut boosts = Vec::new();
        for modifier in modifiers {
            let Known(parameter) = &modifier.parameter else {
                continue;
            };
            let value = match parameter {
                Parameter::SignedPoints(p) => format!("{:+}", p.0),
                Parameter::RationalPercent(p) => {
                    format!("{}%", signed_ratio(p.numerator, p.denominator))
                }
                Parameter::Cells(v) => v.0.to_string(),
                Parameter::Milliseconds(v) => v.0.to_string(),
                Parameter::Boolean(_) => String::new(),
                Parameter::Element(e) => format!("{e:?}"),
            };
            let label = match modifier.kind {
                Kind::MagicLevelPoints => "Magic Level",
                Kind::DeathMagicLevelPoints => "Death Magic Level",
                Kind::EarthMagicLevelPoints => "Earth Magic Level",
                Kind::EnergyMagicLevelPoints => "Energy Magic Level",
                Kind::FireMagicLevelPoints => "Fire Magic Level",
                Kind::HolyMagicLevelPoints => "Holy Magic Level",
                Kind::IceMagicLevelPoints => "Ice Magic Level",
                Kind::HealingMagicLevelPoints => "Healing Magic Level",
                Kind::Speed => "Speed",
                Kind::CleavePercent => "Cleave",
                Kind::ReflectDamage => "Damage Reflection",
                Kind::PerfectShotDamage | Kind::PerfectShotRange => continue,
                Kind::ElementalBond => "Elemental Bond",
                Kind::Invisibility if matches!(parameter, Parameter::Boolean(true)) => {
                    "Invisibility"
                }
                Kind::ManaShield if matches!(parameter, Parameter::Boolean(true)) => "Mana Shield",
                Kind::SuppressDrunk if matches!(parameter, Parameter::Boolean(true)) => {
                    "Hard Drinking"
                }
                Kind::SkillAxe
                | Kind::SkillClub
                | Kind::SkillDistance
                | Kind::SkillFist
                | Kind::SkillShield
                | Kind::SkillSword
                | Kind::CriticalHitChance
                | Kind::CriticalHitDamage
                | Kind::LifeLeechAmount
                | Kind::ManaLeechAmount => {
                    let name = match modifier.kind {
                        Kind::SkillAxe => "axe fighting",
                        Kind::SkillClub => "club fighting",
                        Kind::SkillDistance => "distance fighting",
                        Kind::SkillFist => "fist fighting",
                        Kind::SkillShield => "shielding",
                        Kind::SkillSword => "sword fighting",
                        Kind::CriticalHitChance => "critical hit chance",
                        Kind::CriticalHitDamage => "critical extra damage",
                        Kind::LifeLeechAmount => "life leech",
                        Kind::ManaLeechAmount => "mana leech",
                        _ => return None,
                    };
                    boosts.push(format!("{name} {value}"));
                    continue;
                }
                _ => continue,
            };
            if modifier.kind == Kind::MagicLevelPoints {
                boosts.push(format!("magic level {value}"));
            }
            if modifier.kind == Kind::Speed
                && let Parameter::SignedPoints(points) = parameter
            {
                boosts.push(format!("speed {:+}", points.0 >> 1));
            }
            if value.is_empty() {
                add("Effect", label.to_owned());
            } else {
                add(label, value);
            }
        }
        let parameter = |kind| {
            modifiers
                .iter()
                .find(|m| m.kind == kind)
                .and_then(|m| match &m.parameter {
                    Known(p) => Some(p),
                    _ => None,
                })
        };
        if let (Some(Parameter::SignedPoints(damage)), Some(Parameter::Cells(range))) = (
            parameter(Kind::PerfectShotDamage),
            parameter(Kind::PerfectShotRange),
        ) && range.0 != 0
        {
            add(
                "Perfect Shot",
                format!("{:+} at range {}", damage.0, range.0),
            );
        }
        let flat = match parameter(Kind::MagicShieldCapacityFlat) {
            Some(Parameter::SignedPoints(p)) => Some(p.0),
            _ => None,
        };
        let percent = match parameter(Kind::MagicShieldCapacityPercent) {
            Some(Parameter::RationalPercent(p)) => Some(*p),
            _ => None,
        };
        if flat.is_some_and(|p| p != 0) || percent.is_some_and(|p| p.numerator != 0) {
            add(
                "Magic Shield Capacity",
                format!(
                    "{:+} and {}%",
                    flat.unwrap_or(0),
                    percent.map_or_else(|| "0".into(), |p| ratio(p.numerator, p.denominator))
                ),
            );
        }
        let regenerates = |(gain, ticks)| matches!((parameter(gain),parameter(ticks)),(Some(Parameter::SignedPoints(gain)),Some(Parameter::Milliseconds(ticks))) if gain.0>0 && ticks.0>0);
        if regenerates((Kind::HealthGain, Kind::HealthTicks))
            || regenerates((Kind::ManaGain, Kind::ManaTicks))
        {
            add("Effect", "Faster Regeneration".into());
            boosts.push("faster regeneration".into());
        }
        if !boosts.is_empty() {
            add("Skill Boost", boosts.join(", "));
        }
    }
    if let Some(forge) = &record.authoring.forge {
        add("Classification", forge.classification.to_string());
        if let Some(metadata) = metadata {
            add("Tier", tier_description(record, metadata.tier));
        }
    }
    if let Known(imbuement) = &s.imbuement
        && let Known(slots) = imbuement.slot_count
    {
        if let Some(metadata) = metadata {
            for slot in 0..slots {
                let value = if let Some(i) = metadata
                    .imbuements
                    .iter()
                    .find(|i| i.slot == slot && i.remaining_ms != 0)
                {
                    let tier = match i.tier {
                        1 => "Basic",
                        2 => "Intricate",
                        3 => "Powerful",
                        _ => return None,
                    };
                    let minutes = i.remaining_ms / 60_000;
                    let (_, _, name, description) =
                        imbuement_text::DESCRIPTIONS
                            .iter()
                            .find(|(family, tier, _, _)| {
                                *family == i.category + 1 && *tier == i.tier
                            })?;
                    format!(
                        "{tier} {name} ({description}), lasts {:02}:{:02}h while fighting.",
                        minutes / 60,
                        minutes % 60
                    )
                } else {
                    "empty".to_owned()
                };
                add(&format!("Imbuement Slot {}", slot + 1), value);
            }
        } else if slots != 0 {
            add("Imbuement Slots", slots.to_string());
        }
    }
    if let Known(physical) = &s.physical
        && let Known(weight) = physical.weight
        && (weight != 0 || total_weight.is_some_and(|weight| weight != 0))
    {
        let total = total_weight.unwrap_or(u64::from(weight).checked_mul(u64::from(count))?);
        add(
            if metadata.is_some() && matches!(s.container, Known(_)) {
                "Total Weight"
            } else {
                "Weight"
            },
            format!("{}.{:02} oz", total / 100, total % 100),
        );
    }
    if let Some(metadata) = metadata {
        let charges = metadata.charges.or(match &s.charges {
            Known(charges) => match charges.count {
                Known(count) => Some(count),
                _ => None,
            },
            _ => None,
        });
        if let Some(charges) = charges {
            add("Charges", format!("{charges}/{charges}"));
        }
        if let Some(remaining) = metadata.remaining_ms {
            add("Expires", format!("{} seconds", remaining / 1000));
        }
    } else {
        if let Known(charges) = &s.charges
            && let Known(count) = charges.count
            && count != 0
        {
            add("Charges", format!("{count}/{count}"));
        }
        if let Known(temporal) = &s.temporal
            && let Known(duration) = temporal.duration
        {
            add(
                "Total Expire Time",
                format!("{} seconds", duration.0 / 1000),
            );
        }
    }
    if let Known(trade) = &s.trade_restrictions {
        let tradeable = if metadata.is_some() {
            &trade.tradeable
        } else {
            &trade.marketable
        };
        if tradeable == &Known(true)
            && !metadata.is_some_and(|m| m.imbuements.iter().any(|i| i.remaining_ms != 0))
        {
            add(
                if metadata.is_some() {
                    "Tradeable"
                } else {
                    "Tradeable In Market"
                },
                "yes".into(),
            );
        }
    }
    if let Some(level) = record.authoring.required_magic_level
        && level != 0
    {
        add("Required Magic Level", level.to_string());
    }
    if let Known(equipment) = &s.equipment
        && let Known(patterns) = &equipment.patterns
        && let Some(first) = patterns.first()
    {
        if patterns.iter().all(|p| p.vocations == first.vocations)
            && let Known(vocations) = &first.vocations
            && !vocations.is_empty()
        {
            add(
                "Professions",
                vocations
                    .iter()
                    .map(|v| format!("{v:?}").to_lowercase())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if patterns
            .iter()
            .all(|p| p.primary_slot == first.primary_slot)
            && let Known(slot) = first.primary_slot
        {
            add(
                "Body Position",
                if two_handed {
                    "both hands"
                } else {
                    match slot {
                        crate::content::ReferenceEquipmentSlot::Head => "head",
                        crate::content::ReferenceEquipmentSlot::Torso => "body",
                        crate::content::ReferenceEquipmentSlot::Legs => "legs",
                        crate::content::ReferenceEquipmentSlot::Feet => "feet",
                        crate::content::ReferenceEquipmentSlot::Weapon => "weapon hand",
                        crate::content::ReferenceEquipmentSlot::Shield => "shield hand",
                        crate::content::ReferenceEquipmentSlot::Amulet => "neck",
                        crate::content::ReferenceEquipmentSlot::Ring => "finger",
                        crate::content::ReferenceEquipmentSlot::Container => "container",
                        crate::content::ReferenceEquipmentSlot::Extra => "extra slot",
                    }
                }
                .to_owned(),
            );
        }
        if patterns.iter().all(|p| p.level == first.level)
            && let Known(level) = first.level
            && level != 0
        {
            add("Required Level", level.to_string());
        }
    }
    if let Known(requirements) = &s.use_requirements {
        if record.authoring.required_magic_level.is_none()
            && let Known(level) = requirements.min_magic_level
            && level != 0
        {
            add("Required Magic Level", level.to_string());
        }
        let patterns = match &s.equipment {
            Known(e) => match &e.patterns {
                Known(p) => Some(p),
                _ => None,
            },
            _ => None,
        };
        let has_level = patterns.is_some_and(|p| {
            p.first().is_some_and(|first| {
                matches!(first.level, Known(_)) && p.iter().all(|p| p.level == first.level)
            })
        });
        if !has_level
            && let Known(level) = requirements.min_level
            && level != 0
        {
            add("Required Level", level.to_string());
        }
        let has_vocations = patterns.is_some_and(|p| {
            p.first().is_some_and(|first| {
                matches!(&first.vocations,Known(v) if !v.is_empty())
                    && p.iter().all(|p| p.vocations == first.vocations)
            })
        });
        if !has_vocations
            && let Known(vocations) = &requirements.vocations
            && !vocations.is_empty()
        {
            add(
                "Professions",
                vocations
                    .iter()
                    .map(|v| format!("{v:?}").to_lowercase())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
    }
    let augment_text: Vec<_> = record
        .authoring
        .augments
        .iter()
        .filter_map(|a| a.inspection.as_deref())
        .collect();
    if !augment_text.is_empty() {
        add("Augments", augment_text.join(", "));
    }
    Some(InspectedItem {
        slot,
        definition_ref: reference,
        count,
        tier: metadata.map_or(0, |m| m.tier),
        name: name.clone(),
        attributes,
    })
}
/// Canary 47dfd51 item.cpp/config.lua.dist defaults; owner decision 22:
/// not verified against Global. These are display values, never combat authority.
fn tier_description(record: &ItemProfileRecord, tier: u8) -> String {
    use crate::content::{ReferenceEquipmentSlot as Slot, ReferenceItemField::Known};
    if tier == 0 {
        return "0".into();
    }
    let t = u64::from(tier);
    let effect = if matches!(&record.semantics.weapon,Known(w) if matches!(w.weapon_type,Known(_)))
    {
        Some((5 * t * t + 40 * t + 5, "Onslaught"))
    } else {
        let slot = match &record.semantics.equipment {
            Known(e) => match &e.patterns {
                Known(p) => p.first().and_then(|p| match p.primary_slot {
                    Known(s) => Some(s),
                    _ => None,
                }),
                _ => None,
            },
            _ => None,
        };
        match slot {
            Some(Slot::Head) => Some((5 * t * t + 190 * t + 5, "Momentum")),
            Some(Slot::Torso) => Some((
                (307576 * t * t + 4406970 * t + 260000 + 50000) / 100000,
                "Ruse",
            )),
            Some(Slot::Legs) => Some(((127 * t * t + 1070 * t + 73 + 50) / 100, "Transcendence")),
            Some(Slot::Feet) => Some((40 * t * t + 170 * t + 40, "Amplification")),
            _ => None,
        }
    };
    match effect {
        Some((chance, name)) => format!("{tier} ({}.{:02}% {name})", chance / 100, chance % 100),
        None => tier.to_string(),
    }
}

fn ratio(n: i64, d: u64) -> String {
    if d == 1 {
        n.to_string()
    } else {
        format!("{n}/{d}")
    }
}
fn signed_ratio(n: i64, d: u64) -> String {
    if n >= 0 {
        format!("+{}", ratio(n, d))
    } else {
        ratio(n, d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::content::{ReferenceItemField, ReferenceItemImbuement};
    use crate::durability::imbuement::StoredImbuement;
    #[test]
    fn inspection_permissions_bind_both_actor_generations_and_forget() -> Result<(), &'static str> {
        let permissions = Permissions::default();
        let id = |tag| {
            let mut b = [tag; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            GameSessionId::decode(&b).map_err(|_| "session")
        };
        let owner = id(1)?;
        let observer = id(2)?;
        let target = EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let entity = EntityRef {
            identity: [2; 16],
            generation: 1,
        };
        permissions
            .with(owner, target, |p| {
                p.grants.insert(observer, entity);
                Some(())
            })
            .ok_or("grant")?;
        assert!(permissions.allowed(owner, observer, entity, target));
        assert!(!permissions.allowed(
            owner,
            observer,
            EntityRef {
                generation: 2,
                ..entity
            },
            target
        ));
        assert!(!permissions.allowed(
            owner,
            observer,
            entity,
            EntityRef {
                generation: 2,
                ..target
            }
        ));
        permissions.forget(observer);
        assert!(!permissions.allowed(owner, observer, entity, target));
        permissions
            .with(owner, target, |p| {
                p.all = true;
                Some(())
            })
            .ok_or("all")?;
        assert!(permissions.allowed(owner, observer, entity, target));
        permissions.forget(owner);
        assert!(!permissions.allowed(owner, observer, entity, target));
        Ok(())
    }
    #[test]
    fn inspection_instance_details_include_weight_tier_and_live_imbuement()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = crate::foundation::WorldId::decode(&[
            1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1,
        ])
        .map_err(|e| format!("{e:?}"))?;
        let (controller, _) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let mut record = active
            .native_gameplay()
            .ok_or("native")?
            .item_policy("oteryn:item.tibia.i2854", "definition-r1")
            .ok_or("backpack")?
            .record()
            .clone();
        record.authoring.forge = Some(crate::content::ProjectV2ItemForgeProfile {
            classification: 4,
            max_tier: 10,
        });
        record.semantics.imbuement = Known(ReferenceItemImbuement {
            slot_count: Known(2),
            allowed_family_tiers: ReferenceItemField::Unknown,
            excluded_families: ReferenceItemField::Unknown,
        });
        record
            .authoring
            .augments
            .push(crate::content::ProjectV2AugmentBinding {
                key: "oteryn:augment.inspection.fixture".into(),
                inspection: Some("Auto Attack -> +5% Base Damage".into()),
                target: crate::content::ProjectV2AugmentTarget::AutoAttack,
                effect: None,
                rank_values: Vec::new(),
                fields: Vec::new(),
            });
        use crate::content::{
            ReferenceCells, ReferenceItemSkillModifiers, ReferenceModifierBinding,
            ReferenceModifierParameter as Parameter, ReferenceRationalPercent,
            ReferenceSignedPoints, ReferenceSkillModifierKind as Kind,
        };
        let binding = |kind, parameter| ReferenceModifierBinding {
            kind,
            target_domain: ReferenceItemField::Unknown,
            evaluation_phase: ReferenceItemField::Unknown,
            priority: ReferenceItemField::Unknown,
            parameter: Known(parameter),
        };
        record.semantics.skill_modifiers = Known(ReferenceItemSkillModifiers {
            modifiers: Known(vec![
                binding(
                    Kind::PerfectShotDamage,
                    Parameter::SignedPoints(ReferenceSignedPoints(5)),
                ),
                binding(Kind::PerfectShotRange, Parameter::Cells(ReferenceCells(2))),
                binding(
                    Kind::MagicShieldCapacityFlat,
                    Parameter::SignedPoints(ReferenceSignedPoints(10)),
                ),
                binding(
                    Kind::MagicShieldCapacityPercent,
                    Parameter::RationalPercent(ReferenceRationalPercent {
                        numerator: 20,
                        denominator: 1,
                    }),
                ),
                binding(
                    Kind::Speed,
                    Parameter::SignedPoints(ReferenceSignedPoints(4)),
                ),
            ]),
        });
        let metadata = ItemMetadata {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: record.production_definition.production_key.clone(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            tier: 3,
            imbuements: vec![StoredImbuement {
                slot: 0,
                category: 0,
                tier: 3,
                remaining_ms: 72_000_000,
            }],
            contents: vec![],
            charges: Some(7),
            remaining_ms: Some(5000),
        };
        let item = details(&record, 0, 1, 1, Some(&metadata), Some(12345)).ok_or("details")?;
        let field = |name| {
            item.attributes
                .iter()
                .find(|(label, _)| label == name)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(field("Total Weight"), Some("123.45 oz"));
        assert_eq!(field("Tier"), Some("3"));
        assert_eq!(field("Classification"), Some("4"));
        assert_eq!(field("Augments"), Some("Auto Attack -> +5% Base Damage"));
        assert_eq!(field("Charges"), Some("7/7"));
        assert_eq!(field("Expires"), Some("5 seconds"));
        assert_eq!(field("Perfect Shot"), Some("+5 at range 2"));
        assert_eq!(field("Magic Shield Capacity"), Some("+10 and 20%"));
        assert_eq!(field("Speed"), Some("+4"));
        assert_eq!(field("Skill Boost"), Some("speed +2"));
        assert!(
            field("Imbuement Slot 1")
                .is_some_and(|text| text.contains("Powerful Strike") && text.contains("20:00h"))
        );
        assert_eq!(field("Imbuement Slot 2"), Some("empty"));
        let sword = active
            .native_gameplay()
            .ok_or("native")?
            .item_policy("oteryn:item.tibia.i3264", "definition-r1")
            .ok_or("sword")?;
        assert_eq!(tier_description(sword.record(), 1), "1 (0.50% Onslaught)");
        assert_eq!(tier_description(sword.record(), 0), "0");
        let mut two_handed = sword.record().clone();
        if let Known(equipment) = &mut two_handed.semantics.equipment
            && let Known(patterns) = &mut equipment.patterns
        {
            for pattern in patterns {
                pattern.primary_slot = Known(crate::content::ReferenceEquipmentSlot::Weapon);
                pattern.additional_reserved_slots =
                    Known(vec![crate::content::ReferenceEquipmentSlot::Shield]);
            }
        }
        let described = details(&two_handed, 0, 1, 1, None, None).ok_or("two handed")?;
        assert!(
            described
                .attributes
                .contains(&("Body Position".into(), "both hands".into()))
        );
        assert!(
            described
                .attributes
                .contains(&("Weapon Type".into(), "sword, two-handed".into()))
        );

        let catalogue = details(&record, 0, 1, 1, None, None).ok_or("catalogue")?;
        assert!(
            catalogue
                .attributes
                .contains(&("Imbuement Slots".into(), "2".into()))
        );
        assert!(
            !catalogue
                .attributes
                .iter()
                .any(|(label, _)| label == "Tier" || label.starts_with("Imbuement Slot "))
        );
        Ok(())
    }
}

const MAX_PERMISSION_SESSIONS: usize = 1024;
#[derive(Default)]
pub(super) struct Permissions(Mutex<HashMap<GameSessionId, Permission>>);
struct Permission {
    entity: EntityRef,
    all: bool,
    requests: HashMap<GameSessionId, (EntityRef, tokio::time::Instant)>,
    grants: HashMap<GameSessionId, EntityRef>,
}
impl Permissions {
    fn with<T>(
        &self,
        session: GameSessionId,
        entity: EntityRef,
        f: impl FnOnce(&mut Permission) -> Option<T>,
    ) -> Option<T> {
        let mut book = self.0.lock().ok()?;
        if book.len() >= MAX_PERMISSION_SESSIONS && !book.contains_key(&session) {
            return None;
        }
        let entry = book.entry(session).or_insert_with(|| Permission {
            entity,
            all: false,
            requests: HashMap::new(),
            grants: HashMap::new(),
        });
        if entry.entity != entity {
            *entry = Permission {
                entity,
                all: false,
                requests: HashMap::new(),
                grants: HashMap::new(),
            };
        }
        entry
            .requests
            .retain(|_, (_, at)| at.elapsed() < std::time::Duration::from_secs(60));
        f(entry)
    }
    fn allowed(
        &self,
        owner: GameSessionId,
        observer: GameSessionId,
        entity: EntityRef,
        target: EntityRef,
    ) -> bool {
        self.0.lock().ok().is_some_and(|book| {
            book.get(&owner).is_some_and(|p| {
                p.entity == target && (p.all || p.grants.get(&observer) == Some(&entity))
            })
        })
    }
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut book) = self.0.lock() {
            book.remove(&session);
            for entry in book.values_mut() {
                entry.requests.remove(&session);
                entry.grants.remove(&session);
            }
        }
    }
}
async fn own_entity(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> Option<EntityRef> {
    let runtime = owner.runtime.lock().await;
    let visible = runtime.visible_entities();
    let player = visible.players.iter().find(|p| p.actor == actor)?;
    Some(EntityRef {
        identity: actor.placement_identity(),
        generation: player.generation,
    })
}
async fn permission(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    action: u8,
    target: Option<EntityRef>,
) -> Option<Reply> {
    let actor = admitted.runtime_actor?;
    let own = own_entity(owner, actor).await?;
    let permissions = &owner.states.inspection_permissions;
    let peers = oteryn_protocol_oteryn::inspection::MAX_PERMISSION_PEERS;
    if action == 6 {
        let pending =
            permissions.with(admitted.game_session_id, own, |p| Some(p.requests.clone()))?;
        let mut requests = Vec::new();
        for (session, (entity, _)) in pending {
            if player(owner, actor, entity)
                .await
                .is_some_and(|(_, current)| session == current)
            {
                requests.push(entity);
            }
        }
        requests.sort_by_key(|p| (p.identity, p.generation));
        let mut reply = Reply::refused(8);
        reply.requests = requests;
        return Some(reply);
    }
    if action == 4 || action == 5 {
        permissions.with(admitted.game_session_id, own, |p| {
            p.all = action == 4;
            Some(())
        })?;
        return Some(Reply::refused(8));
    }
    let target = target?;
    let (_, session) = player(owner, actor, target).await?;
    match action {
        1 => {
            permissions.with(session, target, |p| {
                if p.requests.len() >= peers && !p.requests.contains_key(&admitted.game_session_id)
                {
                    return None;
                }
                p.requests
                    .insert(admitted.game_session_id, (own, tokio::time::Instant::now()));
                Some(())
            })?;
            Some(Reply::refused(7))
        }
        2 => {
            permissions.with(admitted.game_session_id, own, |p| {
                if p.grants.len() >= peers && !p.grants.contains_key(&session) {
                    return None;
                }
                p.grants.insert(session, target);
                p.requests.remove(&session);
                Some(())
            })?;
            Some(Reply::refused(8))
        }
        3 => {
            permissions.with(admitted.game_session_id, own, |p| {
                p.grants.remove(&session);
                p.requests.remove(&session);
                Some(())
            })?;
            Some(Reply::refused(8))
        }
        _ => None,
    }
}
