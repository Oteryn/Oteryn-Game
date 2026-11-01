//! Real command 424 -> the existing Atelier owner -> asset custody and receipt replay.
use super::*;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::{atelier as wire, wheel as wheel_wire};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
enum Case {
    Reveal,
    Dismantle,
    SwitchDomain,
    GradeUp,
    NestedReveal,
    StashGrade,
    ExAction,
}

#[test]
fn atelier_wire_reveal_burns_one_gem_and_charges_once() -> TestResult {
    check(Case::Reveal)
}

#[test]
fn atelier_wire_dismantle_mints_crystal_fragments_once() -> TestResult {
    check(Case::Dismantle)
}

#[test]
fn atelier_wire_switch_domain_charges_once_and_restores_after_relog() -> TestResult {
    check(Case::SwitchDomain)
}

#[test]
fn atelier_wire_grade_up_burns_fragments_and_gold_once() -> TestResult {
    check(Case::GradeUp)
}

#[test]
fn atelier_wire_nested_gem_refuses_before_gold_or_custody_change() -> TestResult {
    check(Case::NestedReveal)
}

#[test]
fn atelier_wire_stash_fragments_refuse_before_gold_or_custody_change() -> TestResult {
    check(Case::StashGrade)
}

#[test]
fn wheel_atelier_wire_share_fresh_exhaustion_but_keep_replay_and_queries_open() -> TestResult {
    check(Case::ExAction)
}

fn check(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Atelier wire PostgreSQL thread panic")??;
    eprintln!("ATELIER_{case:?} POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn query(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<wire::AtelierReply> {
    wire::decode_reply(
        &send_atelier(owner, admitted, wire::COMMAND_TYPE_ATELIER_QUERY, &[]).await?,
    )
    .map_err(debug)
}

async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_atelier_item_records), \
         (SELECT count(*) FROM game_item_fee_burns), \
         (SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid \
          AND world_id=encode($2,'hex')::uuid)",
    )
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

// A scope refusal must not pass because its real input or backpack is UNKNOWN.
// Read the exact SQL definition/revision through the same native item-facts owner
// used by custody, before the input is nested or retired into the real stash.
async fn assert_scope_asset_qualification(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    case: Case,
) -> TestResult {
    use crate::durability::item_loop_move::ItemLoopContent;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemStackClass;
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    use oteryn_gameplay_content::content::ReferenceItemField::Known;
    let (input, key, count, weight): (u8, &str, i64, u32) = match case {
        Case::NestedReveal => (220, "oteryn:item.tibia.i44608", 2, 20),
        Case::StashGrade => (229, "oteryn:item.tibia.i46625", 100, 5),
        _ => return Err("expected a custody-scope case".into()),
    };
    let native = owner.active_generation.and_then(|active| active.native_gameplay())
        .ok_or("native custody-scope content")?;
    let content = NativeItemWeights(native);
    for (seed, expected_key, expected_count, expected_weight) in [
        (210_u8, "oteryn:item.tibia.i2854", 1_i64, None),
        (input, key, count, Some(weight)),
    ] {
        let (family, production_key, revision_ref, quantity): (String, String, String, i64) =
            sqlx::query_as("SELECT definition_family,definition_production_key,definition_revision_ref,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1")
                .bind(id(seed).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(production_key, expected_key);
        assert_eq!(quantity, expected_count);
        let definition = TypedDefinitionRef { family, production_key, revision_ref };
        let facts = content.loop_facts(&definition).ok_or("qualified custody-scope definition")?;
        assert_eq!(facts.definition, definition);
        if let Some(weight) = expected_weight {
            assert_eq!(facts.weight, Some(weight));
            assert!(matches!(facts.stack, ItemStackClass::Stackable { proven_maximum: Some(100) }));
            assert!(facts.container_capacity.is_none());
            if matches!(case, Case::StashGrade) {
                let policy = native.item_policy(&definition.production_key, &definition.revision_ref)
                    .ok_or("native fragment stash policy")?;
                let semantics = &policy.record().semantics;
                assert!(matches!(&semantics.trade_restrictions,
                    Known(trade) if trade.marketable == Known(true)),
                    "the actual fragment must qualify for the existing stash owner");
                assert!(!matches!(&semantics.container, Known(_)));
            }
        } else {
            assert!(facts.weight.is_some());
            assert!(matches!(facts.stack, ItemStackClass::NonStackable));
            assert!(facts.container_slot_equip_pattern);
            assert!(facts.container_capacity.is_some_and(|capacity| capacity > 0));
        }
    }
    Ok(())
}

async fn nest_lesser_gems(h: &Harness) -> TestResult {
    // Fixture custody only: the actual command below uses the restricted runtime
    // role. Copy the admitted backpack definition so this is a real native bag.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,1,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(250).as_slice()).bind(id(210).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,8,encode($1,'hex')::uuid)")
        .bind(id(250).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(210).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,placed_transaction_id FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(220).as_slice()).bind(id(250).as_slice()).execute(&mut *tx).await?;
    assert_eq!(sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(220).as_slice()).execute(&mut *tx).await?.rows_affected(), 1);
    tx.commit().await?;
    let nested: i64 = sqlx::query_scalar("SELECT i.quantity FROM game_item_instances i JOIN game_item_nested_entries n USING(item_instance_id) JOIN game_item_container_entries b ON b.item_instance_id=n.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND b.parent_item_instance_id=encode($2,'hex')::uuid AND b.character_id=encode($3,'hex')::uuid")
        .bind(id(220).as_slice()).bind(id(210).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(nested, 2, "the reveal input exists inside a carried nested bag");
    Ok(())
}

async fn stow_basic_fragments(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::character_stash::StashRequest;
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    use oteryn_protocol_oteryn::stash::{StashAction, StashIntent, StashOutcome};
    let native = owner.active_generation.and_then(|active| active.native_gameplay())
        .ok_or("native stash content")?;
    let permit = owner.spell_lane_permit().await.ok_or("stash lane")?;
    // Prepare the stash through its real atomic writer, using this admitted
    // fixture's resolved main-backpack item and current item fence.
    let receipt = owner.root.stash_conversion(
        &permit, owner.character, owner.holder, admitted.item_fence.ok_or("stash item fence")?,
        StashRequest {
            command: CommandRef::new(admitted.game_session_id, CommandId::new(150).map_err(debug)?),
            intent: StashIntent {
                page: 0,
                action: StashAction::Stow(std::num::NonZeroU64::new(1).ok_or("stash handle")?),
            },
            source: Some(id(229)),
            row_open: true,
            access_refusal: None,
        },
        native, &NativeItemWeights(native), false,
    ).await.map_err(debug)?.ok_or("stash receipt")?;
    assert_eq!(receipt.outcome, StashOutcome::Committed);
    assert_eq!(receipt.stowed_quantity, 100);
    Ok(())
}

async fn scope_footprint(h: &Harness) -> TestResult<serde_json::Value> {
    // Include identities and quantities, not just row counts: a refused operation
    // must not burn/reparent an input or change the stash/bank/character revision.
    Ok(sqlx::query_scalar("SELECT jsonb_build_object( \
        'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i), \
        'main',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e), \
        'nested',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e), \
        'stash',(SELECT jsonb_agg(to_jsonb(b) ORDER BY definition_key,definition_revision) FROM game_character_stash_balances b), \
        'bank',(SELECT jsonb_agg(to_jsonb(b) ORDER BY account_id,world_id) FROM game_account_bank_balances b), \
        'revision',(SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid), \
        'atelier',(SELECT state_json FROM game_character_atelier_state WHERE character_id=encode($1,'hex')::uuid), \
        'wheel',(SELECT to_jsonb(w) FROM game_character_wheel_state w WHERE character_id=encode($1,'hex')::uuid), \
        'wheel_receipts',(SELECT count(*) FROM game_character_wheel_receipts), \
        'receipts',(SELECT count(*) FROM game_character_atelier_receipts), \
        'custody',(SELECT count(*) FROM game_atelier_item_records), \
        'fees',(SELECT count(*) FROM game_item_fee_burns), \
        'stash_entries',(SELECT count(*) FROM game_character_stash_entries))")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

fn scenario(case: Case) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "atelier_paid_wire", true, false).await?;
        let admission_request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) = super::super::death_ml_loss_composed::production_generation_with_pin(
            world, "content-1",
        )?;
        super::super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin, room, crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root.latch_premium_activation(
            &activation.activation_id, activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        ).await.map_err(debug)?;
        let free = SpellPremiumCoordinator::new(
            Default::default(), Some(clock.clone()), Some(activation.clone()), Some(h.root.clone()),
        );
        let gate = SpellPremiumCoordinator::new(
            consumer, Some(clock), Some(activation), Some(h.root.clone()),
        );
        let data = WheelGemData::embedded().map_err(debug)?;
        let ruleset = Arc::new(WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?);
        let mut owner = deps.owner(&h, &authority);
        if matches!(case, Case::ExAction) {
            // The existing owner clock saturates elapsed future instants to zero.
            // Freeze this fixture across PostgreSQL awaits, so the forbidden window
            // is deterministic and does not depend on database or machine speed.
            owner.clock_origin = Instant::now().checked_add(Duration::from_secs(3600))
                .ok_or("frozen ExAction clock")?;
        }
        owner.active_generation = generation.active();
        owner.wheel_ruleset = Some(Arc::clone(&ruleset));
        owner.premium_coordinator = Some(&gate);
        let session = deps.admitted.game_session_id;
        owner.admit_premium(id(40), session);
        fixture::eligible(&h.pool).await?;
        let purchase = fixture::prepare(&h).await?;
        let fence = owner.current_quest_fence(session).await.map_err(|_| "fence")?.ok_or("fence")?;
        let promoted = owner.revision_sequencer.acquire(fence.character_id).await
            .commit_npc_purchase(&h.root, &authority, &h.node, fence, purchase, &gate)
            .await.map_err(debug)?;
        assert!(matches!(promoted, PurchaseOutcome::Committed(_)));
        fixture::paid_assets(&h).await?;
        super::super::tutorial_vitals_fixture::native_asset_revisions(
            &h, generation.active().ok_or("active native generation")?,
        ).await?;
        super::super::tutorial_vitals_fixture::admit(&owner, &deps, &admission_request).await?;
        owner.admit_wheel_session(session).await;
        let (current, _) = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session).await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(), &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
        ).ok_or("Wheel capability")?;
        let before = query(&owner, &mut admitted).await?;
        assert_eq!(before.gems.len(), 8);
        assert_eq!(before.quality_quotes.len(), 3);
        assert_eq!(before.grade_quotes.len(), 6);
        if matches!(case, Case::NestedReveal | Case::StashGrade) {
            assert_scope_asset_qualification(&h, &owner, case).await?;
        }
        match case {
            Case::NestedReveal => nest_lesser_gems(&h).await?,
            Case::StashGrade => stow_basic_fragments(&owner, &admitted).await?,
            _ => {}
        }
        let scope_before = if matches!(case, Case::NestedReveal | Case::StashGrade) {
            Some(scope_footprint(&h).await?)
        } else {
            None
        };
        let gem = before.gems.iter().find(|gem| gem.domain == 0 && gem.quality == 1)
            .ok_or("initial regular green gem")?;
        let original_gem = gem.clone();
        let mutation = match case {
            Case::Reveal | Case::NestedReveal | Case::ExAction => wire::Mutation::Reveal { quality: 0 },
            Case::Dismantle => wire::Mutation::Dismantle { gem: gem.id },
            Case::SwitchDomain => wire::Mutation::SwitchDomain { gem: gem.id },
            Case::GradeUp | Case::StashGrade => wire::Mutation::GradeUp {
                kind: 0, mod_id: *gem.modifiers.first().ok_or("wire modifier")?,
            },
        };
        let request = wire::MutateRequest {
            occurrence: id(180), expected_revision: before.revision, mutation,
        };
        let payload = wire::encode_mutate(&request).map_err(debug)?;
        if scope_before.is_some() {
            assert!(owner.refresh_and_apply_spell_access(deps.actor, session).await
                .premium_gate().premium_current(), "custody refusal requires current Premium");
            let required_gold = if matches!(case, Case::NestedReveal) { 125_000 } else { 2_000_000 };
            assert!(footprint(&h).await?.2 >= required_gold, "custody refusal has enough real gold");
            // Expire every earlier window of this same monotonic owner. No fresh
            // Wheel/Atelier action runs between this point and the tested command.
            let previous_time = owner.owner_now().get();
            advance_ex_action(&mut owner)?;
            assert!(owner.owner_now().get().saturating_sub(previous_time) >= 490_000,
                "custody refusal must not be caused by ExAction exhaustion");
        }
        let reply_bytes = send_atelier(&owner, &mut admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?;
        let reply = wire::decode_reply(&reply_bytes).map_err(debug)?;
        if let Some(scope_before) = scope_before {
            // ANSWER6544069763 accepts only direct main-backpack custody in this
            // slice. Plenty of gold and the actual nested/stashed resource exist.
            assert!(reply.refused, "{case:?}");
            assert_eq!(scope_footprint(&h).await?, scope_before, "{case:?} changed custody or charged a fee");
            assert_eq!(query(&owner, &mut admitted).await?, before);
            let fee: Option<i64> = sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_occurrence_id=encode($1,'hex')::uuid")
                .bind(request.occurrence.as_slice()).fetch_optional(&h.pool).await?;
            assert_eq!(fee, None);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        assert!(!reply.refused, "{case:?}");
        assert_eq!(reply.revision, before.revision + 1);
        assert!(reply.quality_quotes.is_empty() && reply.grade_quotes.is_empty());
        let stored = stored(&h).await?;
        assert_eq!(stored.revision, reply.revision);
        let expected_fee = match case {
            Case::Reveal | Case::NestedReveal | Case::ExAction => {
                assert_eq!(reply.gems.len(), before.gems.len() + 1);
                assert!(reply.gems.iter().any(|gem| gem.id == id(180) && gem.quality == 0));
                let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(220).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(quantity, 1);
                Some(125_000_i64)
            }
            Case::Dismantle => {
                assert!(!reply.gems.iter().any(|gem| gem.id == original_gem.id));
                let (key, count): (String, i32) = sqlx::query_as("SELECT outputs->0->>'key',(outputs->0->>'count')::int FROM game_atelier_item_records WHERE occurrence=encode($1,'hex')::uuid")
                    .bind(request.occurrence.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(key, "oteryn:item.tibia.i46625");
                assert!((2..=10).contains(&count));
                None
            }
            Case::SwitchDomain => {
                assert_eq!(reply.gems.iter().find(|gem| gem.id == original_gem.id).ok_or("rotated gem")?.domain, 1);
                Some(250_000)
            }
            Case::GradeUp | Case::StashGrade => {
                assert!(reply.grades.iter().any(|grade| grade.kind == 0 && grade.mod_id == original_gem.modifiers[0] && grade.grade == 1));
                let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(229).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(quantity, 95);
                Some(2_000_000)
            }
        };
        let fee: Option<i64> = sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_occurrence_id=encode($1,'hex')::uuid")
            .bind(request.occurrence.as_slice()).fetch_optional(&h.pool).await?;
        assert_eq!(fee, expected_fee);
        let advertised_fee = match case {
            Case::Reveal | Case::NestedReveal | Case::ExAction => Some(before.quality_quotes.iter().find(|quote| quote.quality == 0)
                .ok_or("lesser quote")?.reveal_gold as i64),
            Case::Dismantle => None,
            Case::SwitchDomain => Some(before.quality_quotes.iter().find(|quote| quote.quality == 1)
                .ok_or("regular quote")?.switch_gold as i64),
            Case::GradeUp | Case::StashGrade => Some(before.grade_quotes.iter().find(|quote| quote.kind == 0 && quote.grade == 1)
                .ok_or("basic grade quote")?.gold as i64),
        };
        assert_eq!(fee, advertised_fee, "the committed debit matches the server's displayed quote");
        if matches!(case, Case::ExAction) {
            exaction(&h, &mut owner, &mut admitted, &payload, &reply_bytes, &reply).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_millis(490))
            .ok_or("next fresh Atelier action")?;
        // Another command changes the current projection; replay must still return
        // this occurrence's retained result, rather than a newly synthesized response.
        let another = reply.gems.first().ok_or("remaining gem")?.id;
        let lock = wire::encode_set_lock(&wire::SetLockRequest {
            occurrence: id(181), expected_revision: reply.revision, gem: another, locked: true,
        }).map_err(debug)?;
        send_atelier(&owner, &mut admitted, wire::COMMAND_TYPE_ATELIER_SET_LOCK, &lock).await?;
        let current = query(&owner, &mut admitted).await?;
        assert_eq!(current.revision, reply.revision + 1);
        let unchanged = footprint(&h).await?;
        owner.active_generation = None;
        assert_eq!(send_atelier(&owner, &mut admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?, reply_bytes);
        assert_eq!(footprint(&h).await?, unchanged, "missing live resources cannot charge a replay");
        owner.active_generation = generation.active();
        owner.premium_coordinator = Some(&free);
        assert_eq!(send_atelier(&owner, &mut admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?, reply_bytes);
        owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_millis(490))
            .ok_or("fresh Premium refusal after ExAction")?;
        let unpaid = wire::encode_mutate(&wire::MutateRequest {
            occurrence: id(182), expected_revision: current.revision,
            mutation: wire::Mutation::Reveal { quality: 0 },
        }).map_err(debug)?;
        assert!(wire::decode_reply(&send_atelier(&owner, &mut admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &unpaid).await?).map_err(debug)?.refused);
        assert_eq!(footprint(&h).await?, unchanged, "Free refuses a fresh action and preserves the receipt");
        let mut restarted = deps.owner(&h, &authority);
        restarted.clock_origin = owner.clock_origin;
        restarted.active_generation = generation.active();
        restarted.wheel_ruleset = Some(ruleset);
        restarted.premium_coordinator = Some(&gate);
        restarted.admit_premium(id(40), session);
        super::super::tutorial_vitals_fixture::admit(&restarted, &deps, &admission_request).await?;
        restarted.admit_wheel_session(session).await;
        // A reconstructed Channel owner has a new command stream. Retaining the
        // old CommandId without its session-owned high-water correctly fails closed;
        // the durable Atelier occurrence below remains the same across this reload.
        admitted.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            restarted.offered_capabilities(), &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
        ).ok_or("reloaded Wheel capability")?;
        assert_eq!(query(&restarted, &mut admitted).await?, current);
        assert_eq!(send_atelier(&restarted, &mut admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?, reply_bytes);
        assert_eq!(footprint(&h).await?, unchanged);
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}


async fn query_wheel(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<wheel_wire::WheelAllocationReply> {
    let payload = wheel_wire::encode_wheel_request(&wheel_wire::WheelAllocationRequest {
        action: 0, occurrence: [0; 16], expected_revision: 0,
        points: [0; wheel_wire::WHEEL_SLOTS], vessels: None,
    }).map_err(debug)?;
    wheel_wire::decode_wheel_reply(&send_atelier(
        owner, admitted, wheel_wire::COMMAND_TYPE_WHEEL_ALLOCATION, &payload,
    ).await?).map_err(debug)
}

async fn exaction(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    original_payload: &[u8],
    original_reply_bytes: &[u8],
    original_reply: &wire::AtelierReply,
) -> TestResult {
    assert_eq!(owner.owner_now().get(), 0);
    // ANSWER6544165094: the native pair projects one open. Both reads work
    // immediately after a committed paid action, while its shared gate is closed.
    let wheel_before = query_wheel(owner, admitted).await?;
    let atelier_before = query(owner, admitted).await?;
    assert!(!wheel_before.refused && !atelier_before.refused);
    assert_eq!(wheel_before.revision, 0);
    assert_eq!(atelier_before.revision, original_reply.revision);
    assert_eq!(atelier_before.quality_quotes.len(), 3);
    assert_eq!(atelier_before.grade_quotes.len(), 6);
    let untouched = scope_footprint(h).await?;

    let mut points = [0; wheel_wire::WHEEL_SLOTS];
    points[14] = 1; // Sorcerer core slot15; within this level60 Character's budget.
    let save = wheel_wire::encode_wheel_request(&wheel_wire::WheelAllocationRequest {
        action: 1, occurrence: id(183), expected_revision: wheel_before.revision,
        points, vessels: Some([None; wheel_wire::WHEEL_VESSELS]),
    }).map_err(debug)?;
    let refused = wheel_wire::decode_wheel_reply(&send_atelier(
        owner, admitted, wheel_wire::COMMAND_TYPE_WHEEL_ALLOCATION, &save,
    ).await?).map_err(debug)?;
    assert!(refused.refused, "Atelier's paid action exhausts Wheel save/vessels too");
    assert_eq!((refused.revision, refused.points), (wheel_before.revision, wheel_before.points));

    let gem = atelier_before.gems.first().ok_or("remaining gem")?.id;
    let lock = wire::encode_set_lock(&wire::SetLockRequest {
        occurrence: id(184), expected_revision: atelier_before.revision, gem, locked: true,
    }).map_err(debug)?;
    assert!(wire::decode_reply(&send_atelier(
        owner, admitted, wire::COMMAND_TYPE_ATELIER_SET_LOCK, &lock,
    ).await?).map_err(debug)?.refused, "lock shares the same fresh-mutation limit");
    let paid = wire::encode_mutate(&wire::MutateRequest {
        occurrence: id(185), expected_revision: atelier_before.revision,
        mutation: wire::Mutation::Reveal { quality: 0 },
    }).map_err(debug)?;
    assert!(wire::decode_reply(&send_atelier(
        owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &paid,
    ).await?).map_err(debug)?.refused, "a second affordable reveal must still wait");
    assert_eq!(owner.owner_now().get(), 0);
    assert_eq!(scope_footprint(h).await?, untouched, "exhaustion must precede fee, custody and revision writes");

    assert_eq!(send_atelier(
        owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, original_payload,
    ).await?, original_reply_bytes, "the immutable receipt is recognized before the closed gate");
    assert_eq!(query_wheel(owner, admitted).await?, wheel_before);
    assert_eq!(query(owner, admitted).await?, atelier_before);
    assert_eq!(scope_footprint(h).await?, untouched);

    // Move the same existing clock to the accepted side of 490ms without sleeping.
    // The gate unit test separately pins the exact 489999/490000us boundary.
    owner.clock_origin = Instant::now().checked_sub(Duration::from_millis(490))
        .ok_or("advance ExAction clock")?;
    let accepted = wheel_wire::decode_wheel_reply(&send_atelier(
        owner, admitted, wheel_wire::COMMAND_TYPE_WHEEL_ALLOCATION, &save,
    ).await?).map_err(debug)?;
    assert!(!accepted.refused, "the previously throttled occurrence has no committed refusal receipt");
    assert_eq!(accepted.revision, wheel_before.revision + 1);
    assert_eq!(accepted.points, points);
    assert_eq!(accepted.vessels, Some([None; wheel_wire::WHEEL_VESSELS]));
    let accepted_bytes = wheel_wire::encode_wheel_reply(&accepted).map_err(debug)?;
    assert_eq!(send_atelier(
        owner, admitted, wheel_wire::COMMAND_TYPE_WHEEL_ALLOCATION, &save,
    ).await?, accepted_bytes);
    assert!(!query_wheel(owner, admitted).await?.refused);
    let current = query(owner, admitted).await?;
    assert!(!current.refused);

    owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_millis(490))
        .ok_or("next fresh lock")?;
    let lock = wire::encode_set_lock(&wire::SetLockRequest {
        occurrence: id(184), expected_revision: current.revision, gem, locked: true,
    }).map_err(debug)?;
    let locked = wire::decode_reply(&send_atelier(
        owner, admitted, wire::COMMAND_TYPE_ATELIER_SET_LOCK, &lock,
    ).await?).map_err(debug)?;
    assert!(!locked.refused);
    assert_eq!(locked.revision, current.revision + 1);
    assert!(locked.gems.iter().any(|value| value.id == gem && value.locked));
    let paid_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_atelier_item_records WHERE occurrence=encode($1,'hex')::uuid",
    ).bind(id(180).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(paid_receipts, 1);
    Ok(())
}
