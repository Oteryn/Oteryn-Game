//! Startup selectors must reach the compiled real World without rewriting source B3 files.
use super::TestResult;
use oteryn_world_bundle_compiler::{bundle, compile, source};

#[test]
fn questpc5_real_world_startup_selectors_reach_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_startup_bindings",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED real_world_startup_selectors {:?}",
            started.elapsed()
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let project = source::project_regions(&root, &[(125, 121), (125, 122), (125, 126)])?;
        let (registry, palette) = source::registry(&root, &project)?;
        let compiled = compile::compile(
            &compile::Input {
                regions: &project.regions,
                palette: &palette,
                identity: bundle::Identity::default(),
                world: source::world_extent(&root)?,
                build_class: bundle::BuildClass::NonProduction,
                draft_areas: vec![],
                families: &project.families,
            },
            &registry,
        )?;
        let data = bundle::read(&compiled.bytes)?;
        for (x, y, floor, appearance, unique) in [
            (32157, 31126, -9, 1949, 3059),
            (32157, 31126, -10, 1949, 3060),
            (32157, 31126, -11, 1949, 3061),
            (32211, 31273, -10, 4285, 20003),
            (32225, 32285, -10, 2772, 30005),
            (32225, 32268, -9, 431, 25010),
        ] {
            let matches: Vec<_> = data
                .sectors
                .iter()
                .filter(|s| s.floor == floor)
                .flat_map(|s| &s.tiles)
                .filter(|t| (t.x, t.y) == (x, y))
                .flat_map(|t| &t.items)
                .filter(|i| i.depth == 0)
                .filter(|i| {
                    data.manifest.palette[i.palette as usize]
                        .key
                        .ends_with(&format!(".i{appearance}"))
                })
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "one native source object at {x},{y},{floor}"
            );
            assert_eq!(
                matches[0].attrs.unique,
                Some(unique),
                "startup selector at {x},{y},{floor}"
            );
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED real_world_startup_selectors {:?}",
            started.elapsed()
        );
        Ok(())
    })
}
