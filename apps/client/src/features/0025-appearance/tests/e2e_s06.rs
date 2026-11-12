//! S06 replay: the native appearance catalogue snapshots, the APPEARANCE_CHANGE commit result and
//! the unearned-addon refusal, applied by the production client reducers `catalogue` and `apply`
//! (`protocol.rs`); the request comes from the production `command`. Captures: `s06-before.bin`,
//! `s06-after.bin` (the `gated_snapshot` payloads), `s06-commit.bin`, `s06-refusal.bin` (command
//! results) and `s06-expected.txt`. Without captures inline payloads built with the production
//! encoders are used.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;

struct Capture {
    before: Vec<u8>,
    after: Vec<u8>,
    commit: Vec<u8>,
    refusal: Vec<u8>,
    outfit: String,
    colours: [u16; 4],
    committed: u64,
    content_revision: String,
}

fn catalogue_of(
    outfit: &str,
    root_revision: u64,
    selected: Option<wire::Selected>,
) -> wire::Catalogue {
    wire::Catalogue {
        context: wire::Context {
            content_generation: [1; 32],
            cosmetics_digest: [2; 32],
        },
        root_revision,
        selected,
        palette: vec![[0, 0, 0]; 6],
        outfits: vec![wire::OutfitOption {
            key: outfit.to_owned(),
            look_type: 128,
            permitted_addons: 3,
        }],
        mounts: vec!["oteryn:content.mount/starter".to_owned()],
    }
}

fn snapshot_bytes(catalogue: wire::Catalogue) -> Result<Vec<u8>, String> {
    wire::Snapshot {
        catalogue,
        bindings: Vec::new(),
        high_water: 0,
    }
    .encode()
    .map_err(|e| format!("{e:?}"))
}

fn capture() -> Result<Capture, String> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let read = |name: &str| std::fs::read(root.join(name)).map_err(|e| format!("{name}: {e}"));
        let expected = std::fs::read_to_string(root.join("s06-expected.txt"))
            .map_err(|e| format!("s06-expected.txt: {e}"))?;
        let value = |key: &str| {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or_else(|| format!("{key}= missing"))
        };
        let colours = value("colours")?
            .split(',')
            .map(|c| c.parse::<u16>().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Capture {
            before: read("s06-before.bin")?,
            after: read("s06-after.bin")?,
            commit: read("s06-commit.bin")?,
            refusal: read("s06-refusal.bin")?,
            outfit: value("outfit")?.to_owned(),
            colours: colours.try_into().map_err(|_| "four colours".to_owned())?,
            committed: value("committed")?.parse().map_err(|e| format!("{e}"))?,
            content_revision: value("content_revision")?.to_owned(),
        });
    }
    let outfit = "oteryn:content.outfit/starter";
    let colours = [4, 3, 2, 1];
    let selected = wire::Selected {
        selection: wire::Selection {
            outfit_key: outfit.to_owned(),
            colours,
            addons: 0,
            mount_key: None,
        },
        revision: 8,
        provenance: "player".to_owned(),
    };
    Ok(Capture {
        before: snapshot_bytes(catalogue_of(outfit, 7, None))?,
        after: snapshot_bytes(catalogue_of(outfit, 8, Some(selected)))?,
        commit: wire::ChangeResult::Committed(8).encode().map_err(|e| format!("{e:?}"))?,
        refusal: wire::ChangeResult::Rejected.encode().map_err(|e| format!("{e:?}"))?,
        outfit: outfit.to_owned(),
        colours,
        committed: 8,
        content_revision: "rev-1".to_owned(),
    })
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending(panel: &Panel) -> Result<(Panel, FeatureCommand), String> {
    let command = command(panel, Intent::Apply)?;
    let mut next = panel.clone();
    assert!(next.begin(command.kind));
    Ok((next, command))
}

#[test]
fn e2e_s06_outfit_commit_and_unearned_refusal_replay() -> Result<(), String> {
    let capture = capture()?;
    let before = wire::Snapshot::decode(&capture.before).map_err(|e| format!("{e:?}"))?;
    let after = wire::Snapshot::decode(&capture.after).map_err(|e| format!("{e:?}"))?;
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    catalogue(&mut panel, before.catalogue.clone());
    assert!(panel.summary.contains(&format!(
        "Outfit: {}; allowed addon mask: ",
        capture.outfit
    )));

    let [head, body, legs, feet] = capture.colours;
    panel.fields = [
        capture.outfit.clone(),
        "0".into(),
        String::new(),
        format!("{head},{body},{legs},{feet}"),
        capture.content_revision.clone(),
    ];
    let (mut applying, change) = pending(&panel)?;
    assert_eq!(
        (change.capability, change.kind),
        (wire::CAPABILITY_APPEARANCE_V1, wire::COMMAND_TYPE_APPEARANCE_CHANGE)
    );
    let sent = wire::Change {
        context: before.catalogue.context,
        expected_root_revision: before.catalogue.root_revision,
        prior: before.catalogue.selected.clone(),
        selection: wire::Selection {
            outfit_key: capture.outfit.clone(),
            colours: capture.colours,
            addons: 0,
            mount_key: None,
        },
        content_revision: capture.content_revision.clone(),
    };
    assert_eq!(change.payload, sent.encode().map_err(|e| format!("{e:?}"))?);
    apply(&mut applying, &change, &result(CommandStatus::Accepted, &capture.commit));
    assert!(!applying.pending);
    assert_eq!(
        applying.feedback,
        format!(
            "Appearance committed at revision {}; catalogue updates follow",
            capture.committed
        )
    );
    catalogue(&mut applying, after.catalogue.clone());
    assert!(applying.summary.starts_with(&format!(
        "Current outfit: {}\r\nAddons: 0\r\nMount: None\r\nColours: [{head}, {body}, {legs}, {feet}]\r\n",
        capture.outfit
    )));
    let confirmed = applying.confirmed.clone().ok_or("selection not confirmed")?;
    assert_eq!(confirmed.selection.outfit_key, capture.outfit);
    assert_eq!(confirmed.selection.colours, capture.colours);
    assert_eq!(after.catalogue.root_revision, capture.committed);

    // The refusal leaves the confirmed selection and the shown catalogue unchanged.
    for (status, payload, message) in [
        (CommandStatus::Rejected, capture.refusal.clone(), "The server refused the appearance request"),
        (CommandStatus::Accepted, capture.refusal.clone(), "The server refused the appearance request"),
        (CommandStatus::Accepted, vec![0xff, 0x00], "Invalid appearance reply"),
    ] {
        let (mut bad, change) = pending(&applying)?;
        apply(&mut bad, &change, &result(status, &payload));
        assert!(!bad.pending);
        assert_eq!(bad.feedback, message);
        assert_eq!(bad.confirmed, applying.confirmed);
        assert_eq!(bad.summary, applying.summary);
        assert_eq!(bad.catalogue, applying.catalogue);
    }
    Ok(())
}
