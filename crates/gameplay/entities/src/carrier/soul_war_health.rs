//! Source callback context; actual creator role is never inferred from credit.
use super::soul_war_player_source::{SoulWarPlayerSourceFacts, SoulWarPlayerSourceOrigin};
use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum HealthSourceKind {
    Null,
    NonPlayer,
    Player(Option<SoulWarPlayerSourceFacts>),
}
#[derive(Debug, Clone, Copy)]
pub(super) struct HealthSourceContext {
    pub kind: HealthSourceKind,
    /// Independently checked physical caster; reward/master credit is separate.
    pub actor: Option<ExactActorRef>,
    pub content: [u8; 32],
    pub invoke_health_change: bool,
}
impl ChannelActorCarrier {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn soul_war_health_context(
        &self,
        continuity: &NamespaceContinuityGuard,
        source: Option<ExactActorRef>,
        command: Option<CommandRef>,
        now_ms: Option<u64>,
        origin: SoulWarPlayerSourceOrigin,
        content: &ChannelContentPin,
    ) -> Result<HealthSourceContext, CarrierError> {
        self.validate_current_continuity(continuity)?;
        let Some(source) = source else {
            return Ok(HealthSourceContext {
                kind: HealthSourceKind::Null,
                actor: None,
                content: content.server_artifact_digest(),
                invoke_health_change: true,
            });
        };
        let index = self.validate_ref(continuity, source.0)?;
        let kind = match &self.slots[index] {
            Slot::Occupied {
                game_session_id: Some(session),
                spell_combat,
                position: Some(position),
                ..
            } => {
                let lease = self
                    .bound_attacker_lease(continuity, source.0, *session)
                    .ok();
                let data = spell_combat
                    .soul_war_player_source
                    .as_ref()
                    .filter(|data| {
                        data.actor == source
                            && data.session == *session
                            && data.command == command
                            && Some(data.now_us / 1000) == now_ms
                            && data.scope == self.runtime_scope
                            && data.generation == self.scope_generation.get()
                            && data.server == content.server_artifact_digest()
                            && data.client == content.client_artifact_digest()
                            && data.frame == content.frame_binding_digest()
                            && data.map == content.map_revision_digest()
                            && lease.as_ref().is_some_and(|lease| {
                                data.character == *lease.character_id().as_bytes()
                                    && data.lease == lease.generation()
                            })
                            && data.position.x == position.position.x
                            && data.position.y == position.position.y
                            && data.position.floor == position.position.floor
                            && data.facts.origin == origin
                    })
                    .map(|data| data.facts);
                HealthSourceKind::Player(data)
            }
            Slot::CreatureOccupied { spell_combat, .. } if spell_combat.source_death_completed => {
                // The source Game registry has removed this creature after
                // its deferred onDeath. Physical SQL drop retention does not
                // keep that old creature available as a damage caster.
                return Ok(HealthSourceContext {
                    kind: HealthSourceKind::Null,
                    actor: None,
                    content: content.server_artifact_digest(),
                    invoke_health_change: true,
                });
            }
            Slot::CreatureOccupied { .. } => HealthSourceKind::NonPlayer,
            _ => return Err(CarrierError::CreatureTargetMismatch),
        };
        Ok(HealthSourceContext {
            kind,
            actor: Some(source),
            content: content.server_artifact_digest(),
            invoke_health_change: true,
        })
    }
}

pub(super) fn source_rule_draw(
    binding: &[u8],
    actor: ActorRef,
    domain: &[u8],
    bound: u32,
) -> Result<u32, CarrierError> {
    use sha2::{Digest, Sha256};
    if bound == 0 {
        return Err(CarrierError::InvalidCommitBinding);
    }
    let limit = u64::MAX - u64::MAX % u64::from(bound);
    for attempt in 0u64..64 {
        let digest = Sha256::new()
            .chain_update(b"srx3:source-health-rule:v1")
            .chain_update(ExactActorRef(actor).placement_identity())
            .chain_update(binding)
            .chain_update(domain)
            .chain_update(attempt.to_be_bytes())
            .finalize();
        let mut bytes = [0; 8];
        bytes.copy_from_slice(&digest[..8]);
        let draw = u64::from_be_bytes(bytes);
        if draw < limit {
            return Ok((draw % u64::from(bound)) as u32);
        }
    }
    Err(CarrierError::InvalidCommitBinding)
}

/// Immutable views of the actual five native policies installed with this image.
/// This does not define policy or create a source grant; the exact bounded callback
/// witness admitted the target and children before the owner attached these Arcs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MirrorBirthCatalogue {
    content: [u8; 32],
    children: [Arc<oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy>; 5],
}
impl MirrorBirthCatalogue {
    pub(super) fn from_installed(
        table: &oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicies,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        if table.source_digest() != content {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        use oteryn_gameplay_domain::domain::soul_war_mirror::Apparition;
        let mut children = Vec::with_capacity(5);
        for apparition in Apparition::ALL {
            let mut actual = table
                .policies()
                .iter()
                .filter(|p| p.definition_key == apparition.creature_key());
            let policy = actual.next().ok_or(CarrierError::InvalidCreatureTarget)?;
            if actual.next().is_some() {
                return Err(CarrierError::InvalidCreatureTarget);
            }
            children.push(Arc::clone(policy));
        }
        let children = children
            .try_into()
            .map_err(|_| CarrierError::InvalidCreatureTarget)?;
        Ok(Self { content, children })
    }
    pub(super) fn policy(
        &self,
        content: [u8; 32],
        apparition: oteryn_gameplay_domain::domain::soul_war_mirror::Apparition,
    ) -> Result<
        Arc<oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy>,
        CarrierError,
    > {
        if self.content != content {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        self.children
            .iter()
            .find(|p| p.definition_key == apparition.creature_key())
            .map(Arc::clone)
            .ok_or(CarrierError::InvalidCreatureTarget)
    }
}

pub(super) fn apparition_vocation(
    vocation: oteryn_gameplay_content::content::spell_data::Vocation,
) -> Option<oteryn_gameplay_domain::domain::soul_war_mirror::Apparition> {
    use oteryn_gameplay_content::content::spell_data::Vocation as V;
    use oteryn_gameplay_domain::domain::soul_war_mirror::Apparition as A;
    match vocation {
        V::None => None,
        V::Druid | V::ElderDruid => Some(A::Druid),
        V::Knight | V::EliteKnight => Some(A::Knight),
        V::Paladin | V::RoyalPaladin => Some(A::Paladin),
        V::Sorcerer | V::MasterSorcerer => Some(A::Sorcerer),
        V::Monk | V::ExaltedMonk => Some(A::Monk),
    }
}
