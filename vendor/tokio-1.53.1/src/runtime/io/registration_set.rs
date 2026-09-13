use crate::loom::sync::atomic::AtomicUsize;
use crate::runtime::io::ScheduledIo;
use crate::util::linked_list::{self, LinkedList};

use std::io;
use std::ptr::NonNull;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::Arc;

// Kind of arbitrary, but buffering 16 `ScheduledIo`s doesn't seem like much
const NOTIFY_AFTER: usize = 16;

pub(super) struct RegistrationSet {
    num_pending_release: AtomicUsize,
}

pub(super) struct Synced {
    // True when the I/O driver shutdown. At this point, no more registrations
    // should be added to the set.
    is_shutdown: bool,

    // List of all registrations tracked by the set
    registrations: LinkedList<RegistrationHandle>,

    // Registrations that are pending drop. When a `Registration` is dropped, it
    // stores its `ScheduledIo` in this list. The I/O driver is responsible for
    // dropping it. This ensures the `ScheduledIo` is not freed while it can
    // still be included in an I/O event.
    pending_release: Vec<RegistrationHandle>,
}

impl RegistrationSet {
    pub(super) fn new() -> (RegistrationSet, Synced) {
        let set = RegistrationSet {
            num_pending_release: AtomicUsize::new(0),
        };

        let synced = Synced {
            is_shutdown: false,
            registrations: LinkedList::new(),
            pending_release: Vec::with_capacity(NOTIFY_AFTER),
        };

        (set, synced)
    }

    pub(super) fn is_shutdown(&self, synced: &Synced) -> bool {
        synced.is_shutdown
    }

    /// Returns `true` if there are registrations that need to be released
    pub(super) fn needs_release(&self) -> bool {
        self.num_pending_release.load(Acquire) != 0
    }

    pub(super) fn allocate(
        &self,
        synced: &mut Synced,
        #[cfg(feature = "rt")] owner: Option<Arc<dyn crate::task::BlockingOwner>>,
    ) -> io::Result<RegistrationHandle> {
        if synced.is_shutdown {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                crate::util::error::RUNTIME_SHUTTING_DOWN_ERROR,
            ));
        }

        #[cfg(feature = "rt")]
        let charge = owner
            .map(|owner| {
                crate::runtime::task::OterynCharge::reserve(
                    owner,
                    RegistrationHandle::allocation_size(),
                )
            })
            .transpose()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "insufficient registration owner balance",
                )
            })?;
        let ret = RegistrationHandle(Some(Arc::new(RegistrationBacking {
            io: ScheduledIo::default(),
            #[cfg(feature = "rt")]
            charge,
        })));

        // Push a ref into the list of all resources.
        synced.registrations.push_front(ret.clone());

        Ok(ret)
    }

    // Returns `true` if the caller should unblock the I/O driver to purge
    // registrations pending release.
    pub(super) fn deregister(
        &self,
        synced: &mut Synced,
        registration: &RegistrationHandle,
    ) -> bool {
        synced.pending_release.push(registration.clone());

        let len = synced.pending_release.len();
        self.num_pending_release.store(len, Release);

        len == NOTIFY_AFTER
    }

    pub(super) fn shutdown(&self, synced: &mut Synced) -> Vec<RegistrationHandle> {
        if synced.is_shutdown {
            return vec![];
        }

        synced.is_shutdown = true;
        synced.pending_release.clear();

        // Building a vec of all outstanding I/O handles could be expensive, but
        // this is the shutdown operation. In theory, shutdowns should be
        // "clean" with no outstanding I/O resources. Even if it is slow, we
        // aren't optimizing for shutdown.
        let mut ret = vec![];

        while let Some(io) = synced.registrations.pop_back() {
            ret.push(io);
        }

        ret
    }

    pub(super) fn release(&self, synced: &mut Synced) {
        let pending = std::mem::take(&mut synced.pending_release);

        for io in pending {
            // safety: the registration is part of our list
            unsafe { self.remove(synced, &io) }
        }

        self.num_pending_release.store(0, Release);
    }

    // This function is marked as unsafe, because the caller must make sure that
    // `io` is part of the registration set.
    pub(super) unsafe fn remove(&self, synced: &mut Synced, io: &RegistrationHandle) {
        // SAFETY: Pointers into an Arc are never null.
        let io = RegistrationHandle::as_raw(io);

        super::EXPOSE_IO.unexpose_provenance(io.as_ptr());
        // SAFETY: the caller guarantees that `io` is part of this list.
        let _ = unsafe { synced.registrations.remove(io) };
    }
}

// Each handle owns one strong reference. The Arc never escapes this module;
// every strong release runs into_inner, including list and pending-release owners.
// No Weak is created, so into_inner frees the Arc backing before returning data.
pub(super) struct RegistrationHandle(Option<Arc<RegistrationBacking>>);

#[repr(C)]
struct RegistrationBacking {
    io: ScheduledIo,
    #[cfg(feature = "rt")]
    charge: Option<crate::runtime::task::OterynCharge>,
}

impl RegistrationHandle {
    #[cfg(feature = "rt")]
    fn allocation_size() -> usize {
        // Rust 1.94.0 alloc::sync::ArcInner is repr(C, align(2)), with
        // strong/weak AtomicUsize fields preceding T. Arc::new uses Box::new
        // on this exact layout. This counts the requested backing, not Arc<T>.
        #[repr(C, align(2))]
        struct ArcLayout {
            _strong: std::sync::atomic::AtomicUsize,
            _weak: std::sync::atomic::AtomicUsize,
            _data: RegistrationBacking,
        }
        std::mem::size_of::<ArcLayout>()
    }

    fn as_raw(&self) -> NonNull<ScheduledIo> {
        let backing = Arc::as_ptr(self.0.as_ref().unwrap());
        // SAFETY: the live Arc pins the backing; addr_of preserves provenance.
        unsafe { NonNull::new_unchecked(std::ptr::addr_of!((*backing).io).cast_mut()) }
    }
}

impl Clone for RegistrationHandle {
    fn clone(&self) -> Self {
        Self(Some(self.0.as_ref().unwrap().clone()))
    }
}

impl std::ops::Deref for RegistrationHandle {
    type Target = ScheduledIo;

    fn deref(&self) -> &ScheduledIo {
        &self.0.as_ref().unwrap().io
    }
}

impl std::fmt::Debug for RegistrationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl Drop for RegistrationHandle {
    fn drop(&mut self) {
        if let Some(backing) = Arc::into_inner(self.0.take().unwrap()) {
            // The Arc allocation is now deallocated (there are no Weak owners).
            // Keep the charge alive while destroying all ScheduledIo contents.
            let RegistrationBacking {
                io,
                #[cfg(feature = "rt")]
                charge,
            } = backing;
            drop(io);
            #[cfg(feature = "rt")]
            drop(charge);
        }
    }
}

// Safety: the Arc pins ScheduledIo until the list releases its strong owner.
unsafe impl linked_list::Link for RegistrationHandle {
    type Handle = RegistrationHandle;
    type Target = ScheduledIo;

    fn as_raw(handle: &Self::Handle) -> NonNull<ScheduledIo> {
        // safety: Arc::as_ptr never returns null
        handle.as_raw()
    }

    unsafe fn from_raw(ptr: NonNull<Self::Target>) -> RegistrationHandle {
        // safety: the linked list currently owns a ref count
        unsafe {
            // The list raw pointer was obtained from this same backing's io
            // field. offset_of handles the concrete repr(C) container layout.
            let backing = ptr
                .as_ptr()
                .cast::<u8>()
                .sub(std::mem::offset_of!(RegistrationBacking, io))
                .cast::<RegistrationBacking>();
            RegistrationHandle(Some(Arc::from_raw(backing)))
        }
    }

    unsafe fn pointers(
        target: NonNull<Self::Target>,
    ) -> NonNull<linked_list::Pointers<ScheduledIo>> {
        // safety: `target.as_ref().linked_list_pointers` is a `UnsafeCell` that
        // always returns a non-null pointer.
        unsafe { NonNull::new_unchecked(target.as_ref().linked_list_pointers.get()) }
    }
}
