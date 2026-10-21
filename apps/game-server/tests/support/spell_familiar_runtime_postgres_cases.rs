//! Familiar runtime/source cases on actual main library types and real PostgreSQL.
//! Dedicated lib registration avoids importing Spell into neutral durability test crates.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::character_familiar::{
    DurableFamiliarState, FamiliarStateOccurrence, FamiliarStateOutcome, FamiliarStateRequest,
};
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}
fn state() -> TestResult<DurableFamiliarState> {
    let state = DurableFamiliarState {
        lifecycle: None,
        selected_look: 991,
        granted_looks: vec![991],
        saved_expiry_unix: 1_800_001_800,
        last_logout_unix: 0,
        lifecycle_epoch: 1,
        familiar_definition: Some("creature:knight/familiar".into()),
        familiar_revision: Some("r20".into()),
        profile_revision: "spell-p2-r20".into(),
        cooldowns: Vec::new(),
    };
    let writer =
        crate::durability::character_familiar::FamiliarLifecycleWriter::from_fence(&fence(1)?)
            .map_err(debug)?;
    Ok(state
        .for_next_write(Some(1_800_000_000_000), &writer)
        .map_err(debug)?)
}

fn save(
    tag: u8,
    before: DurableFamiliarState,
    after: DurableFamiliarState,
) -> TestResult<FamiliarStateRequest> {
    Ok(FamiliarStateRequest {
        occurrence: FamiliarStateOccurrence::from_bytes(id(tag)).map_err(debug)?,
        before,
        after,
        content_revision: "content-1".into(),
        policy_revision: "policy-1".into(),
        policy_digest: [1; 32],
    })
}
async fn snapshot(h: &Harness) -> TestResult<(String, String, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT r.character_revision::text,p.character_revision::text,p.level,p.total_experience,(SELECT count(*) FROM game_character_familiar_receipts) FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}
/// The Channel runtime, its admitted player and the Knight familiar profile shared by the
/// familiar lifecycle cases. Explicit test content pin: activation is not claimed here.
fn knight_familiar_runtime(
    h: &Harness,
) -> TestResult<(
    crate::foundation::ChannelRuntimeV1,
    crate::foundation::ExactActorRef,
    crate::foundation::GameSessionId,
    serde_json::Value,
)> {
    use {
        crate::foundation::{ChannelId, ChannelRuntimeV1, GameSessionId, WorldId},
        oteryn_gameplay_content::content::{
            channel_content_pin::ChannelContentPin,
            creature_policy::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags},
        },
    };
    let world = WorldId::decode(&id(crate::bestiary_postgres_harness::WORLD)).map_err(debug)?;
    let mut physical = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(crate::bestiary_postgres_harness::CHANNEL)).map_err(debug)?,
        crate::foundation::NodeId::decode(h.node.fact().node_id().as_bytes()).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .map_err(debug)?;
    let session = GameSessionId::decode(&id(SESSION)).map_err(debug)?;
    let reservation = physical.reserve_fresh_session(session).map_err(debug)?;
    let actor = physical.commit_fresh_session(reservation).map_err(debug)?;
    physical
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    physical
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                [1; 32],
                vec![CompiledCreaturePolicy {
                    definition_key: "creature:knight/familiar".into(),
                    definition_revision: "r20".into(),
                    display_name: "Knight familiar".into(),
                    maximum_health: 812,
                    base_speed: 220,
                    outfit_look_type: 991,
                    object_look_type: None,
                    summonable: false,
                    convinceable: false,
                    mana_cost: None,
                    is_familiar: true,
                    condition_immunities: Vec::new(),
                    preferred_distance: Some(1),
                    reward_boss: Some(false),
                    defense: Some(1),
                    armor: Some(10),
                    mitigation: None,
                    resistances: Vec::new(),
                    damage_immunities: Vec::new(),
                    healing_from_damage: Vec::new(),
                    flags: CreatureFlags {
                        attackable: true,
                        illusionable: false,
                        health_hidden: false,
                    },
                }],
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))?;
    let profile = catalog["profiles"]
        .as_array()
        .ok_or("profiles")?
        .iter()
        .find(|v| v["name"] == "Knight familiar")
        .ok_or("Knight familiar")?;
    Ok((physical, actor, session, profile.clone()))
}
fn knight_familiar_spell(
    profile: &serde_json::Value,
) -> TestResult<crate::spell::native::CompiledNativeSpell> {
    Ok(crate::spell::native::spell_from_bundle(
        &serde_json::json!({"spell":profile["spell"]}),
        &profile["dependencies"],
    )
    .map_err(debug)?)
}
#[test]
fn source_familiar_look_and_genuine_lethal_cleanup_require_matching_durable_receipts() -> TestResult
{
    run("familiar_hook_receipt", async |h| {
        use crate::ability::{AbilityOccurrence, RevisionSet};
        use crate::foundation::owner_timer::SemanticTimeMicros;
        use crate::foundation::{CommandId, CommandRef};
        use crate::spell::chain::TilePosition;
        use crate::spell::companion_lifecycle::{
            FamiliarOwnerFacts, commit_familiar, prepare_familiar_hook,
        };
        use crate::spell::delayed_execution::CastBinding;
        use oteryn_gameplay_spell_rules::native_companions::FamiliarEvent;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let now = h
            .root
            .read_familiar_unix_time(&authority)
            .await
            .map_err(debug)?;
        let (mut physical, actor, session, profile) = knight_familiar_runtime(h)?;
        let native = knight_familiar_spell(&profile)?;
        let cell = TilePosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        let binding = CastBinding {
            spell: native,
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(&id(CHARACTER)).map_err(debug)?,
            command: CommandRef::new(session, CommandId::new(110).map_err(debug)?),
            occurrence: AbilityOccurrence::new(
                "test:familiar/advance",
                RevisionSet::new("rules-1", "content-1", "policy-1", "formula-1", "sim-1")
                    .map_err(debug)?,
            )
            .map_err(debug)?,
            parent_binding: Vec::new(),
            cast_at: SemanticTimeMicros::from_micros(0),
            cast_position: cell,
            cast_snapshot: None,
        };
        // Source Advance grants the vocation look when premium even below level 200.
        let current = FamiliarOwnerFacts {
            vocation: "knight".into(),
            premium: true,
            level: 50,
            account_at_least_god: false,
            current_speed: 220,
            familiar_minutes: 30,
            vip: false,
            vip_reduction_minutes: 0,
            cooldown_rate: 1.0,
            state: DurableFamiliarState::default(),
            checkpoint_committed_at_ms: None,
            lifecycle_writer:
                crate::durability::character_familiar::FamiliarLifecycleWriter::from_fence(&fence(
                    1,
                )?)
                .map_err(debug)?,
            lever_boss_room: crate::spell::companion_lifecycle::LeverBossRoomMembership::Outside,
        };
        let prepared = prepare_familiar_hook(
            &physical,
            binding,
            FamiliarEvent::Advance,
            &current,
            now,
            None,
        )
        .map_err(debug)?;
        assert!(!prepared.creates_creature());
        assert_eq!(prepared.after().selected_look, 991);
        assert_eq!(prepared.after().granted_looks, vec![991]);
        assert_eq!(prepared.caster_mana_cost().map_err(debug)?, 0);
        assert!(
            prepared
                .schedules(SemanticTimeMicros::from_micros(0), 0)
                .map_err(debug)?
                .is_empty()
        );
        let request = prepared
            .state_request(
                FamiliarStateOccurrence::from_bytes(id(111)).map_err(debug)?,
                "content-1".into(),
                "policy-1".into(),
                [1; 32],
            )
            .ok_or("state request")?;
        assert!(
            commit_familiar(
                &mut physical,
                prepared.clone(),
                &current,
                &fence(1)?,
                Some(&request),
                None
            )
            .is_err()
        );
        assert!(
            physical
                .owned_companions(actor, session)
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(snapshot(h).await?.0, "1");
        let outcome = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(1)?, request.clone())
            .await
            .map_err(debug)?;
        let FamiliarStateOutcome::Committed(receipt) = outcome else {
            return Err("fresh familiar receipt".into());
        };
        let mut changed = current.clone();
        changed.premium = false;
        assert!(
            commit_familiar(
                &mut physical,
                prepared.clone(),
                &changed,
                &fence(1)?,
                Some(&request),
                Some(&receipt)
            )
            .is_err()
        );
        let mut wrong = request.clone();
        wrong.policy_digest = [2; 32];
        assert!(
            commit_familiar(
                &mut physical,
                prepared.clone(),
                &current,
                &fence(1)?,
                Some(&wrong),
                Some(&receipt)
            )
            .is_err()
        );
        assert!(
            physical
                .owned_companions(actor, session)
                .map_err(debug)?
                .is_empty()
        );
        let applied = commit_familiar(
            &mut physical,
            prepared,
            &current,
            &fence(1)?,
            Some(&request),
            Some(&receipt),
        )
        .map_err(debug)?;
        assert_eq!(applied.after(), &request.after);
        assert_eq!(applied.creature(), None);
        assert!(applied.owner_operations().is_empty());
        assert_eq!(
            h.root
                .read_character_familiar_state(
                    &authority,
                    CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?
                )
                .await
                .map_err(debug)?
                .state(),
            applied.after()
        );

        // Explicit isolated test setup: a real typed durable snapshot and actual
        // fixed-slot familiar. This does not claim a live Cast or XP producer.
        let source_state = state()?;
        h.root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence(2)?,
                save(112, applied.after().clone(), source_state.clone())?,
            )
            .await
            .map_err(debug)?;
        let mut spawn = physical
            .prepare_companion_spawn(
                actor,
                session,
                "Knight familiar",
                crate::foundation::MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                Some(source_state.saved_expiry_unix),
                source_state.lifecycle_epoch,
                0,
                false,
            )
            .map_err(debug)?;
        spawn.bind_semantic_creation(0).map_err(debug)?;
        let dead = physical.install_companion_spawn(spawn).map_err(debug)?;
        physical
            .borrow_exact_actor_commit()
            .commit_damage(
                dead,
                crate::foundation::OwnerDamageCommand {
                    target: b"creature:knight/familiar",
                    occurrence: b"test:familiar-lethal:1",
                    binding: b"test:familiar-lethal:1\0source-qualified-damage:812",
                    damage: 812,
                },
            )
            .map_err(debug)?;
        let lethal = physical
            .borrow_combat_death()
            .committed_lethal_receipt(dead)
            .map_err(debug)?;
        let projection = physical
            .borrow_combat_death()
            .project_committed_lethal(lethal)
            .map_err(debug)?
            .clone();
        let native = knight_familiar_spell(&profile)?;
        let stamp = physical.issue_owner_work().map_err(debug)?;
        let binding =
            crate::spell::companion_lifecycle::FamiliarLifecycleBinding::from_committed_lethal(
                &physical,
                native,
                actor,
                fence(3)?,
                stamp,
                SemanticTimeMicros::from_micros(1_000),
                &projection,
            )
            .map_err(debug)?;
        let durable_source = h
            .root
            .read_character_familiar_state(
                &authority,
                CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        // Source Death does not read Premium/level/config benefit inputs. The
        // genuine physical lethal receipt, matching owner and durable state do.
        let current = FamiliarOwnerFacts {
            premium: false,
            level: 0,
            familiar_minutes: 0,
            cooldown_rate: 0.0,
            state: source_state,
            checkpoint_committed_at_ms: durable_source.checkpoint_committed_at_ms(),
            lifecycle_writer:
                crate::durability::character_familiar::FamiliarLifecycleWriter::from_fence(&fence(
                    3,
                )?)
                .map_err(debug)?,
            ..current
        };
        let prepared = crate::spell::companion_lifecycle::prepare_lifecycle_familiar(
            &physical, None, binding, &current, now,
        )
        .map_err(debug)?;
        let request = prepared
            .state_request(
                FamiliarStateOccurrence::from_bytes(id(113)).map_err(debug)?,
                "content-1".into(),
                "policy-1".into(),
                [1; 32],
            )
            .ok_or("death state request")?;
        assert!(
            crate::spell::companion_lifecycle::commit_lifecycle_familiar(
                &mut physical,
                prepared.clone(),
                &current,
                &fence(3)?,
                Some(&request),
                None,
            )
            .is_err()
        );
        assert_eq!(
            physical
                .dead_owned_familiars(actor, session)
                .map_err(debug)?,
            vec![dead]
        );
        let outcome = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(3)?, request.clone())
            .await
            .map_err(debug)?;
        let FamiliarStateOutcome::Committed(receipt) = outcome else {
            return Err("fresh death receipt".into());
        };
        // The independently current successor fence is mandatory even with a
        // genuine receipt. Failure leaves the actual dead slot untouched.
        assert!(
            crate::spell::companion_lifecycle::commit_lifecycle_familiar(
                &mut physical,
                prepared.clone(),
                &current,
                &fence(3)?,
                Some(&request),
                Some(&receipt),
            )
            .is_err()
        );
        assert_eq!(
            physical
                .dead_owned_familiars(actor, session)
                .map_err(debug)?,
            vec![dead]
        );
        let applied = crate::spell::companion_lifecycle::commit_lifecycle_familiar(
            &mut physical,
            prepared.clone(),
            &current,
            &fence(4)?,
            Some(&request),
            Some(&receipt),
        )
        .map_err(debug)?;
        assert_eq!(applied.after(), &request.after);
        assert_eq!(applied.after().saved_expiry_unix, now.seconds());
        assert!(
            physical
                .dead_owned_familiars(actor, session)
                .map_err(debug)?
                .is_empty()
        );
        assert!(physical.companion_snapshot_including_dead(dead).is_err());
        // The death owner's immutable projection remains available for reward
        // history after retirement. It does not restore the physical familiar;
        // replaying its lifecycle below still fails the exact slot predecessor.
        let historical = physical
            .borrow_combat_death()
            .projected_death(dead)
            .map_err(debug)?;
        assert_eq!(historical.0, projection.occurrence().death_key());
        assert!(
            crate::spell::companion_lifecycle::commit_lifecycle_familiar(
                &mut physical,
                prepared,
                &current,
                &fence(4)?,
                Some(&request),
                Some(&receipt),
            )
            .is_err()
        );
        assert_eq!(snapshot(h).await?.0, "4");
        Ok(())
    })
}

/// #2199 Codex P2 4302737307: a due familiar expiry closes the durable row (remaining 0, as a
/// death) through the fenced lifecycle writer before the live familiar is despawned. With no
/// durable receipt, or a stale successor fence, the familiar stays present.
#[test]
fn familiar_expiry_closes_the_durable_row_before_the_despawn() -> TestResult {
    run("familiar_expiry_close", async |h| {
        familiar_end_closes_the_durable_row_before_the_despawn(h, false).await
    })
}

/// #2199 Codex P1 4303261095 (FAMILIARS0 §7: owner dies, removed, 0): the owner's death ends
/// its live familiar exactly as an expiry does, through the fenced lifecycle writer (row
/// closed, remaining 0) before the despawn; the familiar stays present until that commit.
#[test]
fn familiar_owner_death_closes_the_durable_row_before_the_despawn() -> TestResult {
    run("familiar_owner_death_close", async |h| {
        familiar_end_closes_the_durable_row_before_the_despawn(h, true).await
    })
}

async fn familiar_end_closes_the_durable_row_before_the_despawn(
    h: &Harness,
    owner_death: bool,
) -> TestResult {
    {
        use crate::durability::character_familiar::{FamiliarLifecycleV2, FamiliarLifecycleWriter};
        use crate::foundation::owner_timer::SemanticTimeMicros;
        use crate::spell::companion_lifecycle::{
            FamiliarLifecycleBinding, FamiliarOwnerFacts, LeverBossRoomMembership,
            commit_lifecycle_familiar, prepare_lifecycle_familiar,
        };
        use oteryn_gameplay_spell_rules::native_companions::FamiliarAction;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let now = h
            .root
            .read_familiar_unix_time(&authority)
            .await
            .map_err(debug)?;
        let (mut physical, actor, session, profile) = knight_familiar_runtime(h)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        // Explicit isolated setup: an open row of this session with 900 s left and its live
        // familiar. This does not claim a live Cast producer.
        let writer = FamiliarLifecycleWriter::from_fence(&fence(1)?).map_err(debug)?;
        let mut open = state()?;
        open.lifecycle = Some(FamiliarLifecycleV2 {
            version: 2,
            familiar_remaining_ms: 900_000,
            open: true,
            writer,
            clean_end_game_session_id: None,
        });
        h.root
            .commit_character_familiar_state(
                &authority,
                &h.node,
                fence(1)?,
                save(120, DurableFamiliarState::default(), open.clone())?,
            )
            .await
            .map_err(debug)?;
        let durable = h
            .root
            .read_character_familiar_state(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(durable.state(), &open);
        let mut spawn = physical
            .prepare_companion_spawn(
                actor,
                session,
                "Knight familiar",
                crate::foundation::MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                Some(open.saved_expiry_unix),
                open.lifecycle_epoch,
                0,
                false,
            )
            .map_err(debug)?;
        spawn.bind_semantic_creation(0).map_err(debug)?;
        let familiar = physical.install_companion_spawn(spawn).map_err(debug)?;
        let at = SemanticTimeMicros::from_micros(900_000_000);
        let stamp = physical.issue_owner_work().map_err(debug)?;
        let death = crate::durability::character_death::PlayerDeathOccurrence::from_bytes(id(122))
            .map_err(debug)?;
        let issue = |epoch: u64| -> TestResult<FamiliarLifecycleBinding> {
            let spell = knight_familiar_spell(&profile)?;
            let issued = if owner_death {
                FamiliarLifecycleBinding::from_owner_death(
                    &physical,
                    spell,
                    actor,
                    fence(2)?,
                    stamp,
                    at,
                    death,
                    familiar,
                    epoch,
                )
            } else {
                FamiliarLifecycleBinding::from_due_expiry(
                    &physical,
                    spell,
                    actor,
                    fence(2)?,
                    stamp,
                    at,
                    familiar,
                    epoch,
                )
            };
            issued.map_err(|e| debug(e).into())
        };
        // Another lifecycle epoch never issues this end.
        assert!(issue(open.lifecycle_epoch + 1).is_err());
        let binding = issue(open.lifecycle_epoch)?;
        // An expiry reads no Premium/level/config benefit input, as a death does.
        let current = FamiliarOwnerFacts {
            vocation: "knight".into(),
            premium: false,
            level: 0,
            account_at_least_god: false,
            current_speed: 220,
            familiar_minutes: 0,
            vip: false,
            vip_reduction_minutes: 0,
            cooldown_rate: 0.0,
            state: durable.state().clone(),
            checkpoint_committed_at_ms: durable.checkpoint_committed_at_ms(),
            lifecycle_writer: FamiliarLifecycleWriter::from_fence(&fence(2)?).map_err(debug)?,
            lever_boss_room: LeverBossRoomMembership::Outside,
        };
        let prepared =
            prepare_lifecycle_familiar(&physical, None, binding, &current, now).map_err(debug)?;
        let request = prepared
            .state_request(
                FamiliarStateOccurrence::from_bytes(id(121)).map_err(debug)?,
                "content-1".into(),
                "policy-1".into(),
                [1; 32],
            )
            .ok_or("expiry state request")?;
        assert_eq!(
            request.after.lifecycle,
            Some(FamiliarLifecycleV2 {
                familiar_remaining_ms: 0,
                open: false,
                ..open.lifecycle.clone().ok_or("open lifecycle")?
            })
        );
        assert_eq!(request.after.lifecycle_epoch, open.lifecycle_epoch);
        // No durable receipt: the familiar stays present and the row stays open.
        assert!(
            commit_lifecycle_familiar(
                &mut physical,
                prepared.clone(),
                &current,
                &fence(2)?,
                Some(&request),
                None,
            )
            .is_err()
        );
        assert!(physical.companion_snapshot(familiar).is_ok());
        let outcome = h
            .root
            .commit_character_familiar_state(&authority, &h.node, fence(2)?, request.clone())
            .await
            .map_err(debug)?;
        let FamiliarStateOutcome::Committed(receipt) = outcome else {
            return Err("fresh expiry receipt".into());
        };
        // The independently current successor fence is mandatory even with the receipt.
        assert!(
            commit_lifecycle_familiar(
                &mut physical,
                prepared.clone(),
                &current,
                &fence(2)?,
                Some(&request),
                Some(&receipt),
            )
            .is_err()
        );
        assert!(physical.companion_snapshot(familiar).is_ok());
        let applied = commit_lifecycle_familiar(
            &mut physical,
            prepared,
            &current,
            &fence(3)?,
            Some(&request),
            Some(&receipt),
        )
        .map_err(debug)?;
        assert_eq!(applied.after(), &request.after);
        assert_eq!(applied.creature(), None);
        assert_eq!(
            applied.owner_operations(),
            &[FamiliarAction::ResetWarningHandles]
        );
        assert!(physical.companion_snapshot(familiar).is_err());
        assert!(
            physical
                .owned_companions(actor, session)
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(
            h.root
                .read_character_familiar_state(&authority, character)
                .await
                .map_err(debug)?
                .state(),
            &request.after
        );
        assert_eq!(snapshot(h).await?.0, "3");
        Ok(())
    }
}
