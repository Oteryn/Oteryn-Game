use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStep {
    Expire,
    Match,
    HeldCredit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum OperationKind {
    Place = 1,
    Accept = 2,
    Cancel = 3,
    Expire = 4,
    Match = 5,
    HeldCredit = 6,
}
#[derive(Debug, Clone, Copy)]
pub enum Occurrence {
    Command(CommandRef),
    Job {
        offer: MarketOfferId,
        step: JobStep,
        number: u64,
    },
}
impl Occurrence {
    fn key(self) -> Result<Vec<u8>> {
        let mut out = vec![1];
        match self {
            Self::Command(command) => {
                out.push(1);
                out.extend_from_slice(command.game_session_id().as_bytes());
                out.extend_from_slice(&command.command_id().get().to_be_bytes());
            }
            Self::Job {
                offer,
                step,
                number,
            } => {
                if number == 0 {
                    return Err(invalid());
                }
                out.push(2);
                out.extend_from_slice(offer.as_bytes());
                out.push(match step {
                    JobStep::Expire => 1,
                    JobStep::Match => 2,
                    JobStep::HeldCredit => 3,
                });
                out.extend_from_slice(&number.to_be_bytes());
            }
        }
        Ok(out)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct HistoricalOutcome {
    pub transaction_id: [u8; 16],
    pub outcome: Vec<u8>,
}
#[derive(Debug)]
pub enum OperationReadError {
    ConflictingBinding,
    Storage(DurabilityError),
}
impl From<sqlx::Error> for OperationReadError {
    fn from(e: sqlx::Error) -> Self {
        Self::Storage(e.into())
    }
}
impl From<DurabilityError> for OperationReadError {
    fn from(e: DurabilityError) -> Self {
        Self::Storage(e)
    }
}
/// Immutable historical read, not a recovery/session authority constructor. The owner must
/// validate native recovery BEFORE calling and return a match before fresh authority/state.
/// Payload is an opaque recorded outcome; this does not invent the future Market wire codec.
pub async fn read_operation(
    connection: &mut PgConnection,
    world: WorldId,
    occurrence: Occurrence,
    kind: OperationKind,
    actor: Option<CharacterId>,
    binding: &[u8; 32],
) -> std::result::Result<Option<HistoricalOutcome>, OperationReadError> {
    let valid = match occurrence {
        Occurrence::Command(_) => {
            actor.is_some()
                && matches!(
                    kind,
                    OperationKind::Place | OperationKind::Accept | OperationKind::Cancel
                )
        }
        Occurrence::Job { step, .. } => {
            actor.is_none()
                && kind
                    == match step {
                        JobStep::Expire => OperationKind::Expire,
                        JobStep::Match => OperationKind::Match,
                        JobStep::HeldCredit => OperationKind::HeldCredit,
                    }
        }
    };
    if !valid {
        return Err(invalid().into());
    }
    let row=sqlx::query("SELECT world_id::text,transaction_id::text,kind,actor::text,request_binding,outcome FROM game_market_operations WHERE occurrence=$1")
        .bind(occurrence.key()?).fetch_optional(connection).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_actor: Option<String> = row.try_get("actor")?;
    let stored_actor = stored_actor.map(|v| uuid_text(&v)).transpose()?;
    let stored_binding: Vec<u8> = row.try_get("request_binding")?;
    if uuid_text(row.try_get("world_id")?)? != *world.as_bytes()
        || row.try_get::<i16, _>("kind")? != kind as i16
        || stored_actor != actor.map(|v| *v.as_bytes())
        || stored_binding.as_slice() != binding
    {
        return Err(OperationReadError::ConflictingBinding);
    }
    let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
    super::super::item_mint_audit::check_uuid_v7(&transaction_id).map_err(|_| invalid())?;
    let outcome: Vec<u8> = row.try_get("outcome")?;
    if outcome.is_empty() || outcome.len() > 512 {
        return Err(invalid().into());
    }
    Ok(Some(HistoricalOutcome {
        transaction_id,
        outcome,
    }))
}

/// Administrative fixture access to the exact production encoder; never a command grant.
#[cfg(any(test, feature = "test-support"))]
pub fn fixture_occurrence_key(occurrence: Occurrence) -> Result<Vec<u8>> {
    occurrence.key()
}
