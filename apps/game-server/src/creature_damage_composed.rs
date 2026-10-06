//! Source-qualified bounded sixteen multi-effect attacks; no new timing/RNG authority.
//! SOURCE native callbacks are preserved. PROJECT snapshot primary-then-callback batches,
//! stronger current PZ gates and deterministic uniform draws are explicitly GlobalUnverified.
use super::*;
use crate::content::{ProjectV2AffectsKind, ProjectV2Draft, ProjectV2EffectAffects};
use crate::creature_area_heal::CreatureHealCatalog;
use crate::foundation::{OwnerDamageCommand, OwnerDamageResult};
use crate::player_lethal::PlayerHealReceipt;
#[derive(Debug, Clone)]
pub(crate) struct ComposedSource {
    caster: Ref,
    pub(crate) ability: Ref,
    index: usize,
    details: Box<crate::content::ProjectV2AbilityDetails>,
    primary: Option<SpellSource>,
    callbacks: Vec<Callback>,
    fingerprint: [u8; 32],
    content: [u8; 32],
    range: u16,
    magnitude: Option<ProjectV2Magnitude>,
}
#[derive(Debug, Clone)]
struct Callback {
    filter: ProjectV2EffectAffects,
    heal: bool,
    minimum: u32,
    maximum: u32,
}
impl ComposedSource {
    pub(crate) fn from_native(
        draft: &ProjectV2Draft,
        caster: &Ref,
        index: usize,
        content: [u8; 32],
    ) -> Result<Self, AttackError> {
        let expected: serde_json::Value =
            serde_json::from_str(SOURCE_BODIES).map_err(|_| AttackError::InvalidSource)?;
        let case = expected["cases"]
            .as_array()
            .ok_or(AttackError::InvalidSource)?
            .iter()
            .find(|c| c["creature"] == caster.key && c["entry"] == index)
            .ok_or(AttackError::UnsupportedShape)?;
        if caster.family != ProjectV2Family::Creature || caster.revision != "definition-r1" {
            return Err(AttackError::InvalidSource);
        }
        let records = &draft.core.records;
        let profiles = &draft.state.authoring_profiles;
        let mut map = BTreeMap::new();
        for p in profiles {
            if map.insert(p.target.clone(), &p.data).is_some() {
                return Err(AttackError::InvalidSource);
            }
        }
        let Some(ProjectReferenceRecord::Creature{behavior,..})=records.iter().find(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..}if identity.key==caster.key&&identity.revision==caster.revision))else{return Err(AttackError::InvalidSource)};
        let bref = Ref {
            family: ProjectV2Family::Behavior,
            key: behavior.key.clone(),
            revision: behavior.revision.clone(),
        };
        let Some(Data::Behavior(b)) = map.get(&bref).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let entry = b.attacks.get(index).ok_or(AttackError::InvalidSource)?;
        if index >= 16 || b.attacks.len() > 16 || case["ability"] != entry.ability.key {
            return Err(AttackError::InvalidSource);
        }
        let Some(Data::Creature(cp)) = map.get(caster).copied() else {
            return Err(AttackError::InvalidSource);
        };
        if !cp.abilities.contains(&entry.ability) {
            return Err(AttackError::InvalidSource);
        }
        // Exact source-corrected native closure; no guessing a removed/changed secondary branch.
        let body = expected["abilities"]
            .get(&entry.ability.key)
            .ok_or(AttackError::InvalidSource)?;
        for p in body["profiles"]
            .as_array()
            .ok_or(AttackError::InvalidSource)?
        {
            let reference: Ref = serde_json::from_value(p["target"].clone())
                .map_err(|_| AttackError::InvalidSource)?;
            let actual = map
                .get(&reference)
                .copied()
                .ok_or(AttackError::InvalidSource)?;
            if serde_json::to_value(actual).map_err(|_| AttackError::InvalidSource)? != p["data"] {
                return Err(AttackError::InvalidSource);
            }
        }
        for record in body["records"]
            .as_array()
            .ok_or(AttackError::InvalidSource)?
        {
            let key = record["identity"]["key"]
                .as_str()
                .ok_or(AttackError::InvalidSource)?;
            let actual = records
                .iter()
                .find(|r| {
                    serde_json::to_value(r)
                        .ok()
                        .is_some_and(|v| v["identity"]["key"] == key)
                })
                .ok_or(AttackError::InvalidSource)?;
            if serde_json::to_value(actual).map_err(|_| AttackError::InvalidSource)? != *record {
                return Err(AttackError::InvalidSource);
            }
        }
        let Some(Data::Ability(a)) = map.get(&entry.ability).copied() else {
            return Err(AttackError::InvalidSource);
        };
        let details = a.details.as_deref().ok_or(AttackError::InvalidSource)?;
        let primary = body["primary"]
            .as_u64()
            .map(|i| {
                SpellSource::from_native_executable(
                    caster, index, records, profiles, content, i as usize,
                )
            })
            .transpose()?;
        let mut callbacks = Vec::new();
        for c in body["callbacks"]
            .as_array()
            .ok_or(AttackError::InvalidSource)?
        {
            let filter: ProjectV2EffectAffects = serde_json::from_value(c["filter"].clone())
                .map_err(|_| AttackError::InvalidSource)?;
            let minimum = c["minimum"]
                .as_u64()
                .and_then(|x| u32::try_from(x).ok())
                .ok_or(AttackError::InvalidSource)?;
            let maximum = c["maximum"]
                .as_u64()
                .and_then(|x| u32::try_from(x).ok())
                .ok_or(AttackError::InvalidSource)?;
            if minimum > maximum {
                return Err(AttackError::InvalidSource);
            }
            callbacks.push(Callback {
                filter,
                heal: c["heal"].as_bool().ok_or(AttackError::InvalidSource)?,
                minimum,
                maximum,
            });
        }
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(caster, index, entry, body, content))
                .map_err(|_| AttackError::InvalidSource)?,
        )
        .into();
        Ok(Self {
            caster: caster.clone(),
            ability: entry.ability.clone(),
            index,
            details: Box::new(details.clone()),
            primary,
            callbacks,
            fingerprint,
            content,
            range: entry.range_tiles.unwrap_or(details.range_tiles),
            magnitude: entry.magnitude,
        })
    }
}
/// Every fact is read from the existing current map/combat/role owner. No default allow.
pub(crate) trait ComposedWorldReader: SpellWorldReader {
    fn creature_combat(
        &mut self,
        _r: &ChannelRuntimeV1,
        _issuer: ExactActorRef,
        _target: ExactActorRef,
        _element: Option<&str>,
        _stamp: RuntimeWorkStamp,
    ) -> Option<CreatureCombatFacts> {
        None
    }
    fn callback_allowed(
        &mut self,
        _r: &ChannelRuntimeV1,
        _issuer: ExactActorRef,
        _target: ExactActorRef,
        _ability: &Ref,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn non_player_side(
        &mut self,
        _r: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn top_creature(
        &mut self,
        _r: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CreatureCombatFacts {
    pub(crate) allowed: bool,
    pub(crate) immune: bool,
    pub(crate) armor: u32,
    pub(crate) multiplier_ppm: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BranchReceipt {
    Creature(OwnerDamageResult),
    PlayerDamage(PlayerDamageReceipt),
    PlayerHeal(PlayerHealReceipt),
    Zero,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComposedOutcome {
    pub(crate) primary: Option<SpellOutcome>,
    pub(crate) branches: Vec<(usize, ExactActorRef, Result<BranchReceipt, AttackError>)>,
    pub(crate) qualification: &'static str,
}
struct Memo {
    actor: ExactActorRef,
    index: usize,
    sequence: u64,
    at: SemanticTimeMicros,
    binding: [u8; 32],
    activation: u64,
    result: ComposedOutcome,
}
#[derive(Default)]
pub(crate) struct ComposedOwner {
    memos: Vec<Memo>,
}
enum Request {
    CreatureDamage {
        key: String,
        amount: u32,
        occurrence: String,
        binding: Vec<u8>,
    },
    CreatureHeal {
        requests: Vec<(ExactActorRef, String, u64, u64)>,
    },
    Player {
        session: GameSessionId,
        amount: u32,
        heal: bool,
        occurrence: String,
    },
}
struct Task {
    branch: usize,
    target: ExactActorRef,
    request: Request,
}
impl ComposedOwner {
    // Keep execute ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        &mut self,
        r: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        vitals: &mut dyn PlayerLethalVitals,
        catalog: &CreatureHealCatalog,
        source: &ComposedSource,
        proposal: &ProfileAbilityProposal,
        reader: &mut dyn ComposedWorldReader,
        now: SemanticTimeMicros,
    ) -> Result<ComposedOutcome, AttackError> {
        let binding = r.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(AttackError::StaleOwner);
        }
        if r.content_pin().server_artifact_digest() != source.content || !catalog.matches_current(r)
        {
            return Err(AttackError::ContentChanged);
        }
        // All retained native source callback/primary formulas in this bounded source set
        // are definition-r1; preparing a branch must not bypass the occurrence revision guard.
        if proposal.occurrence.revisions().formula() != "definition-r1" {
            return Err(AttackError::InvalidSource);
        }
        if !r.matches_live_creature_identity(proposal.issuer, source.caster.key.as_bytes()) {
            return Err(AttackError::StaleIssuer);
        }
        if proposal.list != ScheduleList::Attack
            || proposal.ability != source.ability
            || proposal.entry_index != source.index
            || proposal.range_tiles != source.range
            || proposal.magnitude != source.magnitude
        {
            return Err(AttackError::InvalidSource);
        }
        let prefix = format!("ai-profile:{}:", hex(&proposal.issuer.placement_identity()));
        let tail = proposal
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(AttackError::InvalidPlan)?;
        let (raw, suffix) = tail.split_once(':').ok_or(AttackError::InvalidPlan)?;
        let sequence = raw.parse::<u64>().map_err(|_| AttackError::InvalidPlan)?;
        if suffix != format!("attack:{}", source.index) {
            return Err(AttackError::InvalidPlan);
        }
        let proof = Sha256::digest(
            serde_json::to_vec(&(source.fingerprint, format!("{proposal:?}")))
                .map_err(|_| AttackError::InvalidPlan)?,
        )
        .into();
        let activation = r.content_pin().activation_sequence();
        self.memos
            .retain(|m| r.contains_live_creature(m.actor) && m.activation == activation);
        let old = self
            .memos
            .iter()
            .position(|m| m.actor == proposal.issuer && m.index == source.index);
        if let Some(i) = old {
            let m = &self.memos[i];
            if sequence < m.sequence || now < m.at {
                return Err(AttackError::InvalidPlan);
            }
            if sequence == m.sequence {
                return if m.binding == proof {
                    Ok(m.result.clone())
                } else {
                    Err(AttackError::InvalidPlan)
                };
            }
        }
        if old.is_none() {
            if self.memos.len() >= 64 * 16 {
                return Err(AttackError::LedgerFull);
            }
            self.memos
                .try_reserve(1)
                .map_err(|_| AttackError::LedgerFull)?;
        }
        let generic = DamageSpellOwner::default();
        let prepared = source
            .primary
            .as_ref()
            .map(|p| generic.prepare_target(r, p, proposal, reader, stamp, None))
            .transpose()?;
        // Independently current player state, including immutable maximum, before ANY owner write.
        if let Some(p) = &prepared {
            for t in &p.targets {
                vitals
                    .source_player_health(r, t.target, t.session)
                    .ok_or(AttackError::MissingVitals)?;
            }
        }
        let tiles = source.tiles(r, proposal, reader, stamp)?;
        let candidates = r
            .current_live_creature_candidates(64)
            .map_err(|_| AttackError::InvalidPlan)?;
        let players = reader
            .current_players(r, stamp)
            .ok_or(AttackError::MissingCombatFacts)?;
        if players.len() > 64 {
            return Err(AttackError::LedgerFull);
        }
        let mut seen = std::collections::BTreeSet::new();
        if players
            .iter()
            .any(|(a, _)| !seen.insert(a.placement_identity()))
        {
            return Err(AttackError::InvalidPlan);
        }
        let root = GameplayDecisionRoot::from_bytes(source.content);
        let digest = Sha256::new()
            .chain_update(b"oteryn:composed-source-callback:v1")
            .chain_update(proposal.occurrence.id().as_str().as_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let id = DecisionOccurrenceId::from_bytes(id);
        let mut tasks = Vec::new();
        // Primary HP precedes tile callbacks in pinned Combat::CombatFunc. PROJECT batches
        // current snapshot tiles; cross-owner commits retain partial receipts, not atomic fiction.
        if let (Some(primary), Some(prepared)) = (&source.primary, &prepared) {
            for target in &candidates {
                if *target == proposal.issuer
                    || !source.contains(r, *target, &tiles, proposal.issuer)?
                {
                    continue;
                }
                let facts = reader
                    .creature_combat(
                        r,
                        proposal.issuer,
                        *target,
                        primary.element.as_deref(),
                        stamp,
                    )
                    .ok_or(AttackError::MissingCombatFacts)?;
                if !facts.allowed {
                    continue;
                }
                if facts.multiplier_ppm > 10_000_000 {
                    return Err(AttackError::InvalidPlan);
                }
                let (key, _) = catalog
                    .current_target(r, *target)
                    .map_err(|_| AttackError::InvalidSource)?;
                let raw = prepared.result.requested;
                let armor = if primary.armor {
                    let (lo, hi) =
                        crate::creature_auto_attack::canonical_armor_bounds(facts.armor)?;
                    if lo == hi {
                        lo
                    } else {
                        let d = deterministic_decision_u64(
                            &root,
                            id,
                            "composed_npc_armor",
                            source.index as u64,
                        )
                        .map_err(|_| AttackError::InvalidPlan)?;
                        u32::try_from(crate::spell::uniform_draw(d, i64::from(lo), i64::from(hi)))
                            .map_err(|_| AttackError::NumericOverflow)?
                    }
                } else {
                    0
                };
                let amount = if facts.immune {
                    0
                } else {
                    u32::try_from(
                        u64::from(raw.saturating_sub(armor)) * u64::from(facts.multiplier_ppm)
                            / 1_000_000,
                    )
                    .map_err(|_| AttackError::NumericOverflow)?
                };
                let occurrence = format!(
                    "{}:primary:{}",
                    proposal.occurrence.id().as_str(),
                    hex(&target.placement_identity())
                );
                tasks.try_reserve(1).map_err(|_| AttackError::LedgerFull)?;
                tasks.push(Task {
                    branch: 0,
                    target: *target,
                    request: Request::CreatureDamage {
                        key,
                        amount,
                        binding: damage_binding(&occurrence, &source.fingerprint)?,
                        occurrence,
                    },
                });
            }
        }
        for (i, callback) in source.callbacks.iter().enumerate() {
            for target in &candidates {
                if !source.contains(r, *target, &tiles, proposal.issuer)? {
                    continue;
                }
                let key = std::str::from_utf8(
                    r.current_live_creature_identity(*target)
                        .map_err(|_| AttackError::StaleTarget)?,
                )
                .map_err(|_| AttackError::InvalidSource)?;
                if *target == proposal.issuer && !callback.filter.includes_caster
                    || callback.filter.excludes_caster_name && key == source.caster.key
                {
                    continue;
                }
                let admitted = match callback.filter.kind {
                    ProjectV2AffectsKind::NamedCreatures => {
                        callback.filter.creatures.iter().any(|c| c.key == key)
                    }
                    ProjectV2AffectsKind::NonPlayerSide => reader
                        .non_player_side(r, *target, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?,
                    ProjectV2AffectsKind::PlayerSide => !reader
                        .non_player_side(r, *target, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?,
                    ProjectV2AffectsKind::Players => false,
                    _ => return Err(AttackError::UnsupportedShape),
                };
                if !admitted {
                    continue;
                }
                if callback.filter.top_creature_only
                    && !reader
                        .top_creature(r, *target, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?
                {
                    continue;
                }
                if !reader
                    .callback_allowed(r, proposal.issuer, *target, &source.ability, stamp)
                    .ok_or(AttackError::MissingCombatFacts)?
                {
                    continue;
                }
                let (key, max) = catalog
                    .current_target(r, *target)
                    .map_err(|_| AttackError::InvalidSource)?;
                let (amount, occurrence) = callback.draw(&root, id, proposal, *target, i)?;
                let request = if callback.heal {
                    let mut requests = Vec::new();
                    requests
                        .try_reserve_exact(1)
                        .map_err(|_| AttackError::LedgerFull)?;
                    requests.push((*target, key, max, u64::from(amount)));
                    Request::CreatureHeal { requests }
                } else {
                    Request::CreatureDamage {
                        key,
                        amount,
                        binding: damage_binding(&occurrence, &source.fingerprint)?,
                        occurrence,
                    }
                };
                tasks.try_reserve(1).map_err(|_| AttackError::LedgerFull)?;
                tasks.push(Task {
                    branch: i + 1,
                    target: *target,
                    request,
                });
            }
            if matches!(
                callback.filter.kind,
                ProjectV2AffectsKind::Players | ProjectV2AffectsKind::PlayerSide
            ) {
                for (target, session) in &players {
                    if !source.contains(r, *target, &tiles, proposal.issuer)? {
                        continue;
                    }
                    if callback.filter.top_creature_only
                        && !reader
                            .top_creature(r, *target, stamp)
                            .ok_or(AttackError::MissingCombatFacts)?
                    {
                        continue;
                    }
                    let facts = reader
                        .combat(r, proposal.issuer, *target, *session, None, stamp)
                        .ok_or(AttackError::MissingCombatFacts)?;
                    let a = facts.attack;
                    if a.issuer != proposal.issuer
                        || a.target != *target
                        || a.session != *session
                        || a.revision == 0
                    {
                        return Err(AttackError::InvalidPlan);
                    }
                    if !a.visible
                        || a.issuer_pz
                        || a.target_pz
                        || a.issuer_protected
                        || a.target_protected
                    {
                        continue;
                    }
                    vitals
                        .source_player_health(r, *target, *session)
                        .ok_or(AttackError::MissingVitals)?;
                    let (amount, occurrence) = callback.draw(&root, id, proposal, *target, i)?;
                    tasks.try_reserve(1).map_err(|_| AttackError::LedgerFull)?;
                    tasks.push(Task {
                        branch: i + 1,
                        target: *target,
                        request: Request::Player {
                            session: *session,
                            amount,
                            heal: callback.heal,
                            occurrence,
                        },
                    });
                }
            }
        }
        let mut result = ComposedOutcome {
            primary: None,
            branches: Vec::new(),
            qualification: "SOURCE_ALL_HP_BRANCHES;PROJECT_SNAPSHOT_PRIMARY_THEN_CALLBACK_ORDER;PROJECT_UNIFORM_DRAW;STRONGER_PZ_GATES;GLOBAL_UNVERIFIED;PRESENTATION_NETWORK_PENDING",
        };
        result
            .branches
            .try_reserve(tasks.len())
            .map_err(|_| AttackError::LedgerFull)?;
        let mut memo_result = result.clone();
        memo_result
            .branches
            .try_reserve(tasks.len())
            .map_err(|_| AttackError::LedgerFull)?;
        // Presentation ownership is prepared, never claimed network-delivered. Preallocate memo
        // target receipts before HP. Every later branch refusal is retained alongside earlier writes.
        let mut primary_memo = prepared.as_ref().map(|p| p.result.clone());
        if let (Some(p), Some(m)) = (&prepared, &mut primary_memo) {
            m.targets
                .try_reserve(p.targets.len())
                .map_err(|_| AttackError::LedgerFull)?;
        }
        if let (Some(primary), Some(prepared)) = (&source.primary, prepared) {
            let hit = generic
                .commit_prepared(r, vitals, primary, proposal, prepared, now, fence, stamp)?;
            if let Some(m) = &mut primary_memo {
                m.targets.extend(hit.targets.iter().cloned());
            }
            result.primary = Some(hit);
            memo_result.primary = primary_memo;
        }
        for Task {
            branch,
            target,
            request,
        } in tasks
        {
            let receipt = match request {
                Request::CreatureHeal { requests } => r
                    .commit_source_creature_heal_batch(source.content, &requests)
                    .map_err(|_| AttackError::MissingVitals)
                    .and_then(|v| {
                        v.first()
                            .copied()
                            .map(BranchReceipt::Creature)
                            .ok_or(AttackError::MissingVitals)
                    }),
                Request::CreatureDamage {
                    key,
                    amount,
                    occurrence,
                    binding,
                } => {
                    if amount == 0 {
                        Ok(BranchReceipt::Zero)
                    } else {
                        r.borrow_exact_actor_commit()
                            .commit_damage(
                                target,
                                OwnerDamageCommand {
                                    target: key.as_bytes(),
                                    occurrence: occurrence.as_bytes(),
                                    binding: &binding,
                                    damage: i64::from(amount),
                                },
                            )
                            .map(BranchReceipt::Creature)
                            .map_err(|_| AttackError::MissingVitals)
                    }
                }
                Request::Player {
                    session,
                    amount,
                    heal,
                    occurrence,
                } => {
                    if heal {
                        vitals
                            .apply_source_player_heal(r, target, session, amount, &occurrence, now)
                            .map(BranchReceipt::PlayerHeal)
                            .ok_or(AttackError::MissingVitals)
                    } else if amount == 0 {
                        Ok(BranchReceipt::Zero)
                    } else {
                        vitals
                            .apply_attack_damage(r, target, session, amount, &occurrence, now)
                            .map(BranchReceipt::PlayerDamage)
                            .ok_or(AttackError::MissingVitals)
                    }
                }
            };
            memo_result.branches.push((branch, target, receipt.clone()));
            result.branches.push((branch, target, receipt));
        }
        let memo = Memo {
            actor: proposal.issuer,
            index: source.index,
            sequence,
            at: now,
            binding: proof,
            activation,
            result: memo_result,
        };
        if let Some(i) = old {
            self.memos[i] = memo
        } else {
            self.memos.push(memo)
        }
        Ok(result)
    }
}
impl ComposedSource {
    fn contains(
        &self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
        tiles: &[(i32, i32, i16)],
        issuer: ExactActorRef,
    ) -> Result<bool, AttackError> {
        let a = r
            .read_actor_position(target)
            .map_err(|_| AttackError::StaleTarget)?;
        let b = r
            .read_actor_position(issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        Ok(a.context() == b.context()
            && tiles.contains(&(a.position().x, a.position().y, a.position().floor)))
    }
    fn tiles(
        &self,
        r: &ChannelRuntimeV1,
        p: &ProfileAbilityProposal,
        reader: &mut dyn ComposedWorldReader,
        stamp: RuntimeWorkStamp,
    ) -> Result<Vec<(i32, i32, i16)>, AttackError> {
        let from = r
            .read_actor_position(p.issuer)
            .map_err(|_| AttackError::StaleIssuer)?;
        let to = r
            .read_actor_position(p.target)
            .map_err(|_| AttackError::StaleTarget)?;
        let fp = from.position();
        let tp = to.position();
        if from.context() != to.context() || fp.floor != tp.floor {
            return Err(AttackError::OutOfRange);
        }
        if self.range != 0
            && (i64::from(tp.x) - i64::from(fp.x))
                .abs()
                .max((i64::from(tp.y) - i64::from(fp.y)).abs())
                > i64::from(self.range)
        {
            return Err(AttackError::OutOfRange);
        }
        let center = if self.details.needs_target {
            (tp.x, tp.y)
        } else if self.details.needs_direction {
            let f = reader
                .current_facing(r, p.issuer, stamp)
                .ok_or(AttackError::MissingCombatFacts)?;
            let (x, y) = geometry::step(f);
            (
                fp.x.checked_add(x).ok_or(AttackError::NumericOverflow)?,
                fp.y.checked_add(y).ok_or(AttackError::NumericOverflow)?,
            )
        } else {
            (fp.x, fp.y)
        };
        let area = self
            .details
            .area
            .as_ref()
            .ok_or(AttackError::UnsupportedShape)?;
        let diagonal =
            matches!(area,ProjectV2AbilityArea::Matrix{diagonal,..}if !diagonal.is_empty());
        let f = geometry::direction(
            i64::from(center.0) - i64::from(fp.x),
            i64::from(center.1) - i64::from(fp.y),
            diagonal,
        );
        let mut tiles = Vec::new();
        for (dx, dy) in geometry::offsets(area, f).map_err(|_| AttackError::UnsupportedShape)? {
            let x = center
                .0
                .checked_add(dx)
                .ok_or(AttackError::NumericOverflow)?;
            let y = center
                .1
                .checked_add(dy)
                .ok_or(AttackError::NumericOverflow)?;
            match reader.tile_allowed(r, p.issuer, x, y, fp.floor, stamp) {
                None => return Err(AttackError::MissingCombatFacts),
                Some(false) => continue,
                Some(true) => tiles.push((x, y, fp.floor)),
            }
        }
        Ok(tiles)
    }
}
impl Callback {
    fn draw(
        &self,
        root: &GameplayDecisionRoot,
        _parent: DecisionOccurrenceId,
        p: &ProfileAbilityProposal,
        target: ExactActorRef,
        branch: usize,
    ) -> Result<(u32, String), AttackError> {
        let occurrence = format!(
            "{}:callback:{branch}:{}",
            p.occurrence.id().as_str(),
            hex(&target.placement_identity())
        );
        let hash = Sha256::new()
            .chain_update(p.occurrence.id().as_str().as_bytes())
            .chain_update(occurrence.as_bytes())
            .finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&hash[..16]);
        let id = DecisionOccurrenceId::from_bytes(bytes);
        let draw = deterministic_decision_u64(root, id, "source_callback_health", branch as u64)
            .map_err(|_| AttackError::InvalidPlan)?;
        let amount = u32::try_from(crate::spell::uniform_draw(
            draw,
            i64::from(self.minimum),
            i64::from(self.maximum),
        ))
        .map_err(|_| AttackError::NumericOverflow)?;
        Ok((amount, occurrence))
    }
}

fn damage_binding(occurrence: &str, fingerprint: &[u8; 32]) -> Result<Vec<u8>, AttackError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve(occurrence.len() + 33)
        .map_err(|_| AttackError::LedgerFull)?;
    bytes.extend_from_slice(occurrence.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(fingerprint);
    Ok(bytes)
}

const SOURCE_BODIES: &str = r##"{"cases":[{"creature":"oteryn:creature.a_greedy_eye","entry":0,"ability":"oteryn:ability.creature.a_greedy_eye.optional-field-core-attacks-1"},{"creature":"oteryn:creature.aggressive_lava","entry":1,"ability":"oteryn:ability.spell.aggressivelavawave"},{"creature":"oteryn:creature.ember_beast","entry":1,"ability":"oteryn:ability.spell.emberbeastarea"},{"creature":"oteryn:creature.ember_beast","entry":2,"ability":"oteryn:ability.spell.emberbeasthur"},{"creature":"oteryn:creature.fiery_blood","entry":1,"ability":"oteryn:ability.spell.aggressivelavawave"},{"creature":"oteryn:creature.fiery_heart","entry":0,"ability":"oteryn:ability.spell.aggressivelavawave"},{"creature":"oteryn:creature.freed_soul","entry":0,"ability":"oteryn:ability.spell.freed_soul_spell"},{"creature":"oteryn:creature.frozen_minion","entry":2,"ability":"oteryn:ability.spell.frozen_minion_wave"},{"creature":"oteryn:creature.frozen_minion","entry":3,"ability":"oteryn:ability.spell.frozen_minion_beam"},{"creature":"oteryn:creature.ravenous_lava_lurker","entry":1,"ability":"oteryn:ability.spell.ravennouslavalurkerwave"},{"creature":"oteryn:creature.ravenous_lava_lurker","entry":2,"ability":"oteryn:ability.spell.ravennouslavalurkertarget"},{"creature":"oteryn:creature.the_corruptor_of_souls","entry":2,"ability":"oteryn:ability.spell.remorseless_wave"},{"creature":"oteryn:creature.the_flaming_orchid","entry":4,"ability":"oteryn:ability.spell.aggressivelavawave"},{"creature":"oteryn:creature.the_remorseless_corruptor","entry":2,"ability":"oteryn:ability.spell.remorseless_wave"},{"creature":"oteryn:creature.the_source_of_corruption","entry":1,"ability":"oteryn:ability.spell.source_of_corruption_wave"},{"creature":"oteryn:creature.unchained_fire","entry":1,"ability":"oteryn:ability.spell.unchained_fire_beam"}],"abilities":{"oteryn:ability.creature.a_greedy_eye.optional-field-core-attacks-1":{"primary":0,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.poor_soul","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"heal":false,"minimum":1000,"maximum":1000,"raw_effect_key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["x","x","C"],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1-drowning","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"drowning","presentation":{"impact_asset_binding":"canary.appearance:effect/smallclouds"}}},"target":{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1-drowning","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1000,"minimum":1000}},"target":{"family":"Formula","key":"oteryn:formula.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.poor_soul","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"untyped","presentation":{"impact_asset_binding":"canary.appearance:effect/smallclouds"}}},"target":{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1000,"minimum":1000}},"target":{"family":"Formula","key":"oteryn:formula.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1-drowning","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1-drowning","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.a_greedy_eye.optional-field-core-attacks-1","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.aggressivelavawave":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.aggressive_lava","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.fiery_heart","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_baron_from_below","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_duke_of_the_depths","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_fire_empowered_duke","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_hungry_baron_from_below","revision":"definition-r1"}],"excludes_caster_name":true,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"heal":true,"minimum":0,"maximum":650,"raw_effect_key":"oteryn:effect.spell.aggressivelavawave.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":[".xxx.","xxxxx","xxCxx","xxxxx",".xxx."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.aggressivelavawave","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.aggressive_lava","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.fiery_heart","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_baron_from_below","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_duke_of_the_depths","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_fire_empowered_duke","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.the_hungry_baron_from_below","revision":"definition-r1"}],"excludes_caster_name":true,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"healing"}},"target":{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":650,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.aggressivelavawave.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"fire","presentation":{"impact_asset_binding":"canary.appearance:effect/hitbyfire"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.aggressivelavawave","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Heal","formula":{"family":"Formula","key":"oteryn:formula.spell.aggressivelavawave.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.aggressivelavawave.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.aggressivelavawave.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.emberbeastarea":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.the_count_of_the_core","revision":"definition-r1"}],"excludes_caster_name":true,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"heal":true,"minimum":0,"maximum":1500,"raw_effect_key":"oteryn:effect.spell.emberbeastarea.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["xxx","xCx","xxx"],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.emberbeastarea","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.the_count_of_the_core","revision":"definition-r1"}],"excludes_caster_name":true,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"healing"}},"target":{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1500,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.emberbeastarea.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"fire","presentation":{"impact_asset_binding":"canary.appearance:effect/explosionhit"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.emberbeastarea","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Heal","formula":{"family":"Formula","key":"oteryn:formula.spell.emberbeastarea.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.emberbeastarea.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.emberbeastarea.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.emberbeasthur":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.the_count_of_the_core","revision":"definition-r1"}],"excludes_caster_name":true,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"heal":true,"minimum":0,"maximum":1500,"raw_effect_key":"oteryn:effect.spell.emberbeasthur.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["x","x","x","x","x","C"],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.emberbeasthur","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.the_count_of_the_core","revision":"definition-r1"}],"excludes_caster_name":true,"includes_caster":false,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"healing"}},"target":{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1500,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.emberbeasthur.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"fire","presentation":{"impact_asset_binding":"canary.appearance:effect/explosionhit"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.emberbeasthur","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Heal","formula":{"family":"Formula","key":"oteryn:formula.spell.emberbeasthur.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.emberbeasthur.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.emberbeasthur.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.freed_soul_spell":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.the_souldespoiler","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"heal":false,"minimum":500,"maximum":2000,"raw_effect_key":"oteryn:effect.spell.freed_soul_spell.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["..xxx..",".xxxxx.","xxxxxxx","xxxCxxx","xxxxxxx",".xxxxx.","..xxx.."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.freed_soul_spell","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.the_souldespoiler","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":2000,"minimum":500}},"target":{"family":"Formula","key":"oteryn:formula.spell.freed_soul_spell.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"energy","presentation":{"impact_asset_binding":"canary.appearance:effect/purpleenergy"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.freed_soul_spell","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.freed_soul_spell.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.freed_soul_spell.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.freed_soul_spell.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.frozen_minion_wave":{"primary":null,"callbacks":[{"filter":{"excludes_caster_name":false,"includes_caster":true,"kind":"NonPlayerSide","top_creature_only":false},"heal":true,"minimum":200,"maximum":700,"raw_effect_key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-1"},{"filter":{"excludes_caster_name":false,"includes_caster":false,"kind":"PlayerSide","top_creature_only":false},"heal":true,"minimum":200,"maximum":700,"raw_effect_key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-2"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["....xxx....","....xxx....","....xxx....",".....x.....",".....C....."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-2","revision":"definition-r1"},"kind":"Executable"},{"effect":{"key":"oteryn:effect.spell.frozen_minion_wave.effect-presentation","operation":{"operation":"PresentationOnly"},"presentation":{"impact_asset_binding":"canary.appearance:effect/poff"}},"kind":"Inline"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.frozen_minion_wave","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"excludes_caster_name":false,"includes_caster":true,"kind":"NonPlayerSide","top_creature_only":false},"damage_type":"healing"}},"target":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":700,"minimum":200}},"target":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_wave.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"excludes_caster_name":false,"includes_caster":false,"kind":"PlayerSide","top_creature_only":false},"damage_type":"ice"}},"target":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-2","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":700,"minimum":200}},"target":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_wave.formula-callback-2","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-2","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.frozen_minion_wave","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Heal","formula":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_wave.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_wave.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_wave.formula-callback-2","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_wave.effect-callback-2","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_wave.formula-callback-2","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.frozen_minion_beam":{"primary":null,"callbacks":[{"filter":{"excludes_caster_name":false,"includes_caster":true,"kind":"NonPlayerSide","top_creature_only":false},"heal":true,"minimum":200,"maximum":700,"raw_effect_key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-1"},{"filter":{"excludes_caster_name":false,"includes_caster":false,"kind":"PlayerSide","top_creature_only":false},"heal":true,"minimum":200,"maximum":700,"raw_effect_key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-2"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":["x","x","x","x","x","x","C"],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-2","revision":"definition-r1"},"kind":"Executable"},{"effect":{"key":"oteryn:effect.spell.frozen_minion_beam.effect-presentation","operation":{"operation":"PresentationOnly"},"presentation":{"impact_asset_binding":"canary.appearance:effect/poff"}},"kind":"Inline"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.frozen_minion_beam","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"excludes_caster_name":false,"includes_caster":true,"kind":"NonPlayerSide","top_creature_only":false},"damage_type":"healing"}},"target":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":700,"minimum":200}},"target":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_beam.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"excludes_caster_name":false,"includes_caster":false,"kind":"PlayerSide","top_creature_only":false},"damage_type":"ice"}},"target":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-2","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":700,"minimum":200}},"target":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_beam.formula-callback-2","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-2","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.frozen_minion_beam","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Heal","formula":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_beam.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_beam.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_beam.formula-callback-2","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.frozen_minion_beam.effect-callback-2","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.frozen_minion_beam.formula-callback-2","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.ravennouslavalurkerwave":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.gnome_pack_crawler","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.lost_gnome","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"heal":false,"minimum":0,"maximum":1000,"raw_effect_key":"oteryn:effect.spell.ravennouslavalurkerwave.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":[".xxx.","xxxxx","xxCxx","xxxxx",".xxx."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":false,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.ravennouslavalurkerwave","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.gnome_pack_crawler","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.lost_gnome","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1000,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.ravennouslavalurkerwave.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"fire","presentation":{"impact_asset_binding":"canary.appearance:effect/hitbyfire"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.ravennouslavalurkerwave","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.ravennouslavalurkerwave.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.ravennouslavalurkerwave.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkerwave.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.ravennouslavalurkertarget":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.gnome_pack_crawler","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.lost_gnome","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"heal":false,"minimum":0,"maximum":1000,"raw_effect_key":"oteryn:effect.spell.ravennouslavalurkertarget.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":[".......","..xxx..",".xxxxx.","xxxxxxx","xxxCxxx","xxxxxxx",".xxxxx.","..xxx..","......."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":false,"needs_target":true,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.ravennouslavalurkertarget","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.gnome_pack_crawler","revision":"definition-r1"},{"family":"Creature","key":"oteryn:creature.lost_gnome","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1000,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.ravennouslavalurkertarget.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"fire","presentation":{"impact_asset_binding":"canary.appearance:effect/fireattack"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.ravennouslavalurkertarget","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.ravennouslavalurkertarget.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.ravennouslavalurkertarget.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.ravennouslavalurkertarget.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.remorseless_wave":{"primary":2,"callbacks":[{"filter":{"excludes_caster_name":false,"includes_caster":false,"kind":"Players","top_creature_only":true},"heal":false,"minimum":0,"maximum":600,"raw_effect_key":"oteryn:effect.spell.remorseless_wave.effect-callback-1"},{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.stolen_soul","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"heal":false,"minimum":700,"maximum":1500,"raw_effect_key":"oteryn:effect.spell.remorseless_wave.effect-callback-2"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":[".....x.....","....xxx....","...xxxxx...","..xxxxxxx..",".xxxxxxxxx.","xxxxxCxxxxx",".xxxxxxxxx.","..xxxxxxx..","...xxxxx...","....xxx....",".....x....."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-2","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.remorseless_wave","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"excludes_caster_name":false,"includes_caster":false,"kind":"Players","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":600,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.remorseless_wave.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.stolen_soul","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-2","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1500,"minimum":700}},"target":{"family":"Formula","key":"oteryn:formula.spell.remorseless_wave.formula-callback-2","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"death","presentation":{"impact_asset_binding":"canary.appearance:effect/blacksmoke"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-2","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.remorseless_wave","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.remorseless_wave.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.remorseless_wave.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.remorseless_wave.formula-callback-2","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect-callback-2","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.remorseless_wave.formula-callback-2","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.remorseless_wave.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.source_of_corruption_wave":{"primary":2,"callbacks":[{"filter":{"excludes_caster_name":false,"includes_caster":false,"kind":"Players","top_creature_only":true},"heal":false,"minimum":0,"maximum":600,"raw_effect_key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-1"},{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.stolen_soul","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"heal":false,"minimum":700,"maximum":1500,"raw_effect_key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-2"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":[".....",".xCx.","....."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-2","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.source_of_corruption_wave","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"excludes_caster_name":false,"includes_caster":false,"kind":"Players","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":600,"minimum":0}},"target":{"family":"Formula","key":"oteryn:formula.spell.source_of_corruption_wave.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.stolen_soul","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"untyped"}},"target":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-2","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":1500,"minimum":700}},"target":{"family":"Formula","key":"oteryn:formula.spell.source_of_corruption_wave.formula-callback-2","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"energy","presentation":{"impact_asset_binding":"canary.appearance:effect/purpleenergy"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-2","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.source_of_corruption_wave","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.source_of_corruption_wave.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.source_of_corruption_wave.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.spell.source_of_corruption_wave.formula-callback-2","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect-callback-2","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.source_of_corruption_wave.formula-callback-2","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.source_of_corruption_wave.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]},"oteryn:ability.spell.unchained_fire_beam":{"primary":1,"callbacks":[{"filter":{"creatures":[{"family":"Creature","key":"oteryn:creature.magma_bubble","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"heal":true,"minimum":2000,"maximum":5000,"raw_effect_key":"oteryn:effect.spell.unchained_fire_beam.effect-callback-1"}],"profiles":[{"data":{"kind":"Ability","profile":{"details":{"area":{"north":[".x.",".x.",".x.",".x.",".x.",".x.",".x.",".C."],"shape":"Matrix"},"effects":[{"effect":{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect-callback-1","revision":"definition-r1"},"kind":"Executable"},{"effect":{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect","revision":"definition-r1"},"kind":"Executable"}],"kind":"Spell","needs_direction":true,"needs_target":false,"range_tiles":0}}},"target":{"family":"Ability","key":"oteryn:ability.spell.unchained_fire_beam","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"affects":{"creatures":[{"family":"Creature","key":"oteryn:creature.magma_bubble","revision":"definition-r1"}],"excludes_caster_name":false,"includes_caster":true,"kind":"NamedCreatures","top_creature_only":true},"damage_type":"healing"}},"target":{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect-callback-1","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"Range","maximum":5000,"minimum":2000}},"target":{"family":"Formula","key":"oteryn:formula.spell.unchained_fire_beam.formula-callback-1","revision":"definition-r1"}},{"data":{"kind":"Effect","profile":{"damage_type":"fire","presentation":{"impact_asset_binding":"canary.appearance:effect/firearea"}}},"target":{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect","revision":"definition-r1"}},{"data":{"kind":"Formula","profile":{"formula":"CasterMagnitude"}},"target":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"}}],"records":[{"effects":[{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect-callback-1","revision":"definition-r1"},{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect","revision":"definition-r1"}],"identity":{"family":"Ability","key":"oteryn:ability.spell.unchained_fire_beam","revision":"definition-r1"},"kind":"Ability"},{"client_projection":"ServerOnly","effect_family":"Heal","formula":{"family":"Formula","key":"oteryn:formula.spell.unchained_fire_beam.formula-callback-1","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect-callback-1","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.spell.unchained_fire_beam.formula-callback-1","revision":"definition-r1"},"kind":"Formula"},{"client_projection":"ServerOnly","effect_family":"Damage","formula":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"identity":{"family":"Effect","key":"oteryn:effect.spell.unchained_fire_beam.effect","revision":"definition-r1"},"kind":"Effect"},{"identity":{"family":"Formula","key":"oteryn:formula.creature.caster-magnitude","revision":"definition-r1"},"kind":"Formula"}]}}}"##;

#[cfg(test)]
#[path = "creature_damage_composed_tests.rs"]
mod tests;
