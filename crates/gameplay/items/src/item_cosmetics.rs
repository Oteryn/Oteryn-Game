//! Source item acquisition rules; canonical cosmetic mappings remain independently authored.
//! No numeric source look or a matching revision can itself authorize an earned grant.
use crate::reward_bag::reward_bag_bounded_draw;
use oteryn_gameplay_content::content::native_gameplay::NativeCosmeticsView;
use oteryn_gameplay_domain::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetic, Sex, UnlockFact,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const PROVENANCE: &str = "crystal-item-cosmetics-201189f-v1";
pub const MISSING_CATALOGUE: &str = "W19_CANONICAL_COSMETICS_SOURCE_MISSING";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrantKind {
    Outfit,
    Addon,
    Mount,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Grant {
    pub key: String,
    pub kind: GrantKind,
    #[serde(default)]
    pub look_types: Vec<u32>,
    pub addon: Option<u8>,
    pub mount_id: Option<u32>,
    pub look_type: Option<u32>,
    pub donor: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Direct,
    Pieces,
    Parts,
    RandomOutfit,
    RandomMount,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Component {
    pub item_id: u32,
    pub key: String,
    pub count: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Choice {
    pub grants: Vec<String>,
    pub owned: String,
    pub required: Vec<String>,
    pub cues: Vec<crate::novelty_items::NoveltyCue>,
    pub speech: Option<String>,
    #[serde(default)]
    pub private_speech: bool,
    pub feedback: Option<String>,
    pub owned_feedback: Option<String>,
    pub completion_feedback: Option<String>,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub achievement_progress: Vec<crate::novelty_items::AchievementProgress>,
    pub achievement_when_full: Option<String>,
    #[serde(default)]
    pub complete_grants: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub item_id: u32,
    pub donor: String,
    pub mode: Mode,
    pub premium: bool,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub required_items: Vec<u32>,
    #[serde(default)]
    pub components: Vec<Component>,
    pub progress_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Catalogue {
    schema: String,
    crystal_commit: String,
    provenance: String,
    pub grants: Vec<Grant>,
    pub rules: Vec<Rule>,
}

/// Bind source-policy admission to these exact native rules, including all random outcomes.
pub fn catalogue_sha256() -> [u8; 32] {
    static DIGEST: OnceLock<[u8; 32]> = OnceLock::new();
    *DIGEST.get_or_init(|| {
        sha2::Sha256::digest(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/item-cosmetic-actions-native.json"
        ))
        .into()
    })
}

/// Complete grant mappings alone never activate an excluded donor source.
pub fn source_policy_allows(
    rule: &Rule,
    content: Option<&NativeCosmeticsView>,
    item_key: &str,
) -> bool {
    let needs_admission = matches!(rule.mode, Mode::RandomOutfit | Mode::RandomMount)
        || rule.donor.starts_with("data-crystal/scripts/special/");
    !needs_admission
        || content
            .is_some_and(|c| c.allows_item_acquisition(item_key, &rule.donor, catalogue_sha256()))
}

pub fn catalogue() -> Option<&'static Catalogue> {
    static CATALOGUE: OnceLock<Option<Catalogue>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| {
            let c: Catalogue = serde_json::from_str(include_str!(
                "../../../../tools/content-schema/native-gameplay/item-cosmetic-actions-native.json"
            ))
            .ok()?;
            if c.schema != "OTERYN_ITEM_COSMETICS/v1"
                || c.crystal_commit != "201189f871d23597b82caf88fd611454a2ddefb8"
                || c.provenance != PROVENANCE
                || c.grants.is_empty()
                || c.grants.len() > 4096
                || c.rules.is_empty()
                || c.rules.len() > 128
                || c.grants.windows(2).any(|p| p[0].key >= p[1].key)
                || c.rules.windows(2).any(|p| p[0].item_id >= p[1].item_id)
                || c.grants.iter().any(|g| !valid_grant(g))
                || c.rules.iter().any(|r| !valid_rule(&c, r))
            {
                return None;
            }
            Some(c)
        })
        .as_ref()
}

fn valid_grant(g: &Grant) -> bool {
    if g.key.is_empty() || g.donor.is_empty() {
        return false;
    }
    match g.kind {
        GrantKind::Mount => {
            g.mount_id.is_some_and(|id| id > 0)
                && g.look_type.is_some_and(|id| id > 0)
                && g.look_types.is_empty()
        }
        GrantKind::Outfit | GrantKind::Addon => {
            g.look_types.len() == 2
                && g.look_types[0] > 0
                && g.look_types[0] < g.look_types[1]
                && match g.kind {
                    GrantKind::Outfit => g.addon == Some(0),
                    GrantKind::Addon => matches!(g.addon, Some(1 | 2)),
                    GrantKind::Mount => false,
                }
        }
    }
}

fn valid_rule(c: &Catalogue, r: &Rule) -> bool {
    r.item_id > 0
        && !r.donor.is_empty()
        && !r.choices.is_empty()
        && r.choices.len() <= 256
        && r.choices.iter().all(|choice| {
            !choice.grants.is_empty()
                && choice.grants.len() <= 2
                && choice
                    .grants
                    .iter()
                    .chain(&choice.required)
                    .chain(&choice.complete_grants)
                    .chain(std::iter::once(&choice.owned))
                    .all(|key| c.grants.iter().any(|g| g.key == *key))
                && choice.achievement_progress.iter().all(|p| p.target > 0)
        })
        && match r.mode {
            Mode::Direct => {
                r.choices.len() == 1 && r.components.is_empty() && r.required_items.is_empty()
            }
            Mode::RandomOutfit => r.choices.len().is_multiple_of(2) && r.components.is_empty(),
            Mode::RandomMount => r.components.is_empty(),
            Mode::Pieces => {
                r.choices.len() == 1
                    && (2..=8).contains(&r.required_items.len())
                    && r.required_items.iter().all(|id| *id > 0)
                    && r.required_items
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        == r.required_items.len()
                    && r.required_items.contains(&r.item_id)
            }
            Mode::Parts => {
                r.choices.len() == 1
                    && !r.components.is_empty()
                    && r.components.len() <= 3
                    && r.components.iter().all(|p| p.item_id > 0 && p.count > 0)
                    && r.components
                        .iter()
                        .any(|p| p.item_id == r.item_id && Some(&p.key) == r.progress_key.as_ref())
            }
        }
}

pub fn rule(key: &str) -> Option<&'static Rule> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    catalogue()?.rules.iter().find(|r| r.item_id == id)
}

/// Crystal draws an outfit and its addon independently; duplicate selection never rerolls.
pub fn select(rule: &Rule, mut draw: impl FnMut(u8, u64) -> Option<u64>) -> Option<&Choice> {
    let index = match rule.mode {
        Mode::RandomOutfit => {
            let outfits = u64::try_from(rule.choices.len() / 2).ok()?;
            let outfit = draw(224, outfits)?;
            let addon = draw(225, 2)?;
            if outfit >= outfits || addon >= 2 {
                return None;
            }
            outfit.checked_mul(2)?.checked_add(addon)?
        }
        Mode::RandomMount => {
            let count = u64::try_from(rule.choices.len()).ok()?;
            let index = draw(226, count)?;
            if index >= count {
                return None;
            }
            index
        }
        _ => 0,
    };
    rule.choices.get(usize::try_from(index).ok()?)
}

pub fn select_for_command(
    rule: &Rule,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
) -> Option<&Choice> {
    select(rule, |domain, upper| {
        reward_bag_bounded_draw(session, command, source, domain, upper)
    })
}

/// The mapping lookup is independent of source selection, and requires the already-admitted
/// native catalogue plus its explicit compatible grant mapping for each target.
pub trait SourceCosmetics: AppearanceContent<String> {
    fn outfit_pair(&self, looks: &[u32]) -> Option<String>;
    fn outfit_for_grant(&self, looks: &[u32], _grant_key: &str) -> Option<String> {
        self.outfit_pair(looks)
    }
    fn mount_look(&self, look: u32) -> Option<String>;
}

impl SourceCosmetics for NativeCosmeticsView {
    fn outfit_for_grant(&self, looks: &[u32], grant_key: &str) -> Option<String> {
        // Equal looks can have explicitly retained historical aliases. Resolve the
        // authored primary target; resolve() still checks provenance/kind/addon.
        let key = match self.authored_grant_target(grant_key)? {
            EarnedCosmetic::Outfit(key) | EarnedCosmetic::Addon(key, _) => key,
            EarnedCosmetic::Mount(_) => return None,
        };
        let female = self.look_type(key, Sex::Female)?;
        let male = self.look_type(key, Sex::Male)?;
        (female != male && looks.contains(&female) && looks.contains(&male)).then(|| key.to_owned())
    }
    fn outfit_pair(&self, looks: &[u32]) -> Option<String> {
        let mut found = self.outfit_keys().filter(|key| {
            let pair = [
                self.look_type(key, Sex::Female),
                self.look_type(key, Sex::Male),
            ];
            pair.iter()
                .all(|look| look.is_some_and(|id| looks.contains(&id)))
                && pair[0] != pair[1]
        });
        let key = found.next()?;
        if found.next().is_some() {
            return None;
        }
        Some(key.to_owned())
    }
    fn mount_look(&self, look: u32) -> Option<String> {
        let mut found = self
            .mount_keys()
            .filter(|key| self.mount_metadata(key).is_some_and(|m| m.0 == look));
        let key = found.next()?;
        if found.next().is_some() {
            return None;
        }
        Some(key.to_owned())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ResolvedGrant {
    pub grant_key: String,
    pub cosmetic_key: String,
    pub kind: GrantKind,
    pub addon: u8,
}

impl ResolvedGrant {
    pub fn target(&self) -> EarnedCosmetic<'_> {
        match self.kind {
            GrantKind::Outfit => EarnedCosmetic::Outfit(&self.cosmetic_key),
            GrantKind::Addon => EarnedCosmetic::Addon(&self.cosmetic_key, self.addon),
            GrantKind::Mount => EarnedCosmetic::Mount(&self.cosmetic_key),
        }
    }
    pub fn owned(
        &self,
        content: &impl SourceCosmetics,
        owned: &impl AccountUnlocks<String>,
        sex: Sex,
    ) -> bool {
        match self.kind {
            GrantKind::Outfit => {
                content.outfit_is_starter(&self.cosmetic_key, sex)
                    || owned.outfit(&self.cosmetic_key) == UnlockFact::Compatible
            }
            GrantKind::Addon => {
                content.addon_is_starter(&self.cosmetic_key, sex, self.addon)
                    || owned.addon(&self.cosmetic_key, self.addon) == UnlockFact::Compatible
            }
            GrantKind::Mount => owned.mount(&self.cosmetic_key) == UnlockFact::Compatible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    ContentUnavailable,
    PremiumRequired,
    OutfitRequired,
    InvalidProgress,
}

pub fn resolve(
    c: &Catalogue,
    content: &impl SourceCosmetics,
    key: &str,
) -> Result<ResolvedGrant, Refusal> {
    let source = c
        .grants
        .iter()
        .find(|g| g.key == key)
        .ok_or(Refusal::ContentUnavailable)?;
    let cosmetic_key = match source.kind {
        GrantKind::Mount => {
            content.mount_look(source.look_type.ok_or(Refusal::ContentUnavailable)?)
        }
        GrantKind::Outfit | GrantKind::Addon => {
            content.outfit_for_grant(&source.look_types, &source.key)
        }
    }
    .ok_or(Refusal::ContentUnavailable)?;
    let grant = ResolvedGrant {
        grant_key: source.key.clone(),
        cosmetic_key,
        kind: source.kind,
        addon: source.addon.unwrap_or(0),
    };
    if content.earned_unlock(&grant.grant_key, PROVENANCE, grant.target()) != UnlockFact::Compatible
    {
        return Err(Refusal::ContentUnavailable);
    }
    Ok(grant)
}

/// Qualify the complete source table before drawing. Missing mappings cannot narrow a
/// random pool or consume a partial multipart collection.
pub fn qualify_rule(
    c: &Catalogue,
    rule: &Rule,
    content: &impl SourceCosmetics,
) -> Result<(), Refusal> {
    for choice in &rule.choices {
        for key in choice
            .grants
            .iter()
            .chain(&choice.required)
            .chain(&choice.complete_grants)
            .chain(std::iter::once(&choice.owned))
        {
            resolve(c, content, key)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Progress {
    pub key: String,
    pub before: u32,
    pub after: u32,
}

#[derive(Debug)]
pub struct Decision {
    pub consume: bool,
    pub grants: Vec<ResolvedGrant>,
    pub progress: Option<Progress>,
    pub required_items: Vec<u32>,
    pub achievements: Vec<String>,
    pub achievement_progress: Vec<crate::novelty_items::AchievementProgress>,
    pub feedback: Option<String>,
}

/// The Afflicted base outfit removes six items from the player's carried tree
/// (usable_afflicted_outfit_items.lua:35-49). A clicked Ground piece is only the
/// action trigger. The other rules remove the actual clicked item.
pub struct Inputs {
    pub consume_source: bool,
    pub carried_items: Vec<u32>,
}

pub fn inputs(rule: &Rule, decision: &Decision, source_is_carried: bool) -> Inputs {
    let consume_source = decision.consume && (rule.mode != Mode::Pieces || source_is_carried);
    let carried_items = decision
        .required_items
        .iter()
        .filter(|item| !consume_source || **item != rule.item_id)
        .copied()
        .collect();
    Inputs {
        consume_source,
        carried_items,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "source choice, canonical mappings, account facts and current premium are separate inputs"
)]
pub fn decide(
    c: &Catalogue,
    rule: &Rule,
    choice: &Choice,
    content: &impl SourceCosmetics,
    owned: &impl AccountUnlocks<String>,
    sex: Sex,
    premium: bool,
    progress: &BTreeMap<String, u32>,
) -> Result<Decision, Refusal> {
    if rule.premium && !premium {
        return Err(Refusal::PremiumRequired);
    }
    let known = resolve(c, content, &choice.owned)?;
    let mut decision = Decision {
        consume: true,
        grants: Vec::new(),
        progress: None,
        required_items: rule.required_items.clone(),
        achievements: Vec::new(),
        achievement_progress: Vec::new(),
        feedback: None,
    };
    let fully_owned = |grant: &ResolvedGrant| {
        if rule.mode == Mode::RandomOutfit {
            grant.owned(content, owned, Sex::Female) && grant.owned(content, owned, Sex::Male)
        } else {
            grant.owned(content, owned, sex)
        }
    };
    if fully_owned(&known) {
        decision.consume = false;
        decision.required_items.clear();
        decision.feedback = choice.owned_feedback.clone().or_else(|| {
            rule.donor
                .starts_with("data/scripts/actions/items/usable_outfit_items.lua:")
                .then(|| "You already own this outfit part.".into())
        });
        return Ok(decision);
    }
    for key in &choice.required {
        if !resolve(c, content, key)?.owned(content, owned, sex) {
            return Err(Refusal::OutfitRequired);
        }
    }
    let complete = if rule.mode == Mode::Parts {
        let key = rule.progress_key.as_ref().ok_or(Refusal::InvalidProgress)?;
        let before = progress.get(key).copied().unwrap_or(0);
        let after = before.checked_add(1).ok_or(Refusal::InvalidProgress)?;
        decision.progress = Some(Progress {
            key: key.clone(),
            before,
            after,
        });
        rule.components.iter().all(|part| {
            (if part.key == *key {
                after
            } else {
                progress.get(&part.key).copied().unwrap_or(0)
            }) >= part.count
        })
    } else {
        true
    };
    if complete {
        for key in &choice.grants {
            let grant = resolve(c, content, key)?;
            if !fully_owned(&grant) {
                decision.grants.push(grant);
            }
        }
        decision.achievements = choice.achievements.clone();
        decision.achievement_progress = choice.achievement_progress.clone();
        decision.feedback = choice.completion_feedback.clone();
        if let Some(achievement) = &choice.achievement_when_full {
            let mut full = true;
            for key in &choice.complete_grants {
                let grant = resolve(c, content, key)?;
                full &= grant.owned(content, owned, sex) || decision.grants.contains(&grant);
            }
            if full {
                decision.achievements.push(achievement.clone());
            }
        }
    }
    if choice.feedback.is_some() {
        decision.feedback = choice.feedback.clone();
    }
    Ok(decision)
}

#[cfg(test)]
#[path = "item_cosmetics/tests.rs"]
mod tests;
