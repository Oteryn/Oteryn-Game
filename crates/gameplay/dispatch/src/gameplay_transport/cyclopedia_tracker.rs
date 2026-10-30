use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::cyclopedia_tracker::TrackerKind;
use oteryn_protocol_oteryn::cyclopedia_tracker::TrackerRequest;

macro_rules! authority_methods {
    () => {
        fn cyclopedia_tracker(&self, _admitted: AdmittedSession, _kind: crate::durability::cyclopedia_tracker::TrackerKind, _request: oteryn_protocol_oteryn::cyclopedia_tracker::TrackerRequest) -> impl std::future::Future<Output = Option<Vec<String>>> { async { None } }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn cyclopedia_tracker(&self, admitted: AdmittedSession, kind: crate::durability::cyclopedia_tracker::TrackerKind, request: oteryn_protocol_oteryn::cyclopedia_tracker::TrackerRequest) -> Option<Vec<String>> {
            self.execute_cyclopedia_tracker(admitted, kind, request).await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn execute_cyclopedia_tracker(&self, admitted: AdmittedSession, kind: TrackerKind, request: TrackerRequest) -> Option<Vec<String>> {
        let fence = super::character_gameplay_fence::current(self, admitted.game_session_id, false).await?;
        let _actor = admitted.runtime_actor?;
        let _permit = self.spell_lane_permit().await?;
        self.root.update_cyclopedia_tracker(self.character, self.holder, fence, kind, request.action, &request.creature_key).await.ok()
    }
}
