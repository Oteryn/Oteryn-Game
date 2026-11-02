use super::super::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{
        CharacterProgressionError as Error, CurrentCharacterGameplayFence, assert_gameplay_fence,
        uuid_text,
    },
    character_wheel::WheelPremiumGate,
    db::{begin_type2_transaction, lock_admission_relations},
    item_fee_burn::{
        FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, FeeChangeFacts,
        burn_fee_in_transaction,
    },
    runtime_scope_assignment::NodeIncarnationProof,
    store::DeliveryReceipt,
};
use super::{
    draw::{Draw, PreyCatalogue},
    storage,
    types::{PreyMutation, PreyStoreDelivery},
};
use oteryn_gameplay_progression::prey::{
    BonusType, HUNTING_SECONDS, ListReroll, PreyBonus, RerollQuote,
};
use oteryn_protocol_oteryn::prey::{self as wire, Reply, Request, Slot};
use sha2::{Digest, Sha256};
use sqlx::Row;

type CommitResult = (Reply, [bool; 3], Option<DeliveryReceipt>);

impl DurabilityRoot {
    /// The current Character revision slot, admission proof and pinned content are required.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_prey_command<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: [u8; 16],
        mutation: PreyMutation,
        catalogue: &PreyCatalogue,
        coins: Option<FeeChangeFacts>,
        gate: &G,
    ) -> Result<(Reply, [bool; 3]), Error> {
        let (reply, permanent, _) = self
            .commit_prey_inner(
                authority, node, fence, occurrence, mutation, catalogue, coins, gate, None,
            )
            .await?;
        Ok((reply, permanent))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn commit_prey_inner<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: [u8; 16],
        mutation: PreyMutation,
        catalogue: &PreyCatalogue,
        coins: Option<FeeChangeFacts>,
        gate: &G,
        delivery: Option<PreyStoreDelivery>,
    ) -> Result<CommitResult, Error> {
        let mut binding = mutation.binding()?;
        if let Some(d) = &delivery {
            binding.extend_from_slice(b"prey-list-expansion:v1");
            binding.extend_from_slice(&d.account_id);
            binding.extend_from_slice(&d.world_id);
            binding.extend_from_slice(d.fulfilment_unit_id.as_bytes());
        }
        let binding = Sha256::digest(binding).to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate, move |holder, deadline, gate| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let premium = gate.allocation_permitted(&mut tx, uuid_text(&account)?).await?;
            // Current authority is checked even on receipt replay.
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? { Ok(r) => r, Err(e) => return Ok(Err(e)) };
            sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id=$1::text::uuid FOR UPDATE").bind(&account).fetch_one(&mut **tx).await?;
            let world = fence.runtime_scope.world_id();
            let character = fence.character_id.as_bytes();
            if let Some(d) = &delivery {
                if d.account_id != uuid_text(&account)? || d.world_id != *world.as_bytes() || d.operation_id != occurrence {
                    return Ok(Err(Error::AuthorityRejected));
                }
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
                    .bind(d.operation_id.as_slice()).execute(&mut **tx).await?;
            }
            let mut permanent = [false; 3];
            for target in sqlx::query_scalar::<_, i16>("SELECT u.target FROM game_account_prey_unlocks u JOIN game_character_roots r ON r.character_id=u.claiming_character_id WHERE u.account_id=$1::text::uuid AND u.world_id=encode($2,'hex')::uuid AND u.account_id=r.account_id AND u.world_id=r.world_id AND u.unlock_kind='PREY_SLOT_PERMANENT' ORDER BY u.target LIMIT 2")
                .bind(&account).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await? { permanent[target as usize - 1] = true; }
            let readonly = matches!(&mutation, PreyMutation::Command(Request::Query | Request::OfferedToday { .. }));
            if !readonly && let Some(row) = sqlx::query("SELECT request_sha256,result FROM game_character_prey_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid")
                .bind(character.as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut **tx).await? {
                if row.try_get::<Vec<u8>, _>("request_sha256")? != binding { return Ok(Err(Error::ConflictingOccurrence)); }
                let stored: Vec<u8> = row.try_get("result")?;
                let reply = wire::decode_reply(&stored).or_else(|_| wire::decode_legacy_reply(&stored)).map_err(|_| DurabilityError::InvalidStoredState)?;
                let receipt = if let Some(d) = &delivery { Some(super::super::store::write_prey_expansion_receipt(&mut tx, d.operation_id, d.account_id, *character, d.world_id, &d.fulfilment_unit_id, recovery.recovery_generation).await?) } else { None };
                tx.commit(deadline).await?;
                return Ok(Ok((reply, permanent, receipt)));
            }
            let vocation: Option<String> = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid").bind(character.as_slice()).fetch_optional(&mut **tx).await?;
            let eligible = vocation.as_deref().is_some_and(|v| !v.is_empty() && v != "none");
            let mut access = [eligible, eligible && premium, false];
            for (i, flag) in permanent.iter().enumerate() { access[i] |= eligible && *flag; }
            let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(character.as_slice()).fetch_one(&mut **tx).await?;
            let level = u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?;
            let balance: i16 = sqlx::query_scalar("SELECT balance FROM game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE").bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.unwrap_or(0);
            let expansion: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_prey_expansions WHERE character_id=encode($1,'hex')::uuid)").bind(character.as_slice()).fetch_one(&mut **tx).await?;
            let (mut reply, mut metrics) = storage::load(&mut tx, character, balance as u8, level, expansion, &catalogue.rules).await?;
            if matches!(&mutation, PreyMutation::Command(Request::Query)) {
                mask_locked(&mut reply, &access); tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None)));
            }
            if delivery.is_some() && expansion {
                reply.status = wire::STATUS_ALREADY_OWNED;
                mask_locked(&mut reply, &access); tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None)));
            }
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint").fetch_one(&mut **tx).await?;
            let secret: Vec<u8> = sqlx::query_scalar("SELECT secret FROM game_prey_decision_root WHERE singleton").fetch_one(&mut **tx).await?;
            let mut draw = Draw::new(&secret, &occurrence, b"oteryn.prey.list.v1");
            let mut bonus_draw = Draw::new(&secret, &occurrence, b"oteryn.prey.bonus.v1");
            let needs_epoch = delivery.is_none() && match &mutation {
                PreyMutation::Initialize => access.iter().enumerate().any(|(i, open)| *open && reply.slots[i].state == 0),
                PreyMutation::Checkpoint(deltas) => deltas.iter().any(|d| (1..=3).contains(&d.slot)
                    && d.seconds > 0 && d.seconds >= reply.slots[usize::from(d.slot - 1)].hunting_seconds),
                PreyMutation::Command(Request::ListReroll {..} | Request::PickAgain {..} | Request::OfferedToday {..}) => true,
                PreyMutation::Command(_) => false,
            };
            let epoch = if needs_epoch { Some(storage::reset_epoch(&mut tx, world.as_bytes()).await?) } else { None };
            let mut offered = if let Some(epoch) = &epoch {
                let keys: Vec<_> = catalogue.creatures.iter().map(|c| c.0.clone()).collect();
                storage::offered(&mut tx, character, epoch, &keys).await?
            } else { vec![] };
            if let PreyMutation::Command(Request::OfferedToday { slot, after }) = &mutation {
                if !(1..=3).contains(slot) || !access[usize::from(*slot - 1)] || reply.slots[usize::from(*slot - 1)].state == 0 {
                    return Ok(Err(Error::AuthorityRejected));
                }
                let mut candidates = offered.iter().filter(|key| *key > after && !catalogue.unsupported_loot.contains(*key));
                for key in candidates.by_ref().take(wire::MAX_OFFERED_PAGE) {
                    let occupied_slot = reply.slots.iter().filter(|s| s.slot != *slot)
                        .find(|s| s.selected == *key || s.creatures.contains(key)).map_or(0, |s| s.slot);
                    reply.offered_today.push(wire::OfferedCreature { creature: key.clone(),
                        price_gold: catalogue.rules.pick_again_price(level, key).map_err(|_| DurabilityError::InvalidStoredState)?, occupied_slot });
                }
                if candidates.next().is_some() { reply.offered_next = reply.offered_today.last().map_or_else(String::new, |r| r.creature.clone()); }
                mask_locked(&mut reply, &access); tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None)));
            }
            let mut spend = 0_i16;
            let mut fee = 0_u64;
            let mut fee_kind = "NONE";
            let mut fee_slot = None;
            let mut dirty = Vec::new();
            let mut new_offers = Vec::new();
            if delivery.is_none() { match &mutation {
                PreyMutation::Initialize => {
                    for (i, unlocked) in access.iter().enumerate() {
                        if *unlocked && reply.slots[i].state == 0 {
                            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=$2)").bind(character.as_slice()).bind((i + 1) as i16).fetch_one(&mut **tx).await?;
                            if exists { continue; }
                            let mut black = blacklist(&reply, i); black.extend(offered.iter().cloned());
                            let tier = catalogue.rules.effective_tier(reply.slots[i].chosen_tier, level).map_err(|_| DurabilityError::InvalidStoredState)?;
                            let list = draw.list(catalogue, tier, usize::from(reply.list_size), &black)?;
                            offered.extend(list.iter().cloned()); new_offers.extend(list.iter().cloned());
                            reply.slots[i].creatures = list; reply.slots[i].state = 1; reply.slots[i].free_reroll_at = (now / 1000) as u64;
                            dirty.push(i);
                        }
                    }
                }
                PreyMutation::Checkpoint(deltas) => {
                    if deltas.is_empty() || deltas.len() > 3 || !deltas.windows(2).all(|p| p[0].slot < p[1].slot) { return Ok(Err(Error::InvalidInput)); }
                    for delta in deltas {
                        if !(1..=3).contains(&delta.slot) || delta.seconds > HUNTING_SECONDS || delta.loot_gold > i64::MAX as u64 || (delta.seconds == 0 && delta.kills == 0 && delta.loot_gold == 0) { return Ok(Err(Error::InvalidInput)); }
                        let i = usize::from(delta.slot - 1);
                        let slot = &mut reply.slots[i];
                        if slot.state != 2 || slot.selected != delta.creature || slot.selection_epoch != delta.selection_epoch { return Ok(Err(Error::ConflictingOccurrence)); }
                        let m = &mut metrics[i];
                        if m.tier == 0 { m.tier = catalogue.rules.creatures.get(&slot.selected).map_or(0, |value| value.tier); }
                        m.consumed = m.consumed.checked_add(i64::from(delta.seconds.min(slot.hunting_seconds))).ok_or(DurabilityError::InvalidStoredState)?;
                        m.kills = m.kills.checked_add(i64::from(delta.kills)).ok_or(DurabilityError::InvalidStoredState)?;
                        m.loot_gold = m.loot_gold.checked_add(delta.loot_gold as i64).ok_or(DurabilityError::InvalidStoredState)?;
                        if delta.seconds > 0 && slot.hunting_seconds <= delta.seconds {
                            storage::record_end(&mut tx, character, &occurrence, slot, m, now, "EXPIRED").await?;
                            let cost = match slot.option { 1 => 1, 2 => 5, _ => 0 };
                            *m = storage::Metrics::default();
                            if access[i] && cost > 0 && balance - spend >= cost {
                                spend += cost;
                                if slot.option == 1 { set_bonus(slot, bonus_draw.bonus(bonus(slot))); }
                                next_selection(slot)?; slot.hunting_seconds = HUNTING_SECONDS;
                                m.start(now, catalogue.rules.creatures.get(&slot.selected).ok_or(DurabilityError::InvalidStoredState)?.tier, i64::from(cost), 0);
                            } else {
                                slot.state = 1; slot.selected.clear(); slot.bonus = 0; slot.grade = 0; slot.percent = 0; slot.option = 0; slot.hunting_seconds = 0;
                                let mut black = blacklist(&reply, i); black.extend(offered.iter().cloned());
                                let tier = catalogue.rules.effective_tier(reply.slots[i].chosen_tier, level).map_err(|_| DurabilityError::InvalidStoredState)?;
                                let list = draw.list(catalogue, tier, usize::from(reply.list_size), &black)?;
                                offered.extend(list.iter().cloned()); new_offers.extend(list.iter().cloned()); reply.slots[i].creatures = list;
                            }
                        } else { slot.hunting_seconds -= delta.seconds; }
                        dirty.push(i);
                    }
                }
                PreyMutation::Command(request) => {
                    let slot_id = match request {
                        Request::ListReroll {slot} | Request::Select {slot,..} | Request::SelectAny {slot,..} | Request::BonusReroll {slot} | Request::SetOption {slot,..} | Request::SetTier {slot,..} | Request::PickAgain {slot,..} => *slot,
                        Request::Query | Request::OfferedToday {..} => return Ok(Err(Error::InvalidInput)),
                    };
                    if !(1..=3).contains(&slot_id) { return Ok(Err(Error::InvalidInput)); }
                    let i = usize::from(slot_id - 1);
                    if !access[i] || reply.slots[i].state == 0 { return Ok(Err(Error::AuthorityRejected)); }
                    let black = blacklist(&reply, i);
                    let size = usize::from(reply.list_size);
                    let slot = &mut reply.slots[i];
                    let m = &mut metrics[i];
                    match request {
                        Request::SetTier {tier,..} => {
                            if catalogue.rules.check_choice(*tier, level).is_err() { reply.status = wire::STATUS_INVALID_TIER; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            slot.chosen_tier = *tier;
                        }
                        Request::ListReroll {..} => {
                            let quote = match RerollQuote::try_from(ListReroll {now: (now / 1000) as u64, free_reroll_at: slot.free_reroll_at, level}) { Ok(q) => q, Err(_) => return Ok(Err(Error::InvalidInput)) };
                            let mut exclusion = black; exclusion.extend(offered.iter().cloned());
                            let tier = catalogue.rules.effective_tier(slot.chosen_tier, level).map_err(|_| DurabilityError::InvalidStoredState)?;
                            let list = draw.list(catalogue, tier, size, &exclusion)?;
                            // A paid list is a full configured list; depletion refuses before any debit.
                            if list.is_empty() || (quote.gold > 0 && list.len() < size) { reply.status = wire::STATUS_NO_CANDIDATES; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            storage::record_end(&mut tx, character, &occurrence, slot, m, now, "CANCELLED").await?;
                            if slot.state == 2 { *m = storage::Metrics::default(); }
                            if quote.gold == 0 { m.free_rerolls = m.free_rerolls.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?; } else { m.gold_rerolls = m.gold_rerolls.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?; }
                            fee = quote.gold; fee_kind = "LIST_REROLL"; slot.free_reroll_at = quote.free_reroll_at;
                            slot.state = 1; slot.selected.clear(); slot.hunting_seconds = 0;
                            new_offers.extend(list.iter().cloned()); slot.creatures = list;
                        }
                        Request::Select {index,..} => {
                            if slot.state != 1 { return Ok(Err(Error::InvalidInput)); }
                            let Some(key) = slot.creatures.get(usize::from(*index).wrapping_sub(1)).cloned() else { return Ok(Err(Error::InvalidInput)); };
                            if black.contains(&key) { return Ok(Err(Error::InvalidInput)); }
                            if catalogue.unsupported_loot.contains(&key) { reply.status = wire::STATUS_CONTENT_UNAVAILABLE; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            m.start(now, catalogue.rules.creatures.get(&key).ok_or(DurabilityError::InvalidStoredState)?.tier, 0, 0);
                            select(slot, key, &mut bonus_draw)?;
                        }
                        Request::SelectAny {creature,..} => {
                            if slot.state != 1 || !catalogue.creatures.iter().any(|c| &c.0 == creature) || black.contains(creature) { return Ok(Err(Error::InvalidInput)); }
                            if catalogue.unsupported_loot.contains(creature) { reply.status = wire::STATUS_CONTENT_UNAVAILABLE; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            spend = 5; m.start(now, catalogue.rules.creatures.get(creature).ok_or(DurabilityError::InvalidStoredState)?.tier, 5, 0);
                            select(slot, creature.clone(), &mut bonus_draw)?;
                        }
                        Request::PickAgain {creature,expected_price_gold,..} => {
                            if !offered.contains(creature) { reply.status = wire::STATUS_NOT_OFFERED; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            if black.contains(creature) { reply.status = wire::STATUS_OCCUPIED; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            if catalogue.unsupported_loot.contains(creature) { reply.status = wire::STATUS_CONTENT_UNAVAILABLE; mask_locked(&mut reply, &access); write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision, ReceiptChange::default()).await?; tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
                            fee = catalogue.rules.pick_again_price(level, creature).map_err(|_| DurabilityError::InvalidStoredState)?;
                            if fee != *expected_price_gold { return Ok(Err(Error::InvalidInput)); }
                            fee_kind = "PICK_AGAIN";
                            storage::record_end(&mut tx, character, &occurrence, slot, m, now, "REPLACED").await?;
                            if slot.state == 2 { *m = storage::Metrics::default(); }
                            m.start(now, catalogue.rules.creatures.get(creature).ok_or(DurabilityError::InvalidStoredState)?.tier, 0, fee as i64);
                            select(slot, creature.clone(), &mut bonus_draw)?;
                        }
                        Request::BonusReroll {..} => {
                            if slot.state != 2 { return Ok(Err(Error::InvalidInput)); }
                            storage::record_end(&mut tx, character, &occurrence, slot, m, now, "REPLACED").await?;
                            *m = storage::Metrics::default(); spend = 1;
                            set_bonus(slot, bonus_draw.bonus(bonus(slot))); next_selection(slot)?; slot.hunting_seconds = HUNTING_SECONDS;
                            m.start(now, catalogue.rules.creatures.get(&slot.selected).ok_or(DurabilityError::InvalidStoredState)?.tier, 1, 0);
                        }
                        Request::SetOption {option,..} => {
                            let cost = match option {0 => 0, 1 => 1, 2 => 5, _ => return Ok(Err(Error::InvalidInput))};
                            if balance < cost { return Ok(Err(Error::InvalidInput)); } slot.option = *option;
                        }
                        Request::Query | Request::OfferedToday {..} => return Ok(Err(Error::InvalidInput)),
                    }
                    dirty.push(i);
                }
            }}
            if spend > balance { return Ok(Err(Error::InvalidInput)); }
            if dirty.is_empty() && delivery.is_none() { mask_locked(&mut reply, &access); tx.commit(deadline).await?; return Ok(Ok((reply, permanent, None))); }
            if fee > 0 {
                let slot = reply.slots[dirty[0]].slot;
                fee_slot = Some(slot);
                let Some(coins) = coins else { return Ok(Err(Error::InvalidInput)); };
                let cause = if fee_kind == "PICK_AGAIN" { FeeBurnCause::PreyPickAgain {slot, occurrence} } else { FeeBurnCause::PreyListReroll {slot, occurrence} };
                let request = FeeBurnRequest {cause, fee_gold_units: fee, transaction_id: occurrence, event_id: fee_id(occurrence), occurred_at_unix_ms: now, server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(), change: coins};
                match burn_fee_in_transaction(&mut tx, &fence, &request).await { Ok(FeeBurnOutcome::Burned(_)) => (), Err(FeeBurnError::Unavailable(e)) => return Err(e), _ => return Ok(Err(Error::InvalidInput)) }
            }
            if let Some(epoch) = &epoch { storage::record_offers(&mut tx, character, &occurrence, epoch, &new_offers).await?; }
            let delivery_receipt = if let Some(d) = &delivery {
                sqlx::query("INSERT INTO game_prey_expansion_deliveries(operation_id,character_id,world_id,occurrence,fulfilment_unit_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($1,'hex')::uuid,$4)")
                    .bind(d.operation_id.as_slice()).bind(character.as_slice()).bind(d.world_id.as_slice()).bind(&d.fulfilment_unit_id).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_character_prey_expansions(character_id,world_id,delivery_operation_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                    .bind(character.as_slice()).bind(d.world_id.as_slice()).bind(d.operation_id.as_slice()).execute(&mut **tx).await?;
                reply.expansion_owned = true; reply.list_size = catalogue.rules.expanded_list_size;
                Some(super::super::store::write_prey_expansion_receipt(&mut tx, d.operation_id, d.account_id, *character, d.world_id, &d.fulfilment_unit_id, recovery.recovery_generation).await?)
            } else { None };
            reply.wildcard_balance = (balance - spend) as u8;
            for i in dirty { storage::save(&mut tx, character, &reply.slots[i], &metrics[i]).await?; }
            mask_locked(&mut reply, &access);
            write_receipt(&mut tx, character, &occurrence, &binding, &reply, root.revision,
                ReceiptChange { wildcards: spend, fee, fee_kind, fee_slot, expansion: delivery.is_some() }).await?;
            if spend > 0 {
                sqlx::query("INSERT INTO game_account_prey_wildcard_entries(claim_id,account_id,world_id,previous_claim_id,kind,requested,credited,balance_before,balance_after) SELECT encode($3,'hex')::uuid,account_id,world_id,last_claim_id,'SPEND',$4,0,balance,balance-$4 FROM game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid")
                    .bind(&account).bind(world.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(spend).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_account_prey_wildcards SET balance=balance-$4,last_claim_id=encode($3,'hex')::uuid WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid").bind(&account).bind(world.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(spend).execute(&mut **tx).await?;
            }
            tx.commit(deadline).await?; Ok(Ok((reply, permanent, delivery_receipt)))
        })).await?
    }
}
fn blacklist(reply: &Reply, i: usize) -> Vec<String> {
    reply
        .slots
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != i)
        .flat_map(|(_, s)| {
            s.creatures
                .iter()
                .cloned()
                .chain((!s.selected.is_empty()).then_some(s.selected.clone()))
        })
        .collect()
}
fn bonus(slot: &Slot) -> Option<PreyBonus> {
    let kind = match slot.bonus {
        1 => BonusType::Damage,
        2 => BonusType::Reduction,
        3 => BonusType::Experience,
        4 => BonusType::Loot,
        _ => return None,
    };
    Some(PreyBonus {
        kind,
        grade: slot.grade,
        percent: slot.percent,
    })
}
fn set_bonus(slot: &mut Slot, b: PreyBonus) {
    slot.bonus = match b.kind {
        BonusType::Damage => 1,
        BonusType::Reduction => 2,
        BonusType::Experience => 3,
        BonusType::Loot => 4,
    };
    slot.grade = b.grade;
    slot.percent = b.percent;
}
fn next_selection(slot: &mut Slot) -> Result<(), DurabilityError> {
    slot.selection_epoch = slot
        .selection_epoch
        .checked_add(1)
        .filter(|v| *v <= i64::MAX as u64)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(())
}
fn select(slot: &mut Slot, key: String, draw: &mut Draw) -> Result<(), DurabilityError> {
    if slot.bonus == 0 {
        set_bonus(slot, draw.bonus(None));
    }
    slot.creatures.retain(|k| k != &key);
    slot.selected = key;
    slot.state = 2;
    slot.hunting_seconds = HUNTING_SECONDS;
    next_selection(slot)
}
fn fee_id(mut id: [u8; 16]) -> [u8; 16] {
    id[15] ^= 1;
    id
}

#[derive(Default)]
struct ReceiptChange<'a> {
    wildcards: i16,
    fee: u64,
    fee_kind: &'a str,
    fee_slot: Option<u8>,
    expansion: bool,
}

async fn write_receipt(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    character: &[u8; 16],
    occurrence: &[u8; 16],
    binding: &[u8],
    reply: &Reply,
    revision: u64,
    change: ReceiptChange<'_>,
) -> Result<(), DurabilityError> {
    let result = wire::encode_reply(reply).map_err(|_| DurabilityError::InvalidStoredState)?;
    let next = revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let fee_kind = if change.fee_kind.is_empty() {
        "NONE"
    } else {
        change.fee_kind
    };
    sqlx::query("INSERT INTO game_character_prey_receipts(character_id,occurrence,request_sha256,result,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,wildcards_spent,fee_gold_units,fee_kind,fee_slot,store_expansion) SELECT character_id,encode($2,'hex')::uuid,$3,$4,$5::text::numeric,$6::text::numeric,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,$7,$8,$9,$10,$11 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).bind(occurrence.as_slice()).bind(binding).bind(result)
        .bind(revision.to_string()).bind(next.to_string()).bind(change.wildcards).bind(change.fee as i64).bind(fee_kind).bind(change.fee_slot.map(i16::from)).bind(change.expansion)
        .execute(&mut ***tx).await?;
    for query in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
    ] {
        if sqlx::query(query)
            .bind(character.as_slice())
            .bind(next.to_string())
            .bind(revision.to_string())
            .execute(&mut ***tx)
            .await?
            .rows_affected()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

fn mask_locked(reply: &mut Reply, access: &[bool; 3]) {
    for slot in &mut reply.slots {
        if !access[usize::from(slot.slot - 1)] {
            slot.state = 0;
        }
    }
}
