use super::*;

impl LoginWindow {
    pub fn open() -> Result<Self, String> {
        let (updates, incoming) = mpsc::channel();
        let (requests, outgoing) = mpsc::channel();
        let (cancel, cancellations) = mpsc::sync_channel(1);
        let (ready, initialized) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("native-login".into())
            .spawn(move || {
                let result = run(incoming, requests, cancel, &ready);
                if let Err(error) = result {
                    let _ = ready.send(Err(error));
                }
            })
            .map_err(|_| "Cannot start the login window.".to_owned())?;
        initialized
            .recv()
            .map_err(|_| "Login window stopped.".to_owned())??;
        Ok(Self {
            updates: Some(updates),
            requests: outgoing,
            cancellations,
            thread: Some(thread),
        })
    }
    pub fn poll(&self) -> Option<NativeLoginConfig> {
        self.requests.try_recv().ok()
    }
    pub fn cancel_requested(&self) -> bool {
        self.cancellations.try_recv().is_ok()
    }
    pub fn progress(&self, message: String) {
        self.send(Update::Progress(message));
    }
    pub fn bulletin(&self, document: String) {
        self.send(Update::Bulletin(document));
    }
    pub fn show_bulletin(&self) {
        self.send(Update::ShowBulletin);
    }
    pub fn offline(&self, message: String) {
        self.send(Update::Offline(message));
    }
    pub fn show(&self, message: String) {
        self.send(Update::Show(message));
    }
    pub fn finish(&self, result: Result<(), GameplayEntryError>) {
        self.send(Update::Finish(result));
    }
    fn send(&self, update: Update) {
        if let Some(sender) = &self.updates {
            let _ = sender.send(update);
        }
    }
}
impl Drop for LoginWindow {
    fn drop(&mut self) {
        self.updates.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
