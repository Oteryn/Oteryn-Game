use crate::error::Error;
use crate::net::Socket;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetError {
    Unavailable,
    Overflow,
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "resource budget unavailable",
            Self::Overflow => "resource accounting overflow",
        })
    }
}

impl std::error::Error for BudgetError {}

pub trait ResourceBudget: Send + Sync {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError>;
    fn release(&self, bytes: usize);
}

pub struct ResourceReservation {
    budget: Arc<dyn ResourceBudget>,
    bytes: usize,
}

impl ResourceReservation {
    pub fn try_new(
        budget: Arc<dyn ResourceBudget>,
        bytes: usize,
    ) -> Result<Self, BudgetError> {
        budget.try_reserve(bytes)?;
        Ok(Self { budget, bytes })
    }

    pub fn try_new_shared(
        budget: Arc<dyn ResourceBudget>,
        backing_bytes: usize,
    ) -> Result<Arc<Self>, BudgetError> {
        use std::alloc::Layout;
        let counters = Layout::array::<usize>(2).map_err(|_| BudgetError::Overflow)?;
        let (layout, _) = counters
            .extend(Layout::new::<Self>())
            .map_err(|_| BudgetError::Overflow)?;
        let total = backing_bytes
            .checked_add(layout.pad_to_align().size())
            .ok_or(BudgetError::Overflow)?;
        budget.try_reserve(total)?;
        Ok(Arc::new(Self { budget, bytes: total }))
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn budget(&self) -> Arc<dyn ResourceBudget> {
        self.budget.clone()
    }
}

impl Drop for ResourceReservation {
    fn drop(&mut self) {
        self.budget.release(self.bytes);
    }
}

impl std::fmt::Debug for ResourceReservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceReservation")
            .field("bytes", &self.bytes)
            .finish()
    }
}
use bytes::{Bytes, BytesMut};
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
    resource_budget: Option<Arc<dyn ResourceBudget>>,
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

struct OwnedRead {
    buf: Vec<u8>,
    _allocation: ResourceReservation,
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
            resource_budget: None,
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
            resource_budget: Some(budget),
            owned_read: Some(OwnedRead {
                buf: Vec::with_capacity(DEFAULT_BUF_SIZE),
                _allocation: read_allocation,
            }),
        })
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
        loop {
            let read_len = match try_read(&mut self.read_buf.read)? {
                ControlFlow::Continue(read_len) => read_len,
                ControlFlow::Break(ret) => return Ok(ret),
            };

            self.read_buf.read(read_len, &mut self.socket).await?;
        }
    }

    pub fn resource_budget(&self) -> Option<&Arc<dyn ResourceBudget>> {
        self.resource_budget.as_ref()
    }

    pub async fn read_owned_header(&mut self) -> Result<[u8; 5], Error> {
        let read = self.owned_read.as_mut().ok_or_else(denied)?;
        read.buf.clear();
        read.buf.resize(5, 0);
        let mut offset = 0usize;
        while offset < 5 {
            let n = self.socket.read(&mut &mut read.buf[offset..]).await?;
            if n == 0 {
                return Err(Error::Io(io::ErrorKind::UnexpectedEof.into()));
            }
            offset += n;
        }
        let mut header = [0u8; 5];
        header.copy_from_slice(&read.buf[..5]);
        read.buf.clear();
        Ok(header)
    }

    pub async fn read_owned_body(
        &mut self,
        len: usize,
    ) -> Result<(Bytes, Arc<ResourceReservation>), Error> {
        let budget = self.resource_budget.clone().ok_or_else(denied)?;
        let allocation =
            ResourceReservation::try_new_shared(budget, len).map_err(|_| denied())?;
        let mut body = Vec::with_capacity(len);
        body.resize(len, 0);
        let mut offset = 0usize;
        while offset < len {
            let n = self.socket.read(&mut &mut body[offset..]).await?;
            if n == 0 {
                return Err(Error::Io(io::ErrorKind::UnexpectedEof.into()));
            }
            offset += n;
        }
        Ok((Bytes::from(body), allocation))
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

    pub fn write_precharged<'en, T>(&mut self, value: T, encoded_size: usize) -> Result<(), Error>
    where
        T: ProtocolEncode<'en, ()>,
    {
        let target = self
            .write_buf
            .bytes_written
            .checked_add(encoded_size)
            .ok_or_else(denied)?;
        self.write_buf.reserve_owned(target)?;
        value.encode_with(self.write_buf.buf_mut(), ())?;
        self.write_buf.bytes_written = self.write_buf.buf.len();
        self.write_buf.sanity_check();
        Ok(())
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
    }

    pub fn into_inner(self) -> S {
        self.socket
    }

    pub fn boxed(self) -> BufferedSocket<Box<dyn Socket>> {
        BufferedSocket {
            socket: Box::new(self.socket),
            write_buf: self.write_buf,
            read_buf: self.read_buf,
            resource_budget: self.resource_budget,
            owned_read: self.owned_read,
        }
    }
}

fn denied() -> Error {
    Error::Io(io::ErrorKind::OutOfMemory.into())
}

impl WriteBuffer {
    fn reserve_owned(&mut self, target: usize) -> Result<(), Error> {
        let Some(budget) = &self.budget else {
            self.buf
                .try_reserve(target.saturating_sub(self.buf.len()))
                .map_err(|_| denied())?;
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
            self.buf
                .copy_within(self.bytes_flushed..self.bytes_written, 0);
            self.bytes_written -= self.bytes_flushed;
            self.bytes_flushed = 0
        }

        let target = cmp::max(self.bytes_written, DEFAULT_BUF_SIZE);
        if self.budget.is_some() && self.buf.capacity() > target {
            if let Some(budget) = self.budget.clone() {
                if let Ok(allocation) = ResourceReservation::try_new(budget, target) {
                    let mut replacement = Vec::with_capacity(target);
                    replacement.extend_from_slice(&self.buf[..self.bytes_written]);
                    drop(std::mem::replace(&mut self.buf, replacement));
                    self.allocation = Some(allocation);
                }
            }
        } else {
            self.buf.truncate(target);
            self.buf.shrink_to_fit();
        }
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


#[cfg(test)]
mod wp3_resource_budget_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct Ledger {
        limit: usize,
        held: AtomicUsize,
    }

    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            let mut current = self.held.load(Ordering::Acquire);
            loop {
                let next = current.checked_add(bytes).ok_or(BudgetError::Overflow)?;
                if next > self.limit {
                    return Err(BudgetError::Unavailable);
                }
                match self.held.compare_exchange(
                    current,
                    next,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return Ok(()),
                    Err(observed) => current = observed,
                }
            }
        }

        fn release(&self, bytes: usize) {
            self.held.fetch_sub(bytes, Ordering::AcqRel);
        }
    }

    #[test]
    fn wp3_shared_reservation_releases_only_after_final_owner() {
        let ledger = Arc::new(Ledger {
            limit: usize::MAX,
            held: AtomicUsize::new(0),
        });
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        let lease = ResourceReservation::try_new_shared(owner, 131_202).unwrap();
        let held = ledger.held.load(Ordering::Acquire);
        assert!(held >= 131_202);
        let clone = lease.clone();
        drop(lease);
        assert_eq!(ledger.held.load(Ordering::Acquire), held);
        drop(clone);
        assert_eq!(ledger.held.load(Ordering::Acquire), 0);
    }

    #[test]
    fn wp3_denied_reservation_leaves_balance_unchanged() {
        let ledger = Arc::new(Ledger {
            limit: 8,
            held: AtomicUsize::new(0),
        });
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        assert!(matches!(
            ResourceReservation::try_new(owner, 9),
            Err(BudgetError::Unavailable)
        ));
        assert_eq!(ledger.held.load(Ordering::Acquire), 0);
    }
}
