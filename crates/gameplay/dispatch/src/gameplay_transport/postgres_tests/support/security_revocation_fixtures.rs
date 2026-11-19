use super::fixture::{Harness, debug};

use super::TestResult;

/// Advance the injected database clock after the transaction's capability check.
/// The separate connection can commit while admission relations are locked.
pub(super) struct RecoveryExpiryClock {
    clock: super::preadmission_fixtures::FrozenAdmissionClock,
    worker: std::thread::JoinHandle<Result<(), String>>,
    pub(super) check: crate::durability::fresh_admission::CurrentCapabilityCheck,
    observed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl RecoveryExpiryClock {
    pub(super) async fn install(h: &Harness, now: i64) -> TestResult<Self> {
        use sqlx::Connection as _;
        let clock = super::preadmission_fixtures::FrozenAdmissionClock::install(h, now).await?;
        sqlx::query("ALTER TABLE preadm_clock.instant ALTER COLUMN value TYPE NUMERIC").execute(&h.pool).await?;
        sqlx::query("UPDATE game_platform_stream_cursors SET advanced_at=to_timestamp($1::double precision-30) WHERE stream=3").bind(now as f64).execute(&h.pool).await?;
        assert!(h.root.security_revocation_read_fresh_now().await.map_err(debug)?);
        let options = h.pool.connect_options();
        let (request, requests) = std::sync::mpsc::sync_channel(1);
        let (answer, answers) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || -> Result<(), String> {
            let runtime=crate::bestiary_postgres_harness::runtime().map_err(|e|e.to_string())?;
            runtime.block_on(async {
                let mut connection=sqlx::PgConnection::connect_with(&options).await.map_err(|e|format!("{e:?}"))?;
                requests.recv_timeout(std::time::Duration::from_secs(15)).map_err(|e|format!("recovery clock capability stage: {e}"))?;
                sqlx::query("UPDATE preadm_clock.instant SET value=$1::text::numeric/1000").bind((now*1000+1).to_string()).execute(&mut connection).await.map_err(|e|format!("{e:?}"))?;
                answer.send(()).map_err(|e|e.to_string())?;
                Ok(())
            })
        });
        let observed=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let invoked=observed.clone();
        let answers=std::sync::Mutex::new(answers);
        let check: crate::durability::fresh_admission::CurrentCapabilityCheck=std::sync::Arc::new(move |_,_,_| {
            let advanced=request.send(()).is_ok() && answers.lock().is_ok_and(|reply|reply.recv_timeout(std::time::Duration::from_secs(15)).is_ok());
            invoked.store(advanced,std::sync::atomic::Ordering::SeqCst);
            advanced
        });
        Ok(Self {clock,worker,check,observed})
    }
    pub(super) async fn finish(self,h:&Harness)->TestResult {
        assert!(self.observed.load(std::sync::atomic::Ordering::SeqCst),"deciding capability check must run before freshness expires");
        self.worker.join().map_err(|_|"recovery clock worker panicked")??;
        let mut connection=tokio::time::timeout(std::time::Duration::from_secs(15),async {
            loop {
                if let Ok(connection)=h.root.try_acquire_ready() {break connection;}
                tokio::task::yield_now().await;
            }
        }).await?;
        sqlx::query("SET search_path=pg_catalog,public").execute(&mut *connection).await?;
        let returned=connection.return_to_pool_observed_until(std::time::Instant::now()+std::time::Duration::from_secs(5)).await;
        assert!(matches!(returned,sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle));
        drop(self.clock);
        Ok(())
    }
}
