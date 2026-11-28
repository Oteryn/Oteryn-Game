use crate::foundation::MovementLocalPosition;
use crate::node::serve::bundle_npcs::tests::place_rashid_for_clock_test;
use oteryn_gameplay_entities::rashid_schedule::destination_at;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY_MS: i64 = 86_400_000;

#[test]
fn w05clock_bundle_placement_uses_world_weekday() -> Result<(), Box<dyn std::error::Error>> {
    const PROBE: &str = "OTERYN_W05CLOCK_PLACEMENT_PROBE";
    const CLOCK_FILE: &str = "OTERYN_QUALIFICATION_CLOCK_FILE";
    if std::env::var_os(PROBE).is_none() {
        let (_, module) = module_path!().split_once("::").ok_or("test module")?;
        let case = format!("{module}::w05clock_bundle_placement_uses_world_weekday");
        let mut child = std::process::Command::new(std::env::current_exe()?);
        child
            .args(["--exact", &case, "--nocapture"])
            .env(PROBE, "1");
        let status = child.env_remove(CLOCK_FILE).status()?;
        assert!(status.success(), "wall-clock placement: {status}");
        #[cfg(feature = "qualification-clock")]
        {
            use std::io::Write;
            use std::sync::atomic::{AtomicU64, Ordering};

            let now = wall_time()?;
            // Keep the weekdays distinct even when the wall clock crosses midnight during setup.
            let offset = 2 * DAY_MS + DAY_MS / 2 - now.rem_euclid(DAY_MS);
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "oteryn-w05clock-placement-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            let written = file.write_all(offset.to_string().as_bytes());
            drop(file);
            let status = written.and_then(|()| child.env(CLOCK_FILE, &path).status());
            let cleanup = std::fs::remove_file(path);
            let status = status?;
            cleanup?;
            assert!(status.success(), "qualification-clock placement: {status}");
        }
        return Ok(());
    }

    let wall = wall_time()?;
    let offset = std::env::var_os(CLOCK_FILE)
        .map(std::fs::read_to_string)
        .transpose()?
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(0);
    let expected = destination_at(wall + offset).placement;
    if offset > 0 {
        assert_ne!(expected, destination_at(wall).placement);
    }
    let week = (0..7)
        .map(|day| destination_at(day * DAY_MS).placement)
        .collect();
    assert_eq!(
        place_rashid_for_clock_test(week)?,
        MovementLocalPosition {
            x: i32::from(expected.x),
            y: i32::from(expected.y),
            floor: -i16::from(expected.floor),
        }
    );
    Ok(())
}

fn wall_time() -> Result<i64, Box<dyn std::error::Error>> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}
