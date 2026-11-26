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
            _definition: Option<std::num::NonZeroU32>,
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
            definition: Option<std::num::NonZeroU32>,
        ) -> Result<Option<$crate::gameplay_transport::connection::UseOutcome>, ()> {
            self.dispatch_drome_use(actor, command, intent, source, definition)
                .await
        }
    };
}
pub(in crate::gameplay_transport) use composed_methods;
