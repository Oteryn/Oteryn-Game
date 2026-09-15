use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::{cmp, mem};
#[cfg(feature = "std")]
use std::io;
#[cfg(feature = "std")]
use std::io::Read;

#[cfg(feature = "std")]
use crate::error::InvalidMessage;
#[cfg(feature = "std")]
use crate::msgs::codec::DirectDecodedCustody;
#[cfg(feature = "std")]
use crate::msgs::message::OutboundChunks;
#[cfg(feature = "std")]
use crate::{sync::Arc, DeframerBufferOwner};

/// A reservation from the existing connection ledger. Splitting transfers
/// custody without reserving again or allocating another owner/control object.
#[cfg(feature = "std")]
#[derive(Debug)]
pub(crate) struct OutboundTlsCustody {
    owner: Arc<dyn DeframerBufferOwner>,
    bytes: usize,
}

#[cfg(feature = "std")]
impl OutboundTlsCustody {
    pub(crate) fn from_reserved(custody: DirectDecodedCustody) -> Self {
        let (owner, bytes) = custody.into_parts();
        Self { owner, bytes }
    }

    pub(crate) fn owner(&self) -> Arc<dyn DeframerBufferOwner> {
        self.owner.clone()
    }

    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    pub(crate) fn split(&mut self, bytes: usize) -> Result<Self, InvalidMessage> {
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .ok_or(InvalidMessage::MessageTooLarge)?;
        Ok(Self {
            owner: self.owner.clone(),
            bytes,
        })
    }
}

#[cfg(feature = "std")]
impl Drop for OutboundTlsCustody {
    fn drop(&mut self) {
        if self.bytes != 0 {
            self.owner.release(self.bytes);
        }
    }
}

/// Field order destroys the exact vector before releasing its optional debit.
/// No mutable vector access escapes: capacity cannot grow behind the custodian.
#[derive(Debug)]
pub(crate) struct RetainedChunk {
    bytes: Vec<u8>,
    #[cfg(feature = "std")]
    custody: Option<OutboundTlsCustody>,
}

impl RetainedChunk {
    fn unowned(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            #[cfg(feature = "std")]
            custody: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn with_custody(
        bytes: Vec<u8>,
        custody: OutboundTlsCustody,
    ) -> Result<Self, InvalidMessage> {
        // Pair first, so even rejection destroys backing before its reservation.
        let mut chunk = Self {
            bytes,
            custody: Some(custody),
        };
        let custody = chunk.custody.as_mut().unwrap();
        if chunk.bytes.capacity() > custody.bytes {
            return Err(InvalidMessage::MessageTooLarge);
        }
        let unused = custody.bytes - chunk.bytes.capacity();
        drop(custody.split(unused)?);
        Ok(chunk)
    }
}

impl core::ops::Deref for RetainedChunk {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

/// This is a byte buffer that is built from a deque of byte vectors.
///
/// This avoids extra copies when appending a new byte vector,
/// at the expense of more complexity when reading out.
pub(crate) struct ChunkVecBuffer {
    /// How many bytes have been consumed in the first chunk.
    ///
    /// Invariant: zero if `self.is_empty()`
    /// Invariant: 0 <= `prefix_used` < `self.front().unwrap().len()` otherwise
    prefix_used: usize,

    chunks: VecDeque<RetainedChunk>,
    // Ordinary infallible sends may overflow an owned queue. Keep its exact
    // charged backing in place, and put only unowned overflow here. A later
    // owned insertion reserves before consolidating both queues.
    unowned_tail: VecDeque<RetainedChunk>,
    // Kept after logical drain; field order destroys deque backing first.
    #[cfg(feature = "std")]
    control_custody: Option<OutboundTlsCustody>,

    /// The total upper limit (in bytes) of this object.
    limit: Option<usize>,
}

impl ChunkVecBuffer {
    pub(crate) fn new(limit: Option<usize>) -> Self {
        Self {
            prefix_used: 0,
            chunks: VecDeque::new(),
            unowned_tail: VecDeque::new(),
            #[cfg(feature = "std")]
            control_custody: None,
            limit,
        }
    }

    /// Sets the upper limit on how many bytes this
    /// object can store.
    ///
    /// Setting a lower limit than the currently stored
    /// data is not an error.
    ///
    /// A [`None`] limit is interpreted as no limit.
    pub(crate) fn set_limit(&mut self, new_limit: Option<usize>) {
        self.limit = new_limit;
    }

    /// If we're empty
    pub(crate) fn is_empty(&self) -> bool {
        self.chunks.is_empty() && self.unowned_tail.is_empty()
    }

    /// How many bytes we're storing
    pub(crate) fn len(&self) -> usize {
        self.chunks
            .iter()
            .chain(self.unowned_tail.iter())
            .fold(0usize, |acc, chunk| acc + chunk.len())
            - self.prefix_used
    }

    /// For a proposed append of `len` bytes, how many
    /// bytes should we actually append to adhere to the
    /// currently set `limit`?
    pub(crate) fn apply_limit(&self, len: usize) -> usize {
        if let Some(limit) = self.limit {
            let space = limit.saturating_sub(self.len());
            cmp::min(len, space)
        } else {
            len
        }
    }

    /// Take and append the given `bytes`.
    pub(crate) fn append(&mut self, bytes: Vec<u8>) -> usize {
        let len = bytes.len();

        if !bytes.is_empty() {
            if self.is_empty() {
                debug_assert_eq!(self.prefix_used, 0);
            }

            #[cfg(feature = "std")]
            if !self.unowned_tail.is_empty()
                || (self.chunks.len() == self.chunks.capacity() && self.control_custody.is_some())
            {
                self.unowned_tail.push_back(RetainedChunk::unowned(bytes));
                return len;
            }
            self.chunks.push_back(RetainedChunk::unowned(bytes));
        }

        len
    }

    /// Take one of the chunks from this object.
    ///
    /// This function returns `None` if the object `is_empty`.
    pub(crate) fn pop(&mut self) -> Option<Vec<u8>> {
        self.pop_with_custody().map(|mut chunk| {
            // Only plaintext/unowned consumers may take a bare vector.
            #[cfg(feature = "std")]
            assert!(chunk.custody.is_none());
            mem::take(&mut chunk.bytes)
        })
    }

    /// Move the backing and its debit together, including a consumed prefix.
    pub(crate) fn pop_with_custody(&mut self) -> Option<RetainedChunk> {
        let mut first = if self.chunks.is_empty() {
            self.unowned_tail.pop_front()
        } else {
            self.chunks.pop_front()
        };

        if let Some(first) = &mut first {
            // slice off `prefix_used` if needed (uncommon)
            let prefix = mem::take(&mut self.prefix_used);
            first.bytes.drain(0..prefix);
        }

        first
    }

    /// Reserve before creating owner-aware queue/control backing. An unowned
    /// allocation is replaced even if it has spare room: never charge it after
    /// allocation. Existing owned capacity remains charged across replacement.
    #[cfg(feature = "std")]
    pub(crate) fn prepare_owned_append(
        &mut self,
        owner: Arc<dyn DeframerBufferOwner>,
        additional: usize,
    ) -> Result<(), InvalidMessage> {
        if additional == 0 {
            return Ok(());
        }
        if self
            .chunks
            .iter()
            .chain(self.unowned_tail.iter())
            .any(|chunk| {
                chunk
                    .custody
                    .as_ref()
                    .is_some_and(|custody| !Arc::ptr_eq(&custody.owner, &owner))
            })
        {
            return Err(InvalidMessage::MessageTooLarge);
        }
        if let Some(custody) = &self.control_custody {
            if !Arc::ptr_eq(&custody.owner, &owner) {
                return Err(InvalidMessage::MessageTooLarge);
            }
            if self.unowned_tail.is_empty()
                && self.chunks.capacity() - self.chunks.len() >= additional
            {
                return Ok(());
            }
        }
        let capacity = self
            .chunks
            .len()
            .checked_add(self.unowned_tail.len())
            .and_then(|len| len.checked_add(additional))
            .ok_or(InvalidMessage::MessageTooLarge)?;
        let bytes = core::alloc::Layout::array::<RetainedChunk>(capacity)
            .map_err(|_| InvalidMessage::MessageTooLarge)?
            .size();
        let custody =
            OutboundTlsCustody::from_reserved(DirectDecodedCustody::reserve(owner, bytes)?);
        let mut replacement = VecDeque::with_capacity(capacity);
        // Rust 1.94 VecDeque::with_capacity uses this exact element capacity.
        if replacement.capacity() != capacity {
            drop(replacement);
            return Err(InvalidMessage::MessageTooLarge);
        }
        replacement.append(&mut self.chunks);
        replacement.append(&mut self.unowned_tail);
        drop(mem::replace(&mut self.chunks, replacement));
        drop(mem::take(&mut self.unowned_tail));
        drop(self.control_custody.replace(custody));
        Ok(())
    }

    /// Insertion itself is allocation-free; capacity was prepared on the same
    /// ledger before constructing the covered record backing.
    #[cfg(feature = "std")]
    pub(crate) fn append_with_custody(
        &mut self,
        chunk: RetainedChunk,
    ) -> Result<usize, InvalidMessage> {
        let Some(custody) = &chunk.custody else {
            return Err(InvalidMessage::MessageTooLarge);
        };
        if !self
            .control_custody
            .as_ref()
            .is_some_and(|control| Arc::ptr_eq(&control.owner, &custody.owner))
            || self.chunks.len() == self.chunks.capacity()
            || !self.unowned_tail.is_empty()
        {
            return Err(InvalidMessage::MessageTooLarge);
        }
        let len = chunk.len();
        if len != 0 {
            self.chunks.push_back(chunk);
        }
        Ok(len)
    }

    #[cfg(read_buf)]
    /// Read data out of this object, writing it into `cursor`.
    pub(crate) fn read_buf(
        &mut self,
        mut cursor: core::io::BorrowedCursor<'_, u8>,
    ) -> io::Result<()> {
        while !self.is_empty() && cursor.capacity() > 0 {
            let chunk = &self.front().unwrap()[self.prefix_used..];
            let used = cmp::min(chunk.len(), cursor.capacity());
            cursor.append(&chunk[..used]);
            self.consume(used);
        }

        Ok(())
    }

    /// Inspect the first chunk from this object.
    pub(crate) fn peek(&self) -> Option<&[u8]> {
        self.front().map(|ch| ch.as_slice())
    }

    fn front(&self) -> Option<&RetainedChunk> {
        self.chunks.front().or_else(|| self.unowned_tail.front())
    }
}

#[cfg(feature = "std")]
impl ChunkVecBuffer {
    pub(crate) fn is_full(&self) -> bool {
        self.limit
            .map(|limit| self.len() > limit)
            .unwrap_or_default()
    }

    /// Append a copy of `bytes`, perhaps a prefix if
    /// we're near the limit.
    pub(crate) fn append_limited_copy(&mut self, payload: OutboundChunks<'_>) -> usize {
        let take = self.apply_limit(payload.len());
        self.append(payload.split_at(take).0.to_vec());
        take
    }

    /// Read data out of this object, writing it into `buf`
    /// and returning how many bytes were written there.
    pub(crate) fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut offs = 0;

        while offs < buf.len() && !self.is_empty() {
            let used = (&self.front().unwrap()[self.prefix_used..]).read(&mut buf[offs..])?;

            self.consume(used);
            offs += used;
        }

        Ok(offs)
    }

    pub(crate) fn consume_first_chunk(&mut self, used: usize) {
        // this backs (infallible) `BufRead::consume`, where `used` is
        // user-supplied.
        assert!(
            used <= self
                .chunk()
                .map(|ch| ch.len())
                .unwrap_or_default(),
            "illegal `BufRead::consume` usage",
        );
        self.consume(used);
    }

    fn consume(&mut self, used: usize) {
        // first, mark the rightmost extent of the used buffer
        self.prefix_used += used;

        // then reduce `prefix_used` by discarding wholly-covered
        // buffers
        while let Some(buf) = self.front() {
            if self.prefix_used < buf.len() {
                return;
            } else {
                self.prefix_used -= buf.len();
                if self.chunks.is_empty() {
                    self.unowned_tail.pop_front();
                } else {
                    self.chunks.pop_front();
                }
            }
        }

        debug_assert_eq!(
            self.prefix_used, 0,
            "attempted to `ChunkVecBuffer::consume` more than available"
        );
    }

    /// Read data out of this object, passing it `wr`
    pub(crate) fn write_to(&mut self, wr: &mut dyn io::Write) -> io::Result<usize> {
        if self.is_empty() {
            return Ok(0);
        }

        let mut prefix = self.prefix_used;
        let mut bufs = [io::IoSlice::new(&[]); 64];
        for (iov, chunk) in bufs
            .iter_mut()
            .zip(self.chunks.iter().chain(self.unowned_tail.iter()))
        {
            *iov = io::IoSlice::new(&chunk[prefix..]);
            prefix = 0;
        }
        let len = cmp::min(bufs.len(), self.chunks.len() + self.unowned_tail.len());
        let bufs = &bufs[..len];
        let used = wr.write_vectored(bufs)?;
        let available_bytes = bufs.iter().map(|ch| ch.len()).sum();

        if used > available_bytes {
            // This is really unrecoverable, since the amount of data written
            // is now unknown.  Consume all the potentially-written data in
            // case the caller ignores the error.
            // See <https://github.com/rustls/rustls/issues/2316> for background.
            self.consume(available_bytes);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                std::format!("illegal write_vectored return value ({used} > {available_bytes})"),
            ));
        }
        self.consume(used);
        Ok(used)
    }

    /// Returns the first contiguous chunk of data, or None if empty.
    pub(crate) fn chunk(&self) -> Option<&[u8]> {
        self.front().map(|chunk| &chunk[self.prefix_used..])
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::ChunkVecBuffer;

    #[test]
    fn short_append_copy_with_limit() {
        let mut cvb = ChunkVecBuffer::new(Some(12));
        assert_eq!(cvb.append_limited_copy(b"hello"[..].into()), 5);
        assert_eq!(cvb.append_limited_copy(b"world"[..].into()), 5);
        assert_eq!(cvb.append_limited_copy(b"hello"[..].into()), 2);
        assert_eq!(cvb.append_limited_copy(b"world"[..].into()), 0);

        let mut buf = [0u8; 12];
        assert_eq!(cvb.read(&mut buf).unwrap(), 12);
        assert_eq!(buf.to_vec(), b"helloworldhe".to_vec());
    }

    #[test]
    fn read_byte_by_byte() {
        let mut cvb = ChunkVecBuffer::new(None);
        cvb.append(b"test fixture data".to_vec());
        assert!(!cvb.is_empty());
        for expect in b"test fixture data" {
            let mut byte = [0];
            assert_eq!(cvb.read(&mut byte).unwrap(), 1);
            assert_eq!(byte[0], *expect);
        }

        assert_eq!(cvb.read(&mut [0]).unwrap(), 0);
    }

    #[test]
    fn every_possible_chunk_interleaving() {
        let input = (0..=0xffu8)
            .cycle()
            .take(4096)
            .collect::<Vec<u8>>();

        for input_chunk_len in 1..64usize {
            for output_chunk_len in 1..65usize {
                std::println!("check input={input_chunk_len} output={output_chunk_len}");
                let mut cvb = ChunkVecBuffer::new(None);
                for chunk in input.chunks(input_chunk_len) {
                    cvb.append(chunk.to_vec());
                }

                assert_eq!(cvb.len(), input.len());
                let mut buf = vec![0u8; output_chunk_len];

                for expect in input.chunks(output_chunk_len) {
                    assert_eq!(expect.len(), cvb.read(&mut buf).unwrap());
                    assert_eq!(expect, &buf[..expect.len()]);
                }

                assert_eq!(cvb.read(&mut [0]).unwrap(), 0);
            }
        }
    }

    #[cfg(read_buf)]
    #[test]
    fn read_buf() {
        use core::io::BorrowedBuf;
        use core::mem::MaybeUninit;

        {
            let mut cvb = ChunkVecBuffer::new(None);
            cvb.append(b"test ".to_vec());
            cvb.append(b"fixture ".to_vec());
            cvb.append(b"data".to_vec());

            let mut buf = [MaybeUninit::<u8>::uninit(); 8];
            let mut buf: BorrowedBuf<'_, u8> = buf.as_mut_slice().into();
            cvb.read_buf(buf.unfilled()).unwrap();
            assert_eq!(buf.filled(), b"test fix");
            buf.clear();
            cvb.read_buf(buf.unfilled()).unwrap();
            assert_eq!(buf.filled(), b"ture dat");
            buf.clear();
            cvb.read_buf(buf.unfilled()).unwrap();
            assert_eq!(buf.filled(), b"a");
        }

        {
            let mut cvb = ChunkVecBuffer::new(None);
            cvb.append(b"short message".to_vec());

            let mut buf = [MaybeUninit::<u8>::uninit(); 1024];
            let mut buf: BorrowedBuf<'_, u8> = buf.as_mut_slice().into();
            cvb.read_buf(buf.unfilled()).unwrap();
            assert_eq!(buf.filled(), b"short message");
        }
    }
}

#[cfg(bench)]
mod benchmarks {
    use alloc::vec;

    use super::ChunkVecBuffer;

    #[bench]
    fn read_one_byte_from_large_message(b: &mut test::Bencher) {
        b.iter(|| {
            let mut cvb = ChunkVecBuffer::new(None);
            cvb.append(vec![0u8; 16_384]);
            assert_eq!(1, cvb.read(&mut [0u8]).unwrap());
        });
    }

    #[bench]
    fn read_all_individual_from_large_message(b: &mut test::Bencher) {
        b.iter(|| {
            let mut cvb = ChunkVecBuffer::new(None);
            cvb.append(vec![0u8; 16_384]);
            loop {
                if let Ok(0) = cvb.read(&mut [0u8]) {
                    break;
                }
            }
        });
    }

    #[bench]
    fn read_half_bytes_from_large_message(b: &mut test::Bencher) {
        b.iter(|| {
            let mut cvb = ChunkVecBuffer::new(None);
            cvb.append(vec![0u8; 16_384]);
            assert_eq!(8192, cvb.read(&mut [0u8; 8192]).unwrap());
            assert_eq!(8192, cvb.read(&mut [0u8; 8192]).unwrap());
        });
    }

    #[bench]
    fn read_entire_large_message(b: &mut test::Bencher) {
        b.iter(|| {
            let mut cvb = ChunkVecBuffer::new(None);
            cvb.append(vec![0u8; 16_384]);
            assert_eq!(16_384, cvb.read(&mut [0u8; 16_384]).unwrap());
        });
    }
}

#[cfg(all(test, feature = "std"))]
pub(crate) mod outbound_custody_tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[derive(Debug)]
    pub(crate) struct Owner {
        pub(crate) used: AtomicUsize,
        pub(crate) peak: AtomicUsize,
        pub(crate) attempts: AtomicUsize,
        pub(crate) limit: AtomicUsize,
        pub(crate) released: Mutex<Vec<usize>>,
    }

    impl Owner {
        pub(crate) fn new(limit: usize) -> Arc<Self> {
            Arc::new(Self {
                used: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                attempts: AtomicUsize::new(0),
                limit: AtomicUsize::new(limit),
                released: Mutex::new(Vec::new()),
            })
        }

        pub(crate) fn used(&self) -> usize {
            self.used.load(Ordering::SeqCst)
        }

        pub(crate) fn reserve(self: &Arc<Self>, bytes: usize) -> OutboundTlsCustody {
            OutboundTlsCustody::from_reserved(
                DirectDecodedCustody::reserve(self.clone(), bytes).unwrap(),
            )
        }

        pub(crate) fn chunk(self: &Arc<Self>, bytes: &[u8]) -> RetainedChunk {
            let custody = self.reserve(bytes.len());
            RetainedChunk::with_custody(crate::msgs::codec::exact_vec_copy(bytes), custody).unwrap()
        }
    }

    impl DeframerBufferOwner for Owner {
        fn try_reserve(&self, bytes: usize) -> Result<(), crate::DeframerBufferError> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            let total = self
                .used()
                .checked_add(bytes)
                .ok_or(crate::DeframerBufferError)?;
            if total > self.limit.load(Ordering::SeqCst) {
                return Err(crate::DeframerBufferError);
            }
            self.used.store(total, Ordering::SeqCst);
            self.peak.fetch_max(total, Ordering::SeqCst);
            Ok(())
        }
        fn release(&self, bytes: usize) {
            assert!(self.used.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
            if bytes != 0 {
                self.released.lock().unwrap().push(bytes);
            }
        }
    }

    pub(crate) fn control_bytes(chunks: usize) -> usize {
        size_of::<RetainedChunk>() * chunks
    }

    #[test]
    fn custody_partial_wouldblock_error_pop_and_drop() {
        let owner = Owner::new(control_bytes(2) + 9);
        let mut queue = ChunkVecBuffer::new(Some(12));
        queue.prepare_owned_append(owner.clone(), 2).unwrap();
        let first = owner.chunk(b"hello");
        let pointer = first.as_ptr();
        queue.append_with_custody(first).unwrap();
        queue.append_with_custody(owner.chunk(b"rust")).unwrap();
        let charged = owner.used();
        let attempts = owner.attempts.load(Ordering::SeqCst);
        let mut output = [0; 2];
        assert_eq!(queue.write_to(&mut output.as_mut_slice()).unwrap(), 2);
        assert_eq!(&output, b"he");
        assert_eq!(owner.used(), charged);
        struct Fails(io::ErrorKind);
        impl io::Write for Fails {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(self.0.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for kind in [io::ErrorKind::WouldBlock, io::ErrorKind::BrokenPipe] {
            assert_eq!(queue.write_to(&mut Fails(kind)).unwrap_err().kind(), kind);
            assert_eq!(owner.used(), charged);
        }
        let moved = queue.pop_with_custody().unwrap();
        assert_eq!(moved.as_ptr(), pointer);
        assert_eq!(moved.capacity(), 5);
        assert_eq!(moved.as_slice(), b"llo");
        assert_eq!(owner.used(), charged);
        assert_eq!(owner.attempts.load(Ordering::SeqCst), attempts);
        drop(moved);
        assert_eq!(owner.used(), control_bytes(2) + 4);
        let mut output = Vec::new();
        assert_eq!(queue.write_to(&mut output).unwrap(), 4);
        assert_eq!(output, b"rust");
        assert!(queue.is_empty());
        assert_eq!(owner.used(), control_bytes(2));
        drop(queue);
        assert_eq!(owner.used(), 0);
        assert_eq!(*owner.released.lock().unwrap(), [5, 4, control_bytes(2)]);
    }

    #[test]
    fn custody_control_growth_denial_overlap_and_churn() {
        let owner = Owner::new(control_bytes(3) + 8 - 1);
        let mut queue = ChunkVecBuffer::new(None);
        queue.prepare_owned_append(owner.clone(), 1).unwrap();
        queue.append_with_custody(owner.chunk(b"aaaa")).unwrap();
        let second = owner.chunk(b"bbbb");
        let first_pointer = queue.peek().unwrap().as_ptr();
        assert!(queue.prepare_owned_append(owner.clone(), 1).is_err());
        assert_eq!(queue.chunks.capacity(), 1);
        assert_eq!(queue.peek().unwrap().as_ptr(), first_pointer);
        assert_eq!(owner.used(), control_bytes(1) + 8);
        owner.limit.fetch_add(1, Ordering::SeqCst);
        queue.prepare_owned_append(owner.clone(), 1).unwrap();
        assert_eq!(owner.peak.load(Ordering::SeqCst), control_bytes(3) + 8);
        assert_eq!(owner.used(), control_bytes(2) + 8);
        queue.append_with_custody(second).unwrap();
        let mut output = Vec::new();
        assert_eq!(queue.write_to(&mut output).unwrap(), 8);
        assert_eq!(output, b"aaaabbbb");
        for _ in 0..5 {
            queue.prepare_owned_append(owner.clone(), 1).unwrap();
            queue.append_with_custody(owner.chunk(b"data")).unwrap();
            let moved = queue.pop_with_custody().unwrap();
            assert_eq!(owner.used(), control_bytes(2) + 4);
            drop(moved);
            assert_eq!(owner.used(), control_bytes(2));
        }
        drop(queue);
        assert_eq!(owner.used(), 0);
    }

    #[test]
    fn custody_unowned_overflow_retains_control_and_owner_identity() {
        let owner = Owner::new(usize::MAX);
        let other = Owner::new(usize::MAX);
        let mut queue = ChunkVecBuffer::new(None);
        queue.append(b"before".to_vec());
        assert_eq!(owner.used(), 0);
        queue.prepare_owned_append(owner.clone(), 1).unwrap();
        queue.append_with_custody(owner.chunk(b"owned")).unwrap();
        // Ordinary overflow must keep the owned queue backing charged, without
        // making the infallible send reserve or charge application-data storage.
        queue.append(b"after".to_vec());
        assert_eq!(owner.used(), control_bytes(2) + 5);
        assert!(queue.prepare_owned_append(other.clone(), 1).is_err());
        assert_eq!(other.used(), 0);
        queue.prepare_owned_append(owner.clone(), 1).unwrap();
        queue.append_with_custody(owner.chunk(b"last")).unwrap();
        let mut output = Vec::new();
        queue.write_to(&mut output).unwrap();
        assert_eq!(output, b"beforeownedafterlast");
        assert_eq!(owner.used(), control_bytes(4));
        drop(queue);
        assert_eq!(owner.used(), 0);
    }

    #[test]
    fn custody_drop_queue_and_invalid_writer_release_once() {
        let owner = Owner::new(usize::MAX);
        let mut queue = ChunkVecBuffer::new(None);
        queue.prepare_owned_append(owner.clone(), 2).unwrap();
        queue.append_with_custody(owner.chunk(b"abc")).unwrap();
        queue.append_with_custody(owner.chunk(b"defg")).unwrap();
        struct InvalidWriter;
        impl io::Write for InvalidWriter {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Ok(usize::MAX)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(queue.write_to(&mut InvalidWriter).is_err());
        assert_eq!(owner.used(), control_bytes(2));
        queue
            .append_with_custody(owner.chunk(b"cancelled"))
            .unwrap();
        drop(queue);
        assert_eq!(owner.used(), 0);
        assert_eq!(*owner.released.lock().unwrap(), [3, 4, 9, control_bytes(2)]);
    }

    #[test]
    fn custody_initial_control_denial_precedes_allocation() {
        let owner = Owner::new(control_bytes(1) - 1);
        let mut queue = ChunkVecBuffer::new(None);
        assert!(queue.prepare_owned_append(owner.clone(), 1).is_err());
        assert_eq!(queue.chunks.capacity(), 0);
        assert_eq!(owner.used(), 0);
        owner.limit.store(usize::MAX, Ordering::SeqCst);
        assert!(queue
            .prepare_owned_append(owner.clone(), usize::MAX)
            .is_err());
        assert_eq!(owner.attempts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn custody_partial_unowned_tail_survives_owned_consolidation() {
        let owner = Owner::new(usize::MAX);
        let mut queue = ChunkVecBuffer::new(None);
        queue.prepare_owned_append(owner.clone(), 1).unwrap();
        queue.append_with_custody(owner.chunk(b"first")).unwrap();
        queue.append(b"second".to_vec());
        let mut prefix = [0; 7];
        assert_eq!(queue.read(&mut prefix).unwrap(), 7);
        assert_eq!(&prefix, b"firstse");
        assert_eq!(owner.used(), control_bytes(1));
        assert_eq!(queue.chunk().unwrap(), b"cond");
        queue.prepare_owned_append(owner.clone(), 1).unwrap();
        queue.append_with_custody(owner.chunk(b"third")).unwrap();
        let mut output = Vec::new();
        assert_eq!(queue.write_to(&mut output).unwrap(), 9);
        assert_eq!(output, b"condthird");
        assert_eq!(owner.used(), control_bytes(2));
        // Once logically empty, a new unowned send may use existing main
        // capacity without releasing or growing that owned allocation.
        queue.append(b"again".to_vec());
        assert_eq!(queue.pop_with_custody().unwrap().as_slice(), b"again");
        assert_eq!(owner.used(), control_bytes(2));
        drop(queue);
        assert_eq!(owner.used(), 0);
    }
}
