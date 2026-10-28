//! Live Exaltation Forge composition and capability offer.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::character_forge_dust::ForgeDustLimitError;
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::item_forge::CAPABILITY_ITEM_FORGE_V1;

macro_rules! authority_methods {
    () => {
        fn item_forge_available(&self) -> bool {
            false
        }
        fn item_forge(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _request: oteryn_protocol_oteryn::item_forge::Request,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { Some(Vec::new()) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn item_forge_available(&self) -> bool {
            true
        }
        async fn item_forge(
            &self,
            admitted: AdmittedSession,
            command_id: u64,
            request: oteryn_protocol_oteryn::item_forge::Request,
        ) -> Option<Vec<u8>> {
            item_forge::execute(self, admitted, command_id, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: oteryn_protocol_oteryn::item_forge::Request,
) -> Option<Vec<u8>> {
    let Some(fence) = admitted.item_fence else {
        return Some(Vec::new());
    };
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    match owner
        .root
        .execute_item_forge(owner.character, owner.holder, fence, command, request)
        .await
    {
        Ok(result) => serde_json::to_vec(&result).ok(),
        Err(ForgeDustLimitError::Unavailable(_)) => None,
        Err(_) => Some(Vec::new()),
    }
}

pub(super) fn offered(
    base: &[super::capabilities::OfferedCapability],
    available: bool,
) -> Vec<super::capabilities::OfferedCapability> {
    let mut out: Vec<_> = base
        .iter()
        .copied()
        .filter(|v| v.id != CAPABILITY_ITEM_FORGE_V1)
        .collect();
    if available {
        let at = out.partition_point(|v| v.id < CAPABILITY_ITEM_FORGE_V1);
        out.insert(
            at,
            super::capabilities::OfferedCapability {
                id: CAPABILITY_ITEM_FORGE_V1,
                requires: &[14],
            },
        );
    }
    out
}
