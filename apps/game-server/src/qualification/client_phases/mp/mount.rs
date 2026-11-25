//! Multi-player scenarios, mounted once from the world journey.
use super::*;

mod bots;
mod diagnostics;
mod mp01_party_pvp;
mod mp09_party_xp;
pub(super) mod observations;
mod starter;

pub(in crate::qualification) use starter::legacy_d150;

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
    towns: &oteryn_world_bundle::subarea::Table,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
) {
    run_rows(
        |ready| {
            verdict::indexed(
                "MP01.full",
                mp01_party_pvp::run(core, packets, world, towns, ready),
            )
        },
        |ready| {
            verdict::indexed(
                "MP09.full",
                mp09_party_xp::run(core, packets, world, rewards, ready),
            )
        },
    )
    .await;
}

async fn run_rows<First, Second>(
    mp01: impl FnOnce(tokio::sync::oneshot::Sender<bool>) -> First,
    mp09: impl FnOnce(bool) -> Second,
) where
    First: Future<Output = ()>,
    Second: Future<Output = ()>,
{
    let (ready, profiles) = tokio::sync::oneshot::channel();
    let ((), fallback) = tokio::join!(mp01(ready), async {
        // Only verified and released fixtures can be reused during Open PvP.
        if profiles.await == Ok(true) {
            mp09(true).await;
            None
        } else {
            Some(mp09)
        }
    });
    if let Some(mp09) = fallback {
        mp09(false).await;
    }
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;

    #[test]
    fn mp09_runs_while_open_pvp_is_pending() -> TestResult {
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        runtime.block_on(async {
            for verification in [Some(true), Some(false), None] {
                let observed = &std::cell::Cell::new(None);
                let (finish, open_pvp) = tokio::sync::oneshot::channel();
                let mut rows = std::pin::pin!(run_rows(
                    |ready| async {
                        if let Some(verified) = verification {
                            let _ = ready.send(verified);
                        } else {
                            drop(ready);
                        }
                        let _ = open_pvp.await;
                    },
                    |ready| async move {
                        observed.set(Some(ready));
                    },
                ));
                std::future::poll_fn(|cx| {
                    let _ = rows.as_mut().poll(cx);
                    std::task::Poll::Ready(())
                })
                .await;
                assert_eq!(observed.get(), verification.filter(|verified| *verified));
                let _ = finish.send(());
                rows.await;
                assert_eq!(observed.get(), Some(verification.unwrap_or(false)));
            }
            Ok(())
        })
    }
}
