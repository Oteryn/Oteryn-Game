//! Read-only trade admission preserves classified refusals and holder availability.
use super::*;

pub(in crate::gameplay_transport) async fn read_retry<T>(
    root: &crate::durability::DurabilityRoot,
    read: impl AsyncFnMut() -> Result<T, CheckError>,
) -> Result<T, ResultCode> {
    read_retry_detailed(root, read)
        .await
        .map_err(|outcome| outcome.code)
}

pub(super) async fn read_retry_detailed<T>(
    root: &crate::durability::DurabilityRoot,
    mut read: impl AsyncFnMut() -> Result<T, CheckError>,
) -> Result<T, CommandOutcome> {
    let budget = super::super::AdmissionRetryBudget::for_foreground_command(root)
        .map_err(|_| CommandOutcome::budget_exhausted())?;
    match budget
        .retry_unavailable(async || match read().await {
            Ok(value) => Ok(Ok(value)),
            Err(CheckError::Refused(reason)) => Ok(Err(reason.into())),
            Err(CheckError::Caused(outcome)) => Ok(Err(outcome)),
            Err(CheckError::Unavailable) => {
                Err(super::super::connection::AdmissionRefusal::Unavailable)
            }
        })
        .await
    {
        Ok(result) => result,
        Err(_) => Err(CommandOutcome::budget_exhausted()),
    }
}
