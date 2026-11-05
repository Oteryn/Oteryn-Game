//! Direct PostgreSQL owners replace retired holders without a node maintenance loop.
use super::fixture::{Harness, debug, run};
use super::{TestResult, maintain_ready};

#[test]
fn direct_postgres_retired_holder_is_replaced() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let h = Harness::create(admin, "direct_serve_readiness", false).await?;
                let holder = h.root.try_acquire_ready().map_err(debug)?;
                holder.close().await?;
                h.root.request_ready();
                assert!(!h.root.is_ready());
                assert!(h.root.has_ready_demand());

                maintain_ready(&h.root).await?;
                assert!(h.root.is_ready());
                assert!(!h.root.has_ready_demand());
                // An already-ready root needs no new maintenance demand.
                maintain_ready(&h.root).await?;
                assert!(h.root.is_ready());
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "direct readiness thread panicked")?
        .map_err(Into::into)
}
