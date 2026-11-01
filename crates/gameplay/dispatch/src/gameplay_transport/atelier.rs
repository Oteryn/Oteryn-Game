//! Native dispatch boundary for Gem Atelier/Fragment Workshop commands.

pub use oteryn_gameplay_progression::atelier::{
    AtelierAction, AtelierCommit, AtelierRefusal, AtelierState,
};

/// Pure state-transition boundary; the wire path below adds the admitted Character fence,
/// ordinary item/fee custody and immutable durable receipts.
pub fn dispatch_atelier(
    state: &mut AtelierState,
    expected_revision: u64,
    action: AtelierAction,
) -> Result<AtelierCommit, AtelierRefusal> {
    state.commit(expected_revision, action)
}

/// Server commands on the admitted Character owner, shared by admission and the Wheel wire route.
pub enum GemBuildCommand<'a> {
    Atelier(crate::durability::character_atelier::AtelierRequest<'a>),
    Wheel(crate::durability::character_wheel::WheelChangeRequest),
}
#[derive(Debug)]
pub enum GemBuildOutcome {
    Atelier(crate::durability::character_atelier::AtelierOutcome),
    Wheel(Box<crate::durability::character_wheel::WheelCommitOutcome>),
}

#[derive(Debug)]
pub enum AtelierWireReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn atelier_wire(
        &self,
        admitted: super::connection::AdmittedSession,
        kind: u32,
        payload: &[u8],
    ) -> AtelierWireReply {
        use oteryn_protocol_oteryn::atelier as wire;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_WHEEL_ALLOCATION_V1)
        {
            return AtelierWireReply::Rejected;
        }
        let Some(expected) = admitted.item_fence else {
            return AtelierWireReply::Rejected;
        };
        if expected.game_session_id != admitted.game_session_id
            || expected.connection_generation.get() != admitted.continuity.connection_generation
        {
            return AtelierWireReply::Rejected;
        }
        let request = match kind {
            wire::COMMAND_TYPE_ATELIER_QUERY => {
                if wire::decode_query(payload).is_err() {
                    return AtelierWireReply::Rejected;
                }
                None
            }
            wire::COMMAND_TYPE_ATELIER_SET_LOCK => {
                let Ok(request) = wire::decode_set_lock(payload) else {
                    return AtelierWireReply::Rejected;
                };
                Some(crate::durability::character_atelier::AtelierRequest {
                    occurrence: request.occurrence,
                    expected_atelier_revision: request.expected_revision,
                    mutation: crate::durability::character_atelier::AtelierMutation::SetLock {
                        gem: request.gem,
                        locked: request.locked,
                    },
                    resources: None,
                })
            }
            wire::COMMAND_TYPE_ATELIER_MUTATE => {
                let Ok(request) = wire::decode_mutate(payload) else {
                    return AtelierWireReply::Rejected;
                };
                let Some(mutation) = paid_wire_mutation(request.mutation) else {
                    return AtelierWireReply::Rejected;
                };
                Some(crate::durability::character_atelier::AtelierRequest {
                    occurrence: request.occurrence,
                    expected_atelier_revision: request.expected_revision,
                    mutation,
                    resources: None,
                })
            }
            _ => return AtelierWireReply::Rejected,
        };
        let mut refused = false;
        let mut recorded = None;
        if let Some(mut request) = request {
            let Some(permit) = self.spell_lane_permit().await else {
                return AtelierWireReply::Unavailable;
            };
            {
                let states = self.states.spell_states.lock().await;
                if admitted.runtime_actor.is_none_or(|actor| {
                    states.is_dead(actor)
                        || states.has_pending_spell_commit(actor, admitted.game_session_id)
                }) {
                    return AtelierWireReply::Rejected;
                }
            }
            let backpack = match self
                .root
                .read_character_backpack(self.character, expected.character_id)
                .await
            {
                Ok(backpack) => backpack,
                Err(_) => return AtelierWireReply::Unavailable,
            };
            let content = self
                .active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay)
                .map(|native| {
                    super::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(self.premium_coordinator)
                });
            {
                let runtime = self.runtime.lock().await;
                let binding = runtime.binding();
                if admitted.world_id != binding.world_id()
                    || binding.channel_id() != Some(admitted.channel_id)
                    || expected.runtime_scope != binding.runtime_scope()
                    || expected.scope_ownership_generation != binding.scope_generation()
                    || runtime.owner_fence().is_err()
                    || !admitted.runtime_actor.is_some_and(|actor| {
                        runtime
                            .player_control_facts(actor, admitted.game_session_id)
                            .is_ok_and(|facts| facts.control_loss.is_none())
                            && runtime
                                .bound_attacker_lease(actor, admitted.game_session_id)
                                .is_ok_and(|lease| {
                                    lease.character_id().as_bytes()
                                        == expected.character_id.as_bytes()
                                        && lease.generation() == expected.character_lease_generation
                                })
                            && runtime.assert_actor_spell_unreserved(actor).is_ok()
                    })
                {
                    return AtelierWireReply::Rejected;
                }
                // Pinned native definitions and the ordinary backpack/coin owner supply all
                // economic facts. Missing resources do not hide an already committed receipt.
                if let Some(content) = &content {
                    request.resources = atelier_wire_resources(
                        self,
                        &runtime,
                        backpack.as_ref(),
                        content,
                        admitted.game_session_id,
                        &request,
                    );
                }
            }
            let result = self
                .commit_wheel_client(
                    admitted.game_session_id,
                    GemBuildCommand::Atelier(request),
                    false,
                    Some(expected),
                )
                .await;
            drop(permit);
            match result {
                Ok(GemBuildOutcome::Atelier(
                    crate::durability::character_atelier::AtelierOutcome::Refused,
                )) => refused = true,
                Ok(GemBuildOutcome::Atelier(
                    crate::durability::character_atelier::AtelierOutcome::Committed { state, .. }
                    | crate::durability::character_atelier::AtelierOutcome::AlreadyCommitted { state, .. },
                )) => recorded = Some(state),
                Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput
                    | crate::durability::character_progression::CharacterProgressionError::AuthorityRejected
                    | crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence) => return AtelierWireReply::Rejected,
                _ => return AtelierWireReply::Unavailable,
            }
            if let Some(actor) = admitted.runtime_actor {
                self.refresh_wheel_vitals(actor, admitted.game_session_id)
                    .await;
            }
        }
        let state = recorded.or_else(|| {
            self.states
                .atelier_sessions
                .lock()
                .ok()
                .and_then(|states| states.get(&admitted.game_session_id).cloned().flatten())
        });
        let state = match state {
            Some(state) => state,
            None => match self
                .root
                .read_character_atelier(self.character, expected.character_id)
                .await
            {
                Ok(Some(state)) => state,
                // Admission intentionally creates no Atelier row below eligibility. A
                // selected client may still query the panel; this is a rule refusal,
                // not a failed storage read or an indeterminate committed mutation.
                Ok(None) => {
                    return wire::encode_reply(&wire::AtelierReply {
                        refused: true,
                        revision: 0,
                        gems: Vec::new(),
                        grades: Vec::new(),
                        quality_quotes: Vec::new(),
                        grade_quotes: Vec::new(),
                    })
                    .map_or(AtelierWireReply::Unavailable, AtelierWireReply::Payload);
                }
                Err(_) => return AtelierWireReply::Unavailable,
            },
        };
        let (quality_quotes, grade_quotes) = if kind == wire::COMMAND_TYPE_ATELIER_QUERY {
            let Some(quotes) = atelier_price_quotes() else {
                return AtelierWireReply::Unavailable;
            };
            quotes
        } else {
            // Mutation replies are immutable receipt projections. Current prices and
            // derived vessel flags come from a subsequent live query.
            (Vec::new(), Vec::new())
        };
        let gems = state
            .gems
            .values()
            .map(|gem| wire::GemView {
                id: gem.id.to_be_bytes(),
                quality: match gem.quality {
                    oteryn_gameplay_progression::atelier::GemQuality::Lesser => 0,
                    oteryn_gameplay_progression::atelier::GemQuality::Regular => 1,
                    oteryn_gameplay_progression::atelier::GemQuality::Greater => 2,
                },
                domain: match gem.domain {
                    oteryn_gameplay_progression::wheel::WheelDomain::Green => 0,
                    oteryn_gameplay_progression::wheel::WheelDomain::Red => 1,
                    oteryn_gameplay_progression::wheel::WheelDomain::Purple => 2,
                    oteryn_gameplay_progression::wheel::WheelDomain::Blue => 3,
                },
                locked: gem.locked,
                vessel: gem.vessel,
                modifiers: gem.mods.clone(),
            })
            .collect();
        wire::encode_reply(&wire::AtelierReply {
            refused,
            revision: state.revision,
            gems,
            grades: state
                .grades
                .iter()
                .map(|(&(kind, mod_id), &grade)| wire::GradeView {
                    kind: match kind {
                        oteryn_gameplay_progression::atelier::FragmentKind::Basic => 0,
                        oteryn_gameplay_progression::atelier::FragmentKind::Supreme => 1,
                    },
                    mod_id,
                    grade,
                })
                .collect(),
            quality_quotes,
            grade_quotes,
        })
        .map_or(AtelierWireReply::Unavailable, AtelierWireReply::Payload)
    }
    /// Runtime Gem Atelier entry point.
    ///
    /// All player-facing Atelier and vessel commands must enter here rather than calling a
    /// durability writer directly. The entry resolves the current session fence, serializes the
    /// Character revision, and refreshes both projections only after the atomic writer returns.
    pub async fn execute_gem_atelier(
        &self,
        session: crate::foundation::GameSessionId,
        command: GemBuildCommand<'_>,
    ) -> Result<GemBuildOutcome, crate::durability::character_progression::CharacterProgressionError>
    {
        self.commit_gem_build(session, command).await
    }

    async fn commit_gem_build(
        &self,
        session: crate::foundation::GameSessionId,
        command: GemBuildCommand<'_>,
    ) -> Result<GemBuildOutcome, crate::durability::character_progression::CharacterProgressionError>
    {
        self.commit_wheel_client(session, command, false, None)
            .await
    }

    pub(super) async fn commit_wheel_client(
        &self,
        session: crate::foundation::GameSessionId,
        command: GemBuildCommand<'_>,
        at_temple: bool,
        expected: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
    ) -> Result<GemBuildOutcome, crate::durability::character_progression::CharacterProgressionError>
    {
        use crate::durability::character_progression::CharacterProgressionError as Error;
        let ruleset = self.wheel_ruleset.as_ref().ok_or(Error::InvalidInput)?;
        let gate = self.premium_coordinator.ok_or(Error::InvalidInput)?;
        let fence = self
            .current_quest_fence(session)
            .await
            .map_err(|_| Error::AuthorityRejected)?
            .ok_or(Error::AuthorityRejected)?;
        if expected.is_some_and(|expected| {
            expected.character_id != fence.character_id
                || expected.game_session_id != fence.game_session_id
                || expected.connection_generation != fence.connection_generation
                || expected.character_lease_generation != fence.character_lease_generation
                || expected.runtime_scope != fence.runtime_scope
                || expected.scope_ownership_generation != fence.scope_ownership_generation
        }) {
            return Err(Error::AuthorityRejected);
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        // A failed or unknown commit must not leave a usable stale actor projection.
        self.states
            .wheel_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, None);
        self.states
            .atelier_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, None);
        // The durable writer invokes this only after its locked receipt lookup.
        // Keep the first decision across the Character sequencer's revision retry;
        // the leaf gate lock is released before any database or owner await.
        let admitted = std::sync::OnceLock::new();
        let admit = || {
            *admitted.get_or_init(|| {
                self.states.world_gate.lock().is_ok_and(|mut gate| {
                    gate.admit_wheel_mutation(
                        session,
                        std::time::Duration::from_micros(self.owner_now().get()),
                    )
                })
            })
        };
        let outcome = match command {
            GemBuildCommand::Atelier(request) => GemBuildOutcome::Atelier(
                slot.atelier_with_admission(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    std::sync::Arc::clone(ruleset),
                    gate,
                    &admit,
                )
                .await?,
            ),
            GemBuildCommand::Wheel(request) => GemBuildOutcome::Wheel(Box::new(
                slot.commit_wheel_with_admission(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    std::sync::Arc::clone(ruleset),
                    crate::durability::character_wheel::WheelChangeFacts { at_temple },
                    gate,
                    &admit,
                )
                .await?,
            )),
        };
        // Reload current committed rows, including on receipt replay after a lost acknowledgement.
        let allocation = self
            .root
            .read_character_wheel(
                self.character,
                fence.character_id,
                std::sync::Arc::clone(ruleset),
            )
            .await?;
        let atelier = self
            .root
            .read_character_atelier(self.character, fence.character_id)
            .await?;
        self.states
            .atelier_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, atelier);
        self.states
            .wheel_sessions
            .lock()
            .map_err(|_| Error::InvalidInput)?
            .insert(session, Some(allocation));
        Ok(outcome)
    }
}

fn atelier_price_quotes() -> Option<(
    Vec<oteryn_protocol_oteryn::atelier::QualityQuote>,
    Vec<oteryn_protocol_oteryn::atelier::GradeQuote>,
)> {
    use oteryn_gameplay_progression::{atelier::GemQuality, wheel_gem_data::WheelGemData};
    use oteryn_protocol_oteryn::atelier::{GradeQuote, QualityQuote};
    let data = WheelGemData::embedded().ok()?;
    let catalogue = &data.gems()["catalogue"];
    let atelier = &catalogue["atelier"];
    let mut qualities = Vec::with_capacity(3);
    for (quality, key, kind) in [
        (0, "lesser", GemQuality::Lesser),
        (1, "regular", GemQuality::Regular),
        (2, "greater", GemQuality::Greater),
    ] {
        let (fragment_min, fragment_max) = kind.revealed_fragment_yield();
        qualities.push(QualityQuote {
            quality,
            reveal_gold: atelier["fees"]["reveal"][key].as_u64()?,
            switch_gold: atelier["fees"]["switch_domain"][key].as_u64()?,
            fragment_kind: match atelier["fragment_yields"][key]["fragment"].as_str()? {
                "basic" => 0,
                "supreme" => 1,
                _ => return None,
            },
            fragment_min,
            fragment_max,
        });
    }
    let mut grades = Vec::with_capacity(6);
    for grade in 1..=3 {
        for (kind, key) in [(0, "basic"), (1, "supreme")] {
            let cost = &catalogue["grade_costs"][usize::from(grade - 1)][key];
            grades.push(GradeQuote {
                kind,
                grade,
                gold: cost["gold"].as_u64()?,
                fragments: u32::try_from(cost["fragments"].as_u64()?).ok()?,
            });
        }
    }
    Some((qualities, grades))
}

fn paid_wire_mutation(
    action: oteryn_protocol_oteryn::atelier::Mutation,
) -> Option<crate::durability::character_atelier::AtelierMutation> {
    use crate::durability::character_atelier::AtelierMutation;
    use oteryn_gameplay_progression::atelier::{FragmentKind, GemQuality};
    use oteryn_protocol_oteryn::atelier::Mutation;
    Some(match action {
        Mutation::Reveal { quality } => AtelierMutation::Reveal {
            quality: match quality {
                0 => GemQuality::Lesser,
                1 => GemQuality::Regular,
                2 => GemQuality::Greater,
                _ => return None,
            },
        },
        Mutation::Dismantle { gem } => AtelierMutation::Dismantle { gem },
        Mutation::SwitchDomain { gem } => AtelierMutation::SwitchDomain { gem },
        Mutation::GradeUp { kind, mod_id } => AtelierMutation::GradeUp {
            kind: match kind {
                0 => FragmentKind::Basic,
                1 => FragmentKind::Supreme,
                _ => return None,
            },
            mod_id,
        },
    })
}

fn atelier_change_id(
    session: crate::foundation::GameSessionId,
    occurrence: [u8; 16],
    slot: u8,
) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn:atelier:coin-change:v1");
    // An occurrence is Character-local; coin ItemInstance identities are global.
    hash.update(session.as_bytes());
    hash.update(occurrence);
    hash.update([slot]);
    let digest = hash.finalize();
    let mut id = occurrence;
    id[6..].copy_from_slice(&digest[..10]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

fn atelier_wire_resources<'a>(
    admission: &super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &crate::foundation::ChannelRuntimeV1,
    backpack: Option<&crate::durability::item_transfer::CharacterBackpack>,
    content: &'a dyn crate::durability::item_loop_move::ItemLoopContent,
    session: crate::foundation::GameSessionId,
    request: &crate::durability::character_atelier::AtelierRequest<'_>,
) -> Option<Box<crate::durability::character_atelier::AtelierResources<'a>>> {
    use crate::durability::character_atelier::{AtelierMutation, AtelierResources};
    use oteryn_gameplay_progression::atelier::{FragmentKind, GemQuality};
    if matches!(
        request.mutation,
        AtelierMutation::SetLock { .. } | AtelierMutation::Initialize
    ) {
        return None;
    }
    let supreme = match request.mutation {
        AtelierMutation::GradeUp { kind, .. } => kind == FragmentKind::Supreme,
        AtelierMutation::Dismantle { gem } => admission
            .states
            .atelier_sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(&session)
                    .and_then(Option::as_ref)
                    .and_then(|state| state.gems.get(&u128::from_be_bytes(gem)))
                    .map(|gem| gem.quality == GemQuality::Greater)
            })
            .unwrap_or(false),
        _ => false,
    };
    let key = if supreme {
        "oteryn:item.tibia.i46626"
    } else {
        "oteryn:item.tibia.i46625"
    };
    let index = super::item_ref_admission::pinned_index(runtime, admission.active_generation)?;
    let coins = super::charm_progression::fee_change(
        admission,
        runtime,
        backpack,
        [
            atelier_change_id(session, request.occurrence, 0),
            atelier_change_id(session, request.occurrence, 1),
        ],
    )?;
    Some(Box::new(AtelierResources {
        content,
        coins,
        fragment: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: index.revision(key)?.into(),
        },
    }))
}

macro_rules! authority_methods {
    () => {
        fn atelier_command(&self, _admitted: $crate::gameplay_transport::connection::AdmittedSession, _kind: u32, _payload: &[u8]) -> impl std::future::Future<Output = $crate::gameplay_transport::atelier::AtelierWireReply> {
            async { $crate::gameplay_transport::atelier::AtelierWireReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn atelier_command(
            &self,
            admitted: connection::AdmittedSession,
            kind: u32,
            payload: &[u8],
        ) -> atelier::AtelierWireReply {
            self.atelier_wire(admitted, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test fixtures")]
mod tests {
    use super::*;
    use oteryn_gameplay_progression::{
        atelier::{FragmentKind, GemQuality},
        wheel::WheelDomain,
    };

    fn funded_state() -> AtelierState {
        AtelierState {
            vocation: "elite_knight".to_owned(),
            gold: 100_000_000,
            ..AtelierState::default()
        }
    }

    fn apply(state: &mut AtelierState, action: AtelierAction) -> AtelierCommit {
        dispatch_atelier(state, state.revision, action).unwrap()
    }

    #[test]
    fn atelier_change_identity_binds_the_server_session_and_output_slot() {
        let mut session_id = [1; 16];
        session_id[6] = 0x70;
        session_id[8] = 0x80;
        let session = crate::foundation::GameSessionId::decode(&session_id).unwrap();
        session_id[15] = 2;
        let other = crate::foundation::GameSessionId::decode(&session_id).unwrap();
        let occurrence = [7; 16];
        let first = atelier_change_id(session, occurrence, 0);
        assert_eq!(first, atelier_change_id(session, occurrence, 0));
        assert_ne!(first, atelier_change_id(session, occurrence, 1));
        assert_ne!(first, atelier_change_id(other, occurrence, 0));
        assert_eq!(first[6] >> 4, 7);
        assert_eq!(first[8] >> 6, 2);
    }

    #[test]
    fn atelier_quotes_use_the_writer_catalogue_and_crystal_fragment_bounds() {
        let (qualities, grades) = atelier_price_quotes().unwrap();
        assert_eq!(
            qualities
                .iter()
                .map(|quote| quote.reveal_gold)
                .collect::<Vec<_>>(),
            vec![125_000, 1_000_000, 6_000_000]
        );
        assert_eq!(
            qualities
                .iter()
                .map(|quote| quote.switch_gold)
                .collect::<Vec<_>>(),
            vec![125_000, 250_000, 500_000]
        );
        assert_eq!(
            qualities
                .iter()
                .map(|quote| (quote.fragment_min, quote.fragment_max))
                .collect::<Vec<_>>(),
            vec![(1, 5), (2, 10), (1, 5)]
        );
        assert_eq!(
            grades
                .iter()
                .map(|quote| (quote.kind, quote.grade, quote.gold, quote.fragments))
                .collect::<Vec<_>>(),
            vec![
                (0, 1, 2_000_000, 5),
                (1, 1, 5_000_000, 5),
                (0, 2, 5_000_000, 15),
                (1, 2, 12_000_000, 15),
                (0, 3, 30_000_000, 30),
                (1, 3, 75_000_000, 30)
            ]
        );
    }

    fn reveal(state: &mut AtelierState, id: u128, quality: GemQuality, mods: Vec<u16>) {
        apply(state, AtelierAction::Drop { quality });
        apply(
            state,
            AtelierAction::Reveal {
                id,
                quality,
                domain: WheelDomain::Green,
                mods,
            },
        );
    }

    fn refused(state: &mut AtelierState, action: AtelierAction, expected: AtelierRefusal) {
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(state, state.revision, action),
            Err(expected)
        );
        assert_eq!(
            *state, before,
            "refusals must preserve every balance and revision"
        );
    }

    #[test]
    fn atelier_reveal_checks_vocation_positions_compatibility_and_exact_fees() {
        let mut state = funded_state();
        for (id, quality, mods, fee) in [
            (1, GemQuality::Lesser, vec![3], 125_000),
            (2, GemQuality::Regular, vec![3, 1], 1_000_000),
            (3, GemQuality::Greater, vec![3, 1, 0], 6_000_000),
        ] {
            let gold = state.gold;
            reveal(&mut state, id, quality, mods);
            assert_eq!(state.gold, gold - fee);
            assert_eq!(state.unrevealed[&quality], 0);
        }
        apply(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Greater,
            },
        );
        for mods in [
            vec![0, 1, 0],
            vec![3, 3, 0],
            vec![3, 30, 0],
            vec![3, 1, 93],
            vec![3],
        ] {
            refused(
                &mut state,
                AtelierAction::Reveal {
                    id: 4,
                    quality: GemQuality::Greater,
                    domain: WheelDomain::Green,
                    mods,
                },
                AtelierRefusal::InvalidDraw,
            );
        }
        state.gold = 5_999_999;
        refused(
            &mut state,
            AtelierAction::Reveal {
                id: 4,
                quality: GemQuality::Greater,
                domain: WheelDomain::Green,
                mods: vec![3, 1, 0],
            },
            AtelierRefusal::InsufficientGold,
        );
    }

    #[test]
    fn atelier_one_socket_per_domain_and_remove_releases_the_vessel() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Lesser, vec![3]);
        reveal(&mut state, 2, GemQuality::Lesser, vec![3]);
        apply(&mut state, AtelierAction::Socket { id: 1 });
        refused(
            &mut state,
            AtelierAction::Socket { id: 2 },
            AtelierRefusal::VesselOccupied,
        );
        refused(
            &mut state,
            AtelierAction::SwitchDomain { id: 1 },
            AtelierRefusal::InVessel,
        );
        apply(&mut state, AtelierAction::Remove { id: 1 });
        apply(&mut state, AtelierAction::Socket { id: 2 });
        assert!(!state.gems[&1].vessel);
        assert!(state.gems[&2].vessel);
    }

    #[test]
    fn atelier_switch_lock_dismantle_ranges_and_last_domain_guards() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Regular, vec![3, 1]);
        refused(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 2,
            },
            AtelierRefusal::LastDomainGem,
        );
        apply(
            &mut state,
            AtelierAction::Lock {
                id: 1,
                locked: true,
            },
        );
        refused(
            &mut state,
            AtelierAction::SwitchDomain { id: 1 },
            AtelierRefusal::Locked,
        );
        refused(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 2,
            },
            AtelierRefusal::Locked,
        );
        apply(
            &mut state,
            AtelierAction::Lock {
                id: 1,
                locked: false,
            },
        );
        let before = state.gold;
        for domain in [
            WheelDomain::Red,
            WheelDomain::Purple,
            WheelDomain::Blue,
            WheelDomain::Green,
        ] {
            apply(&mut state, AtelierAction::SwitchDomain { id: 1 });
            assert_eq!(state.gems[&1].domain, domain);
        }
        assert_eq!(state.gold, before - 4 * 250_000);
        reveal(&mut state, 2, GemQuality::Lesser, vec![3]);
        // Crystal 201189f player_wheel.cpp:1311-1312: Regular yields 2..=10.
        for yield_amount in [1, 11] {
            refused(
                &mut state,
                AtelierAction::Dismantle {
                    id: 1,
                    yield_amount,
                },
                AtelierRefusal::InvalidDraw,
            );
        }
        apply(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 10,
            },
        );
        assert_eq!(state.fragments[&FragmentKind::Basic], 10);
        assert!(!state.gems.contains_key(&1));
    }

    #[test]
    fn atelier_grade_catalogue_costs_revisions_and_json_reload() {
        let mut state = funded_state();
        reveal(&mut state, u128::MAX, GemQuality::Regular, vec![3, 1]);
        for (kind, mod_id) in [(FragmentKind::Basic, 32), (FragmentKind::Supreme, 93)] {
            refused(
                &mut state,
                AtelierAction::GradeUp { kind, mod_id },
                AtelierRefusal::InvalidDraw,
            );
        }
        refused(
            &mut state,
            AtelierAction::GradeUp {
                kind: FragmentKind::Basic,
                mod_id: 3,
            },
            AtelierRefusal::InsufficientFragments,
        );
        state.fragments.insert(FragmentKind::Basic, 50);
        for (gold, fragments, points) in
            [(2_000_000, 5, 0), (5_000_000, 15, 0), (30_000_000, 30, 1)]
        {
            let before = state.clone();
            let outcome = apply(
                &mut state,
                AtelierAction::GradeUp {
                    kind: FragmentKind::Basic,
                    mod_id: 3,
                },
            );
            assert_eq!(state.gold, before.gold - gold);
            assert_eq!(
                state.fragments[&FragmentKind::Basic],
                before.fragments[&FragmentKind::Basic] - fragments
            );
            assert_eq!(outcome.promotion_points, points);
            let stored = serde_json::to_value(&state).unwrap();
            assert_eq!(
                serde_json::from_value::<AtelierState>(stored).unwrap(),
                state
            );
        }
        refused(
            &mut state,
            AtelierAction::GradeUp {
                kind: FragmentKind::Basic,
                mod_id: 3,
            },
            AtelierRefusal::MaxGrade,
        );
        state.grade_ruleset_revision = Some("old-ruleset".to_owned());
        refused(
            &mut state,
            AtelierAction::GradeUp {
                kind: FragmentKind::Basic,
                mod_id: 3,
            },
            AtelierRefusal::RulesetMigrationPending,
        );
        assert_eq!(
            apply(
                &mut state,
                AtelierAction::Drop {
                    quality: GemQuality::Lesser
                }
            )
            .promotion_points,
            0
        );
    }

    #[test]
    fn atelier_overflow_stale_revision_and_noncurrent_gems_preserve_state() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Lesser, vec![3]);
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(&mut state, 0, AtelierAction::Socket { id: 1 }),
            Err(AtelierRefusal::StaleRevision)
        );
        assert_eq!(state, before);
        state.gems.get_mut(&1).unwrap().ruleset_revision = "old-ruleset".to_owned();
        for action in [
            AtelierAction::Socket { id: 1 },
            AtelierAction::Lock {
                id: 1,
                locked: true,
            },
            AtelierAction::Remove { id: 1 },
            AtelierAction::SwitchDomain { id: 1 },
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 1,
            },
        ] {
            refused(&mut state, action, AtelierRefusal::RulesetMigrationPending);
        }
        state.revision = u64::MAX;
        refused(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
            AtelierRefusal::InvalidDraw,
        );
        state.revision = 0;
        state.unrevealed.insert(GemQuality::Lesser, u32::MAX);
        refused(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
            AtelierRefusal::InvalidDraw,
        );
    }

    #[test]
    fn atelier_storage_cap_fragment_overflow_and_invalid_grade_json() {
        let mut state = funded_state();
        reveal(&mut state, 1, GemQuality::Lesser, vec![3]);
        let template = state.gems[&1].clone();
        for id in 2..=225 {
            let mut gem = template.clone();
            gem.id = id;
            state.gems.insert(id, gem);
        }
        apply(
            &mut state,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
        );
        refused(
            &mut state,
            AtelierAction::Reveal {
                id: 226,
                quality: GemQuality::Lesser,
                domain: WheelDomain::Green,
                mods: vec![3],
            },
            AtelierRefusal::Full,
        );
        state.fragments.insert(FragmentKind::Basic, u32::MAX);
        refused(
            &mut state,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 1,
            },
            AtelierRefusal::InvalidDraw,
        );
        let mut json = serde_json::to_value(&state).unwrap();
        json["grades"] = serde_json::json!([[["Basic", 3], 4]]);
        assert!(serde_json::from_value::<AtelierState>(json.clone()).is_err());
        json["grades"] = serde_json::json!([[["Basic", 3], 1], [["Basic", 3], 2]]);
        assert!(serde_json::from_value::<AtelierState>(json).is_err());
    }

    #[test]
    fn real_dispatch_reveals_sockets_dismantles_and_grades_without_partial_refusals() {
        let mut state = AtelierState {
            gold: 100_000_000,
            vocation: "elite_knight".to_owned(),
            ..AtelierState::default()
        };
        dispatch_atelier(
            &mut state,
            0,
            AtelierAction::Drop {
                quality: GemQuality::Greater,
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            1,
            AtelierAction::Reveal {
                id: 1,
                quality: GemQuality::Greater,
                domain: WheelDomain::Green,
                mods: vec![3, 1, 0],
            },
        )
        .unwrap();
        dispatch_atelier(&mut state, 2, AtelierAction::Socket { id: 1 }).unwrap();
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(
                &mut state,
                3,
                AtelierAction::Dismantle {
                    id: 1,
                    yield_amount: 3
                }
            ),
            Err(AtelierRefusal::InVessel)
        );
        assert_eq!(state, before);
        dispatch_atelier(&mut state, 3, AtelierAction::Remove { id: 1 }).unwrap();
        dispatch_atelier(
            &mut state,
            4,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            5,
            AtelierAction::Reveal {
                id: 2,
                quality: GemQuality::Lesser,
                domain: WheelDomain::Green,
                mods: vec![3],
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            6,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 3,
            },
        )
        .unwrap();
        state.fragments.insert(FragmentKind::Basic, 50);
        for expected in 7..10 {
            dispatch_atelier(
                &mut state,
                expected,
                AtelierAction::GradeUp {
                    kind: FragmentKind::Basic,
                    mod_id: 0,
                },
            )
            .unwrap();
        }
        assert_eq!(state.grades[&(FragmentKind::Basic, 0)], 3);
    }
}
