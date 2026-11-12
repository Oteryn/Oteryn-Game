//! Read-only trade admission preserves classified refusals and holder availability.
use super::*;

pub(in crate::gameplay_transport) async fn read_retry<T>(
    root: &crate::durability::DurabilityRoot,
    mut read: impl AsyncFnMut() -> Result<T, CheckError>,
) -> Result<T, ResultCode> {
    let budget = super::super::AdmissionRetryBudget::for_foreground_command(root)
        .map_err(|_| ResultCode::Rejected)?;
    match budget
        .retry_unavailable(async || match read().await {
            Ok(value) => Ok(Ok(value)),
            Err(CheckError::Refused(reason)) => Ok(Err(reason)),
            Err(CheckError::Unavailable) => {
                Err(super::super::connection::AdmissionRefusal::Unavailable)
            }
        })
        .await
    {
        Ok(result) => result,
        Err(_) => Err(ResultCode::Rejected),
    }
}
