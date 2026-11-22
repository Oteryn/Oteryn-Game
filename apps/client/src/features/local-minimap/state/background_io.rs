use super::*;
use bevy_tasks::{IoTaskPool, Task, futures::check_ready};

type Load = Task<(Option<CacheAccess>, io::Result<Option<Saved>>)>;
#[derive(Default, Debug)]
pub(super) struct CacheIo {
    load: Option<(PathBuf, Load)>,
    retry_at: Option<Instant>,
    imports: BTreeMap<PathBuf, Task<io::Result<Saved>>>,
}

impl Minimap {
    pub(super) fn background_io_owns_cache(&self) -> bool {
        self.path().is_some_and(|path| {
            self.cache_io
                .load
                .as_ref()
                .is_some_and(|(loading, _)| loading == &path)
                || self.cache_io.imports.contains_key(&path)
        })
    }

    pub(super) fn background_io_pending(&self) -> bool {
        self.cache_io.load.is_some() || !self.cache_io.imports.is_empty()
    }

    pub(super) fn load_in_background(&mut self, path: PathBuf) {
        if self.cache_io.imports.contains_key(&path) {
            return;
        }
        let already_owned = matches!(
            self.cache_access.get(&path),
            Some(CacheAccess::Owner { .. })
        );
        let reload_busy = self.is_read_only();
        let source = path.clone();
        self.cache_io.load = Some((
            path,
            IoTaskPool::get().spawn(async move {
                let access = (!already_owned).then(|| CacheAccess::open(&source));
                let saved = if reload_busy && matches!(access, Some(CacheAccess::ReadOnly(_))) {
                    Ok(None)
                } else {
                    Saved::read_from(&source)
                };
                (access, saved)
            }),
        ));
    }

    pub(super) fn poll_cache_load(&mut self) -> bool {
        let mut changed = false;
        if let Some((path, task)) = &mut self.cache_io.load
            && let Some((access, loaded)) = check_ready(task)
        {
            let path = path.clone();
            self.cache_io.load = None;
            if self.path().as_ref() == Some(&path) {
                if let Some(access) = access {
                    self.cache_access.insert(path, access);
                }
                match loaded {
                    Ok(Some(saved)) => {
                        let observed = std::mem::replace(&mut self.cells, saved_cells(saved.cells));
                        for (position, color) in observed {
                            if self.cells.len() < MAX_CELLS || self.cells.contains_key(&position) {
                                self.cells.insert(position, color);
                            }
                        }
                        self.load_failed = false;
                        self.markers = saved.markers;
                        changed = true;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        self.load_failed = true;
                        self.feedback = lfmt!("Could not load minimap: {error}", error = error);
                    }
                }
            }
        }
        if let Some(path) = self.path()
            && matches!(self.cache_access.get(&path), Some(CacheAccess::ReadOnly(error)) if error.kind() == io::ErrorKind::WouldBlock)
            && !self.background_io_owns_cache()
            && self.cache_io.retry_at.is_none_or(|at| at <= Instant::now())
        {
            self.cache_io.retry_at = Some(Instant::now() + Duration::from_millis(100));
            self.load_in_background(path);
        }
        changed
    }

    pub(in crate::play) fn start_background_import(&mut self, source: &Path) -> io::Result<bool> {
        if !self.background_save {
            return Ok(false);
        }
        let path = self
            .path()
            .ok_or_else(|| io::Error::other("Minimap storage is unavailable"))?;
        self.require_cache_owner(&path)?;
        let Some(CacheAccess::Owner { _lock }) = self.cache_access.get(&path) else {
            return Err(io::Error::other("Minimap storage is unavailable"));
        };
        let lock = _lock.clone();
        let previous = self.save_task.take();
        self.cells_dirty |= previous.is_some();
        let active = path.clone();
        let source = source.to_owned();
        let world = self.world.clone();
        let binding = self.binding;
        self.cache_io.imports.insert(
            path,
            IoTaskPool::get().spawn(async move {
                let _lock = lock;
                if let Some(previous) = previous {
                    let _ = previous.await;
                }
                let saved = Self::read_import(&source, world.as_deref(), binding)?;
                saved.write_to(&active)?;
                Ok(saved)
            }),
        );
        Ok(true)
    }

    pub(in crate::play) fn poll_background_import(&mut self) -> Option<io::Result<()>> {
        let active = self.path();
        let mut result = None;
        self.cache_io.imports.retain(|path, task| {
            let Some(completed) = check_ready(task) else {
                return true;
            };
            if active.as_ref() == Some(path) {
                result = Some(completed);
            } else if let Err(error) = completed {
                eprintln!("Minimap import failed: {error}");
            }
            false
        });
        result.map(|completed| completed.map(|saved| self.install_import(saved)))
    }
    pub(super) fn detach_imports(&mut self) {
        for task in std::mem::take(&mut self.cache_io.imports).into_values() {
            IoTaskPool::get()
                .spawn(async move {
                    if let Err(error) = task.await {
                        eprintln!("Minimap import failed: {error}");
                    }
                })
                .detach();
        }
    }
}
