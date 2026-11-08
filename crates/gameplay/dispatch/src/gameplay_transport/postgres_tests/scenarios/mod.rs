mod s1_hunt;
mod s2_economy;
mod s3_death;
mod s4_store;
mod s5_disconnect;

use super::full_world_support::*;
use crate::gameplay_transport::connection::{AdmittedSession, SessionContinuity};

fn timed(name: &'static str, scenario: fn() -> TestResult) -> TestResult {
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, scenario);
    eprintln!(
        "XMECH-E2E-1 {name} duration={:.3}s result={}",
        start.elapsed().as_secs_f64(),
        if result.is_ok() { "ok" } else { "failed" }
    );
    result
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_array(\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_container_entries i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_character_inbox_locations i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_character_depot_locations i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_ground_locations i),\
        (SELECT jsonb_agg(jsonb_build_array(character_id,total_experience,level) ORDER BY character_id) FROM game_character_progression_state),\
        (SELECT jsonb_agg(jsonb_build_array(character_id,stamina_minutes) ORDER BY character_id) FROM game_character_build_state),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY account_id) FROM game_account_bank_balances i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY offer_id) FROM game_market_offers i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_market_escrow_locations i),\
        (SELECT jsonb_agg(to_jsonb(i) ORDER BY character_id) FROM game_character_forge_dust i))::text")
        .fetch_one(&h.pool).await?)
}

async fn count(h: &Harness, query: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(query).fetch_one(&h.pool).await?)
}

fn command(admitted: AdmittedSession, command_id: u64) -> UseCommand {
    UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    }
}

/// Each call is a new connection with the previous connection's continuity. The owner, not
/// the test, sequences CommandIds. Value replays below also call the durable route directly.
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ty: u32,
    payload: &[u8],
) -> TestResult<(oteryn_protocol_oteryn::CommandStatus, Vec<u8>)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let frame = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type: ty,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(next)
        | ConnectionEnd::AdmittedThenClosed(next, _) => *admitted = next,
        other => return Err(format!("scenario connection: {other:?}").into()),
    }
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut cursor = bytes.as_slice();
    let mut result = None;
    while !cursor.is_empty() {
        let len = u32::from_be_bytes(cursor.get(..4).ok_or("length")?.try_into()?) as usize;
        let frame = cursor.get(4..4 + len).ok_or("frame")?;
        cursor = &cursor[4 + len..];
        let env = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
        if env.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let reply =
                oteryn_protocol_oteryn::decode_command_result(env.payload()).map_err(debug)?;
            assert!(result.is_none());
            result = Some((reply.status, reply.payload.to_vec()));
        }
    }
    result.ok_or_else(|| "missing command result".into())
}

async fn logout_relogin(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<AdmittedSession> {
    let before = snapshot(h).await?;
    let at = position(owner, admitted.runtime_actor.ok_or("actor")?).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(120))
        .ok_or("clock")?;
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    assert_eq!(snapshot(h).await?, before, "logout conserves durable value");
    admit_successor(h, owner, admitted, at, before).await
}

async fn admit_successor(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    at: MovementLocalPosition,
    before: String,
) -> TestResult<AdmittedSession> {
    let character = *admitted
        .item_fence
        .ok_or("previous fence")?
        .character_id
        .as_bytes();
    let session = if character == id(41) {
        relogin(h).await?
    } else {
        relogin_as(
            h,
            character,
            crate::durability::guild_chat_fixture::sid(90, 2),
            id(111),
        )
        .await?
    };
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(session)
        .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("successor fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        actor
    };
    let gameplay = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "successor gameplay")?
        .ok_or("successor")?;
    let decided = owner
        .read_admission_position(gameplay)
        .await
        .map_err(|()| "position")?;
    {
        let mut runtime = owner.runtime.lock().await;
        owner
            .initialize_admitted_position(&mut runtime, actor, session, &decided)
            .await
            .map_err(debug)?;
    }
    assert!(owner.track_restored(session, actor, &decided, gameplay));
    let mut next = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        item_fence: Some(fence),
        continuity: SessionContinuity::FRESH,
        ..admitted
    };
    owner.admit_progression_session(&mut next).await;
    owner.admit_loop_training(&mut next).await;
    assert_eq!(position(owner, actor).await?, at);
    assert_eq!(
        snapshot(h).await?,
        before,
        "relogin conserves durable value"
    );
    Ok(next)
}

async fn at_locker(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    while position(owner, actor).await?.x > LOCKER_CELL.x {
        step_west(owner, actor, admitted.game_session_id).await?;
    }
    assert_eq!(position(owner, actor).await?, LOCKER_CELL);
    Ok(())
}

async fn deposit(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    item: [u8; 16],
    id: u64,
) -> TestResult {
    let intent = DepotIntent {
        town: Some(0),
        box_number: DEPOT_BOX,
        page: 0,
        action: DepotAction::Deposit(std::num::NonZeroU64::new(1).ok_or("handle")?),
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .depot(
                actor,
                command(admitted, id),
                intent,
                Some(ItemKey::Instance(item))
            )
            .await
            .ok_or("deposit")?
            .reply
            .outcome,
        DepotOutcome::Moved
    );
    let after = snapshot(h).await?;
    assert_eq!(
        owner
            .depot(actor, command(admitted, id), intent, None)
            .await
            .ok_or("deposit replay")?
            .reply
            .outcome,
        DepotOutcome::Moved
    );
    assert_eq!(snapshot(h).await?, after);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(n, 1);
    Ok(())
}

/// A second historical Character root receives a genuine independently verified Fresh session.
/// The shared verifier and publications own every claim; commands retain all production guards.
async fn peer(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    base: AdmittedSession,
) -> TestResult<AdmittedSession> {
    use crate::durability::guild_chat_fixture::{historical_character, sid};
    let character = crate::domain::CharacterId::from_bytes(sid(90, 1)).map_err(debug)?;
    historical_character(&h.pool, 90, character, sid(90, 2), id(42), "Scenario Peer").await?;
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityPublicationV1,
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let template = super::fresh_fixture::fresh::Source::new(now)?;
    let current = crate::foundation::fnd04_verifier::FreshCurrentEvidence {
        account_id: crate::gameplay_transport::canonical_uuid(&sid(90, 2)),
        character_id: crate::foundation::CharacterId::decode(character.as_bytes())
            .map_err(debug)?,
        ..template.current
    };
    let mut source = super::fresh_fixture::fresh::Source::with_current(now, current.clone())?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let mut publications = super::fresh_fixture::fresh::Source::with_current(now, current)?;
    publications
        .rows
        .retain(|r| matches!(r.key, Key::Account { .. } | Key::Character(_)));
    for row in &mut source.rows {
        if matches!(row.key, Key::Runtime(_) | Key::SigningTrust { .. }) {
            *row = guards
                .load(std::slice::from_ref(&row.key))
                .await?
                .into_iter()
                .next()
                .flatten()
                .ok_or("shared publication")?;
            if matches!(row.key, Key::SigningTrust { .. }) {
                row.precondition=crate::foundation::admission_authority_publication::AdmissionPublicationPreconditionV1::CompareAndSet{expected_publication_revision:row.publication_revision};
                row.publication_revision += 1;
                row.source.source_revision += 1;
                row.source.decision_identity =
                    format!("xmech-peer-trust-{}", row.source.source_revision);
                row.source.source_observed_at = now;
                publications.rows.push(row.clone());
            }
        }
    }
    guards
        .publish(&AdmissionAuthorityPublicationV1::prepare(&publications, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    let peer_session = GameSessionId::decode(&id(70)).map_err(debug)?;
    let transport =
        crate::foundation::AuthenticatedTransportRefV1::decode(&id(71)).map_err(debug)?;
    let grant = source.request_with_nonce(
        peer_session,
        transport,
        Some((
            &source.current.account_id,
            &crate::gameplay_transport::canonical_uuid(character.as_bytes()),
        )),
        now,
        Some([90; 32]),
    )?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&grant).await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let current = store.current_session(peer_session).await?;
    assert_eq!(current.current_character_lease().generation(), 2);
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(character.as_bytes().as_slice()).execute(&h.pool).await?;

    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 9,
                    y: 1,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                fence.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(character.as_bytes()).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    fence.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Knight,
                        // Match the leader's level-50 combat fixture: the weakest
                        // fist hit must penetrate Rat defense/armor/mitigation.
                        // Durable progression remains owned by admission below.
                        level: 50,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: fence.game_session_id,
        runtime_actor: Some(actor),
        item_fence: Some(fence),
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport,
            account_id: sid(90, 2),
        }),
        continuity: SessionContinuity::FRESH,
        ..base
    };
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_loop_training(&mut admitted).await;
    let gameplay = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "peer fence")?
        .ok_or("peer current")?;
    assert!(owner.track_restored(
        admitted.game_session_id,
        actor,
        &AdmissionPosition::FirstLogin,
        gameplay
    ));
    Ok(admitted)
}

async fn join_party(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: AdmittedSession,
    other: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::party_command::SessionPartyView;
    use oteryn_protocol_oteryn::party::*;
    let offered = owner.offered_capabilities();
    let mut leader = leader;
    let mut other = other;
    for admitted in [&mut leader, &mut other] {
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6, CAPABILITY_PARTY_V1])
                .ok_or("party capability")?;
    }
    let target = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        let p = visible
            .players
            .iter()
            .find(|p| Some(p.actor) == other.runtime_actor)
            .ok_or("peer visible")?;
        EntityRef {
            identity: p.actor.placement_identity(),
            generation: p.generation,
        }
    };
    let mut a = SessionPartyView::default();
    let mut b = SessionPartyView::default();
    let invite = encode_party_intent(PartyIntent::Invite(target)).map_err(debug)?;
    let bytes = owner
        .party_wire_command(leader, 800, COMMAND_TYPE_PARTY_INTENT, &invite, &mut a)
        .await
        .ok_or("invite")?;
    assert_eq!(
        decode_party_result(&bytes).map_err(debug)?.disposition,
        PartyDisposition::Ok
    );
    assert_eq!(
        count(h, "SELECT count(*) FROM game_party_invitations").await?,
        1
    );
    let query = owner
        .party_wire_command(other, 800, COMMAND_TYPE_PARTY_QUERY, &[], &mut b)
        .await
        .ok_or("party query")?;
    let invitation = decode_party_result(&query)
        .map_err(debug)?
        .invitations
        .first()
        .ok_or("invitation")?
        .handle;
    let accept = encode_party_intent(PartyIntent::Accept(invitation)).map_err(debug)?;
    let bytes = owner
        .party_wire_command(other, 801, COMMAND_TYPE_PARTY_INTENT, &accept, &mut b)
        .await
        .ok_or("accept")?;
    assert_eq!(
        decode_party_result(&bytes).map_err(debug)?.disposition,
        PartyDisposition::Ok
    );
    assert_eq!(
        count(h, "SELECT count(*) FROM game_party_members").await?,
        2
    );
    Ok(())
}

async fn kill_rat(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    other: AdmittedSession,
    pin: ChannelContentPin,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let rat = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .realize_native_qualification_spawn(pin, "oteryn:creature.rat", RAT_CELL)
            .map_err(debug)?;
        let c = *runtime.visible_entities().creatures.first().ok_or("rat")?;
        EntityRef {
            identity: c.actor.placement_identity(),
            generation: c.generation,
        }
    };
    // Qualify the other member with real nonlethal damage before the lethal swing.
    let creature = {
        let r = owner.runtime.lock().await;
        r.visible_entities().creatures.first().ok_or("rat")?.actor
    };
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .set_test_creature_health(creature, 1000)
            .map_err(debug)?;
        // ATTACK-0 / TibiaTools damage.ts:13-25 bounds precede Canary
        // 99902524 creature.cpp:958-1007 defense, armor and mitigation. Establish positive
        // worst-case damage from the actual qualified target and live peer facts;
        // more RNG attempts cannot repair an underpowered fixture.
        {
            use crate::combat_attack::{
                AttackConstants, FightMode, armor_reduction_bounds, player_fist_formula,
            };
            let constants = AttackConstants::checked_in().ok_or("attack constants")?;
            let states = owner.states.spell_states.lock().await;
            let peer = states
                .get(
                    &runtime,
                    other.runtime_actor.ok_or("peer")?,
                    other.game_session_id,
                )
                .ok_or("peer spell state")?;
            let facts = peer.character_facts();
            let skill = peer
                .swing_skills(0, owner.owner_now().get())
                .ok_or("peer swing skills")?
                .map_or(constants.starting_skill, |skills| skills.attack);
            let (minimum, _) = player_fist_formula(constants)
                .bounds(&crate::spell::formula::FormulaInputs {
                    level: facts.level,
                    magic_level: facts.magic_level,
                    base_power: None,
                    attack_skill: skill,
                    attack_value: constants.fist.attack,
                    attack_factor: constants.fight_mode(FightMode::Balanced).attack_factor,
                    shielding_skill: 10,
                    shield_defense: None,
                })
                .map_err(debug)?;
            let target = runtime.companion_snapshot(creature).map_err(debug)?;
            let policy = &target.state.policy;
            let defense = policy.defense.ok_or("Rat defense")?;
            let (_, maximum_armor) =
                armor_reduction_bounds(constants, policy.armor.ok_or("Rat armor")?);
            let after_blocks = minimum
                .saturating_sub(i64::from(defense))
                .saturating_sub(i64::from(maximum_armor))
                .max(0);
            assert!(
                oteryn_gameplay_combat_rules::resolution::creature_physical_response(
                    policy,
                    after_blocks
                )
                .ok_or("Rat physical response")?
                    > 0,
                "the peer's minimum real fist hit must penetrate the qualified Rat defenses"
            );
        }
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms().gate_auto_attack(),
                other.runtime_actor.ok_or("peer actor")?,
                other.game_session_id,
                owner.owner_now(),
                Some(rat),
                1
            ),
            AttackIntentDisposition::Ok
        );
    }
    for pass in 0..32 {
        owner
            .refresh_attack_weapon(other.runtime_actor.ok_or("peer")?, other.game_session_id)
            .await;
        owner.drain_auto_attacks().await;
        if owner
            .runtime
            .lock()
            .await
            .live_creature_health(creature)
            .ok_or("health")?
            < 1000
        {
            break;
        }
        assert!(pass < 31, "peer lands a qualifying hit");
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    {
        let mut runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.set_target(
            &runtime,
            owner.rooms().gate_auto_attack(),
            other.runtime_actor.ok_or("peer")?,
            other.game_session_id,
            owner.owner_now(),
            None,
            2,
        );
        runtime
            .set_test_creature_health(creature, 20)
            .map_err(debug)?;
    }
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms().gate_auto_attack(),
                actor,
                admitted.game_session_id,
                owner.owner_now(),
                Some(rat),
                1
            ),
            AttackIntentDisposition::Ok
        );
    }
    for pass in 0..128 {
        owner
            .refresh_attack_weapon(actor, admitted.game_session_id)
            .await;
        owner.drain_auto_attacks().await;
        if owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(admitted.game_session_id)
        {
            break;
        }
        assert!(pass < 127, "rat must die");
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    for pass in 0..8 {
        if owner.drain_kill_rewards(admitted.game_session_id).await == DrainOutcome::Drained {
            return Ok(());
        }
        assert!(pass < 7, "kill must settle");
    }
    Err("kill drain exhausted".into())
}

async fn mint_carried(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    key: &str,
    quantity: u32,
    seed: u8,
) -> TestResult<[u8; 16]> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let definition = crate::gameplay_transport::item_look::typed_definition(
        index,
        index
            .definition_ref("Item", key, "definition-r1")
            .ok_or("definition")?,
    )
    .ok_or("typed")?;
    let backpack = crate::gameplay_transport::item_look::typed_definition(
        index,
        index
            .definition_ref("Item", BACKPACK, "definition-r1")
            .ok_or("pack")?,
    )
    .ok_or("pack typed")?;
    let item_facts = generation_item_facts(native, &definition)
        .map_err(|e| format!("mint facts {key}: {e:?}"))?;
    let backpack_facts = generation_item_facts(native, &backpack).map_err(debug)?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let mut mint =
        h.root
            .freeze_item_mint(
                owner.character,
                &h.node,
                ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        admitted.world_id,
                        admitted.channel_id,
                        fence.scope_ownership_generation,
                        u32::from(seed),
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:xmech.loot".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:purpose.xmech.loot".into(),
                        0,
                    ),
                    item: definition,
                    quantity,
                    ground: GroundPlacement {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                10, 0, -7,
                            ),
                        corpse_ref: id(seed).to_vec(),
                        map_revision: "map-1".into(),
                        content_revision: "content-1".into(),
                        native_room_placement_context: vec![1],
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("mint lane")?;
    let ItemMintOutcome::Committed(item) = h
        .root
        .commit_item_mint(&permit, owner.character, &h.node, &mut mint)
        .await
        .map_err(debug)?
    else {
        return Err("mint".into());
    };
    let id = item.item_instance_id;
    let mut transfer = h
        .root
        .freeze_item_transfer(
            owner.character,
            &h.node,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(1000 + u64::from(seed)).map_err(debug)?,
                ),
                source_item_instance_id: id,
                destination: ItemTransferDestination::MainBackpack,
                item: item_facts,
                backpack: Some(backpack_facts),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    h.root
        .commit_item_transfer(
            &permit,
            owner.character,
            &h.node,
            fence,
            &mut transfer,
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    Ok(id)
}

async fn merchant(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    key: &str,
) -> TestResult<(
    EntityRef,
    oteryn_protocol_oteryn::npc_service::NpcTradeWindow,
)> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let npc = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: key.into(),
                        placements: vec![NpcPlacement {
                            floor: -7,
                            x: 9,
                            y: 1,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        let n = *runtime.visible_entities().npcs.last().ok_or("npc")?;
        EntityRef {
            identity: n.actor.placement_identity(),
            generation: n.generation,
        }
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let mut window = None;
    for (id, text) in [(20, "hi"), (21, "trade")] {
        let result = owner
            .npc_talk(
                actor,
                command(admitted, id),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(result.disposition, NpcIntentDisposition::Ok);
        window = result.window;
    }
    Ok((npc, window.unwrap_or_default()))
}

/// Drop a real socket after SQL proves the mutation committed, before consuming its reply.
/// This gives the successor an unknown acknowledgement, not a fabricated session-row change.
async fn lost_reply(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    ty: u32,
    payload: &[u8],
    committed: &'static str,
) -> TestResult<AdmittedSession> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let dropping = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if oteryn_protocol_oteryn::decode_wire_envelope(&frame)
                .map_err(debug)?
                .message_type()
                == oteryn_protocol_oteryn::MessageType::SnapshotCommit
            {
                break;
            }
        }
        let frame = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: admitted.continuity.next_command_id,
                command_type: ty,
                payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &frame).await?;
        let committed_wait = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if count(h, committed).await? == 1 {
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        match committed_wait {
            Ok(result) => result?,
            Err(error) => {
                let diagnostic = if ty == oteryn_protocol_oteryn::market::COMMAND_TYPE_MARKET {
                    owner
                        .market_command(admitted, admitted.continuity.next_command_id, payload)
                        .await
                } else {
                    None
                };
                return Err(format!(
                    "lost reply receipt wait: {error}; market diagnostic={diagnostic:?}"
                )
                .into());
            }
        }
        drop(client);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, dropped) = super::fixture::join_two(serving, dropping).await;
    dropped?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(next)
        | ConnectionEnd::AdmittedThenClosed(next, _) => Ok(next),
        other => Err(format!("lost-reply connection: {other:?}").into()),
    }
}

fn market_ware(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(TypedDefinitionRef, u32, u32)> {
    let index = native.item_index().ok_or("index")?;
    for reference in 1..=100_000 {
        let r = std::num::NonZeroU32::new(reference).ok_or("ref")?;
        let Some((key, revision)) = index.definition_at(r) else {
            break;
        };
        let Some(policy) = native.item_policy(key, revision) else {
            continue;
        };
        if policy.record().admission.stack_class
            != crate::content::native_gameplay::NativeItemStackClass::StackCapable
            || !matches!(&policy.record().semantics.trade_restrictions,crate::content::ReferenceItemField::Known(trade) if trade.marketable==crate::content::ReferenceItemField::Known(true))
        {
            continue;
        }
        let Some(definition) = crate::gameplay_transport::item_look::typed_definition(index, r)
        else {
            continue;
        };
        let Ok(facts) = generation_item_facts(native, &definition) else {
            continue;
        };
        if facts.weight.is_none()
            || !matches!(
                facts.stack,
                crate::durability::item_transfer::ItemStackClass::Stackable { .. }
            )
        {
            continue;
        }
        let Some(id) = key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        return Ok((definition, reference, id));
    }
    Err("admitted marketable ware".into())
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(key).fetch_one(&h.pool).await?)
}
