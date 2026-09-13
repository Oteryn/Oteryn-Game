use crate::error::Error;
use crate::net::resource_budget::{ResourceBudget, ResourceReservation};
use crate::net::Socket;
use bytes::BytesMut;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::{cmp, io};

use crate::io::{AsyncRead, AsyncReadExt, ProtocolDecode, ProtocolEncode};

// Tokio, async-std, and std all use this as the default capacity for their buffered I/O.
const DEFAULT_BUF_SIZE: usize = 8192;

pub struct BufferedSocket<S> {
    socket: S,
    write_buf: WriteBuffer,
    read_buf: ReadBuffer,
    owned_read: Option<OwnedRead>,
}

pub struct WriteBuffer {
    buf: Vec<u8>,
    bytes_written: usize,
    bytes_flushed: usize,
    allocation: Option<ResourceReservation>,
    budget: Option<Arc<dyn ResourceBudget>>,
}

pub struct ReadBuffer {
    read: BytesMut,
    available: BytesMut,
}

impl<S: Socket> BufferedSocket<S> {
    pub fn new(socket: S) -> Self
    where
        S: Sized,
    {
        BufferedSocket {
            socket,
            write_buf: WriteBuffer {
                buf: Vec::with_capacity(DEFAULT_BUF_SIZE),
                bytes_written: 0,
                bytes_flushed: 0,
                allocation: None,
                budget: None,
            },
            read_buf: ReadBuffer {
                read: BytesMut::new(),
                available: BytesMut::with_capacity(DEFAULT_BUF_SIZE),
            },
            owned_read: None,
        }
    }

    pub fn new_owned(socket: S, budget: Arc<dyn ResourceBudget>) -> Result<Self, Error> {
        let write_allocation =
            ResourceReservation::try_new(budget.clone(), DEFAULT_BUF_SIZE).map_err(|_| denied())?;
        let read_allocation =
            ResourceReservation::try_new(budget.clone(), DEFAULT_BUF_SIZE).map_err(|_| denied())?;
        Ok(Self {
            socket,
            write_buf: WriteBuffer {
                buf: Vec::with_capacity(DEFAULT_BUF_SIZE),
                bytes_written: 0,
                bytes_flushed: 0,
                allocation: Some(write_allocation),
                budget: Some(budget.clone()),
            },
            read_buf: ReadBuffer {
                read: BytesMut::new(),
                available: BytesMut::new(),
            },
            owned_read: Some(OwnedRead {
                buf: Vec::with_capacity(DEFAULT_BUF_SIZE),
                allocation: read_allocation,
                budget,
            }),
        })
    }

    pub async fn peek_owned(&mut self, len: usize) -> Result<&[u8], Error> {
        let read = self.owned_read.as_mut().ok_or_else(denied)?;
        read.reserve(len)?;
        while read.buf.len() < len {
            let start = read.buf.len();
            let mut pending = PendingRead {
                buf: &mut read.buf,
                committed: start,
            };
            pending.buf.resize(len, 0);
            match self.socket.read(&mut &mut pending.buf[start..]).await {
                Ok(0) => return Err(Error::Io(io::ErrorKind::UnexpectedEof.into())),
                Ok(n) => pending.committed = start + n,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(&read.buf[..len])
    }

    pub async fn read_owned_buffered(&mut self, len: usize) -> Result<OwnedBytes, Error> {
        self.peek_owned(len).await?;
        let read = self.owned_read.as_mut().ok_or_else(denied)?;
        let output = OwnedBytes::try_copy_from_slice(&read.buf[..len], read.budget.clone())
            .map_err(|_| denied())?;
        read.buf.copy_within(len.., 0);
        read.buf.truncate(read.buf.len() - len);
        Ok(output)
    }

    /// `size` is a source-proven encoder upper bound, not a sizing hint.
    pub fn write_precharged(
        &mut self,
        size: usize,
        encode: impl FnOnce(&mut Vec<u8>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let len = self.write_buf.bytes_written;
        let target = len.checked_add(size).ok_or_else(denied)?;
        self.write_buf.reserve_owned(target)?;
        self.write_buf.buf.truncate(len);
        if let Err(error) = encode(&mut self.write_buf.buf) {
            self.write_buf.buf.truncate(len);
            return Err(error);
        }
        self.write_buf.bytes_written = self.write_buf.buf.len();
        Ok(())
    }

    pub async fn read_buffered(&mut self, len: usize) -> Result<BytesMut, Error> {
        self.try_read(|buf| {
            Ok(if buf.len() < len {
                ControlFlow::Continue(len)
            } else {
                ControlFlow::Break(buf.split_to(len))
            })
        })
        .await
    }

    /// Retryable read operation.
    ///
    /// The callback should check the contents of the buffer passed to it and either:
    ///
    /// * Remove a full message from the buffer and return [`ControlFlow::Break`], or:
    /// * Return [`ControlFlow::Continue`] with the expected _total_ length of the buffer,
    ///   _without_ modifying it.
    ///
    /// Cancel-safe as long as the callback does not modify the passed `BytesMut`
    /// before returning [`ControlFlow::Continue`].
    pub async fn try_read<F, R>(&mut self, mut try_read: F) -> Result<R, Error>
    where
        F: FnMut(&mut BytesMut) -> Result<ControlFlow<R, usize>, Error>,
    {
        if self.owned_read.is_some() {
            return Err(denied());
        }
        loop {
            let read_len = match try_read(&mut self.read_buf.read)? {
                ControlFlow::Continue(read_len) => read_len,
                ControlFlow::Break(ret) => return Ok(ret),
            };

            self.read_buf.read(read_len, &mut self.socket).await?;
        }
    }

    pub fn write_buffer(&self) -> &WriteBuffer {
        &self.write_buf
    }

    pub fn write_buffer_mut(&mut self) -> &mut WriteBuffer {
        &mut self.write_buf
    }

    pub async fn read<'de, T>(&mut self, byte_len: usize) -> Result<T, Error>
    where
        T: ProtocolDecode<'de, ()>,
    {
        self.read_with(byte_len, ()).await
    }

    pub async fn read_with<'de, T, C>(&mut self, byte_len: usize, context: C) -> Result<T, Error>
    where
        T: ProtocolDecode<'de, C>,
    {
        T::decode_with(self.read_buffered(byte_len).await?.freeze(), context)
    }

    #[inline(always)]
    pub fn write<'en, T>(&mut self, value: T) -> Result<(), Error>
    where
        T: ProtocolEncode<'en, ()>,
    {
        self.write_with(value, ())
    }

    #[inline(always)]
    pub fn write_with<'en, T, C>(&mut self, value: T, context: C) -> Result<(), Error>
    where
        T: ProtocolEncode<'en, C>,
    {
        if self.write_buf.budget.is_some() {
            return Err(denied());
        }
        value.encode_with(self.write_buf.buf_mut(), context)?;
        self.write_buf.bytes_written = self.write_buf.buf.len();
        self.write_buf.sanity_check();

        Ok(())
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        while !self.write_buf.is_empty() {
            let written = self.socket.write(self.write_buf.get()).await?;
            if written == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            self.write_buf.consume(written);
            self.write_buf.sanity_check();
        }

        self.socket.flush().await?;

        Ok(())
    }

    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.flush().await?;
        self.socket.shutdown().await
    }

    pub fn shrink_buffers(&mut self) {
        // Won't drop data still in the buffer.
        self.write_buf.shrink();
        self.read_buf.shrink();
        if let Some(read) = &mut self.owned_read {
            read.shrink();
        }
    }

    pub fn into_inner(self) -> S {
        self.socket
    }

    pub fn boxed(self) -> BufferedSocket<Box<dyn Socket>> {
        assert!(
            self.owned_read.is_none(),
            "owned socket must retain its charged box"
        );
        BufferedSocket {
            socket: Box::new(self.socket),
            write_buf: self.write_buf,
            read_buf: self.read_buf,
            owned_read: self.owned_read,
        }
    }
}

fn denied() -> Error {
    Error::Io(io::ErrorKind::OutOfMemory.into())
}

struct PendingRead<'a> {
    buf: &'a mut Vec<u8>,
    committed: usize,
}
impl Drop for PendingRead<'_> {
    fn drop(&mut self) {
        self.buf.truncate(self.committed);
    }
}

struct OwnedRead {
    buf: Vec<u8>,
    allocation: ResourceReservation,
    budget: Arc<dyn ResourceBudget>,
}

impl OwnedRead {
    fn reserve(&mut self, target: usize) -> Result<(), Error> {
        if target <= self.buf.capacity() {
            return Ok(());
        }
        let allocation =
            ResourceReservation::try_new(self.budget.clone(), target).map_err(|_| denied())?;
        let mut replacement = Vec::with_capacity(target);
        replacement.extend_from_slice(&self.buf);
        drop(std::mem::replace(&mut self.buf, replacement));
        self.allocation = allocation;
        Ok(())
    }
    fn shrink(&mut self) {
        let target = self.buf.len().max(DEFAULT_BUF_SIZE);
        if target >= self.buf.capacity() {
            return;
        }
        if let Ok(allocation) = ResourceReservation::try_new(self.budget.clone(), target) {
            let mut replacement = Vec::with_capacity(target);
            replacement.extend_from_slice(&self.buf);
            drop(std::mem::replace(&mut self.buf, replacement));
            self.allocation = allocation;
        }
    }
}

impl WriteBuffer {
    fn reserve_owned(&mut self, target: usize) -> Result<(), Error> {
        let Some(budget) = &self.budget else {
            self.buf.reserve(target.saturating_sub(self.buf.len()));
            return Ok(());
        };
        if target <= self.buf.capacity() {
            return Ok(());
        }
        let allocation =
            ResourceReservation::try_new(budget.clone(), target).map_err(|_| denied())?;
        let mut replacement = Vec::with_capacity(target);
        replacement.extend_from_slice(&self.buf[..self.bytes_written]);
        drop(std::mem::replace(&mut self.buf, replacement));
        self.allocation = Some(allocation);
        Ok(())
    }

    fn sanity_check(&self) {
        assert_ne!(self.buf.capacity(), 0);
        assert!(self.bytes_written <= self.buf.len());
        assert!(self.bytes_flushed <= self.bytes_written);
    }

    pub fn buf_mut(&mut self) -> &mut Vec<u8> {
        assert!(
            self.budget.is_none(),
            "owned encoding requires prospective reservation"
        );
        self.buf.truncate(self.bytes_written);
        self.sanity_check();
        &mut self.buf
    }

    pub fn init_remaining_mut(&mut self) -> &mut [u8] {
        self.buf.resize(self.buf.capacity(), 0);
        self.sanity_check();
        &mut self.buf[self.bytes_written..]
    }

    pub fn put_slice(&mut self, slice: &[u8]) {
        assert!(
            self.budget.is_none() || slice.len() <= self.buf.capacity() - self.bytes_written,
            "owned write exceeds reserved backing"
        );
        // If we already have an initialized area that can fit the slice,
        // don't change `self.buf.len()`
        if let Some(dest) = self.buf[self.bytes_written..].get_mut(..slice.len()) {
            dest.copy_from_slice(slice);
        } else {
            self.buf.truncate(self.bytes_written);
            self.buf.extend_from_slice(slice);
        }
        self.advance(slice.len());
        self.sanity_check();
    }

    pub fn advance(&mut self, amt: usize) {
        let new_bytes_written = self
            .bytes_written
            .checked_add(amt)
            .expect("self.bytes_written + amt overflowed");

        assert!(new_bytes_written <= self.buf.len());

        self.bytes_written = new_bytes_written;

        self.sanity_check();
    }

    /// Read into the buffer from `source`, returning the number of bytes read.
    ///
    /// The buffer is automatically advanced by the number of bytes read.
    pub async fn read_from(&mut self, mut source: impl AsyncRead + Unpin) -> io::Result<usize> {
        if self.budget.is_some() {
            return Err(io::ErrorKind::Unsupported.into());
        }
        let read = match () {
            // Tokio lets us read into the buffer without zeroing first
            #[cfg(feature = "_rt-tokio")]
            _ => source.read_buf(self.buf_mut()).await?,
            #[cfg(not(feature = "_rt-tokio"))]
            _ => source.read(self.init_remaining_mut()).await?,
        };

        if read > 0 {
            self.advance(read);
        }

        Ok(read)
    }

    pub fn is_empty(&self) -> bool {
        self.bytes_flushed >= self.bytes_written
    }

    pub fn is_full(&self) -> bool {
        self.bytes_written == self.buf.len()
    }

    pub fn get(&self) -> &[u8] {
        &self.buf[self.bytes_flushed..self.bytes_written]
    }

    pub fn get_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.bytes_flushed..self.bytes_written]
    }

    pub fn shrink(&mut self) {
        if self.bytes_flushed > 0 {
            // Move any data that remains to be flushed to the beginning of the buffer,
            // if necessary.
            self.buf
                .copy_within(self.bytes_flushed..self.bytes_written, 0);
            self.bytes_written -= self.bytes_flushed;
            self.bytes_flushed = 0
        }

        if let Some(budget) = &self.budget {
            let target = self.bytes_written.max(DEFAULT_BUF_SIZE);
            if target < self.buf.capacity() {
                if let Ok(allocation) = ResourceReservation::try_new(budget.clone(), target) {
                    let mut replacement = Vec::with_capacity(target);
                    replacement.extend_from_slice(&self.buf[..self.bytes_written]);
                    drop(std::mem::replace(&mut self.buf, replacement));
                    self.allocation = Some(allocation);
                }
            }
            return;
        }

        // Drop excess capacity.
        self.buf
            .truncate(cmp::max(self.bytes_written, DEFAULT_BUF_SIZE));
        self.buf.shrink_to_fit();
    }

    fn consume(&mut self, amt: usize) {
        let new_bytes_flushed = self
            .bytes_flushed
            .checked_add(amt)
            .expect("self.bytes_flushed + amt overflowed");

        assert!(new_bytes_flushed <= self.bytes_written);

        self.bytes_flushed = new_bytes_flushed;

        if self.bytes_flushed == self.bytes_written {
            // Reset cursors to zero if we've consumed the whole buffer
            self.bytes_flushed = 0;
            self.bytes_written = 0;
        }

        self.sanity_check();
    }
}

impl ReadBuffer {
    async fn read(&mut self, len: usize, socket: &mut impl Socket) -> io::Result<()> {
        // Because of how `BytesMut` works, we should only be shifting capacity back and forth
        // between `read` and `available` unless we have to read an oversize message.
        while self.read.len() < len {
            self.reserve(len - self.read.len());

            let read = socket.read(&mut self.available).await?;

            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!(
                        "expected to read {} bytes, got {} bytes at EOF",
                        len,
                        self.read.len()
                    ),
                ));
            }

            self.advance(read);
        }

        Ok(())
    }

    fn reserve(&mut self, amt: usize) {
        if let Some(additional) = amt.checked_sub(self.available.capacity()) {
            self.available.reserve(additional);
        }
    }

    fn advance(&mut self, amt: usize) {
        self.read.unsplit(self.available.split_to(amt));
    }

    fn shrink(&mut self) {
        if self.available.capacity() > DEFAULT_BUF_SIZE {
            // `BytesMut` doesn't have a way to shrink its capacity,
            // but we only use `available` for spare capacity anyway so we can just replace it.
            //
            // If `self.read` still contains data on the next call to `advance` then this might
            // force a memcpy as they'll no longer be pointing to the same allocation,
            // but that's kind of unavoidable.
            //
            // The `async-std` impl of `Socket` will also need to re-zero the buffer,
            // but that's also kind of unavoidable.
            //
            // We should be warning the user not to call this often.
            self.available = BytesMut::with_capacity(DEFAULT_BUF_SIZE);
        }
    }
}

/// Bytes whose private shared custody follows every permitted descendant.
/// Raw `Bytes` cannot escape from an owned value. The data handle is destroyed
/// before its finality handle, including when final drops run concurrently.
#[derive(Clone, Debug)]
pub struct OwnedBytes {
    bytes: bytes::Bytes,
    lease: Option<BytesLease>,
}

#[derive(Debug)]
struct BytesLease(Option<std::sync::Arc<BytesCustody>>);

struct BytesCustody {
    reservation: crate::net::resource_budget::ResourceReservation,
    budget: std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>,
}
impl std::fmt::Debug for BytesCustody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reservation.fmt(f)
    }
}

impl Clone for BytesLease {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Drop for BytesLease {
    fn drop(&mut self) {
        // Every strong reference is private and follows this finalization path.
        // into_inner frees the Arc control block before returning the debit.
        drop(std::sync::Arc::into_inner(
            self.0.take().expect("live bytes lease"),
        ));
    }
}

impl OwnedBytes {
    pub fn unowned(bytes: bytes::Bytes) -> Self {
        Self { bytes, lease: None }
    }
    pub fn new() -> Self {
        Self::unowned(bytes::Bytes::new())
    }
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self::unowned(bytes::Bytes::from_static(bytes))
    }
    pub fn copy_from_slice(bytes: &[u8]) -> Self {
        Self::unowned(bytes::Bytes::copy_from_slice(bytes))
    }
    pub fn try_copy_from_slice(
        bytes: &[u8],
        budget: std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>,
    ) -> Result<Self, crate::net::resource_budget::BudgetError> {
        use crate::net::resource_budget::{BudgetError, ResourceReservation};
        use std::alloc::Layout;
        use std::sync::{atomic::AtomicUsize, Arc};
        if bytes.is_empty() {
            return Ok(Self::new());
        }
        // Pinned bytes 1.12.1 Shared is { ptr, capacity, AtomicUsize }.
        // Promotion can allocate this block on the first clone; precharge it
        // before constructing the Vec and force promotion before publication.
        let shared = Layout::new::<(*mut u8, usize, AtomicUsize)>().size();
        let control = Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<BytesCustody>())
            .map_err(|_| BudgetError::Overflow)?
            .0
            .pad_to_align()
            .size();
        let total = bytes
            .len()
            .checked_add(shared)
            .and_then(|n| n.checked_add(control))
            .ok_or(BudgetError::Overflow)?;
        let reservation = ResourceReservation::try_new(budget.clone(), total)?;
        let mut data = Vec::with_capacity(bytes.len());
        data.extend_from_slice(bytes);
        let data = bytes::Bytes::from(data);
        drop(data.clone());
        Ok(Self {
            bytes: data,
            lease: Some(BytesLease(Some(Arc::new(BytesCustody {
                reservation,
                budget,
            })))),
        })
    }
    pub fn budget(
        &self,
    ) -> Option<std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>> {
        self.lease
            .as_ref()
            .map(|lease| lease.0.as_ref().expect("live bytes lease").budget.clone())
    }
    pub fn slice(&self, range: impl std::ops::RangeBounds<usize>) -> Self {
        Self {
            bytes: self.bytes.slice(range),
            lease: self.lease.clone(),
        }
    }
    pub fn slice_ref(&self, subset: &[u8]) -> Self {
        Self {
            bytes: self.bytes.slice_ref(subset),
            lease: self.lease.clone(),
        }
    }
    pub fn split_to(&mut self, at: usize) -> Self {
        Self {
            bytes: self.bytes.split_to(at),
            lease: self.lease.clone(),
        }
    }
    pub fn get_bytes_nul(&mut self) -> Result<Self, Error> {
        let nul = self
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| Error::Io(io::ErrorKind::InvalidData.into()))?;
        let value = self.slice(..nul);
        bytes::Buf::advance(self, nul + 1);
        Ok(value)
    }
    pub fn into_unowned(self) -> Result<bytes::Bytes, Error> {
        if self.lease.is_some() {
            return Err(Error::Io(io::ErrorKind::Unsupported.into()));
        }
        Ok(self.bytes)
    }
    pub fn truncate(&mut self, len: usize) {
        self.bytes.truncate(len);
    }
}

impl Default for OwnedBytes {
    fn default() -> Self {
        Self::new()
    }
}
impl std::ops::Deref for OwnedBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}
impl AsRef<[u8]> for OwnedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
impl bytes::Buf for OwnedBytes {
    fn copy_to_bytes(&mut self, len: usize) -> bytes::Bytes {
        assert!(
            self.lease.is_none(),
            "owned backing cannot escape as naked Bytes"
        );
        bytes::Buf::copy_to_bytes(&mut self.bytes, len)
    }
    fn remaining(&self) -> usize {
        self.bytes.len()
    }
    fn chunk(&self) -> &[u8] {
        &self.bytes
    }
    fn advance(&mut self, cnt: usize) {
        bytes::Buf::advance(&mut self.bytes, cnt);
    }
}
impl From<&'static [u8]> for OwnedBytes {
    fn from(bytes: &'static [u8]) -> Self {
        Self::from_static(bytes)
    }
}
impl From<Vec<u8>> for OwnedBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self::unowned(bytes.into())
    }
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::net::resource_budget::BudgetError;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll};

    struct Ledger {
        held: AtomicUsize,
        peak: AtomicUsize,
        limit: usize,
    }
    impl Ledger {
        fn new(limit: usize) -> Arc<Self> {
            Arc::new(Self {
                held: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                limit,
            })
        }
        fn held(&self) -> usize {
            self.held.load(Ordering::SeqCst)
        }
    }
    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            let before = self
                .held
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |held| {
                    held.checked_add(bytes).filter(|next| *next <= self.limit)
                })
                .map_err(|_| BudgetError::Unavailable)?;
            self.peak.fetch_max(before + bytes, Ordering::SeqCst);
            Ok(())
        }
        fn release(&self, bytes: usize) {
            assert!(self.held.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
        }
    }
    #[derive(Default)]
    struct TestSocket {
        input: &'static [u8],
        written: usize,
        stall: bool,
    }
    impl Socket for TestSocket {
        fn try_read(&mut self, buf: &mut dyn crate::io::ReadBuf) -> io::Result<usize> {
            if self.stall {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let size = self.input.len().min(buf.remaining_mut()).min(2);
            buf.put_slice(&self.input[..size]);
            self.input = &self.input[size..];
            Ok(size)
        }
        fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let size = buf.len().min(2);
            self.written += size;
            Ok(size)
        }
        fn poll_read_ready(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
        fn poll_write_ready(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
        fn poll_shutdown(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
    #[test]
    fn initial_and_growth_denial_preserve_backing() {
        let denied = Ledger::new(2 * DEFAULT_BUF_SIZE - 1);
        assert!(BufferedSocket::new_owned(TestSocket::default(), denied.clone()).is_err());
        assert_eq!(denied.held(), 0);
        let budget = Ledger::new(2 * DEFAULT_BUF_SIZE + DEFAULT_BUF_SIZE);
        let mut socket = BufferedSocket::new_owned(TestSocket::default(), budget.clone()).unwrap();
        socket
            .write_precharged(3, |buf| {
                buf.extend_from_slice(b"old");
                Ok(())
            })
            .unwrap();
        assert!(socket
            .write_precharged(DEFAULT_BUF_SIZE, |_| panic!("encoder ran after denial"))
            .is_err());
        assert_eq!(socket.write_buffer().get(), b"old");
        assert_eq!(budget.held(), 2 * DEFAULT_BUF_SIZE);
        drop(socket);
        assert_eq!(budget.held(), 0);
    }
    #[test]
    fn shared_descendants_hold_one_debit_until_final_drop() {
        let budget = Ledger::new(usize::MAX);
        let mut bytes = OwnedBytes::try_copy_from_slice(b"abcdef", budget.clone()).unwrap();
        let charge = budget.held();
        let split = bytes.split_to(2);
        let slice = bytes.slice(1..3);
        let copy = slice.clone();
        assert_eq!(budget.held(), charge);
        drop((bytes, split, slice));
        assert_eq!(budget.held(), charge);
        assert_eq!(&*copy, b"de");
        drop(copy);
        assert_eq!(budget.held(), 0);
        let denied = Ledger::new(charge - 1);
        assert!(OwnedBytes::try_copy_from_slice(b"abcdef", denied.clone()).is_err());
        assert_eq!(denied.held(), 0);
    }
    #[test]
    fn funded_growth_covers_overlap_and_shrink_releases_old() {
        let budget = Ledger::new(usize::MAX);
        let mut socket = BufferedSocket::new_owned(TestSocket::default(), budget.clone()).unwrap();
        socket
            .write_precharged(DEFAULT_BUF_SIZE + 1, |buf| {
                buf.resize(DEFAULT_BUF_SIZE + 1, 1);
                Ok(())
            })
            .unwrap();
        assert_eq!(budget.peak.load(Ordering::SeqCst), 3 * DEFAULT_BUF_SIZE + 1);
        socket.write_buf.consume(DEFAULT_BUF_SIZE + 1);
        socket.shrink_buffers();
        assert_eq!(budget.held(), 2 * DEFAULT_BUF_SIZE);
        drop(socket);
        assert_eq!(budget.held(), 0);
    }
    #[test]
    fn partial_read_eof_and_cancellation_keep_only_committed_bytes() {
        use futures_util::FutureExt;
        let budget = Ledger::new(usize::MAX);
        let mut socket = BufferedSocket::new_owned(
            TestSocket {
                input: b"abc",
                ..Default::default()
            },
            budget.clone(),
        )
        .unwrap();
        assert!(socket.peek_owned(5).now_or_never().unwrap().is_err());
        assert_eq!(socket.owned_read.as_ref().unwrap().buf, b"abc");
        socket.socket.stall = true;
        assert!(socket.peek_owned(5).now_or_never().is_none());
        assert_eq!(socket.owned_read.as_ref().unwrap().buf, b"abc");
        socket.socket.stall = false;
        let data = socket
            .read_owned_buffered(3)
            .now_or_never()
            .unwrap()
            .unwrap();
        drop(socket);
        assert!(budget.held() > 0);
        assert_eq!(&*data, b"abc");
        drop(data);
        assert_eq!(budget.held(), 0);
    }
    #[test]
    fn ordinary_and_owned_partial_writes_complete() {
        use futures_util::FutureExt;
        let budget = Ledger::new(usize::MAX);
        let mut owned = BufferedSocket::new_owned(TestSocket::default(), budget.clone()).unwrap();
        owned
            .write_precharged(5, |buf| {
                buf.extend_from_slice(b"hello");
                Ok(())
            })
            .unwrap();
        owned.shutdown().now_or_never().unwrap().unwrap();
        assert_eq!(owned.socket.written, 5);
        drop(owned);
        assert_eq!(budget.held(), 0);
        let mut ordinary = BufferedSocket::new(TestSocket::default());
        ordinary.write_buffer_mut().put_slice(b"hello");
        ordinary.shutdown().now_or_never().unwrap().unwrap();
        assert_eq!(ordinary.socket.written, 5);
    }
}
