//! W09 client-to-server chain: the real client `oteryn_session::Session` admits through the
//! production `admit_frame`, joins from `serve_admitted`, sends the player's USE of the Drome
//! Cube it carries and applies the server's inventory delta in its own decoder; a fresh
//! admission then reads the durable inventory back through the join snapshot.
use super::*;
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::connection::{
    ConnectionEnd, ConnectionIdentifiers, IDLE_LIVENESS, SERVER_SCHEMA_REVISION, admit_frame,
    serve_admitted,
};
use crate::gameplay_transport::tcp_tls::read_frame;
use oteryn_dev_client::{Admission, CLIENT_SUPPORTED_CAPABILITIES, CharacterInventory, Session};
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use tokio::io::DuplexStream;

/// The production owner with an admission decision: the fixture's admitted session.
struct SessionOwner<'a, 'b, 'f, 's> {
    owner: &'a ComposedFreshAdmission<'b, 'f, 's>,
    admitted: AdmittedSession,
}

impl FreshAdmissionAuthority for SessionOwner<'_, '_, '_, '_> {
    /// The production offer of a shipped bundle World. The fixture's World has no bundle map, and
    /// its own offer lacks Item use (15), which gates the Cube route in `routing/use_item.rs`;
    /// selection itself still runs through the production `admit_frame`.
    fn offered_capabilities(
        &self,
    ) -> &'static [crate::gameplay_transport::capabilities::OfferedCapability] {
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES
    }
    fn take_tutorial_notice(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        WireOwner(self.owner).take_tutorial_notice(session)
    }
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        WireOwner(self.owner).observe(actor).await
    }
    async fn admit(
        &self,
        _: crate::gameplay_transport::connection::FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, crate::gameplay_transport::connection::AdmissionRefusal> {
        Ok(self.admitted)
    }
    async fn observe_character_inventory(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<crate::gameplay_transport::item_view::InventoryItems> {
        WireOwner(self.owner)
            .observe_character_inventory(actor, session)
            .await
    }
    async fn observe_container(
        &self,
        actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<crate::gameplay_transport::container_view::ContainerObservation> {
        WireOwner(self.owner)
            .observe_container(actor, fence, target)
            .await
    }
    async fn use_drome_cube(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        WireOwner(self.owner)
            .use_drome_cube(actor, command, intent, source)
            .await
    }
}

struct Identifiers(GameSessionId);
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        Some(self.0)
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[109; 16]).ok()
    }
}

/// The listener's sequence for one connection: entry frame, admission, then the served session.
async fn serve(
    owner: &SessionOwner<'_, '_, '_, '_>,
    session: GameSessionId,
    mut server: DuplexStream,
) -> TestResult<AdmittedSession> {
    let frame = read_frame(&mut server).await?;
    let admitted = admit_frame(&mut server, &frame, owner, &Identifiers(session))
        .await
        .map_err(|end| format!("admission refused: {end:?}"))?;
    match serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await {
        ConnectionEnd::AdmittedThenDisconnected(retained) => Ok(retained),
        end => Err(format!("served session ended with {end:?}").into()),
    }
}

/// The client's own capabilities this chain needs: item view/move (4), its prerequisite
/// world-spatial entities (6), Item use (15) and tutorial notices (31). The full client set selects domains
/// whose join observations this owner does not compose, so the test advertises this subset.
const NEEDED: [u32; 4] = [4, 6, 15, 31];

async fn admit(client: DuplexStream) -> TestResult<Session<DuplexStream>> {
    let supported: Vec<u32> = CLIENT_SUPPORTED_CAPABILITIES
        .iter()
        .copied()
        .filter(|id| NEEDED.contains(id))
        .collect();
    assert_eq!(supported, NEEDED, "the real client supports every needed capability");
    let character = oteryn_protocol_oteryn::CharacterId::decode(&id(41)).map_err(debug)?;
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: SERVER_SCHEMA_REVISION,
            character_id: character,
            admission_material: b"w09-session-client",
            client_build_id: "w09-session-client",
            supported_capabilities: &supported,
            deadline: Duration::from_secs(10),
        },
    )
    .await
    .map_err(debug)?;
    assert!(
        NEEDED.iter().all(|id| session.is_selected(*id)),
        "the server selects every capability the client asked for"
    );
    Ok(session)
}

/// Item quantities by definition, as the client's own inventory state holds them.
fn quantities(inventory: Option<&CharacterInventory>) -> TestResult<BTreeMap<NonZeroU32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &inventory.ok_or("client holds no inventory")?.entries {
        *result.entry(entry.item_definition_ref).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
}

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let reference = |key: &str| -> TestResult<NonZeroU32> {
        Ok(index
            .definition_ref("Item", key, index.revision(key).ok_or("revision")?)
            .ok_or("definition")?)
    };
    let cube = reference(DROME_CUBE_KEY)?;
    let selected = reward_index(id(110), 1)
        .and_then(reward)
        .ok_or("source-unit reward")?;
    let flask = reference(selected.production_key())?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(cube),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;

    let first = SessionOwner {
        owner,
        admitted: cases::admitted(owner, actor, session)?,
    };
    let (server, client) = tokio::io::duplex(1 << 20);
    let (served, used) = tokio::join!(serve(&first, session, server), async {
        let mut client = admit(client).await?;
        let before = quantities(client.inventory())?;
        assert_eq!(
            before.get(&cube),
            Some(&1),
            "join snapshot shows the carried cube"
        );
        assert_eq!(before.get(&flask), None);
        let reply = client.world_context_use(&payload).await.map_err(debug)?;
        assert_eq!(reply.status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
                .map_err(debug)?,
            UseDisposition::Committed
        );
        // The refreshed inventory is a pushed delta: service the connection until the client
        // decoder has applied it, bounded by a deadline.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut after = quantities(client.inventory())?;
        while after.contains_key(&cube) {
            if std::time::Instant::now() >= deadline {
                return Err("client never applied the post-use inventory".into());
            }
            client
                .service_liveness(Duration::from_millis(200))
                .await
                .map_err(debug)?;
            after = quantities(client.inventory())?;
        }
        assert_eq!(after.get(&flask), Some(&1), "client shows the rolled reward");
        let mut expected = before;
        expected.remove(&cube);
        expected.insert(flask, 1);
        assert_eq!(after, expected, "only the cube and its reward changed");
        TestResult::Ok(after)
    });
    served?;
    let after = used?;
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    assert_eq!(source_quantity(harness).await?, 0);

    // A fresh admission reads the custody from PostgreSQL into the client's join snapshot.
    let second = SessionOwner {
        owner,
        admitted: cases::admitted(owner, actor, session)?,
    };
    let (server, client) = tokio::io::duplex(1 << 20);
    let (served, relogged) = tokio::join!(serve(&second, session, server), async {
        let client = admit(client).await?;
        quantities(client.inventory())
    });
    served?;
    assert_eq!(
        relogged?,
        after,
        "the used cube and its reward are durable"
    );
    assert_eq!(harness.count("game_drome_cube_uses").await?, 1);
    eprintln!("DROME_CUBE SESSION_CLIENT_CASE_COMPLETED");
    Ok(())
}
