#[cfg(feature = "std")]
use crate::sync::Arc;
use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::fmt;
use core::mem;
use core::ops::Range;
#[cfg(feature = "std")]
use std::io;

#[cfg(feature = "std")]
use crate::msgs::message::MAX_WIRE_SIZE;

/// Owner-provided accounting for the buffered TLS deframer allocation.
///
/// Implementations must reserve on one operation ledger. A successful reservation
/// remains owned until rustls calls [`DeframerBufferOwner::release`] after the
/// corresponding backing allocation is destroyed.
#[cfg(feature = "std")]
pub trait DeframerBufferOwner: fmt::Debug + Send + Sync {
    /// Reserve `bytes` before rustls allocates the backing.
    fn try_reserve(&self, bytes: usize) -> Result<(), DeframerBufferError>;

    /// Release a successful reservation after its backing is destroyed.
    fn release(&self, bytes: usize);

    /// Reserve provider-resident bytes against the same executor root.
    ///
    /// Successful reservations are intentionally retained until process
    /// teardown. Owners which do not implement shared-root accounting are
    /// unsupported and fail closed.
    fn try_reserve_provider_shared(&self, _bytes: usize) -> Result<(), DeframerBufferError> {
        Err(DeframerBufferError)
    }

    /// Observe completed synchronous consumption of an owned KX secret.
    ///
    /// This evidence hook must not release custody. Rustls drops the reservation
    /// only after the consuming key-schedule call has returned.
    #[doc(hidden)]
    fn kx_secret_consumed(&self) {}
}

/// Bounded failure from a deframer allocation owner.
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeframerBufferError;

#[cfg(feature = "std")]
impl fmt::Display for DeframerBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TLS deframer buffer owner denied allocation")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DeframerBufferError {}

#[cfg(feature = "std")]
#[derive(Debug)]
struct PendingReservation {
    owner: Arc<dyn DeframerBufferOwner>,
    bytes: usize,
}

#[cfg(feature = "std")]
impl PendingReservation {
    fn commit(mut self) {
        self.bytes = 0;
    }
}

#[cfg(feature = "std")]
impl Drop for PendingReservation {
    fn drop(&mut self) {
        self.owner.release(self.bytes);
    }
}

/// Conversion from a slice within a larger buffer into
/// a `Range` offset within.
#[derive(Debug)]
pub(crate) struct Locator {
    bounds: Range<*const u8>,
}

impl Locator {
    #[inline]
    pub(crate) fn new(slice: &[u8]) -> Self {
        Self {
            bounds: slice.as_ptr_range(),
        }
    }

    #[inline]
    pub(crate) fn locate(&self, slice: &[u8]) -> Range<usize> {
        let bounds = slice.as_ptr_range();
        debug_assert!(self.fully_contains(slice));
        let start = bounds.start as usize - self.bounds.start as usize;
        let len = bounds.end as usize - bounds.start as usize;
        Range {
            start,
            end: start + len,
        }
    }

    #[inline]
    pub(crate) fn fully_contains(&self, slice: &[u8]) -> bool {
        let bounds = slice.as_ptr_range();
        bounds.start >= self.bounds.start && bounds.end <= self.bounds.end
    }
}

/// Conversion from a `Range` offset to the original slice.
pub(crate) struct Delocator<'b> {
    slice: &'b [u8],
}

impl<'b> Delocator<'b> {
    #[inline]
    pub(crate) fn new(slice: &'b [u8]) -> Self {
        Self { slice }
    }

    #[inline]
    pub(crate) fn slice_from_range(&'_ self, range: &Range<usize>) -> &'b [u8] {
        // safety: this unwrap is safe so long as `range` came from `locate()`
        // for the same buffer
        self.slice.get(range.clone()).unwrap()
    }

    #[inline]
    pub(crate) fn locator(self) -> Locator {
        Locator::new(self.slice)
    }
}

/// Reordering the underlying buffer based on ranges.
pub(crate) struct Coalescer<'b> {
    slice: &'b mut [u8],
}

impl<'b> Coalescer<'b> {
    #[inline]
    pub(crate) fn new(slice: &'b mut [u8]) -> Self {
        Self { slice }
    }

    #[inline]
    pub(crate) fn copy_within(&mut self, from: Range<usize>, to: Range<usize>) {
        debug_assert!(from.len() == to.len());
        debug_assert!(self.slice.get(from.clone()).is_some());
        debug_assert!(self.slice.get(to.clone()).is_some());
        self.slice.copy_within(from, to.start);
    }

    #[inline]
    pub(crate) fn delocator(self) -> Delocator<'b> {
        Delocator::new(self.slice)
    }
}

/// Accounting structure tracking progress in parsing a single buffer.
#[derive(Clone, Debug)]
pub(crate) struct BufferProgress {
    /// Prefix of the buffer that has been processed so far.
    ///
    /// `processed` may exceed `discard`, that means we have parsed
    /// some buffer, but are still using it.  This happens due to
    /// in-place decryption of incoming records, and in-place
    /// reassembly of handshake messages.
    ///
    /// 0 <= processed <= len
    processed: usize,

    /// Prefix of the buffer that can be removed.
    ///
    /// If `discard` exceeds `processed`, that means we are ignoring
    /// data without processing it.
    ///
    /// 0 <= discard <= len
    discard: usize,
}

impl BufferProgress {
    pub(super) fn new(processed: usize) -> Self {
        Self {
            processed,
            discard: 0,
        }
    }

    #[inline]
    pub(crate) fn add_discard(&mut self, discard: usize) {
        self.discard += discard;
    }

    #[inline]
    pub(crate) fn add_processed(&mut self, processed: usize) {
        self.processed += processed;
    }

    #[inline]
    pub(crate) fn take_discard(&mut self) -> usize {
        // the caller is about to discard `discard` bytes
        // from the front of the buffer.  adjust `processed`
        // down by the same amount.
        self.processed = self.processed.saturating_sub(self.discard);
        mem::take(&mut self.discard)
    }

    #[inline]
    pub(crate) fn processed(&self) -> usize {
        self.processed
    }
}

#[derive(Default, Debug)]
pub(crate) struct DeframerVecBuffer {
    /// Buffer of data read from the socket, in the process of being parsed into messages.
    ///
    /// For buffer size management, checkout out the [`DeframerVecBuffer::prepare_read()`] method.
    buf: Vec<u8>,

    /// What size prefix of `buf` is used.
    used: usize,

    #[cfg(feature = "std")]
    owner: Option<Arc<dyn DeframerBufferOwner>>,

    #[cfg(feature = "std")]
    charged_capacity: usize,
}

impl DeframerVecBuffer {
    /// Discard `taken` bytes from the start of our buffer.
    pub(crate) fn discard(&mut self, taken: usize) {
        #[allow(clippy::comparison_chain)]
        if taken < self.used {
            /* Before:
             * +----------+----------+----------+
             * | taken    | pending  |xxxxxxxxxx|
             * +----------+----------+----------+
             * 0          ^ taken    ^ self.used
             *
             * After:
             * +----------+----------+----------+
             * | pending  |xxxxxxxxxxxxxxxxxxxxx|
             * +----------+----------+----------+
             * 0          ^ self.used
             */

            self.buf.copy_within(taken..self.used, 0);
            self.used -= taken;
        } else if taken >= self.used {
            self.used = 0;
        }
    }

    pub(crate) fn filled_mut(&mut self) -> &mut [u8] {
        &mut self.buf[..self.used]
    }

    pub(crate) fn filled(&self) -> &[u8] {
        &self.buf[..self.used]
    }
}

#[cfg(feature = "std")]
impl DeframerVecBuffer {
    pub(crate) fn set_owner(
        &mut self,
        owner: Arc<dyn DeframerBufferOwner>,
    ) -> Result<(), DeframerBufferError> {
        let capacity = self.buf.capacity();
        owner.try_reserve(capacity)?;
        if let Some(previous) = self.owner.replace(owner) {
            previous.release(self.charged_capacity);
        }
        self.charged_capacity = capacity;
        Ok(())
    }

    fn replace_with_exact_capacity(&mut self, capacity: usize) -> Result<(), DeframerBufferError> {
        let Some(owner) = self.owner.clone() else {
            if capacity > self.buf.len() {
                self.buf.resize(capacity, 0);
            } else {
                self.buf.resize(capacity, 0);
                self.buf.shrink_to(capacity);
            }
            return Ok(());
        };

        owner.try_reserve(capacity)?;
        let pending = PendingReservation {
            owner: owner.clone(),
            bytes: capacity,
        };
        // `vec![value; n]` creates a backing whose reported capacity is exactly
        // `n`; unlike reserve growth, this does not rely on a private schedule.
        let mut replacement = alloc::vec![0; capacity];
        debug_assert_eq!(replacement.capacity(), capacity);
        replacement[..self.used].copy_from_slice(&self.buf[..self.used]);
        let old = mem::replace(&mut self.buf, replacement);
        let old_charge = mem::replace(&mut self.charged_capacity, capacity);
        drop(old);
        owner.release(old_charge);
        pending.commit();
        Ok(())
    }

    /// Read some bytes from `rd`, and add them to the buffer.
    pub(crate) fn read(&mut self, rd: &mut dyn io::Read, in_handshake: bool) -> io::Result<usize> {
        self.prepare_read(in_handshake)?;

        // Try to do the largest reads possible. Note that if
        // we get a message with a length field out of range here,
        // we do a zero length read.  That looks like an EOF to
        // the next layer up, which is fine.
        let new_bytes = rd.read(&mut self.buf[self.used..])?;
        self.used += new_bytes;
        Ok(new_bytes)
    }

    /// Resize the internal `buf` if necessary for reading more bytes.
    fn prepare_read(&mut self, is_joining_hs: bool) -> io::Result<()> {
        /// TLS allows for handshake messages of up to 16MB.  We
        /// restrict that to 64KB to limit potential for denial-of-
        /// service.
        const MAX_HANDSHAKE_SIZE: u32 = 0xffff;

        const READ_SIZE: usize = 4096;

        // We allow a maximum of 64k of buffered data for handshake messages only. Enforce this
        // by varying the maximum allowed buffer size here based on whether a prefix of a
        // handshake payload is currently being buffered. Given that the first read of such a
        // payload will only ever be 4k bytes, the next time we come around here we allow a
        // larger buffer size. Once the large message and any following handshake messages in
        // the same flight have been consumed, `pop()` will call `discard()` to reset `used`.
        // At this point, the buffer resizing logic below should reduce the buffer size.
        let allow_max = match is_joining_hs {
            true => MAX_HANDSHAKE_SIZE as usize,
            false => MAX_WIRE_SIZE,
        };

        if self.used >= allow_max {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "message buffer full",
            ));
        }

        // If we can and need to increase the buffer size to allow a 4k read, do so. After
        // dealing with a large handshake message (exceeding `OutboundOpaqueMessage::MAX_WIRE_SIZE`),
        // make sure to reduce the buffer size again (large messages should be rare).
        // Also, reduce the buffer size if there are neither full nor partial messages in it,
        // which usually means that the other side suspended sending data.
        let need_capacity = Ord::min(allow_max, self.used + READ_SIZE);
        if need_capacity > self.buf.len() || self.used == 0 || self.buf.len() > allow_max {
            self.replace_with_exact_capacity(need_capacity)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
        }

        Ok(())
    }

    /// Append `bytes` to the end of this buffer.
    ///
    /// Return a `Range` saying where it went.
    pub(crate) fn extend(&mut self, bytes: &[u8]) -> Range<usize> {
        let len = bytes.len();
        let start = self.used;
        let end = start + len;
        if self.buf.len() < end {
            self.buf.resize(end, 0);
        }
        self.buf[start..end].copy_from_slice(bytes);
        self.used += len;
        Range { start, end }
    }
}

/// A borrowed version of [`DeframerVecBuffer`] that tracks discard operations
#[derive(Debug)]
pub(crate) struct DeframerSliceBuffer<'a> {
    // a fully initialized buffer that will be deframed
    buf: &'a mut [u8],
    // number of bytes to discard from the front of `buf` at a later time
    discard: usize,
}

impl<'a> DeframerSliceBuffer<'a> {
    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, discard: 0 }
    }

    /// Tracks a pending discard operation of `num_bytes`
    pub(crate) fn queue_discard(&mut self, num_bytes: usize) {
        self.discard += num_bytes;
    }

    pub(crate) fn pending_discard(&self) -> usize {
        self.discard
    }

    pub(crate) fn filled_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.discard..]
    }
}

#[cfg(feature = "std")]
impl Drop for DeframerVecBuffer {
    fn drop(&mut self) {
        let backing = mem::take(&mut self.buf);
        drop(backing);
        if let Some(owner) = self.owner.take() {
            owner.release(mem::take(&mut self.charged_capacity));
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod owner_tests {
    use super::*;
    use std::io::{self, Read};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct Owner {
        limit: usize,
        used: AtomicUsize,
    }
    impl DeframerBufferOwner for Owner {
        fn try_reserve(&self, bytes: usize) -> Result<(), DeframerBufferError> {
            self.used
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |used| {
                    used.checked_add(bytes).filter(|next| *next <= self.limit)
                })
                .map(|_| ())
                .map_err(|_| DeframerBufferError)
        }
        fn release(&self, bytes: usize) {
            self.used.fetch_sub(bytes, Ordering::SeqCst);
        }
    }

    struct CountingReader {
        calls: usize,
        error: io::ErrorKind,
    }
    impl Read for CountingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            Err(self.error.into())
        }
    }

    #[test]
    fn denial_precedes_allocation_and_reader() {
        let owner = Arc::new(Owner {
            limit: 4095,
            used: AtomicUsize::new(0),
        });
        let mut buffer = DeframerVecBuffer::default();
        buffer.set_owner(owner.clone()).unwrap();
        let mut reader = CountingReader {
            calls: 0,
            error: io::ErrorKind::Other,
        };
        assert_eq!(
            buffer.read(&mut reader, false).unwrap_err().kind(),
            io::ErrorKind::Other
        );
        assert_eq!(reader.calls, 0);
        assert_eq!(buffer.buf.capacity(), 0);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn exact_capacity_growth_shrink_and_drop_custody() {
        let owner = Arc::new(Owner {
            limit: 16_384,
            used: AtomicUsize::new(0),
        });
        let mut buffer = DeframerVecBuffer::default();
        buffer.set_owner(owner.clone()).unwrap();
        buffer.prepare_read(false).unwrap();
        assert_eq!(
            (buffer.buf.capacity(), owner.used.load(Ordering::SeqCst)),
            (4096, 4096)
        );
        buffer.used = 4096;
        buffer.prepare_read(true).unwrap();
        assert_eq!(
            (buffer.buf.capacity(), owner.used.load(Ordering::SeqCst)),
            (8192, 8192)
        );
        buffer.used = 0;
        buffer.prepare_read(false).unwrap();
        assert_eq!(
            (buffer.buf.capacity(), owner.used.load(Ordering::SeqCst)),
            (4096, 4096)
        );
        drop(buffer);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn read_errors_and_would_block_retain_resized_backing() {
        for kind in [io::ErrorKind::Other, io::ErrorKind::WouldBlock] {
            let owner = Arc::new(Owner {
                limit: 4096,
                used: AtomicUsize::new(0),
            });
            let mut buffer = DeframerVecBuffer::default();
            buffer.set_owner(owner.clone()).unwrap();
            let mut reader = CountingReader {
                calls: 0,
                error: kind,
            };
            assert_eq!(buffer.read(&mut reader, false).unwrap_err().kind(), kind);
            assert_eq!(reader.calls, 1);
            assert_eq!(
                (buffer.buf.capacity(), owner.used.load(Ordering::SeqCst)),
                (4096, 4096)
            );
        }
    }

    #[test]
    fn unhooked_behavior_is_unchanged() {
        let mut buffer = DeframerVecBuffer::default();
        buffer.prepare_read(false).unwrap();
        assert!(buffer.owner.is_none());
        assert!(buffer.buf.capacity() >= 4096);
    }
}
