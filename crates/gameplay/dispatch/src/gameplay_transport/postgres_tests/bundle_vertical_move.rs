//! GAP-SV8: casts reach the bundle boot's qualified immovable facts, not captured source tiles.
use super::TestResult;

#[test]
fn bundle_vertical_move_postgres_dispatch_levitate_up_down_and_magic_rope() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            super::fixture::run(async |admin| {
                for case in [0, 1, 3] {
                    let began = std::time::Instant::now();
                    super::vertical_move::scenario(admin.clone(), case, true).await?;
                    eprintln!(
                        "GAP-SV8 PostgreSQL bundle case {case}: committed and replayed in {:?}",
                        began.elapsed()
                    );
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "bundle vertical movement thread panicked")?
        .map_err(Into::into)
}
