//! The existing ordinary USE command routes Drome items without another wire command.

#[path = "kooldown_aid.rs"]
mod kooldown_aid;

macro_rules! authority_methods {
    () => {
        fn use_drome_cube(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _intent: oteryn_protocol_oteryn::world_object::UseIntent,
            _source: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> impl std::future::Future<
            Output = Result<Option<$crate::gameplay_transport::connection::UseOutcome>, ()>,
        > {
            async { Ok(None) }
        }
    };
}
pub(in crate::gameplay_transport) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn use_drome_cube(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            intent: oteryn_protocol_oteryn::world_object::UseIntent,
            source: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> Result<Option<$crate::gameplay_transport::connection::UseOutcome>, ()> {
            if let Some(outcome) = self
                .use_kooldown_aid_impl(actor, command, &intent, source)
                .await?
            {
                return Ok(Some(outcome));
            }
            self.use_drome_cube_impl(actor, command, intent, source)
                .await
        }
    };
}
pub(in crate::gameplay_transport) use composed_methods;
