//! Synthetic unit coverage for appearance changes and malformed mappings.
use super::*;
use oteryn_protocol_oteryn::appearance;

const GOLDEN_SNAPSHOT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
));

struct Capture {
    snapshot: Vec<u8>,
    revision: u64,
    mapping: Vec<u8>,
    identity: [u8; 16],
    generation: u64,
    baseline_ref: u32,
    expected: appearance::Look,
}

fn fixture() -> Result<Capture, BoxError> {
    let mut snapshot =
        appearance::Snapshot::decode(GOLDEN_SNAPSHOT).map_err(|e| format!("{e:?}"))?;
    snapshot.catalogue.context.content_generation = CONTENT_GENERATION;
    snapshot.bindings[0].context = snapshot.catalogue.context;
    snapshot.bindings[0].entity.identity = OWN;
    let mut binding = snapshot.bindings[0].clone();
    binding.root_revision = 2;
    binding.look = appearance::Look::Outfit {
        look_type: 111,
        colours: [0, 0, 0, 0],
        addons: 0,
    };
    let expected = binding.look.clone();
    let mapping = appearance::Mappings {
        context: snapshot.catalogue.context,
        upserts: vec![binding],
        forgets: Vec::new(),
    }
    .encode()
    .map_err(|e| format!("{e:?}"))?;
    Ok(Capture {
        snapshot: snapshot.encode().map_err(|e| format!("{e:?}"))?,
        revision: 1,
        mapping,
        identity: OWN,
        generation: snapshot.bindings[0].entity.generation,
        baseline_ref: snapshot.bindings[0].appearance_ref,
        expected,
    })
}

#[test]
fn creature_illusion_mapping_rejects_malformed_update() -> Result<(), BoxError> {
    block_on(async {
        let capture = fixture()?;
        let snapshot =
            appearance::Snapshot::decode(&capture.snapshot).map_err(|e| format!("{e:?}"))?;
        let baseline = snapshot
            .bindings
            .iter()
            .find(|binding| binding.entity.identity == capture.identity)
            .ok_or("c17c own binding absent from the appearance snapshot")?
            .clone();
        assert_eq!(baseline.appearance_ref, capture.baseline_ref);
        assert_ne!(baseline.look, capture.expected);
        let mut entity = own(at(0, 0));
        entity.entity.identity = capture.identity;
        entity.entity.generation = capture.generation;
        let EntityDetail::Actor { appearance_ref, .. } = &mut entity.detail else {
            return Err("actor".into());
        };
        *appearance_ref = capture.baseline_ref;
        let spatial = (
            SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
                content_generation: CONTENT_GENERATION,
                actor_position: at(0, 0),
                own_identity: capture.identity,
                entities: vec![entity],
            })
            .map_err(|e| format!("{e:?}"))?,
        );
        let domain = (21, capture.revision, 1, capture.snapshot.clone());
        let mapping = capture.mapping.clone();
        let revision = capture.revision;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[6, 25],
                &[],
                spatial,
                &[domain],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(1, 41, 21, revision, revision + 1, 1, &mapping)?,
            )
            .await?;
            std::future::pending::<()>().await;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert_eq!(
            session.visible_appearances()[0].look.as_ref(),
            Some(&baseline.look)
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while session.visible_appearances()[0].look.as_ref() != Some(&capture.expected) {
            if tokio::time::Instant::now() >= deadline {
                return Err("C17c illusion look never reached the visible appearance".into());
            }
            session.service_liveness(Duration::from_millis(50)).await?;
        }
        assert_eq!(
            session.visible_appearances()[0].look.as_ref(),
            Some(&capture.expected)
        );

        // A malformed Mappings delta is rejected and leaves the illusion look unchanged.
        let malformed = encode_state_delta(1, 42, 21, revision + 1, revision + 2, 1, &[0xff])?;
        let envelope = decode_wire_envelope(&malformed)?;
        assert!(session.apply_state_delta(42, envelope.payload()).is_err());
        assert_eq!(
            session.visible_appearances()[0].look.as_ref(),
            Some(&capture.expected)
        );
        peer.abort();
        assert!(peer.await.is_err());
        Ok(())
    })?
}
