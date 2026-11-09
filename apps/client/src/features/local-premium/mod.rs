//! Dedicated read-only Premium overview. No entitlement is granted by this UI.
#[cfg(windows)]
mod native;
#[cfg(test)]
mod tests;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::{AccessToken, NativeLoginClient, premium::*};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Refresh,
    Back,
}
#[derive(Debug, Default)]
pub struct View {
    pub status: Option<PremiumStatus>,
    pub feedback: String,
    requested_at: Option<Instant>,
    fresh_until: Option<Instant>,
}
impl View {
    pub fn fresh(&self) -> bool {
        self.fresh_until.is_some_and(|t| t > Instant::now())
    }
    pub fn status_text(&self) -> String {
        let mut text = self.feedback.clone();
        let Some(status) = &self.status else {
            text.push_str(tr!("\nPremium status unavailable. Refresh to try again."));
            return text;
        };
        let elapsed = self.requested_at.map_or(0, |t| t.elapsed().as_secs());
        let now = status.generated_at_unix.saturating_add(elapsed);
        let active =
            status.state == PremiumState::Active && status.expires_at_unix.is_some_and(|t| t > now);
        text.push_str(&lfmt!(
            "\nLast server Premium status: {:?}{}",
            status.state,
            if self.fresh() {
                ""
            } else {
                tr!(" (stale — refresh required)")
            }
        ));
        if let Some(until) = status.expires_at_unix {
            let remaining = until.saturating_sub(now);
            text.push_str(&lfmt!("\nExpiry: {until} UTC Unix seconds. Time remaining: {} days, {} hours, {} minutes.",remaining/86400,remaining/3600%24,remaining/60%60,until=until));
            if status.state == PremiumState::Active && !active {
                text.push_str(tr!(
                    "\nThe displayed Premium period has ended; refresh status."
                ));
            }
        } else {
            text.push_str(tr!("\nNo Premium expiry is recorded."));
        }
        text.push_str(tr!("\n\nPremium features (reported by Oteryn):"));
        if status.features.is_empty() {
            text.push_str(tr!("\nNo feature information supplied."));
        }
        for f in &status.features {
            let available = self.fresh() && active && f.enabled;
            text.push_str(&format!(
                "\n{} [{}]\n{}",
                f.title,
                if available {
                    "available"
                } else {
                    tr!("not confirmed available")
                },
                f.description
            ));
        }
        text.push_str(tr!(
            "\n\nThis overview grants no access. Game checks current eligibility for every action."
        ));
        text
    }
}
pub type Presenter = Arc<dyn Fn(&View, CancellationToken) -> Action + Send + Sync>;
pub fn present(view: &View, cancel: CancellationToken) -> Action {
    #[cfg(windows)]
    {
        native::present(view, cancel)
    }
    #[cfg(not(windows))]
    {
        let _ = (view, cancel);
        Action::Back
    }
}
async fn refresh(
    view: &mut View,
    client: &NativeLoginClient,
    token: &AccessToken,
    cancel: CancellationToken,
) {
    #[cfg(windows)]
    let _progress = crate::account_services::window::Progress::open(cancel.clone());
    let at = Instant::now();
    let result = client.premium_status(token, cancel).await;
    match result {
        Ok(status) => {
            let inconsistent = view.status.as_ref().is_some_and(|old| {
                status.authority_revision < old.authority_revision
                    || status.generated_at_unix < old.generated_at_unix
                    || (status.authority_revision == old.authority_revision
                        && (status.state != old.state
                            || status.expires_at_unix != old.expires_at_unix
                            || status.features != old.features))
            });
            if inconsistent {
                view.fresh_until = None;
                view.feedback = tr!("Inconsistent Premium status refused. Refresh again.").into();
                return;
            }
            view.fresh_until = Some(at + Duration::from_secs(u64::from(status.valid_for_seconds)));
            view.requested_at = Some(at);
            view.status = Some(status);
            view.feedback = tr!("Premium status refreshed from Oteryn.").into();
        }
        Err(error) => {
            view.fresh_until = None;
            view.feedback = match error {
                oteryn_platform_client::native_login::NativeLoginError::Platform(
                    oteryn_platform_client::PlatformClientError::HttpStatus(404 | 501),
                ) => tr!("Premium details are not available on this service yet. Game access is checked separately.").into(),
                _ => lfmt!("Premium service unavailable ({:?}).", error.public_class()),
            };
        }
    }
}
pub(crate) async fn run(
    client: &NativeLoginClient,
    token: &AccessToken,
    presenter: &Presenter,
    cancel: CancellationToken,
) -> Result<(), crate::GameplayEntryError> {
    let mut view = View::default();
    refresh(&mut view, client, token, cancel.clone()).await;
    loop {
        let show = presenter.clone();
        let cancellation = cancel.clone();
        let (returned, action) = tokio::task::spawn_blocking(move || {
            let action = show(&view, cancellation);
            (view, action)
        })
        .await
        .map_err(|_| {
            crate::GameplayEntryError::Rejected(
                oteryn_platform_client::native_login::PublicClass::TemporarilyUnavailable,
            )
        })?;
        view = returned;
        if matches!(action, Action::Back) || cancel.is_cancelled() {
            return Ok(());
        }
        refresh(&mut view, client, token, cancel.clone()).await;
    }
}
