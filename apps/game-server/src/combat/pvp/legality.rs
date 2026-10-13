//! PARTY-PVP-0 §7.2 rules in order, §7.3 damage factor, PVP-0 §6.3 (rule 3b) and §6.6 (rule 6b).
//!
//! A pure decision over facts the Channel runtime reads; it never trusts a client fact. Unknown
//! zone facts and an unconfirmed party/guild relation refuse (fail closed).

/// `PARTYPVP0-RL-09`: both sides need this level.
pub(crate) const PVP_MIN_LEVEL: u32 = 8;
/// `PARTYPVP0-RL-11`: PvP damage factor in percent; 100 against a black-skulled target.
const PVP_DAMAGE_PERCENT: u64 = 50;

/// The World PvP type (`rulesets/pvp/` `pvp_type`, §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorldPvpType {
    Optional,
    Open,
    Hardcore,
}

/// ATTACK-0 fight modes' `expert_mode` (Dove 0, White Hand 1, Yellow Hand 2, Red Fist 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ExpertMode {
    #[default]
    Dove,
    WhiteHand,
    YellowHand,
    RedFist,
}

/// Why a character target is refused (`PVP_REFUSED {reason}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PvpRefusal {
    WorldType,
    Level,
    ProtectionZone,
    NoPvpTile,
    PostLoginImmunity,
    PartyState,
    PartyMember,
    SecureMode,
    ExpertMode,
    BlackSkull,
}

/// The actor (a summon acts as its owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PvpActor {
    pub(crate) level: u32,
    pub(crate) in_protection_zone: bool,
    /// `None`: the tile's zone facts are unknown.
    pub(crate) on_no_pvp_tile: Option<bool>,
    /// Admitted less than `PARTYPVP0-RL-10` (10 s) ago.
    pub(crate) post_login_immune: bool,
    pub(crate) black_skull: bool,
    pub(crate) secure_mode: bool,
    /// The server state of the last accepted fight modes intent.
    pub(crate) expert_mode: ExpertMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PvpTarget {
    pub(crate) level: u32,
    pub(crate) in_protection_zone: bool,
    pub(crate) on_no_pvp_tile: Option<bool>,
    /// The target shows the actor a skull (white, yellow toward the actor, red, black, orange
    /// toward the actor).
    pub(crate) marked: bool,
}

/// Relations between the two, confirmed by the caller against durable rows (§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PvpRelation {
    /// `war_between(a, b)` (§12); always false until GUILD-WAR-0.
    pub(crate) war_between: bool,
    /// Same party or guild; `None` when the durable confirmation failed or the view is stale.
    pub(crate) same_party_or_guild: Option<bool>,
    /// Both have aggression toward one enemy (§7.4 friendly fire).
    pub(crate) friendly_fire: bool,
    /// The target has aggression toward the actor (the actor may answer it).
    pub(crate) target_aggressor_of_actor: bool,
    /// The target has aggression toward a member of the actor's party or guild.
    pub(crate) target_aggressor_of_actor_group: bool,
}

/// Decide whether `actor` may harm `target`. Rules run in the §7.2 order; the first refusal wins.
pub(crate) fn check_pvp_legality(
    world: WorldPvpType,
    actor: &PvpActor,
    target: &PvpTarget,
    relation: &PvpRelation,
) -> Result<(), PvpRefusal> {
    // 1. World type: Hardcore allows, Open follows the rules, Optional only in a war.
    if world == WorldPvpType::Optional && !relation.war_between {
        return Err(PvpRefusal::WorldType);
    }
    // 2. Level.
    if actor.level < PVP_MIN_LEVEL || target.level < PVP_MIN_LEVEL {
        return Err(PvpRefusal::Level);
    }
    // 3. Protection zone.
    if actor.in_protection_zone || target.in_protection_zone {
        return Err(PvpRefusal::ProtectionZone);
    }
    // 3b. No-PvP tile, every World type; unknown facts refuse.
    if actor.on_no_pvp_tile != Some(false) || target.on_no_pvp_tile != Some(false) {
        return Err(PvpRefusal::NoPvpTile);
    }
    // 4. Post-login immunity: no new aggression, answering an aggressor is allowed.
    if actor.post_login_immune && !relation.target_aggressor_of_actor {
        return Err(PvpRefusal::PostLoginImmunity);
    }
    // 5. Party and guild, except friendly fire.
    match relation.same_party_or_guild {
        None => return Err(PvpRefusal::PartyState),
        Some(true) if !relation.friendly_fire => return Err(PvpRefusal::PartyMember),
        Some(_) => {}
    }
    let hardcore = world == WorldPvpType::Hardcore;
    // 6. Secure mode (always off on Hardcore).
    if !hardcore && actor.secure_mode && !target.marked {
        return Err(PvpRefusal::SecureMode);
    }
    // 6b. Expert mode (Hardcore is locked to Red Fist, which restricts nothing).
    if !hardcore && !expert_allows(actor.expert_mode, target, relation) {
        return Err(PvpRefusal::ExpertMode);
    }
    // 7. A black-skulled actor cannot harm an unmarked character.
    if actor.black_skull && !target.marked {
        return Err(PvpRefusal::BlackSkull);
    }
    Ok(())
}

fn expert_allows(mode: ExpertMode, target: &PvpTarget, relation: &PvpRelation) -> bool {
    let aggressor = relation.target_aggressor_of_actor;
    match mode {
        ExpertMode::Dove => aggressor,
        ExpertMode::WhiteHand => aggressor || relation.target_aggressor_of_actor_group,
        ExpertMode::YellowHand => {
            aggressor || relation.target_aggressor_of_actor_group || target.marked
        }
        ExpertMode::RedFist => true,
    }
}

/// §7.3: PvP damage after the draw and before mitigation: 50%, 100% on a black-skulled target.
/// Rounds down.
pub(crate) fn pvp_damage(damage: u32, target_black_skull: bool) -> u32 {
    if target_black_skull {
        damage
    } else {
        // `damage * 50 / 100 <= u32::MAX`, so the cast is exact.
        (u64::from(damage) * PVP_DAMAGE_PERCENT / 100) as u32
    }
}
