//! One background writer orders each character's high records and explicit resets.
use super::*;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
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
    requests: Option<mpsc::Sender<Request>>,
    thread: Option<std::thread::JoinHandle<()>>,
    replies: Mutex<mpsc::Receiver<Reply>>,
}
impl Worker {
    fn start() -> io::Result<Self> {
        let (requests, work) = mpsc::channel::<Request>();
        let (finished, replies) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("analyser-highs".into())
            .spawn(move || {
                for request in work {
                    let result = update(&request.path, request.values, request.reset);
                    if finished
                        .send(Reply {
                            generation: request.generation,
                            revision: request.revision,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            requests: Some(requests),
            thread: Some(thread),
            replies: Mutex::new(replies),
        })
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.requests.take();
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
            w.requests.as_ref().is_some_and(|requests| {
                requests
                    .send(Request {
                        path,
                        generation,
                        revision,
                        values,
                        reset,
                    })
                    .is_ok()
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
