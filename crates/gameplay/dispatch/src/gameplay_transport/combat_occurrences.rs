use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{
    character_info::PageRequest,
    cyclopedia_character::kill_credits,
    death_protection::{DeathOccurrence, DeathOccurrenceReply},
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn death_occurrence_wire(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        let fence = super::death_protection::query_fence(admitted)?;
        let death = self
            .root
            .read_latest_character_death(self.character, self.holder, fence)
            .await
            .ok()?;
        DeathOccurrenceReply(death.map(|(death, _, final_blow)| DeathOccurrence {
            occurrence: *death.occurrence.as_bytes(),
            final_blow,
        }))
        .encode()
    }

    pub(super) async fn player_kill_credits_wire(
        &self,
        admitted: AdmittedSession,
        page: PageRequest,
    ) -> Option<Vec<u8>> {
        let fence = super::death_protection::query_fence(admitted)?;
        let credits = self
            .root
            .read_player_kill_credits(self.character, self.holder, fence, page)
            .await
            .ok()?;
        kill_credits::encode_reply(&credits)
    }
}

macro_rules! authority_methods {
    () => {
        fn death_occurrence(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }

        fn player_kill_credits(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _page: oteryn_protocol_oteryn::character_info::PageRequest,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn death_occurrence(&self, admitted: connection::AdmittedSession) -> Option<Vec<u8>> {
            self.death_occurrence_wire(admitted).await
        }

        async fn player_kill_credits(
            &self,
            admitted: connection::AdmittedSession,
            page: oteryn_protocol_oteryn::character_info::PageRequest,
        ) -> Option<Vec<u8>> {
            self.player_kill_credits_wire(admitted, page).await
        }
    };
}
pub(super) use composed_methods;
