#![allow(clippy::expect_used, clippy::panic)]
use super::*;
use crate::foundation::{
    ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, RuntimeScopeRefV1, WorldId,
};
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
fn fixture() -> (ChannelRuntimeV1, ProjectV2Draft) {
    let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/multi/composed/native-fixture.json"))).expect("qualified fixture");
    let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
    let w = WorldId::decode(&id(1)).expect("qualified fixture");
    let r = ChannelRuntimeV1::from_committed_assignment(
        w,
        ChannelId::decode(&id(2)).expect("qualified fixture"),
        NodeId::decode(&id(3)).expect("qualified fixture"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::test(w),
    )
    .expect("qualified fixture");
    let draft = ProjectV2Draft {
        core: crate::content::ProjectDraft {
            project_revision: "fixture-r1".into(),
            package_key: "oteryn:fixture".into(),
            semantic_schema_version: "v2".into(),
            licensing_metadata: "fixture".into(),
            world_id: r
                .content_pin()
                .world_id()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            coordinate_frame: "fixture".into(),
            records: serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
            imports: vec![],
            metadata: vec![],
        },
        state: crate::content::ProjectV2State {
            authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                .expect("qualified fixture"),
            source_identity_bindings: serde_json::from_value(v["source_identity_bindings"].clone())
                .expect("qualified fixture"),
            sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
            ..Default::default()
        },
    };
    (r, draft)
}
fn cref(name: &str) -> Ref {
    Ref {
        family: ProjectV2Family::Creature,
        key: format!("oteryn:creature.{name}"),
        revision: "definition-r1".into(),
    }
}
struct World {
    target: ExactActorRef,
    session: GameSessionId,
    missing: bool,
    missing_npc: bool,
    npc_primary: bool,
}
impl SpellWorldReader for World {
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        r.player_control_facts(self.target, self.session).ok()?;
        Some(vec![(self.target, self.session)])
    }
    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        Some(Facing::West)
    }
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        c: ExactActorRef,
        _: i32,
        _: i32,
        z: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        let p = r.read_actor_position(c).ok()?;
        if self.missing || p.context() != r.pinned_movement_context() {
            return None;
        }
        Some(z == p.position().floor)
    }
    fn combat(
        &mut self,
        r: &ChannelRuntimeV1,
        i: ExactActorRef,
        t: ExactActorRef,
        s: GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        r.player_control_facts(t, s).ok()?;
        r.read_actor_position(i).ok()?;
        if self.missing {
            return None;
        }
        Some(SpellCombatFacts {
            attack: AttackFacts {
                issuer: i,
                target: t,
                session: s,
                revision: 1,
                visible: true,
                issuer_pz: false,
                target_pz: false,
                issuer_protected: false,
                target_protected: false,
                armor: 0,
                defense: 0,
            },
            multiplier_ppm: 1_000_000,
            immune: false,
            condition_policy: None,
        })
    }
}
impl ComposedWorldReader for World {
    fn creature_combat(
        &mut self,
        r: &ChannelRuntimeV1,
        c: ExactActorRef,
        t: ExactActorRef,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<CreatureCombatFacts> {
        if self.missing
            || self.missing_npc
            || !r.contains_live_creature(c)
            || !r.contains_live_creature(t)
        {
            return None;
        }
        Some(CreatureCombatFacts {
            allowed: self.npc_primary,
            immune: false,
            armor: 0,
            multiplier_ppm: 1_000_000,
        })
    }
    fn callback_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        c: ExactActorRef,
        t: ExactActorRef,
        _: &Ref,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        if self.missing || !r.contains_live_creature(c) {
            return None;
        }
        r.read_actor_position(t).ok()?;
        Some(true)
    }
    fn non_player_side(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        if self.missing {
            return None;
        }
        r.native_summon_role(t).ok().map(|master| master.is_none())
    }
    fn top_creature(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        if self.missing {
            return None;
        }
        r.read_actor_position(t).ok()?;
        Some(true)
    }
}
fn setup(
    name: &str,
    index: usize,
    npc_name: &str,
    npc_health: u64,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ComposedSource,
    CreatureHealCatalog,
    ProfileAbilityProposal,
    World,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
    ExactActorRef,
) {
    let (mut r, draft) = fixture();
    let session = GameSessionId::decode(&[1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 8])
        .expect("qualified fixture");
    let reserved = r.reserve_fresh_session(session).expect("qualified fixture");
    let target = r.commit_fresh_session(reserved).expect("qualified fixture");
    r.initialize_first_entry_position(target)
        .expect("qualified fixture");
    let at = r
        .read_actor_position(target)
        .expect("qualified fixture")
        .position();
    let caster = cref(name);
    let health = draft
        .state
        .authoring_profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Creature(c) if p.target == caster => c.health,
            _ => None,
        })
        .expect("qualified fixture");
    let actor = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
            &caster.key,
            health as i64,
        )
        .expect("qualified fixture");
    let npc = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x - 2,
                y: at.y,
                floor: at.floor,
            },
            &cref(npc_name).key,
            npc_health as i64,
        )
        .expect("qualified fixture");
    let source =
        ComposedSource::from_native(&draft, &caster, index, [1; 32]).expect("qualified fixture");
    let catalog = CreatureHealCatalog::from_native(&draft, [1; 32]).expect("qualified fixture");
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    let atom = format!("fixture:{}", hex(&actor.placement_identity()));
    let proposal = ProfileAbilityProposal {
        issuer: actor,
        target,
        ability: source.ability.clone(),
        list: ScheduleList::Attack,
        entry_index: index,
        magnitude: source.magnitude.clone(),
        range_tiles: source.range,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!(
                "ai-profile:{}:0:attack:{index}",
                hex(&actor.placement_identity())
            ),
            revisions,
        )
        .expect("qualified fixture"),
        intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
            .expect("qualified fixture"),
    };
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            target,
            session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let b = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture");
    (
        r,
        states,
        source,
        catalog,
        proposal,
        World {
            target,
            session,
            missing: false,
            missing_npc: false,
            npc_primary: false,
        },
        fence,
        stamp,
        npc,
    )
}
#[test]
fn all16_multi_source_closures_qualify_and_secondary_substitution_refuses() {
    let (_, mut d) = fixture();
    let expected: serde_json::Value =
        serde_json::from_str(SOURCE_BODIES).expect("qualified fixture");
    for c in expected["cases"].as_array().expect("qualified fixture") {
        let s = ComposedSource::from_native(
            &d,
            &Ref {
                family: ProjectV2Family::Creature,
                key: c["creature"].as_str().expect("qualified fixture").into(),
                revision: "definition-r1".into(),
            },
            c["entry"].as_u64().expect("qualified fixture") as usize,
            [1; 32],
        )
        .expect("qualified fixture");
        assert!(!s.callbacks.is_empty());
    }
    let callback = d
        .state
        .authoring_profiles
        .iter_mut()
        .find(|p| p.target.key == "oteryn:effect.spell.frozen_minion_wave.effect-callback-2")
        .expect("qualified fixture");
    if let Data::Effect(e) = &mut callback.data {
        e.damage_type = "fire".into()
    } else {
        panic!("native effect")
    };
    assert!(matches!(
        ComposedSource::from_native(&d, &cref("frozen_minion"), 2, [1; 32]),
        Err(AttackError::InvalidSource)
    ));
}
#[test]
fn frozen_positive_ice_source_heals_real_player_and_creature_once_no_primary_damage() {
    let (mut r, mut states, source, catalog, p, mut world, fence, stamp, npc) =
        setup("frozen_minion", 2, "frozen_minion", 1000);
    states
        .apply_attack_damage(
            &mut r,
            p.target,
            world.session,
            50,
            "frozen-before-source-heal",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                ),
            ),
        )
        .expect("qualified fixture");
    let mut owner = ComposedOwner::default();
    let first = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &catalog,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1),
        )
        .expect("qualified fixture");
    assert!(first.primary.is_none());
    let heal = first
        .branches
        .iter()
        .find_map(|(_, t, x)| match x {
            Ok(BranchReceipt::PlayerHeal(h)) if *t == p.target => Some(*h),
            _ => None,
        })
        .expect("positive ICE callback heals actual player");
    assert_eq!(
        (heal.health_before, heal.health_after, heal.applied),
        (135, 185, 50)
    );
    let npc_heal = first
        .branches
        .iter()
        .find_map(|(_, t, x)| match x {
            Ok(BranchReceipt::Creature(h)) if *t == npc => Some(*h),
            _ => None,
        })
        .expect("nonplayer callback real HP");
    assert!((1200..=1700).contains(&npc_heal.health_after));
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &catalog,
                &source,
                &p,
                &mut world,
                SemanticTimeMicros::from_micros(2)
            )
            .expect("qualified fixture"),
        first
    );
    assert_eq!(
        states
            .source_player_health(&r, p.target, world.session)
            .expect("qualified fixture"),
        (185, 185, 3)
    );
    assert_eq!(
        r.read_source_creature_health(npc, "oteryn:creature.frozen_minion", 3725)
            .expect("qualified fixture"),
        npc_heal.health_after as u64
    );
}
#[test]
fn missing_callback_policy_refuses_before_primary_hp_and_raw_health_secondary_executes() {
    let (mut r, mut states, source, catalog, p, mut world, fence, stamp, npc) =
        setup("freed_soul", 0, "the_souldespoiler", 10000);
    let mut owner = ComposedOwner::default();
    world.missing = true;
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &catalog,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    assert_eq!(
        states
            .source_player_health(&r, p.target, world.session)
            .expect("qualified fixture")
            .0,
        185
    );
    assert_eq!(
        catalog
            .current_target(&r, npc)
            .expect("qualified fixture")
            .0,
        "oteryn:creature.the_souldespoiler"
    );
    world.missing = false;
    let result = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &catalog,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(2),
        )
        .expect("qualified fixture");
    let hp = result
        .branches
        .iter()
        .find_map(|(_, t, x)| match x {
            Ok(BranchReceipt::Creature(h)) if *t == npc => Some(*h),
            _ => None,
        })
        .expect("qualified fixture");
    assert!((8000..=9500).contains(&hp.health_after));
    assert_eq!(hp.health_before, 10000);
    assert!(result.primary.is_some());
}
struct RefusePlayerHeal(ChannelSpellStates);
impl PlayerLethalVitals for RefusePlayerHeal {
    fn apply_attack_damage(
        &mut self,
        r: &mut ChannelRuntimeV1,
        t: ExactActorRef,
        s: GameSessionId,
        n: u32,
        o: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerDamageReceipt> {
        self.0.apply_attack_damage(r, t, s, n, o, now)
    }
    fn source_player_health(
        &self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        s: GameSessionId,
    ) -> Option<(u32, u32, u64)> {
        self.0.source_player_health(r, t, s)
    }
}
#[test]
fn later_player_heal_refusal_retains_earlier_native_creature_receipt_and_replay() {
    let (mut r, states, source, catalog, p, mut world, fence, stamp, npc) =
        setup("frozen_minion", 2, "frozen_minion", 1000);
    let mut vitals = RefusePlayerHeal(states);
    let mut owner = ComposedOwner::default();
    let hit = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &mut vitals,
            &catalog,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1),
        )
        .expect("qualified fixture");
    assert!(
        hit.branches
            .iter()
            .any(|(_, t, x)| *t == npc && matches!(x, Ok(BranchReceipt::Creature(_))))
    );
    assert!(
        hit.branches
            .iter()
            .any(|(_, t, x)| *t == p.target && *x == Err(AttackError::MissingVitals))
    );
    assert_eq!(
        vitals
            .source_player_health(&r, p.target, world.session)
            .expect("qualified fixture")
            .0,
        185
    );
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut vitals,
                &catalog,
                &source,
                &p,
                &mut world,
                SemanticTimeMicros::from_micros(2)
            )
            .expect("qualified fixture"),
        hit
    );
}

#[test]
fn missing_independent_npc_combat_refuses_all_hp_then_primary_precedes_tile_heal() {
    let (mut r, mut states, source, catalog, p, mut world, fence, stamp, npc) =
        setup("aggressive_lava", 1, "fiery_heart", 1000);
    let mut owner = ComposedOwner::default();
    world.missing_npc = true;
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &catalog,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    assert_eq!(
        states
            .source_player_health(&r, p.target, world.session)
            .expect("qualified fixture")
            .0,
        185
    );
    assert_eq!(
        r.read_source_creature_health(npc, "oteryn:creature.fiery_heart", 7500)
            .expect("qualified fixture"),
        1000
    );
    assert!(owner.memos.is_empty());
    world.missing_npc = false;
    world.npc_primary = true;
    let hit = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &catalog,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(2),
        )
        .expect("qualified fixture");
    assert!(
        hit.primary.as_ref().expect("qualified fixture").requested > 0,
        "source fixture must exercise a positive actual primary: {hit:?}"
    );
    let primary = hit
        .branches
        .iter()
        .find_map(|(branch, t, x)| match x {
            Ok(BranchReceipt::Creature(h)) if *branch == 0 && *t == npc => Some(*h),
            _ => None,
        })
        .expect("actual elemental primary to independently admitted Creature");
    let callback = hit
        .branches
        .iter()
        .find_map(|(branch, t, x)| match x {
            Ok(BranchReceipt::Creature(h)) if *branch == 1 && *t == npc => Some(*h),
            _ => None,
        })
        .expect("actual source tile callback follows primary");
    assert_eq!(primary.health_before, 1000);
    assert_eq!(
        primary.health_after,
        1000 - i64::from(hit.primary.as_ref().expect("qualified fixture").requested)
    );
    assert_eq!(callback.health_before, primary.health_after);
    assert!(callback.health_after >= callback.health_before);
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &catalog,
                &source,
                &p,
                &mut world,
                SemanticTimeMicros::from_micros(3)
            )
            .expect("qualified fixture"),
        hit
    );
}

#[test]
fn source_formula_revision_and_catalog_pin_reject_before_any_owner_hp() {
    let (mut r, mut states, source, catalog, p, mut world, fence, stamp, npc) =
        setup("frozen_minion", 2, "frozen_minion", 1000);
    let mut owner = ComposedOwner::default();
    let mut changed = p.clone();
    changed.occurrence = crate::ability::AbilityOccurrence::new(
        p.occurrence.id().as_str(),
        crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r2",
            "sim-r1",
        )
        .expect("qualified fixture"),
    )
    .expect("qualified fixture");
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &catalog,
            &source,
            &changed,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::InvalidSource)
    );
    let (_, draft) = fixture();
    let foreign = CreatureHealCatalog::from_native(&draft, [2; 32]).expect("qualified fixture");
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &foreign,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::ContentChanged)
    );
    assert_eq!(
        states
            .source_player_health(&r, p.target, world.session)
            .expect("qualified fixture")
            .0,
        185
    );
    assert_eq!(
        r.read_source_creature_health(npc, "oteryn:creature.frozen_minion", 3725)
            .expect("qualified fixture"),
        1000
    );
    assert!(owner.memos.is_empty());
}
