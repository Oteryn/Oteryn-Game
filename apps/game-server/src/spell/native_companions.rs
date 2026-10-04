//! Pure plans for source-qualified companion recipes. These snapshots are not
//! session, ownership or commit authority. The production owners must bind fresh
//! facts and execute ordered/conditional operations at their own fenced boundary.
//! The authoring reader must validate the entire canonical profile before `plan`.
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompanionError {
    InvalidParameters(&'static str),
    InvalidFacts(&'static str),
    ArithmeticBounds,
    Refused { message: String, poff: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompanionFacts {
    Familiar(FamiliarFacts),
    Haste(HasteFacts),
    Acquisition(AcquisitionFacts),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompanionPlan {
    Familiar(Vec<FamiliarAction>),
    Haste(HastePlan),
    Acquisition(AcquisitionPlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FamiliarEvent {
    Cast,
    Login,
    Advance,
    FamiliarDeath,
    Expiry,
    Warning { index: usize },
    ManualDispel,
    OwnerRemoval,
    Follow,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FamiliarFacts {
    pub(crate) event: FamiliarEvent,
    pub(crate) vocation: String,
    pub(crate) premium: bool,
    pub(crate) level: u32,
    pub(crate) account_at_least_god: bool,
    pub(crate) owned_summons: usize,
    pub(crate) spawn_room: bool,
    pub(crate) chosen_look: u32,
    pub(crate) has_vocation_look: bool,
    pub(crate) owner_current_speed: i32,
    pub(crate) familiar_base_speed: i32,
    pub(crate) familiar_minutes: i64,
    pub(crate) vip: bool,
    pub(crate) vip_reduction_minutes: i64,
    pub(crate) cooldown_rate: f32,
    pub(crate) now_unix: i64,
    pub(crate) saved_expiry_unix: i64,
    pub(crate) last_logout_unix: i64,
    pub(crate) owner_present: bool,
    pub(crate) creature_present: bool,
    pub(crate) creature_name: String,
    pub(crate) matching_summon_ids: Vec<u64>,
    pub(crate) dx: i32,
    pub(crate) dy: i32,
    pub(crate) dz: i32,
    pub(crate) owner_tile_is_teleport: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FamiliarAction {
    RegisterAdvance,
    RemoveVocationLook(u32),
    SelectLook(u32),
    GrantVocationLook(u32),
    /// Following actions execute only if the world owner creates this creature.
    Create {
        name: String,
        look: u32,
        extended: bool,
        force: bool,
        owned: bool,
    },
    RegisterDeath,
    CreationRefusal {
        message: String,
        poff: bool,
    },
    RegisterPartyProtectionAllOwnedSummons,
    ChangeSpeed(i32),
    CasterMagicBlue,
    CreatureTeleport,
    StoreExpiry(i64),
    ScheduleExpiry {
        delay_ms: u32,
    },
    ScheduleWarning {
        index: usize,
        raw_delay_ms: i64,
        delay_ms: u32,
        message: String,
    },
    Cooldown {
        reference_spell_id: u32,
        ticks_ms: i32,
        pauses_offline: bool,
    },
    CancelAndResetWarnings,
    ResetWarningHandles,
    RemoveCreature,
    RemoveFirstMatchingSummon {
        id: u64,
        creature_poff: bool,
    },
    RemoveOwnedSummons {
        invoke_familiar_death: bool,
    },
    WarningMessage(String),
    FollowOwner {
        teleport: bool,
        can_attack_in_protection_zone: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpeedKind {
    Haste,
    Paralyze,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionSlotId {
    Combat,
    Other(i32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpeedCondition {
    pub(crate) kind: SpeedKind,
    pub(crate) id: ConditionSlotId,
    pub(crate) sub_id: u32,
    pub(crate) ticks_ms: i32,
    pub(crate) delta: i32,
}
impl SpeedCondition {
    pub(crate) fn expiry_speed_change(&self) -> Result<i32, CompanionError> {
        self.delta
            .checked_neg()
            .ok_or(CompanionError::ArithmeticBounds)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpeedRecipient {
    pub(crate) id: u64,
    pub(crate) base_speed: i32,
    pub(crate) is_familiar: bool,
    pub(crate) haste_suppressed: bool,
    pub(crate) paralyze_suppressed: bool,
    pub(crate) conditions: Vec<SpeedCondition>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwiftGrade {
    None,
    Regular,
    Greater,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HasteFacts {
    pub(crate) caster: SpeedRecipient,
    pub(crate) owned_summons: Vec<SpeedRecipient>,
    pub(crate) swift_grade: SwiftGrade,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpeedOperation {
    Suppressed,
    KeepInfinite,
    Refresh {
        condition: SpeedCondition,
        change_delta: i32,
    },
    /// Opposite removals are requests to the condition owner, which applies its
    /// normal removal/delay semantics; they are not arbitrary speed resets.
    Start {
        condition: SpeedCondition,
        remove_opposites: Vec<SpeedCondition>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecipientSpeedPlan {
    pub(crate) recipient: u64,
    pub(crate) operation: SpeedOperation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HastePlan {
    /// Source updates familiar conditions before attempting caster combat.
    pub(crate) familiar_before_combat: Vec<RecipientSpeedPlan>,
    pub(crate) caster_combat: RecipientSpeedPlan,
    pub(crate) caster_magic_green: bool,
    /// Applied only after successful caster combat. None preserves other buffs.
    pub(crate) damage_modifier_after_success: Option<DamageModifierPlan>,
    pub(crate) attacks_and_casts_allowed: bool,
}

/// Request to the attribute-condition owner, not an independent scalar buff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DamageModifierPlan {
    pub(crate) percent: u32,
    pub(crate) ticks_ms: i32,
    pub(crate) id: ConditionSlotId,
    pub(crate) sub_id: u32,
    pub(crate) buff: bool,
    pub(crate) preserve_infinite_from_timed: bool,
    pub(crate) preserve_longer_remaining: bool,
    pub(crate) end_old_effects_then_replace_entire_slot: bool,
}
fn damage_modifier(percent: u32, ticks_ms: i32) -> DamageModifierPlan {
    DamageModifierPlan {
        percent,
        ticks_ms,
        id: ConditionSlotId::Combat,
        sub_id: 0,
        buff: false,
        preserve_infinite_from_timed: true,
        preserve_longer_remaining: true,
        end_old_effects_then_replace_entire_slot: true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcquisitionKind {
    NamedCreature,
    CorpseTile,
    TargetCreature,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcquisitionFacts {
    pub(crate) kind: AcquisitionKind,
    pub(crate) creature_name: String,
    pub(crate) type_found: bool,
    pub(crate) summonable: bool,
    pub(crate) target_is_monster: bool,
    pub(crate) convinceable: bool,
    pub(crate) target_master_name: Option<String>,
    pub(crate) tile_present: bool,
    pub(crate) top_down_item_present: bool,
    pub(crate) is_corpse: bool,
    pub(crate) corpse_movable: bool,
    pub(crate) owned_summons: usize,
    pub(crate) can_summon_all: bool,
    pub(crate) can_convince_all: bool,
    pub(crate) black_skull: bool,
    pub(crate) mana: u64,
    pub(crate) creature_mana_cost: u32,
    pub(crate) has_infinite_mana: bool,
    pub(crate) spawn_room: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AcquisitionAction {
    Create {
        name: String,
        on_target_tile: bool,
        extended: bool,
        force: bool,
        owned: bool,
    },
    SubtractMana(u32),
    AddManaSpent(u32),
    RemoveCorpse,
    AssignMaster,
    CasterMagicBlue,
    TargetMagicBlue,
    CreatureTeleport,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcquisitionPlan {
    pub(crate) actions: Vec<AcquisitionAction>,
    /// Create refusal must abort before corpse removal, spending or ownership.
    pub(crate) requires_creation_success: bool,
    pub(crate) consume_rune_charge_on_success: bool,
}

fn field<'a>(p: &'a Value, path: &'static str) -> Result<&'a Value, CompanionError> {
    p.pointer(path)
        .ok_or(CompanionError::InvalidParameters(path))
}
fn text<'a>(p: &'a Value, path: &'static str) -> Result<&'a str, CompanionError> {
    field(p, path)?
        .as_str()
        .ok_or(CompanionError::InvalidParameters(path))
}
fn number(p: &Value, path: &'static str) -> Result<i64, CompanionError> {
    field(p, path)?
        .as_i64()
        .ok_or(CompanionError::InvalidParameters(path))
}
fn flag(p: &Value, path: &'static str) -> Result<bool, CompanionError> {
    field(p, path)?
        .as_bool()
        .ok_or(CompanionError::InvalidParameters(path))
}
fn decimal(p: &Value, path: &'static str) -> Result<f64, CompanionError> {
    let v = text(p, path)?
        .parse::<f64>()
        .map_err(|_| CompanionError::InvalidParameters(path))?;
    if !v.is_finite() {
        return Err(CompanionError::ArithmeticBounds);
    }
    Ok(v)
}
fn narrow(v: i64) -> Result<i32, CompanionError> {
    i32::try_from(v).map_err(|_| CompanionError::ArithmeticBounds)
}
fn unsigned(v: i64) -> Result<u32, CompanionError> {
    u32::try_from(v).map_err(|_| CompanionError::ArithmeticBounds)
}
fn trunc_i32(v: f64) -> Result<i32, CompanionError> {
    let v = v.trunc();
    if !v.is_finite() || v < f64::from(i32::MIN) || v > f64::from(i32::MAX) {
        return Err(CompanionError::ArithmeticBounds);
    }
    Ok(v as i32)
}
fn mul(a: i64, b: i64) -> Result<i64, CompanionError> {
    a.checked_mul(b).ok_or(CompanionError::ArithmeticBounds)
}
fn sub(a: i64, b: i64) -> Result<i64, CompanionError> {
    a.checked_sub(b).ok_or(CompanionError::ArithmeticBounds)
}
fn refuse(p: &Value, path: &'static str) -> CompanionError {
    match text(p, path) {
        Ok(message) => CompanionError::Refused {
            message: message.to_owned(),
            poff: true,
        },
        Err(error) => error,
    }
}
fn scheduler_delay(p: &Value, raw_ms: i64) -> Result<u32, CompanionError> {
    if text(p, "/warning_dispatch/negative_delay_conversion")? != "unsigned_zero" {
        return Err(CompanionError::InvalidParameters(
            "negative delay conversion",
        ));
    }
    Ok(unsigned(raw_ms.max(0))?.max(unsigned(number(
        p,
        "/warning_dispatch/scheduler_minimum_ms",
    )?)?))
}

/// Caller must first qualify key and full closed canonical parameters. This
/// function neither registers an actor nor consumes a stored record as authority.
pub(crate) fn plan(
    parameters: &Value,
    facts: &CompanionFacts,
) -> Result<CompanionPlan, CompanionError> {
    match facts {
        CompanionFacts::Familiar(f) => familiar(parameters, f).map(CompanionPlan::Familiar),
        CompanionFacts::Haste(f) => haste(parameters, f).map(CompanionPlan::Haste),
        CompanionFacts::Acquisition(f) => acquire(parameters, f).map(CompanionPlan::Acquisition),
    }
}

fn familiar(p: &Value, f: &FamiliarFacts) -> Result<Vec<FamiliarAction>, CompanionError> {
    if f.vocation != text(p, "/vocation")? {
        return Err(CompanionError::InvalidFacts("vocation/profile binding"));
    }
    let look = unsigned(number(p, "/default_look_type")?)?;
    let mut out = Vec::new();
    let look_updates = |out: &mut Vec<FamiliarAction>| {
        if f.chosen_look == 0 {
            out.push(FamiliarAction::SelectLook(look));
        }
        if !f.has_vocation_look {
            out.push(FamiliarAction::GrantVocationLook(look));
        }
    };
    let seconds = match f.event {
        FamiliarEvent::Cast => {
            if !f.premium {
                return Err(refuse(p, "/failure_messages/premium"));
            }
            if f.owned_summons
                >= usize::try_from(number(p, "/summon_limit")?)
                    .map_err(|_| CompanionError::ArithmeticBounds)?
                && !f.account_at_least_god
            {
                return Err(refuse(p, "/failure_messages/summon_limit"));
            }
            Some(mul(
                f.familiar_minutes,
                number(p, "/duration/seconds_multiplier")?,
            )?)
        }
        FamiliarEvent::Login => {
            out.push(FamiliarAction::RegisterAdvance);
            if !f.premium || f.level < unsigned(number(p, "/login/minimum_level")?)? {
                if (!f.premium && f.has_vocation_look) || f.level < 200 {
                    out.push(FamiliarAction::RemoveVocationLook(look));
                }
                return Ok(out);
            }
            look_updates(&mut out);
            let basis = match text(p, "/login/lifetime_basis")? {
                "current_unix_time" => f.now_unix,
                "last_logout_unix_time" => f.last_logout_unix,
                _ => return Err(CompanionError::InvalidParameters("login lifetime basis")),
            };
            let left = sub(f.saved_expiry_unix, basis)?;
            let left = if flag(p, "/login/clamp_remaining_to_zero")? {
                left.max(0)
            } else {
                left
            };
            if left > 0 {
                Some(left)
            } else {
                return Ok(out);
            }
        }
        FamiliarEvent::Advance => {
            // Source conjunction is intentional: premium below200 still gains look.
            if f.level >= 200 || f.premium {
                look_updates(&mut out);
            }
            return Ok(out);
        }
        FamiliarEvent::FamiliarDeath => {
            if f.owner_present && f.creature_name == text(p, "/creature_name")? {
                out.push(FamiliarAction::StoreExpiry(f.now_unix));
                out.push(FamiliarAction::CancelAndResetWarnings);
            }
            return Ok(out);
        }
        FamiliarEvent::Expiry => {
            if f.owner_present && f.creature_present {
                out.push(FamiliarAction::RemoveCreature);
                out.push(FamiliarAction::ResetWarningHandles);
            }
            return Ok(out);
        }
        FamiliarEvent::Warning { index } => {
            let warning = field(p, "/warnings")?
                .as_array()
                .and_then(|v| v.get(index))
                .ok_or(CompanionError::InvalidFacts("warning index"))?;
            if f.owner_present {
                out.push(FamiliarAction::WarningMessage(format!(
                    "{}{}",
                    text(p, "/warning_dispatch/prefix")?,
                    text(warning, "/message")?
                )));
            }
            return Ok(out);
        }
        FamiliarEvent::ManualDispel => {
            if let Some(id) = f.matching_summon_ids.first() {
                out.push(FamiliarAction::CasterMagicBlue);
                out.push(FamiliarAction::RemoveFirstMatchingSummon {
                    id: *id,
                    creature_poff: true,
                });
            }
            return Ok(out);
        }
        FamiliarEvent::OwnerRemoval => {
            out.push(FamiliarAction::RemoveOwnedSummons {
                invoke_familiar_death: false,
            });
            return Ok(out);
        }
        FamiliarEvent::Follow => {
            let distance = f.dx.unsigned_abs().max(f.dy.unsigned_abs());
            let teleport = !f.owner_tile_is_teleport
                && (f.dz != 0
                    || distance > unsigned(number(p, "/return_to_owner/strict_distance_tiles")?)?);
            out.push(FamiliarAction::FollowOwner {
                teleport,
                can_attack_in_protection_zone: false,
            });
            return Ok(out);
        }
    };
    let seconds = seconds.ok_or(CompanionError::InvalidFacts("missing lifetime"))?;
    if !f.spawn_room {
        if f.event == FamiliarEvent::Login {
            out.push(FamiliarAction::CreationRefusal {
                message: text(p, "/failure_messages/spawn_room")?.to_owned(),
                poff: true,
            });
            return Ok(out);
        }
        return Err(refuse(p, "/failure_messages/spawn_room"));
    }
    let creation_delta = f
        .owner_current_speed
        .checked_sub(f.familiar_base_speed)
        .ok_or(CompanionError::ArithmeticBounds)?
        .max(narrow(number(p, "/creation_speed/minimum_delta")?)?);
    // Login fills default look before recreation, cast uses the chosen look verbatim.
    let chosen_look = if f.event == FamiliarEvent::Login && f.chosen_look == 0 {
        look
    } else {
        f.chosen_look
    };
    out.push(FamiliarAction::Create {
        name: text(p, "/creature_name")?.to_owned(),
        look: chosen_look,
        extended: flag(p, "/spawn/extended")?,
        force: flag(p, "/spawn/force")?,
        owned: true,
    });
    out.push(FamiliarAction::RegisterDeath);
    out.push(FamiliarAction::ChangeSpeed(creation_delta));
    out.push(FamiliarAction::CasterMagicBlue);
    out.push(FamiliarAction::CreatureTeleport);
    out.push(FamiliarAction::StoreExpiry(
        f.now_unix
            .checked_add(seconds)
            .ok_or(CompanionError::ArithmeticBounds)?,
    ));
    let millis = mul(seconds, 1000)?;
    out.push(FamiliarAction::ScheduleExpiry {
        delay_ms: scheduler_delay(p, millis)?,
    });
    let warnings = field(p, "/warnings")?
        .as_array()
        .ok_or(CompanionError::InvalidParameters("warnings"))?;
    for (index, warning) in warnings.iter().enumerate() {
        let raw_delay_ms = sub(millis, number(warning, "/remaining_ms")?)?;
        out.push(FamiliarAction::ScheduleWarning {
            index,
            raw_delay_ms,
            delay_ms: scheduler_delay(p, raw_delay_ms)?,
            message: format!(
                "{}{}",
                text(p, "/warning_dispatch/prefix")?,
                text(warning, "/message")?
            ),
        });
    }
    if f.event == FamiliarEvent::Cast {
        if flag(p, "/register_party_protection")? {
            out.push(FamiliarAction::RegisterPartyProtectionAllOwnedSummons);
        }
        if !f.cooldown_rate.is_finite() || f.cooldown_rate <= 0.0 {
            return Err(CompanionError::InvalidFacts(
                "positive finite cooldown rate",
            ));
        }
        let mut cooldown = mul(seconds, number(p, "/cooldown/duration_multiplier")?)?;
        if f.vip {
            cooldown = sub(
                cooldown,
                mul(
                    f.vip_reduction_minutes.min(seconds),
                    number(p, "/cooldown/vip_seconds_multiplier")?,
                )?,
            )?;
        }
        let ticks_ms = trunc_i32(mul(cooldown, 1000)? as f64 / f64::from(f.cooldown_rate))?;
        out.push(FamiliarAction::Cooldown {
            reference_spell_id: unsigned(number(p, "/reference_spell_id")?)?,
            ticks_ms,
            pauses_offline: ticks_ms != -1,
        });
    }
    Ok(out)
}

fn speed_operation(
    recipient: &SpeedRecipient,
    kind: SpeedKind,
    delta: i32,
    ticks_ms: i32,
) -> Result<RecipientSpeedPlan, CompanionError> {
    if recipient.base_speed < 0 {
        return Err(CompanionError::InvalidFacts("negative base speed"));
    }
    let suppressed = if kind == SpeedKind::Haste {
        recipient.haste_suppressed
    } else {
        recipient.paralyze_suppressed
    };
    let condition = SpeedCondition {
        kind,
        id: ConditionSlotId::Combat,
        sub_id: 0,
        ticks_ms,
        delta,
    };
    condition.expiry_speed_change()?;
    let previous = recipient
        .conditions
        .iter()
        .find(|c| c.kind == kind && c.id == ConditionSlotId::Combat && c.sub_id == 0);
    let operation = if suppressed {
        SpeedOperation::Suppressed
    } else if let Some(old) = previous {
        if old.ticks_ms == -1 && ticks_ms > 0 {
            SpeedOperation::KeepInfinite
        } else {
            SpeedOperation::Refresh {
                condition,
                change_delta: delta
                    .checked_sub(old.delta)
                    .ok_or(CompanionError::ArithmeticBounds)?,
            }
        }
    } else {
        SpeedOperation::Start {
            condition,
            remove_opposites: recipient
                .conditions
                .iter()
                .filter(|c| c.kind != kind)
                .cloned()
                .collect(),
        }
    };
    Ok(RecipientSpeedPlan {
        recipient: recipient.id,
        operation,
    })
}
fn haste(p: &Value, f: &HasteFacts) -> Result<HastePlan, CompanionError> {
    if f.caster.base_speed < 0 {
        return Err(CompanionError::InvalidFacts("negative caster base speed"));
    }
    // Separate f32 product/add reproduces the pinned C++ float formula.
    let coefficient = decimal(p, "/caster/formula/mina")? as f32;
    let product = coefficient
        * (f.caster
            .base_speed
            .checked_sub(40)
            .ok_or(CompanionError::ArithmeticBounds)? as f32);
    let total = product + decimal(p, "/caster/formula/minb")? as f32;
    let caster_delta = trunc_i32(f64::from(total))?
        .checked_sub(f.caster.base_speed)
        .ok_or(CompanionError::ArithmeticBounds)?;
    let mut familiars = Vec::new();
    for summon in f.owned_summons.iter().filter(|s| s.is_familiar) {
        if summon.base_speed < 0 {
            return Err(CompanionError::InvalidFacts("negative familiar base speed"));
        }
        let raw = trunc_i32(
            f64::from(f.caster.base_speed.max(summon.base_speed))
                * decimal(p, "/familiar/multiplier")?
                + decimal(p, "/familiar/offset")?,
        )?;
        let kind = if raw > 0 {
            SpeedKind::Haste
        } else {
            SpeedKind::Paralyze
        };
        // Explicit zero speed enters the default zero formula then paralyze clamp.
        let delta = if raw == 0 {
            narrow(number(p, "/familiar/zero_delta_paralyze_total")?)?
                .checked_sub(summon.base_speed)
                .ok_or(CompanionError::ArithmeticBounds)?
        } else {
            raw
        };
        familiars.push(speed_operation(
            summon,
            kind,
            delta,
            narrow(number(p, "/familiar/duration_ms")?)?,
        )?);
    }
    let modifier_duration = narrow(number(p, "/damage_modifier_duration_ms")?)?;
    let grade = match f.swift_grade {
        SwiftGrade::None => "/damage_dealt_percent/none",
        SwiftGrade::Regular => "/damage_dealt_percent/regular",
        SwiftGrade::Greater => "/damage_dealt_percent/greater",
    };
    let percent = unsigned(number(p, grade)?)?;
    Ok(HastePlan {
        familiar_before_combat: familiars,
        caster_combat: speed_operation(
            &f.caster,
            SpeedKind::Haste,
            caster_delta,
            narrow(number(p, "/caster/duration_ms")?)?,
        )?,
        caster_magic_green: true,
        damage_modifier_after_success: if modifier_duration > 0 && percent != 100 {
            Some(damage_modifier(percent, modifier_duration))
        } else {
            None
        },
        attacks_and_casts_allowed: flag(p, "/attacks_and_casts_allowed")?,
    })
}
fn acquire(p: &Value, f: &AcquisitionFacts) -> Result<AcquisitionPlan, CompanionError> {
    let kind = match text(p, "/source")? {
        "named_creature" => AcquisitionKind::NamedCreature,
        "corpse_tile" => AcquisitionKind::CorpseTile,
        "target_creature" => AcquisitionKind::TargetCreature,
        _ => return Err(CompanionError::InvalidParameters("acquisition source")),
    };
    if kind != f.kind {
        return Err(CompanionError::InvalidFacts("acquisition source binding"));
    }
    let bypass = match kind {
        AcquisitionKind::NamedCreature => f.can_summon_all,
        AcquisitionKind::TargetCreature => f.can_convince_all,
        AcquisitionKind::CorpseTile => false,
    };
    let invalid = match kind {
        AcquisitionKind::NamedCreature => !f.type_found || (!bypass && !f.summonable),
        AcquisitionKind::CorpseTile => {
            !(f.tile_present && f.top_down_item_present && f.is_corpse && f.corpse_movable)
        }
        AcquisitionKind::TargetCreature => {
            !f.target_is_monster
                || (!bypass
                    && (!f.convinceable
                        || f.target_master_name
                            .as_deref()
                            .is_some_and(|s| !s.eq_ignore_ascii_case("a carved stone tile"))))
        }
    };
    if invalid {
        return Err(refuse(p, "/failure_messages/invalid_source"));
    }
    let cap =
        usize::try_from(number(p, "/summon_cap")?).map_err(|_| CompanionError::ArithmeticBounds)?;
    if (!bypass && f.owned_summons >= cap) || (flag(p, "/refuse_black_skull")? && f.black_skull) {
        return Err(refuse(p, "/failure_messages/summon_cap"));
    }
    let cost = if kind == AcquisitionKind::CorpseTile {
        0
    } else {
        f.creature_mana_cost
    };
    if f.mana < u64::from(cost) && !f.has_infinite_mana {
        return Err(refuse(p, "/failure_messages/mana"));
    }
    if kind != AcquisitionKind::TargetCreature && !f.spawn_room {
        return Err(refuse(p, "/failure_messages/spawn_room"));
    }
    let mut actions = Vec::new();
    match kind {
        AcquisitionKind::NamedCreature => {
            actions.push(AcquisitionAction::Create {
                name: f.creature_name.clone(),
                on_target_tile: false,
                extended: flag(p, "/spawn/extended")?,
                force: flag(p, "/spawn/force")?,
                owned: true,
            });
            actions.push(AcquisitionAction::SubtractMana(cost));
            actions.push(AcquisitionAction::AddManaSpent(cost));
            actions.push(AcquisitionAction::CasterMagicBlue);
            actions.push(AcquisitionAction::CreatureTeleport);
        }
        AcquisitionKind::CorpseTile => {
            actions.push(AcquisitionAction::Create {
                name: text(p, "/created_creature")?.to_owned(),
                on_target_tile: true,
                extended: flag(p, "/spawn/extended")?,
                force: flag(p, "/spawn/force")?,
                owned: false,
            });
            actions.push(AcquisitionAction::RemoveCorpse);
            actions.push(AcquisitionAction::AssignMaster);
            actions.push(AcquisitionAction::TargetMagicBlue);
        }
        AcquisitionKind::TargetCreature => {
            actions.push(AcquisitionAction::SubtractMana(cost));
            actions.push(AcquisitionAction::AddManaSpent(cost));
            actions.push(AcquisitionAction::AssignMaster);
            actions.push(AcquisitionAction::CasterMagicBlue);
        }
    }
    Ok(AcquisitionPlan {
        actions,
        requires_creation_success: kind != AcquisitionKind::TargetCreature,
        consume_rune_charge_on_success: flag(p, "/rune_charge_on_success")?,
    })
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    // These are arithmetic-consumer fixtures. Full canonical profile/provenance
    // validation is separately required at the authoring reader before planning.
    fn familiar_parameters() -> Value {
        json!({"vocation":"knight", "reference_spell_id":194, "creature_name":"Knight familiar",
            "default_look_type":991, "summon_limit":1,
            "spawn":{"extended":true,"force":false}, "creation_speed":{"minimum_delta":0},
            "duration":{"seconds_multiplier":30}, "cooldown":{"duration_multiplier":2,"vip_seconds_multiplier":60},
            "warnings":[{"remaining_ms":10000,"message":"10 seconds"},{"remaining_ms":60000,"message":"one minute"}],
            "warning_dispatch":{"prefix":"Your summon will disappear in less than ","negative_delay_conversion":"unsigned_zero","scheduler_minimum_ms":100},
            "login":{"minimum_level":200,"lifetime_basis":"current_unix_time","clamp_remaining_to_zero":true},
            "return_to_owner":{"strict_distance_tiles":15},"register_party_protection":false,
            "failure_messages":{"premium":"You need a premium account.","summon_limit":"You can't have other summons.","spawn_room":"not_enough_room"}})
    }
    fn familiar_facts() -> FamiliarFacts {
        FamiliarFacts {
            event: FamiliarEvent::Cast,
            vocation: "knight".into(),
            premium: true,
            level: 200,
            account_at_least_god: false,
            owned_summons: 0,
            spawn_room: true,
            chosen_look: 0,
            has_vocation_look: false,
            owner_current_speed: 100,
            familiar_base_speed: 220,
            familiar_minutes: 30,
            vip: false,
            vip_reduction_minutes: 0,
            cooldown_rate: 1.0,
            now_unix: 1000,
            saved_expiry_unix: 1600,
            last_logout_unix: 1000,
            owner_present: true,
            creature_present: true,
            creature_name: "Knight familiar".into(),
            matching_summon_ids: vec![7, 8],
            dx: 0,
            dy: 0,
            dz: 0,
            owner_tile_is_teleport: false,
        }
    }
    fn speed_parameters(
        a: &str,
        b: &str,
        m: &str,
        o: &str,
        caster_ms: i32,
        familiar_ms: i32,
    ) -> Value {
        json!({"caster":{"formula":{"mina":a,"minb":b},"duration_ms":caster_ms},
            "familiar":{"multiplier":m,"offset":o,"duration_ms":familiar_ms,"zero_delta_paralyze_total":40},
            "damage_modifier_duration_ms":0,"damage_dealt_percent":{"none":100,"regular":100,"greater":100},
            "attacks_and_casts_allowed":true})
    }
    fn recipient(id: u64, base_speed: i32, is_familiar: bool) -> SpeedRecipient {
        SpeedRecipient {
            id,
            base_speed,
            is_familiar,
            haste_suppressed: false,
            paralyze_suppressed: false,
            conditions: vec![],
        }
    }
    fn haste_facts() -> HasteFacts {
        HasteFacts {
            caster: recipient(1, 400, false),
            owned_summons: vec![recipient(2, 220, true), recipient(3, 220, false)],
            swift_grade: SwiftGrade::None,
        }
    }
    fn planned_delta(p: &RecipientSpeedPlan) -> i32 {
        match &p.operation {
            SpeedOperation::Start { condition, .. } | SpeedOperation::Refresh { condition, .. } => {
                condition.delta
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    fn acquire_parameters(kind: AcquisitionKind) -> Value {
        json!({"source":match kind { AcquisitionKind::NamedCreature=>"named_creature", AcquisitionKind::CorpseTile=>"corpse_tile", AcquisitionKind::TargetCreature=>"target_creature"},
            "summon_cap":2,"refuse_black_skull":kind==AcquisitionKind::CorpseTile,
            "spawn":{"extended":true,"force":kind==AcquisitionKind::CorpseTile},"created_creature":"Skeleton",
            "rune_charge_on_success":kind!=AcquisitionKind::NamedCreature,
            "failure_messages":{"invalid_source":"not_possible","summon_cap":"cap","mana":"not_enough_mana","spawn_room":"room"}})
    }
    fn acquire_facts(kind: AcquisitionKind) -> AcquisitionFacts {
        AcquisitionFacts {
            kind,
            creature_name: "Rat".into(),
            type_found: true,
            summonable: true,
            target_is_monster: true,
            convinceable: true,
            target_master_name: None,
            tile_present: true,
            top_down_item_present: true,
            is_corpse: true,
            corpse_movable: true,
            owned_summons: 0,
            can_summon_all: false,
            can_convince_all: false,
            black_skull: false,
            mana: 100,
            creature_mana_cost: 20,
            has_infinite_mana: false,
            spawn_room: true,
        }
    }
    #[test]
    fn caster_float32_and_familiar_double_goldens() {
        for (a, b, m, o, caster, familiar, cm, fm) in [
            ("1.3", "40", "0.3", "-24", 107, 96, 30000, 33000),
            ("1.7", "40", "0.7", "-56", 252, 224, 22000, 22000),
            ("1.9", "40", "0.9", "-72", 324, 288, 5000, 5000),
            ("1.8", "72", "0.8", "-72", 320, 248, 10000, 10000),
        ] {
            let p = haste(&speed_parameters(a, b, m, o, cm, fm), &haste_facts()).unwrap();
            assert_eq!(planned_delta(&p.caster_combat), caster);
            assert_eq!(planned_delta(&p.familiar_before_combat[0]), familiar);
            assert_eq!(p.familiar_before_combat.len(), 1); // ordinary summon excluded
        }
    }
    #[test]
    fn zero_explicit_speed_is_paralyze_default_formula_not_zero_change() {
        let mut f = haste_facts();
        f.caster.base_speed = 80;
        f.owned_summons[0].base_speed = 80;
        let p = haste(
            &speed_parameters("1.3", "40", "0.3", "-24", 30000, 33000),
            &f,
        )
        .unwrap();
        assert_eq!(planned_delta(&p.familiar_before_combat[0]), -40);
        assert!(
            matches!(&p.familiar_before_combat[0].operation,SpeedOperation::Start{condition,..} if condition.kind==SpeedKind::Paralyze)
        );
    }
    #[test]
    fn refresh_weak_replaces_strong_and_infinite_ignores_timed() {
        let mut f = haste_facts();
        f.caster.conditions.push(SpeedCondition {
            kind: SpeedKind::Haste,
            id: ConditionSlotId::Combat,
            sub_id: 0,
            ticks_ms: 22000,
            delta: 252,
        });
        let params = speed_parameters("1.3", "40", "0.3", "-24", 30000, 33000);
        assert!(matches!(
            haste(&params, &f).unwrap().caster_combat.operation,
            SpeedOperation::Refresh {
                change_delta: -145,
                ..
            }
        ));
        f.caster.conditions[0].ticks_ms = -1;
        assert_eq!(
            haste(&params, &f).unwrap().caster_combat.operation,
            SpeedOperation::KeepInfinite
        );
    }
    #[test]
    fn new_haste_removes_opposite_slots_and_suppression_has_no_effect() {
        let mut f = haste_facts();
        let old = SpeedCondition {
            kind: SpeedKind::Paralyze,
            id: ConditionSlotId::Other(9),
            sub_id: 77,
            ticks_ms: 500,
            delta: -100,
        };
        f.caster.conditions.push(old.clone());
        let params = speed_parameters("1.3", "40", "0.3", "-24", 30000, 33000);
        assert!(
            matches!(haste(&params,&f).unwrap().caster_combat.operation,SpeedOperation::Start{remove_opposites,..} if remove_opposites==vec![old])
        );
        f.caster.haste_suppressed = true;
        assert_eq!(
            haste(&params, &f).unwrap().caster_combat.operation,
            SpeedOperation::Suppressed
        );
    }
    #[test]
    fn swift_modifier_is_after_success_and_crystal_greater_keeps_existing_buff() {
        let mut params = speed_parameters("1.8", "72", "0.8", "-72", 10000, 10000);
        params["damage_modifier_duration_ms"] = json!(10000);
        params["damage_dealt_percent"] = json!({"none":70,"regular":70,"greater":70});
        let mut f = haste_facts();
        f.swift_grade = SwiftGrade::Greater;
        assert_eq!(
            haste(&params, &f).unwrap().damage_modifier_after_success,
            Some(damage_modifier(70, 10000))
        );
        params["damage_dealt_percent"] = json!({"none":70,"regular":50,"greater":100});
        assert_eq!(
            haste(&params, &f).unwrap().damage_modifier_after_success,
            None
        );
        f.swift_grade = SwiftGrade::Regular;
        assert_eq!(
            haste(&params, &f).unwrap().damage_modifier_after_success,
            Some(damage_modifier(50, 10000))
        );
    }
    #[test]
    fn creation_look_delta_and_cooldown_preserve_source_units() {
        let params = familiar_parameters();
        let mut f = familiar_facts();
        f.vip = true;
        f.vip_reduction_minutes = 20;
        let p = familiar(&params, &f).unwrap();
        assert!(p.contains(&FamiliarAction::ChangeSpeed(0)));
        assert!(
            p.iter()
                .any(|a| matches!(a, FamiliarAction::Create { look: 0, .. }))
        );
        assert!(p.contains(&FamiliarAction::StoreExpiry(1900)));
        assert!(p.contains(&FamiliarAction::Cooldown {
            reference_spell_id: 194,
            ticks_ms: 600000,
            pauses_offline: true
        }));
        f.vip_reduction_minutes = 50; // source permits a negative cooldown; no fabricated clamp
        assert!(
            familiar(&params, &f)
                .unwrap()
                .contains(&FamiliarAction::Cooldown {
                    reference_spell_id: 194,
                    ticks_ms: -1200000,
                    pauses_offline: true
                })
        );
    }
    #[test]
    fn short_warning_delays_unsigned_zero_then_scheduler_minimum() {
        let mut f = familiar_facts();
        f.familiar_minutes = 1;
        let p = familiar(&familiar_parameters(), &f).unwrap();
        assert!(p.iter().any(|a| matches!(
            a,
            FamiliarAction::ScheduleWarning {
                index: 0,
                delay_ms: 20000,
                ..
            }
        )));
        assert!(p.iter().any(|a| matches!(
            a,
            FamiliarAction::ScheduleWarning {
                index: 1,
                raw_delay_ms: -30000,
                delay_ms: 100,
                ..
            }
        )));
    }
    #[test]
    fn offline_lifetime_has_explicit_source_basis_without_cooldown_restart() {
        let mut f = familiar_facts();
        f.event = FamiliarEvent::Login;
        f.now_unix = 3900;
        f.saved_expiry_unix = 1600;
        let mut params = familiar_parameters();
        let ca = familiar(&params, &f).unwrap();
        assert!(
            !ca.iter()
                .any(|a| matches!(a, FamiliarAction::Create { .. }))
        );
        params["login"]["lifetime_basis"] = json!("last_logout_unix_time");
        params["login"]["clamp_remaining_to_zero"] = json!(false);
        let cr = familiar(&params, &f).unwrap();
        assert!(cr.contains(&FamiliarAction::StoreExpiry(4500)));
        assert!(
            cr.iter()
                .any(|a| matches!(a, FamiliarAction::Create { look: 991, .. }))
        );
        assert!(
            !cr.iter()
                .any(|a| matches!(a, FamiliarAction::Cooldown { .. }))
        );
    }
    #[test]
    fn login_create_failure_keeps_look_grants_and_advance_conjunction() {
        let mut f = familiar_facts();
        f.event = FamiliarEvent::Login;
        f.spawn_room = false;
        let p = familiar(&familiar_parameters(), &f).unwrap();
        assert!(p.contains(&FamiliarAction::GrantVocationLook(991)));
        assert!(
            p.iter()
                .any(|a| matches!(a, FamiliarAction::CreationRefusal { poff: true, .. }))
        );
        f.event = FamiliarEvent::Advance;
        f.level = 100;
        f.premium = true;
        assert!(
            familiar(&familiar_parameters(), &f)
                .unwrap()
                .contains(&FamiliarAction::GrantVocationLook(991))
        );
        f.premium = false;
        assert!(familiar(&familiar_parameters(), &f).unwrap().is_empty());
    }
    #[test]
    fn lifecycle_disposition_preserves_expiry_and_cooldown_distinctions() {
        let params = familiar_parameters();
        let mut f = familiar_facts();
        f.event = FamiliarEvent::FamiliarDeath;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![
                FamiliarAction::StoreExpiry(1000),
                FamiliarAction::CancelAndResetWarnings
            ]
        );
        f.event = FamiliarEvent::ManualDispel;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![
                FamiliarAction::CasterMagicBlue,
                FamiliarAction::RemoveFirstMatchingSummon {
                    id: 7,
                    creature_poff: true
                }
            ]
        );
        f.event = FamiliarEvent::OwnerRemoval;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![FamiliarAction::RemoveOwnedSummons {
                invoke_familiar_death: false
            }]
        );
        f.event = FamiliarEvent::Expiry;
        f.owner_present = false;
        assert!(familiar(&params, &f).unwrap().is_empty());
        f.owner_present = true;
        f.creature_present = false;
        assert!(familiar(&params, &f).unwrap().is_empty());
        f.creature_present = true;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![
                FamiliarAction::RemoveCreature,
                FamiliarAction::ResetWarningHandles
            ]
        );
    }
    #[test]
    fn exact_return_boundary_and_teleport_tile_block_without_pz_exclusion() {
        let params = familiar_parameters();
        let mut f = familiar_facts();
        f.event = FamiliarEvent::Follow;
        f.dx = 15;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![FamiliarAction::FollowOwner {
                teleport: false,
                can_attack_in_protection_zone: false
            }]
        );
        f.dx = 16;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![FamiliarAction::FollowOwner {
                teleport: true,
                can_attack_in_protection_zone: false
            }]
        );
        f.owner_tile_is_teleport = true;
        f.dz = 1;
        assert_eq!(
            familiar(&params, &f).unwrap(),
            vec![FamiliarAction::FollowOwner {
                teleport: false,
                can_attack_in_protection_zone: false
            }]
        );
    }
    #[test]
    fn flag_override_bypasses_cap_but_infinite_mana_still_plans_actual_cost() {
        let params = acquire_parameters(AcquisitionKind::NamedCreature);
        let mut f = acquire_facts(AcquisitionKind::NamedCreature);
        f.summonable = false;
        f.owned_summons = 2;
        assert!(
            matches!(acquire(&params,&f),Err(CompanionError::Refused{message,..}) if message=="not_possible")
        );
        f.can_summon_all = true;
        f.mana = 0;
        assert!(
            matches!(acquire(&params,&f),Err(CompanionError::Refused{message,..}) if message=="not_enough_mana")
        );
        f.has_infinite_mana = true;
        let p = acquire(&params, &f).unwrap();
        assert_eq!(
            &p.actions[1..3],
            &[
                AcquisitionAction::SubtractMana(20),
                AcquisitionAction::AddManaSpent(20)
            ]
        );
        assert!(p.requires_creation_success);
        assert!(!p.consume_rune_charge_on_success);
        f.type_found = false;
        assert!(acquire(&params, &f).is_err());
    }
    #[test]
    fn animate_requires_movable_corpse_no_black_skull_override_and_commit_order() {
        let params = acquire_parameters(AcquisitionKind::CorpseTile);
        let mut f = acquire_facts(AcquisitionKind::CorpseTile);
        f.can_summon_all = true;
        f.corpse_movable = false;
        assert!(acquire(&params, &f).is_err());
        f.corpse_movable = true;
        f.black_skull = true;
        assert!(acquire(&params, &f).is_err());
        f.black_skull = false;
        f.owned_summons = 2;
        assert!(acquire(&params, &f).is_err());
        f.owned_summons = 0;
        f.mana = 0;
        let p = acquire(&params, &f).unwrap();
        assert_eq!(
            p.actions,
            vec![
                AcquisitionAction::Create {
                    name: "Skeleton".into(),
                    on_target_tile: true,
                    extended: true,
                    force: true,
                    owned: false
                },
                AcquisitionAction::RemoveCorpse,
                AcquisitionAction::AssignMaster,
                AcquisitionAction::TargetMagicBlue
            ]
        );
        assert!(p.requires_creation_success);
        assert!(p.consume_rune_charge_on_success);
    }
    #[test]
    fn convince_master_name_exception_is_case_insensitive_and_cost_precedes_master() {
        let params = acquire_parameters(AcquisitionKind::TargetCreature);
        let mut f = acquire_facts(AcquisitionKind::TargetCreature);
        f.target_master_name = Some("A CARVED STONE TILE".into());
        let p = acquire(&params, &f).unwrap();
        assert_eq!(
            p.actions,
            vec![
                AcquisitionAction::SubtractMana(20),
                AcquisitionAction::AddManaSpent(20),
                AcquisitionAction::AssignMaster,
                AcquisitionAction::CasterMagicBlue
            ]
        );
        f.target_master_name = Some("Player".into());
        assert!(acquire(&params, &f).is_err());
        f.can_convince_all = true;
        assert!(acquire(&params, &f).is_ok());
        f.target_is_monster = false;
        assert!(acquire(&params, &f).is_err());
    }
    #[test]
    fn bounds_bindings_and_invalid_rate_fail_closed() {
        let params = familiar_parameters();
        let mut f = familiar_facts();
        f.vocation = "druid".into();
        assert!(matches!(
            familiar(&params, &f),
            Err(CompanionError::InvalidFacts(_))
        ));
        f.vocation = "knight".into();
        f.cooldown_rate = 0.0;
        assert!(familiar(&params, &f).is_err());
        f.cooldown_rate = f32::NAN;
        assert!(familiar(&params, &f).is_err());
        f.cooldown_rate = 1.0;
        f.familiar_minutes = i64::MAX;
        assert!(familiar(&params, &f).is_err());
        assert!(trunc_i32(f64::INFINITY).is_err());
        let cond = SpeedCondition {
            kind: SpeedKind::Haste,
            id: ConditionSlotId::Combat,
            sub_id: 0,
            ticks_ms: 30000,
            delta: 107,
        };
        assert_eq!(cond.expiry_speed_change().unwrap(), -107);
        let mut af = acquire_facts(AcquisitionKind::NamedCreature);
        af.kind = AcquisitionKind::CorpseTile;
        assert!(matches!(
            acquire(&acquire_parameters(AcquisitionKind::NamedCreature), &af),
            Err(CompanionError::InvalidFacts(_))
        ));
    }

    #[test]
    fn all_sixteen_canonical_companion_profiles_execute_pure_plans() {
        let document: Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let profiles = document["profiles"].as_array().unwrap();
        let mut seen = std::collections::BTreeSet::new();
        let mut counts = [0usize; 3];
        for profile in profiles {
            let behavior = &profile["execution"]["native_behavior"];
            let Some(key) = behavior["key"].as_str() else {
                continue;
            };
            if !["familiar_summon", "companion_haste", "acquire_summon"].contains(&key) {
                continue;
            }
            let name = profile["name"].as_str().unwrap().to_ascii_lowercase();
            assert!(
                seen.insert(name.clone()),
                "duplicate companion profile {name}"
            );
            let parameters = &behavior["parameters"];
            match key {
                "familiar_summon" => {
                    counts[0] += 1;
                    let mut facts = familiar_facts();
                    facts.vocation = text(parameters, "/vocation").unwrap().into();
                    facts.creature_name = text(parameters, "/creature_name").unwrap().into();
                    facts.chosen_look =
                        unsigned(number(parameters, "/default_look_type").unwrap()).unwrap();
                    let CompanionPlan::Familiar(actions) =
                        plan(parameters, &CompanionFacts::Familiar(facts.clone())).unwrap()
                    else {
                        panic!("wrong companion plan for {name}");
                    };
                    let reference_id =
                        unsigned(number(parameters, "/reference_spell_id").unwrap()).unwrap();
                    assert_eq!(
                        reference_id,
                        unsigned(number(parameters, "/shared_cooldown_identity").unwrap()).unwrap()
                    );
                    let expected_id = match facts.vocation.as_str() {
                        "knight" => 194,
                        "paladin" => 195,
                        "sorcerer" => 196,
                        "druid" => 197,
                        "monk" => 282,
                        other => panic!("unqualified vocation {other}"),
                    };
                    assert_eq!(
                        reference_id, expected_id,
                        "alias must retain shared reference ID: {name}"
                    );
                    assert!(actions.iter().any(|a| matches!(a, FamiliarAction::Create { name, look, extended:true, force:false, owned:true }
                        if name == &facts.creature_name && *look == facts.chosen_look)));
                    assert!(actions.contains(&FamiliarAction::Cooldown {
                        reference_spell_id: reference_id,
                        ticks_ms: 1800000,
                        pauses_offline: true
                    }));
                    assert!(actions.contains(&FamiliarAction::StoreExpiry(1900)));
                    assert_eq!(
                        actions.contains(&FamiliarAction::RegisterPartyProtectionAllOwnedSummons),
                        name.starts_with("summon "),
                        "Crystal-only aliases retain cast-only party registration"
                    );
                    facts.premium = false;
                    assert!(matches!(plan(parameters, &CompanionFacts::Familiar(facts)),
                        Err(CompanionError::Refused { message, poff:true }) if message == "You need a premium account."));
                }
                "companion_haste" => {
                    counts[1] += 1;
                    let CompanionPlan::Haste(result) =
                        plan(parameters, &CompanionFacts::Haste(haste_facts())).unwrap()
                    else {
                        panic!("wrong haste plan for {name}");
                    };
                    let (caster_delta, familiar_delta, caster_ms, familiar_ms) = match name.as_str()
                    {
                        "haste" => (107, 96, 30000, 33000),
                        "strong haste" => (252, 224, 22000, 22000),
                        "charge" => (324, 288, 5000, 5000),
                        "swift foot" => (320, 248, 10000, 10000),
                        other => panic!("unqualified companion haste {other}"),
                    };
                    assert_eq!(planned_delta(&result.caster_combat), caster_delta, "{name}");
                    assert_eq!(
                        result.familiar_before_combat.len(),
                        1,
                        "ordinary summon must be excluded"
                    );
                    assert_eq!(
                        planned_delta(&result.familiar_before_combat[0]),
                        familiar_delta,
                        "{name}"
                    );
                    assert!(
                        matches!(&result.caster_combat.operation, SpeedOperation::Start { condition, .. }
                        if condition.kind == SpeedKind::Haste && condition.ticks_ms == caster_ms)
                    );
                    assert!(
                        matches!(&result.familiar_before_combat[0].operation, SpeedOperation::Start { condition, .. }
                        if condition.kind == SpeedKind::Haste && condition.ticks_ms == familiar_ms)
                    );
                    assert_eq!(
                        result.damage_modifier_after_success,
                        if name == "swift foot" {
                            Some(damage_modifier(70, 10000))
                        } else {
                            None
                        }
                    );
                }
                "acquire_summon" => {
                    counts[2] += 1;
                    let kind = match name.as_str() {
                        "summon creature" => AcquisitionKind::NamedCreature,
                        "animate dead rune" => AcquisitionKind::CorpseTile,
                        "convince creature rune" => AcquisitionKind::TargetCreature,
                        other => panic!("unqualified acquisition {other}"),
                    };
                    let mut facts = acquire_facts(kind);
                    let CompanionPlan::Acquisition(result) =
                        plan(parameters, &CompanionFacts::Acquisition(facts.clone())).unwrap()
                    else {
                        panic!("wrong acquisition plan for {name}");
                    };
                    assert_eq!(
                        result.requires_creation_success,
                        kind != AcquisitionKind::TargetCreature
                    );
                    assert_eq!(
                        result.consume_rune_charge_on_success,
                        kind != AcquisitionKind::NamedCreature
                    );
                    let expected_actions = match kind {
                        AcquisitionKind::NamedCreature => vec![
                            AcquisitionAction::Create {
                                name: "Rat".into(),
                                on_target_tile: false,
                                extended: true,
                                force: false,
                                owned: true,
                            },
                            AcquisitionAction::SubtractMana(20),
                            AcquisitionAction::AddManaSpent(20),
                            AcquisitionAction::CasterMagicBlue,
                            AcquisitionAction::CreatureTeleport,
                        ],
                        AcquisitionKind::CorpseTile => vec![
                            AcquisitionAction::Create {
                                name: "Skeleton".into(),
                                on_target_tile: true,
                                extended: true,
                                force: true,
                                owned: false,
                            },
                            AcquisitionAction::RemoveCorpse,
                            AcquisitionAction::AssignMaster,
                            AcquisitionAction::TargetMagicBlue,
                        ],
                        AcquisitionKind::TargetCreature => vec![
                            AcquisitionAction::SubtractMana(20),
                            AcquisitionAction::AddManaSpent(20),
                            AcquisitionAction::AssignMaster,
                            AcquisitionAction::CasterMagicBlue,
                        ],
                    };
                    assert_eq!(result.actions, expected_actions, "{name}");
                    match kind {
                        AcquisitionKind::NamedCreature => facts.summonable = false,
                        AcquisitionKind::CorpseTile => facts.corpse_movable = false,
                        AcquisitionKind::TargetCreature => facts.convinceable = false,
                    }
                    assert!(
                        matches!(plan(parameters, &CompanionFacts::Acquisition(facts)),
                        Err(CompanionError::Refused { message, poff:true }) if message == "not_possible")
                    );
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(counts, [9, 4, 3]);
        let expected: std::collections::BTreeSet<String> = [
            "charge",
            "druid familiar",
            "haste",
            "knight familiar",
            "monk familiar",
            "paladin familiar",
            "sorcerer familiar",
            "strong haste",
            "summon creature",
            "summon druid familiar",
            "summon knight familiar",
            "summon paladin familiar",
            "summon sorcerer familiar",
            "swift foot",
            "animate dead rune",
            "convince creature rune",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(seen, expected);
    }
}
