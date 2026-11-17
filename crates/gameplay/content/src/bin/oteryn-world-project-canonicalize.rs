//! Export an admitted WorldProject/v2 through its native writer into a fresh directory.
use oteryn_gameplay_content::content::{
    CanonicalProjectDocuments, capture_world_project, npc_catalogue_preproduction_limits,
};
use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq, Eq)]
struct Roots {
    input: PathBuf,
    output: PathBuf,
}

fn parse_roots(
    arguments: impl IntoIterator<Item = OsString>,
) -> std::result::Result<Roots, &'static str> {
    let mut arguments = arguments.into_iter();
    let mut input = None;
    let mut output = None;
    while let Some(flag) = arguments.next() {
        let slot = if flag == "--input-root" {
            &mut input
        } else if flag == "--output-root" {
            &mut output
        } else {
            return Err("expected --input-root <package> --output-root <fresh-directory>");
        };
        if slot.is_some() {
            return Err("root option specified more than once");
        }
        let value = arguments.next().ok_or("root option has no path")?;
        if value.is_empty() {
            return Err("root path is empty");
        }
        *slot = Some(PathBuf::from(value));
    }
    Ok(Roots {
        input: input.ok_or("--input-root is required")?,
        output: output.ok_or("--output-root is required")?,
    })
}

fn resolve_parent(root: &Path) -> Result<PathBuf> {
    let name = root.file_name().ok_or("root has no basename")?;
    let parent = root
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    Ok(parent.join(name))
}

fn prepare_roots(roots: Roots) -> Result<Roots> {
    // Resolve parent aliases, retaining the final entry for the native no-follow capture.
    let input = resolve_parent(&roots.input)?;
    if !fs::symlink_metadata(&input)?.is_dir() {
        return Err("input root must be a real directory".into());
    }
    let output = resolve_parent(&roots.output)?;
    if input.starts_with(&output) || output.starts_with(&input) {
        return Err("input and output roots overlap".into());
    }
    match fs::symlink_metadata(&output) {
        Ok(_) => return Err("output root already exists".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(Roots { input, output })
}

fn write_fresh(output: &Path, documents: &CanonicalProjectDocuments) -> Result<()> {
    // Reserve the root exclusively after capture too: a late creator must never be replaced.
    // These locator/byte pairs come only from the native, validated canonical writer.
    fs::create_dir(output)?;
    for (locator, bytes) in documents.documents() {
        let path = output.join(locator);
        fs::create_dir_all(path.parent().ok_or("document has no parent")?)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    Ok(())
}

fn canonicalize(roots: Roots) -> Result<()> {
    let roots = prepare_roots(roots)?;
    let limits = npc_catalogue_preproduction_limits();
    let project = capture_world_project(
        roots.input.parent().ok_or("input root has no parent")?,
        roots
            .input
            .file_name()
            .ok_or("input root has no basename")?,
        limits,
    )?;
    if project.v2().is_none() {
        return Err("input must be a WorldProject/v2 package".into());
    }
    let documents = project.canonical_documents(limits.project)?;
    if documents.documents().len() != 11 {
        return Err("native WorldProject/v2 must contain exactly eleven documents".into());
    }
    write_fresh(&roots.output, &documents)
}

fn main() -> Result<()> {
    canonicalize(parse_roots(std::env::args_os().skip(1))?)?;
    std::io::stdout()
        .lock()
        .write_all(b"WORLD_PROJECT_CANONICALIZE documents=11 status=PASS\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "oteryn-canonicalize-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }

        fn roots(&self) -> Roots {
            Roots {
                input: self.0.join("input"),
                output: self.0.join("output"),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn root_options_accept_either_order() -> Result<()> {
        for arguments in [
            ["--input-root", "input", "--output-root", "output"],
            ["--output-root", "output", "--input-root", "input"],
        ] {
            assert_eq!(
                parse_roots(arguments.map(OsString::from))?,
                Roots {
                    input: "input".into(),
                    output: "output".into(),
                }
            );
        }
        Ok(())
    }

    #[test]
    fn missing_duplicate_empty_and_unknown_options_are_refused() {
        for arguments in [
            vec![],
            vec!["--input-root"],
            vec!["--output-root", "output"],
            vec!["--input-root", "input", "--output-root", ""],
            vec![
                "--input-root",
                "input",
                "--input-root",
                "again",
                "--output-root",
                "output",
            ],
            vec![
                "--input-root",
                "input",
                "--output-root",
                "output",
                "--output-root",
                "again",
            ],
            vec!["--input-root", "input", "--output-root", "output", "extra"],
        ] {
            assert!(parse_roots(arguments.into_iter().map(OsString::from)).is_err());
        }
    }

    #[test]
    fn existing_and_overlapping_outputs_are_refused() -> Result<()> {
        let fixture = Fixture::new()?;
        let input = fixture.0.join("input");
        fs::create_dir(&input)?;
        let existing = fixture.0.join("existing");
        fs::write(&existing, b"retain")?;
        let existing_directory = fixture.0.join("existing-directory");
        fs::create_dir(&existing_directory)?;
        let child = input.join("child");
        for output in [&existing, &existing_directory, &input, &child, &fixture.0] {
            assert!(
                prepare_roots(Roots {
                    input: input.clone(),
                    output: output.clone()
                })
                .is_err()
            );
        }
        assert_eq!(fs::read(existing)?, b"retain");
        assert!(prepare_roots(fixture.roots()).is_ok());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn symlink_aliases_cannot_bypass_root_guards() -> Result<()> {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new()?;
        let roots = fixture.roots();
        fs::create_dir(&roots.input)?;
        let alias = fixture.0.join("alias");
        symlink(&roots.input, &alias)?;
        assert!(
            prepare_roots(Roots {
                input: roots.input.clone(),
                output: alias.join("child")
            })
            .is_err()
        );
        assert!(
            prepare_roots(Roots {
                input: alias.clone(),
                output: roots.output.clone()
            })
            .is_err()
        );
        assert!(
            prepare_roots(Roots {
                input: alias.join(""),
                output: roots.output.clone()
            })
            .is_err()
        );
        symlink(fixture.0.join("absent"), &roots.output)?;
        assert!(prepare_roots(roots).is_err());
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn native_documents() -> Result<CanonicalProjectDocuments> {
        use oteryn_gameplay_content::content::{
            DefinitionIdentityDocument, ItemStackDocument, ProjectDraft, ProjectReferenceRecord,
            ProjectionDocument,
        };
        let limits = npc_catalogue_preproduction_limits().project;
        let legacy = CanonicalProjectDocuments::from_draft(
            ProjectDraft {
                project_revision: "canonicalize-unit-r1".into(),
                package_key: "oteryn:content.canonicalize-unit-proof".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "license:project-owned-v1".into(),
                world_id: "0123456789ab70cd8ef0123456789abc".into(),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: vec![ProjectReferenceRecord::Item {
                    identity: DefinitionIdentityDocument {
                        family: "Item".into(),
                        key: "oteryn:reference.item.canonicalize-unit-proof".into(),
                        revision: "definition-r1".into(),
                    },
                    client_projection: ProjectionDocument::ClientSafe,
                    materializable: true,
                    stack_class: ItemStackDocument::StackCapable,
                    semantics: Default::default(),
                }],
                imports: Vec::new(),
                metadata: Vec::new(),
            },
            limits,
        )?;
        let project = legacy.into_snapshot(limits)?.parse(limits)?;
        Ok(CanonicalProjectDocuments::from_v2_draft(
            project.migrate_to_v2(),
            limits,
        )?)
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_round_trip_preserves_all_eleven_document_bytes() -> Result<()> {
        let fixture = Fixture::new()?;
        let roots = fixture.roots();
        let documents = native_documents()?;
        assert_eq!(documents.documents().len(), 11);
        write_fresh(&roots.input, &documents)?;
        canonicalize(roots)?;
        for (locator, bytes) in documents.documents() {
            assert_eq!(&fs::read(fixture.0.join("input").join(locator))?, bytes);
            assert_eq!(&fs::read(fixture.0.join("output").join(locator))?, bytes);
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_late_output_creator_is_never_overwritten() -> Result<()> {
        let fixture = Fixture::new()?;
        let documents = native_documents()?;
        write_fresh(&fixture.0.join("input"), &documents)?;
        let roots = prepare_roots(fixture.roots())?;
        fs::create_dir(&roots.output)?;
        let sentinel = roots.output.join("sentinel");
        fs::write(&sentinel, b"retain")?;
        assert!(write_fresh(&roots.output, &documents).is_err());
        assert_eq!(fs::read(sentinel)?, b"retain");
        assert_eq!(fs::read_dir(roots.output)?.count(), 1);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn an_invalid_input_never_creates_output() -> Result<()> {
        let fixture = Fixture::new()?;
        fs::create_dir(fixture.0.join("input"))?;
        assert!(canonicalize(fixture.roots()).is_err());
        assert!(!fixture.0.join("output").exists());
        Ok(())
    }
}
