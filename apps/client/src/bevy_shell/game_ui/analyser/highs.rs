//! One background writer orders each character's high records and explicit resets.
use super::*;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};

struct Request {
    path: PathBuf,
    generation: u64,
    revision: u64,
    values: [u64; 2],
    reset: bool,
}
struct Reply {
    generation: u64,
    revision: u64,
    result: io::Result<[u64; 2]>,
}
struct Worker {
    pending: Arc<Mutex<Vec<Request>>>,
    wake: Option<mpsc::SyncSender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    replies: Mutex<mpsc::Receiver<Reply>>,
}
impl Worker {
    fn start() -> io::Result<Self> {
        let (wake, work) = mpsc::sync_channel(1);
        let pending = Arc::new(Mutex::new(Vec::<Request>::new()));
        let queued = Arc::clone(&pending);
        let (finished, replies) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("analyser-highs".into())
            .spawn(move || {
                for () in work {
                    let batch =
                        std::mem::take(&mut *queued.lock().unwrap_or_else(|e| e.into_inner()));
                    for request in batch {
                        let result = update(&request.path, request.values, request.reset);
                        if finished
                            .send(Reply {
                                generation: request.generation,
                                revision: request.revision,
                                result,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            })?;
        Ok(Self {
            pending,
            wake: Some(wake),
            thread: Some(thread),
            replies: Mutex::new(replies),
        })
    }

    fn submit(&self, request: Request) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(older) = pending.iter_mut().find(|r| r.path == request.path) {
            // A reset discards all earlier values. Later observations still belong
            // to that reset until the writer has applied it to disk.
            let mut latest = request;
            if !latest.reset {
                latest.reset = older.reset;
                for (new, old) in latest.values.iter_mut().zip(older.values) {
                    *new = (*new).max(old);
                }
            }
            *older = latest;
        } else {
            pending.push(request);
        }
        self.wake.as_ref().is_some_and(|wake| {
            matches!(
                wake.try_send(()),
                Ok(()) | Err(mpsc::TrySendError::Full(()))
            )
        })
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.wake.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn update(path: &Path, mut values: [u64; 2], reset: bool) -> io::Result<[u64; 2]> {
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing profile directory"))?;
    std::fs::create_dir_all(directory)?;
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))?;
    // Another client using this character can briefly own the same profile lock.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match fs2::FileExt::try_lock_exclusive(&lock) {
            Ok(()) => break,
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
    let previous: [u64; 2] = match std::fs::File::open(path) {
        Ok(file) => serde_json::from_reader(file.take(256))?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => [0; 2],
        Err(error) => return Err(error),
    };
    if !reset {
        for (high, old) in values.iter_mut().zip(previous) {
            *high = (*high).max(old);
        }
    }
    if previous != values {
        let file = tempfile::NamedTempFile::new_in(directory)?;
        serde_json::to_writer(&file, &values)?;
        file.persist(path)?;
    }
    Ok(values)
}

#[derive(Default)]
pub(super) struct Saved {
    worker: Option<Worker>,
    submitted: Option<(u64, u64, [u64; 2])>,
}
pub(super) fn sync(owner: NonSend<HostOwner>, mut saved: Local<Saved>) {
    let Some(play) = owner.play.as_ref() else {
        saved.submitted = None;
        return;
    };
    let Some((revision, values)) = play.link.analyser_highs() else {
        saved.submitted = None;
        return;
    };
    let generation = owner.generation();
    if saved.submitted != Some((generation, revision, values)) {
        let Some(path) = owner
            .dock_layout_path()
            .map(|p| p.with_extension("analyser-highs.json"))
        else {
            return;
        };
        let reset = saved.submitted.map_or(revision != 0, |(g, r, _)| {
            if g == generation {
                r != revision
            } else {
                revision != 0
            }
        });
        if saved.worker.is_none() {
            saved.worker = Worker::start().ok();
        }
        saved.submitted = Some((generation, revision, values));
        if !saved.worker.as_ref().is_some_and(|w| {
            w.submit(Request {
                path,
                generation,
                revision,
                values,
                reset,
            })
        }) {
            play.link
                .analyser_feedback("Could not start analyser profile writer");
        }
    }
    let Some(worker) = &mut saved.worker else {
        return;
    };
    let replies = worker.replies.get_mut().unwrap_or_else(|e| e.into_inner());
    for reply in replies.try_iter() {
        if (reply.generation, reply.revision) != (generation, revision) {
            continue;
        }
        match reply.result {
            Ok(values) => {
                play.link.analyser_restore_highs(values);
            }
            Err(_) => play
                .link
                .analyser_feedback("Could not read or save analyser highs"),
        }
    }
}

#[cfg(test)]
#[test]
fn contended_high_updates_have_bounded_shutdown() -> io::Result<()> {
    use std::time::Duration;
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("highs.json");
    let lock = std::fs::File::create(path.with_extension("lock"))?;
    fs2::FileExt::lock_exclusive(&lock)?;
    let worker = Worker::start()?;
    for high in 1..=8 {
        assert!(worker.submit(Request {
            path: path.clone(),
            generation: 1,
            revision: 0,
            values: [high, 0],
            reset: false,
        }));
    }
    let (finished, shutdown) = mpsc::channel();
    let join = std::thread::spawn(move || {
        drop(worker);
        let _ = finished.send(());
    });
    let result = shutdown.recv_timeout(Duration::from_secs(7));
    drop(lock);
    join.join()
        .map_err(|_| io::Error::other("writer panicked"))?;
    assert!(
        result.is_ok(),
        "shutdown waited for superseded profile writes"
    );
    Ok(())
}

#[cfg(test)]
#[test]
fn shutdown_preserves_resets_and_highs_for_each_character() -> io::Result<()> {
    let folder = tempfile::tempdir()?;
    let first = folder.path().join("first.json");
    let second = folder.path().join("second.json");
    std::fs::write(&first, "[80,90]")?;
    let worker = Worker::start()?;
    for (path, generation, revision, values, reset) in [
        (&first, 1, 0, [100, 120], false),
        (&first, 1, 1, [0, 0], true),
        (&first, 1, 1, [0, 7], false),
        (&second, 2, 0, [21, 34], false),
        (&first, 3, 0, [6, 0], false),
    ] {
        assert!(worker.submit(Request {
            path: path.clone(),
            generation,
            revision,
            values,
            reset,
        }));
    }
    drop(worker);
    assert_eq!(std::fs::read_to_string(first)?, "[6,7]");
    assert_eq!(std::fs::read_to_string(second)?, "[21,34]");
    Ok(())
}
