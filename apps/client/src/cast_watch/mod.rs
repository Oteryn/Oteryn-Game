//! Account-only Watch entry. Canary livestream.cpp:314,541: a distinct read-only connection.
#[cfg(windows)]
mod native;
#[cfg(any(windows, test))]
mod picture;
#[cfg(windows)]
pub use native::run;
#[cfg(test)]
mod tests;
use oteryn_client_ui::cast_view::Panel;
use oteryn_session::{cast::Message, cast_viewer::CastViewerSession};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// One-use Platform grant; deliberately has no Debug implementation or CharacterId.
pub struct WatchConfig {
    address: String,
    tls_name: String,
    root: PathBuf,
    ticket: Vec<u8>,
}
impl WatchConfig {
    pub fn from_env(mut read: impl FnMut(&str) -> Option<String>) -> Result<Self, &'static str> {
        let address = read("OTERYN_WATCH_ADDRESS")
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or(tr!("Missing Watch endpoint"))?;
        let tls_name = read("OTERYN_WATCH_TLS_NAME")
            .filter(|s| !s.is_empty() && s.len() <= 253)
            .ok_or(tr!("Missing Watch TLS name"))?;
        let root = read("OTERYN_WATCH_ROOT_CERT")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .ok_or(tr!("Missing Watch trust root"))?;
        let ticket = read("OTERYN_WATCH_TICKET").ok_or(tr!("Missing account observer ticket"))?;
        if ticket.len() != 69
            || !ticket.starts_with("ocv1.")
            || !ticket.as_bytes()[5..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err(tr!("Invalid account observer ticket"));
        }
        Ok(Self {
            address,
            tls_name,
            root,
            ticket: ticket.into_bytes(),
        })
    }
}
#[derive(Debug)]
enum Input {
    Chat(String),
    Leave,
}
#[derive(Clone)]
pub struct WatchLink {
    state: Arc<Mutex<(u64, Panel)>>,
    commands: UnboundedSender<Input>,
}
impl WatchLink {
    pub fn snapshot(&self) -> Option<(u64, Panel)> {
        self.state.lock().ok().map(|state| state.clone())
    }
    pub fn chat(&self, text: String) {
        if let Ok(mut state) = self.state.lock()
            && !state.1.picture.chat_enabled
        {
            state.1.feedback = tr!("Cast chat is disabled").into();
            state.0 = state.0.wrapping_add(1);
            return;
        }
        let _ = self.commands.send(Input::Chat(text));
    }
    pub fn leave(&self) {
        let _ = self.commands.send(Input::Leave);
    }
    fn apply(&self, message: Message) -> Result<(), &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| tr!("Watch state unavailable"))?;
        oteryn_client_ui::cast_view::protocol::apply(&mut state.1, message, Instant::now())?;
        state.0 = state.0.wrapping_add(1);
        Ok(())
    }
    fn fail(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.1.picture.clear();
            state.1.picture.ended = true;
            state.1.feedback =
                tr!("Cast unavailable. Obtain a new account observer ticket to watch again.")
                    .into();
            state.0 = state.0.wrapping_add(1);
        }
    }
}
fn channel() -> (WatchLink, UnboundedReceiver<Input>) {
    let (commands, rx) = tokio::sync::mpsc::unbounded_channel();
    (
        WatchLink {
            state: Arc::new(Mutex::new((0, Panel::default()))),
            commands,
        },
        rx,
    )
}
pub fn spawn(
    runtime: &oteryn_client_runtime::ClientRuntime,
    config: WatchConfig,
) -> Result<WatchLink, oteryn_client_runtime::RuntimeError> {
    let (link, rx) = channel();
    let task_link = link.clone();
    let _watch_task = runtime.spawn(async move {
        if watch(config, &task_link, rx).await.is_err() {
            task_link.fail();
        }
    })?;
    Ok(link)
}
async fn watch(
    config: WatchConfig,
    link: &WatchLink,
    mut inputs: UnboundedReceiver<Input>,
) -> Result<(), &'static str> {
    let root = oteryn_session_tcp::load_root_certificate(&config.root)
        .map_err(|_| tr!("Watch trust root unavailable"))?;
    let addresses: Vec<_> = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::net::lookup_host(config.address.as_str()),
    )
    .await
    .map_err(|_| tr!("Watch lookup timed out"))?
    .map_err(|_| tr!("Watch endpoint unavailable"))?
    .take(16)
    .collect();
    let stream = oteryn_session_tcp::connection::connect_any(&addresses, &config.tls_name, &root)
        .await
        .ok_or(tr!("Watch connection unavailable"))?;
    let mut session = CastViewerSession::join(stream, &config.ticket, Duration::from_secs(10))
        .await
        .map_err(|_| tr!("Watch admission unavailable"))?;
    link.apply(Message::Ready {
        epoch: session.epoch(),
        chat: session.chat_allowed(),
        delay: session.delay(),
    })?;
    let mut keepalive = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            message=session.next()=> {
                let message=message.map_err(|_|"Watch ended")?;
                let end=message==Message::End;
                link.apply(message)?;
                if end {return Ok(());}
            }
            input=inputs.recv()=> match input {
                Some(Input::Chat(text))=>session.chat(&text).await.map_err(|_|"Cast chat unavailable")?,
                Some(Input::Leave)|None=> {let _=session.leave().await;link.apply(Message::End)?;return Ok(());}
            },
            _=keepalive.tick()=>session.keepalive().await.map_err(|_|"Watch keepalive failed")?,
        }
    }
}
