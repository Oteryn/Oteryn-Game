//! C17c client replay: Creature Illusion changes the caster's own look. The owning client state is
//! `AppearanceState` (`crates/session/src/appearance_state.rs`), driven through `Session` and
//! exposed by `Session::visible_appearances`.
//!
//! The raw appearance snapshot is the captured server payload. Appearance Mappings are a
//! session-owned domain delta, so the row re-encodes the applied value with the production
//! encoder (the C09/I11 practice).
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

fn look(text: &str) -> Result<appearance::Look, BoxError> {
    let values = text
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let [look_type, c0, c1, c2, c3, addons] = values.as_slice() else {
        return Err("C17c expected look needs six numbers".into());
    };
    Ok(appearance::Look::Outfit {
        look_type: *look_type,
        colours: [
            u16::try_from(*c0)?,
            u16::try_from(*c1)?,
            u16::try_from(*c2)?,
            u16::try_from(*c3)?,
        ],
        addons: u8::try_from(*addons)?,
    })
}

fn identity(value: &str) -> Result<[u8; 16], BoxError> {
    if value.len() != 32 {
        return Err("invalid C17c identity length".into());
    }
    let mut identity = [0_u8; 16];
    for (index, byte) in identity.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(identity)
}

fn fixture() -> Result<Capture, BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let expected = std::fs::read_to_string(path.join("c17c-expected.txt"))?;
        let value = |key: &str| -> Result<&str, BoxError> {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .ok_or_else(|| format!("c17c expected lacks {key}").into())
        };
        return Ok(Capture {
            snapshot: std::fs::read(path.join("c17c-snapshot.bin"))?,
            revision: value("revision=")?.parse()?,
            mapping: std::fs::read(path.join("c17c-mapping.bin"))?,
            identity: identity(value("identity=")?)?,
            generation: value("generation=")?.parse()?,
            baseline_ref: value("ref=")?.parse()?,
            expected: look(value("look=")?)?,
        });
    }
    let mut snapshot =
        appearance::Snapshot::decode(GOLDEN_SNAPSHOT).map_err(|e| format!("{e:?}"))?;
    snapshot.catalogue.context.content_generation = [0x73; 32];
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
fn e2e_c17c_creature_illusion_replay() -> Result<(), BoxError> {
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
                content_generation: snapshot.catalogue.context.content_generation,
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
