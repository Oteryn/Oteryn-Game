use super::super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, burn_fee_in_transaction,
};
use super::super::item_mint_audit::Type2Transaction;
use super::*;

pub(super) struct TransactionIdentity {
    pub(super) transaction: String,
    pub(super) event: String,
    pub(super) now: i64,
    pub(super) paid: bool,
}

pub(super) fn kind(mutation: &AtelierMutation) -> &'static str {
    match mutation {
        AtelierMutation::Initialize => "GEM_INIT",
        AtelierMutation::SetLock { .. } => "SET_LOCK",
        AtelierMutation::Reveal { .. } => "REVEAL",
        AtelierMutation::SwitchDomain { .. } => "SWITCH_DOMAIN",
        AtelierMutation::Dismantle { .. } => "DISMANTLE",
        AtelierMutation::GradeUp { .. } => "GRADE_UP",
    }
}

fn quality_key(quality: GemQuality) -> &'static str {
    match quality {
        GemQuality::Lesser => "lesser",
        GemQuality::Regular => "regular",
        GemQuality::Greater => "greater",
    }
}
fn fragment_key(kind: FragmentKind) -> &'static str {
    match kind {
        FragmentKind::Basic => "oteryn:item.tibia.i46625",
        FragmentKind::Supreme => "oteryn:item.tibia.i46626",
    }
}
fn number(value: &serde_json::Value) -> std::result::Result<u64, DurabilityError> {
    value.as_u64().ok_or(DurabilityError::InvalidStoredState)
}

async fn draw_root(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    purpose: &str,
    revision: &str,
) -> std::result::Result<GameplayDecisionRoot, DurabilityError> {
    let secret: Vec<u8> =
        sqlx::query_scalar("SELECT secret FROM game_atelier_decision_root WHERE singleton=true")
            .fetch_one(&mut ***tx)
            .await?;
    if secret.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut hash = Sha256::new();
    hash.update(secret);
    hash.update(purpose.as_bytes());
    hash.update(fence.character_id.as_bytes());
    hash.update(revision.as_bytes());
    Ok(GameplayDecisionRoot::from_bytes(hash.finalize().into()))
}
fn draw(
    root: &GameplayDecisionRoot,
    occurrence: [u8; 16],
    purpose: &str,
    index: u64,
    bound: u64,
) -> std::result::Result<u64, DurabilityError> {
    if bound == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    deterministic_decision_u64(
        root,
        DecisionOccurrenceId::from_bytes(occurrence),
        purpose,
        index,
    )
    .map(|v| v % bound)
    .map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &AtelierRequest<'_>,
    state: &mut AtelierState,
) -> std::result::Result<bool, DurabilityError> {
    let data = WheelGemData::embedded().map_err(|_| DurabilityError::InvalidStoredState)?;
    let catalogue = &data.gems()["catalogue"];
    let family = &data.gems()["vocations"]
        [base_vocation(&state.vocation).ok_or(DurabilityError::InvalidStoredState)?];
    let mut preview = state.clone();
    let vessels = super::super::character_wheel::read_vessels(tx, fence.character_id).await?;
    for gem in preview.gems.values_mut() {
        gem.vessel = vessels.contains(&Some(gem.id.to_be_bytes()));
    }
    // These are transition inputs only. Durable assets remain exclusively in item/bank custody.
    if preview.gold != 0 || !preview.unrevealed.is_empty() || !preview.fragments.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut fee = 0;
    let mut burn: Option<(String, u32)> = None;
    let mut mint: Option<(FragmentKind, u32)> = None;
    let action = match request.mutation {
        AtelierMutation::Initialize => return Err(DurabilityError::InvalidStoredState),
        AtelierMutation::SetLock { gem, locked } => AtelierAction::Lock {
            id: u128::from_be_bytes(gem),
            locked,
        },
        AtelierMutation::SwitchDomain { gem } => {
            let Some(gem) = state.gems.get(&u128::from_be_bytes(gem)) else {
                return Ok(false);
            };
            fee = number(&catalogue["atelier"]["fees"]["switch_domain"][quality_key(gem.quality)])?;
            AtelierAction::SwitchDomain { id: gem.id }
        }
        AtelierMutation::GradeUp { kind, mod_id } => {
            let grade = state.grades.get(&(kind, mod_id)).copied().unwrap_or(0);
            if grade >= 3 {
                return Ok(false);
            }
            let class = if kind == FragmentKind::Basic {
                "basic"
            } else {
                "supreme"
            };
            let cost = &catalogue["grade_costs"][usize::from(grade)][class];
            fee = number(&cost["gold"])?;
            let count = u32::try_from(number(&cost["fragments"])?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            burn = Some((fragment_key(kind).into(), count));
            preview.fragments.insert(kind, count);
            AtelierAction::GradeUp { kind, mod_id }
        }
        AtelierMutation::Dismantle { gem } => {
            let Some(gem) = state.gems.get(&u128::from_be_bytes(gem)) else {
                return Ok(false);
            };
            let row = &catalogue["atelier"]["fragment_yields"][quality_key(gem.quality)];
            let low = number(&row["revealed"][0])?;
            let high = number(&row["revealed"][1])?;
            let root = draw_root(tx, fence, "gem_dismantle", data.revision()).await?;
            let amount = u32::try_from(
                low + draw(
                    &root,
                    request.occurrence,
                    "gem_dismantle",
                    0,
                    high.checked_sub(low)
                        .and_then(|v| v.checked_add(1))
                        .ok_or(DurabilityError::InvalidStoredState)?,
                )?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            let kind = if row["fragment"] == "basic" {
                FragmentKind::Basic
            } else {
                FragmentKind::Supreme
            };
            mint = Some((kind, amount));
            AtelierAction::Dismantle {
                id: gem.id,
                yield_amount: amount,
            }
        }
        AtelierMutation::Reveal { quality } => {
            let root = draw_root(tx, fence, "gem_reveal", data.revision()).await?;
            let domains = [
                WheelDomain::Green,
                WheelDomain::Red,
                WheelDomain::Purple,
                WheelDomain::Blue,
            ];
            let domain = domains[draw(&root, request.occurrence, "gem_reveal", 0, 4)? as usize];
            let count = match quality {
                GemQuality::Lesser => 1,
                GemQuality::Regular => 2,
                GemQuality::Greater => 3,
            };
            let mut mods = Vec::new();
            for (index, list) in [
                "basic_mods_position_1",
                "basic_mods_position_2",
                "supreme_mods",
            ]
            .into_iter()
            .take(count)
            .enumerate()
            {
                let choices: Vec<u16> = family[list]
                    .as_array()
                    .ok_or(DurabilityError::InvalidStoredState)?
                    .iter()
                    .filter_map(|v| v.as_u64().and_then(|v| u16::try_from(v).ok()))
                    .filter(|id| index != 1 || !mods.contains(id))
                    .collect();
                let index = draw(
                    &root,
                    request.occurrence,
                    "gem_reveal",
                    index as u64 + 1,
                    choices.len() as u64,
                )?;
                mods.push(choices[index as usize]);
            }
            fee = number(&catalogue["atelier"]["fees"]["reveal"][quality_key(quality)])?;
            let key = family["gem_family"]["items"][quality_key(quality)]
                .as_str()
                .ok_or(DurabilityError::InvalidStoredState)?;
            burn = Some((key.into(), 1));
            preview.unrevealed.insert(quality, 1);
            AtelierAction::Reveal {
                id: u128::from_be_bytes(request.occurrence),
                quality,
                domain,
                mods,
            }
        }
    };
    preview.gold = fee;
    if preview
        .commit(request.expected_atelier_revision, action)
        .is_err()
    {
        return Ok(false);
    }
    if fee > 0 || burn.is_some() || mint.is_some() {
        let Some(resources) = &request.resources else {
            return Ok(false);
        };
        let (transaction,event,now): (String,String,i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
        let identity = TransactionIdentity {
            transaction,
            event,
            now,
            paid: fee > 0,
        };
        // The item lines and fee share one logical transaction, event and physical commit.
        let proof = if burn.is_some() || mint.is_some() {
            let Some(proof) = custody::apply(
                tx,
                fence,
                request,
                resources,
                &identity,
                burn.as_ref().map(|(key, n)| (key.as_str(), *n)),
                mint.map(|(kind, n)| (fragment_key(kind), n)),
            )
            .await?
            else {
                return Ok(false);
            };
            proof
        } else {
            Vec::new()
        };
        if fee > 0 {
            let cause = match request.mutation {
                AtelierMutation::Reveal { .. } => FeeBurnCause::GemReveal {
                    occurrence: request.occurrence,
                    proof,
                },
                AtelierMutation::SwitchDomain { .. } => FeeBurnCause::GemSwitchDomain {
                    occurrence: request.occurrence,
                },
                AtelierMutation::GradeUp { .. } => FeeBurnCause::GemGradeUp {
                    occurrence: request.occurrence,
                    proof,
                },
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            let fee_request = FeeBurnRequest {
                cause,
                fee_gold_units: fee,
                transaction_id: uuid_text(&identity.transaction)?,
                event_id: uuid_text(&identity.event)?,
                occurred_at_unix_ms: identity.now,
                server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
                change: resources.coins.clone(),
            };
            match burn_fee_in_transaction(tx, fence, &fee_request).await {
                Ok(FeeBurnOutcome::Burned(_)) => (),
                Ok(FeeBurnOutcome::AlreadyBurned(_)) => {
                    return Err(DurabilityError::InvalidStoredState);
                }
                Err(FeeBurnError::Unavailable(e)) => return Err(e),
                Err(_) => return Ok(false),
            }
        }
    }
    for gem in preview.gems.values_mut() {
        gem.vessel = false;
    }
    preview.gold = 0;
    preview.fragments.clear();
    preview.unrevealed.clear();
    *state = preview;
    Ok(true)
}
