use alloc::vec::Vec;
use core::fmt::Debug;
#[cfg(feature = "std")]
use core::alloc::Layout;
use core::marker::PhantomData;
#[cfg(feature = "std")]
use core::ops::{Deref, DerefMut};
#[cfg(feature = "std")]
use core::mem::size_of;
#[cfg(feature = "std")]
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::error::InvalidMessage;
#[cfg(feature = "std")]
use crate::sync::Arc;
#[cfg(feature = "std")]
use crate::DeframerBufferOwner;

/// Connection-scoped custody for allocations made while decoding peer input.
///
/// Individual decoded values can move through several private handshake states.
/// Keeping the debit here makes those moves allocation-free and ensures fatal
/// parse paths cannot accidentally release backing that a successor retained.
#[cfg(feature = "std")]
#[derive(Debug)]
pub(crate) struct DecodedOwner {
    owner: Arc<dyn DeframerBufferOwner>,
    charged: AtomicUsize,
    custodied: AtomicUsize,
}

#[cfg(feature = "std")]
#[derive(Clone, Copy)]
pub(crate) struct DecodedCheckpoint { charged: usize, custodied: usize }

#[cfg(feature = "std")]
#[derive(Debug)]
pub(crate) struct DecodedOwnerArcCharge {
    owner: Arc<dyn DeframerBufferOwner>,
    bytes: usize,
}

#[cfg(feature = "std")]
impl Drop for DecodedOwnerArcCharge {
    fn drop(&mut self) {
        self.owner.release(self.bytes);
    }
}

#[cfg(feature = "std")]
impl DecodedOwner {
    pub(crate) fn arc_layout() -> Result<usize, InvalidMessage> {
        // Rust 1.94 alloc::sync::Arc requests the padded ArcInner<T> layout.
        Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<Self>())
            .map(|(layout, _)| layout.pad_to_align().size())
            .map_err(|_| InvalidMessage::MessageTooLarge)
    }

    pub(crate) fn new(
        owner: Arc<dyn DeframerBufferOwner>,
    ) -> Result<(Arc<Self>, DecodedOwnerArcCharge), InvalidMessage> {
        let bytes = Self::arc_layout()?;
        owner.try_reserve(bytes).map_err(|_| InvalidMessage::MessageTooLarge)?;
        let charge = DecodedOwnerArcCharge { owner: owner.clone(), bytes };
        let decoded = Arc::new(Self {
            owner,
            charged: AtomicUsize::new(0),
            custodied: AtomicUsize::new(0),
        });
        Ok((decoded, charge))
    }

    pub(crate) fn reserve(&self, bytes: usize) -> Result<(), InvalidMessage> {
        if bytes == 0 { return Ok(()); }
        self.owner.try_reserve(bytes).map_err(|_| InvalidMessage::MessageTooLarge)?;
        self.charged.fetch_add(bytes, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn release(&self, bytes: usize) {
        if bytes == 0 { return; }
        // `charged` is observability, not an independent source of custody.
        // Refuse to turn an accidental second release into both an integer
        // underflow and a release of live backing in the underlying owner.
        let released = self.charged.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |charged| charged.checked_sub(bytes),
        );
        debug_assert!(released.is_ok(), "decoded custody released twice");
        if released.is_err() {
            return;
        }
        self.owner.release(bytes);
    }

    pub(crate) fn retain_backing(&self, bytes: usize) {
        self.custodied.fetch_add(bytes, Ordering::Relaxed);
    }

    pub(crate) fn release_backing(&self, bytes: usize) {
        self.custodied.fetch_sub(bytes, Ordering::Relaxed);
        self.release(bytes);
    }

    pub(crate) fn checkpoint(&self) -> DecodedCheckpoint {
        DecodedCheckpoint {
            charged: self.charged.load(Ordering::Relaxed),
            custodied: self.custodied.load(Ordering::Relaxed),
        }
    }

    /// Roll back allocations made after `checkpoint`.
    ///
    /// Callers must first destroy all backing covered by those allocations.
    pub(crate) fn rollback(&self, checkpoint: DecodedCheckpoint) {
        let charged = self.charged.load(Ordering::Relaxed);
        let custodied = self.custodied.load(Ordering::Relaxed);
        let local = charged.saturating_sub(checkpoint.charged);
        let transferred = custodied.saturating_sub(checkpoint.custodied);
        debug_assert!(local >= transferred);
        self.release(local.saturating_sub(transferred));
    }
}

#[cfg(feature = "std")]
impl DeframerBufferOwner for DecodedOwner {
    fn try_reserve(&self, bytes: usize) -> Result<(), crate::DeframerBufferError> {
        self.reserve(bytes).map_err(|_| crate::DeframerBufferError)
    }

    fn release(&self, bytes: usize) {
        Self::release(self, bytes);
    }

    fn try_reserve_provider_shared(
        &self,
        bytes: usize,
    ) -> Result<(), crate::DeframerBufferError> {
        self.owner.try_reserve_provider_shared(bytes)
    }
}

#[cfg(feature = "std")]
impl Drop for DecodedOwner {
    fn drop(&mut self) {
        self.owner.release(self.charged.load(Ordering::Relaxed));
    }
}

/// Non-allocating custody for the decoded backing contained by one message.
///
/// This value must be declared after the backing it covers: Rust drops fields
/// in declaration order, so the message payload is destroyed before this
/// token returns its debit.
#[cfg(feature = "std")]
#[derive(Debug)]
pub(crate) struct DecodedCustody {
    owner: Arc<DecodedOwner>,
    bytes: usize,
    tracked_backing: bool,
}

#[cfg(feature = "std")]
impl DecodedCustody {
    pub(crate) fn owner(&self) -> Arc<DecodedOwner> {
        self.owner.clone()
    }

    pub(crate) fn owner_ref(&self) -> &Arc<DecodedOwner> {
        &self.owner
    }
    pub(crate) fn exact(owner: Arc<DecodedOwner>, bytes: usize) -> Self {
        owner.custodied.fetch_add(bytes, Ordering::Relaxed);
        Self { owner, bytes, tracked_backing: true }
    }

    pub(crate) fn since(owner: Arc<DecodedOwner>, checkpoint: DecodedCheckpoint) -> Self {
        let charged = owner.checkpoint();
        let local = charged.charged.saturating_sub(checkpoint.charged);
        let backing = charged.custodied.saturating_sub(checkpoint.custodied);
        debug_assert!(local >= backing);
        Self { owner, bytes: local.saturating_sub(backing), tracked_backing: false }
    }

    pub(crate) fn copy_bytes(
        &self,
        source: &[u8],
    ) -> Result<(Vec<u8>, Self), InvalidMessage> {
        let capacity = source.len();
        self.owner.reserve(capacity)?;

        let mut destination = Vec::with_capacity(capacity);
        destination.extend_from_slice(source);
        if destination.capacity() != capacity {
            drop(destination);
            self.owner.release(capacity);
            return Err(InvalidMessage::MessageTooLarge);
        }

        Ok((
            destination,
            Self::exact(self.owner.clone(), capacity),
        ))
    }

}

/// A reservation for backing that has not yet been committed to its final
/// owner.  This guard must be created before the prospective allocation and
/// declared before the destination value: Rust then destroys the destination
/// before this guard returns the reservation during unwinding.
#[cfg(feature = "std")]
struct ProspectiveDecodedCustody {
    owner: Arc<DecodedOwner>,
    bytes: usize,
    armed: bool,
}

#[cfg(feature = "std")]
impl ProspectiveDecodedCustody {
    fn reserve(owner: Arc<DecodedOwner>, bytes: usize) -> Result<Self, InvalidMessage> {
        owner.reserve(bytes)?;
        Ok(Self { owner, bytes, armed: true })
    }

    fn commit(mut self) -> DecodedCustody {
        self.armed = false;
        DecodedCustody::exact(self.owner.clone(), self.bytes)
    }
}

#[cfg(feature = "std")]
impl Drop for ProspectiveDecodedCustody {
    fn drop(&mut self) {
        if self.armed {
            self.owner.release(self.bytes);
        }
    }
}

#[cfg(feature = "std")]
impl Drop for DecodedCustody {
    fn drop(&mut self) {
        if self.tracked_backing {
            self.owner.custodied.fetch_sub(self.bytes, Ordering::Relaxed);
        }
        self.owner.release(self.bytes);
    }
}

/// A decoded vector whose reservation follows the allocation it describes.
///
/// This is deliberately crate-private: wire-facing APIs continue to use ordinary
/// `Vec` values, while owner-aware private AST fields can opt into exact backing
/// custody without exposing a second public collection type.
#[cfg(feature = "std")]
#[derive(Debug)]
#[allow(dead_code)] // Introduced ahead of the private handshake-field migration.
pub(crate) struct DecodedVec<T> {
    // Backing must be declared before custody so it is destroyed first.
    values: Vec<T>,
    custody: Option<DecodedCustody>,
}

/// Owner-aware custody is a `std`-only capability, but decoded AST fields must
/// keep their upstream `no_std` representation.  This alias lets those fields
/// use `DecodedVec<T>` without adding bookkeeping or changing allocation and
/// clone behavior when `std` is disabled.
#[cfg(not(feature = "std"))]
#[allow(dead_code)] // Consumed once private decoded fields migrate to this name.
pub(crate) type DecodedVec<T> = Vec<T>;

#[cfg(feature = "std")]
impl<T> DecodedVec<T> {
    pub(crate) fn try_copy_filtered<F>(
        owner: Option<Arc<DecodedOwner>>,
        source: &[T],
        keep: F,
    ) -> Result<Self, InvalidMessage>
    where
        T: Copy,
        F: Fn(&T) -> bool,
    {
        let count = source.iter().filter(|item| keep(item)).count();
        if count == 0 {
            return Ok(Self { values: Vec::new(), custody: None });
        }
        let Some(owner) = owner else {
            return Ok(Self {
                values: source.iter().copied().filter(keep).collect(),
                custody: None,
            });
        };
        let bytes = count
            .checked_mul(size_of::<T>())
            .ok_or(InvalidMessage::MessageTooLarge)?;
        let prospective = ProspectiveDecodedCustody::reserve(owner, bytes)?;
        let mut values = Vec::with_capacity(count);
        values.extend(source.iter().copied().filter(keep));
        if values.capacity() != count {
            drop(values);
            return Err(InvalidMessage::MessageTooLarge);
        }
        Ok(Self { values, custody: Some(prospective.commit()) })
    }

    /// Copy an owner-aware vector whose elements are known not to allocate.
    ///
    /// Allocating element copies require a field-specific fallible operation
    /// that reserves their nested backing; this outer-vector helper is
    /// deliberately unavailable for arbitrary `T: Clone`.
    #[allow(dead_code)] // Used by the next private handshake-field migration.
    pub(crate) fn try_copy_with_resource_owner(&self) -> Result<Self, InvalidMessage>
    where
        T: Copy,
    {
        let Some(custody) = &self.custody else {
            return Ok(Self { values: self.values.clone(), custody: None });
        };
        let capacity = self.values.capacity();
        let bytes = capacity.checked_mul(size_of::<T>())
            .ok_or(InvalidMessage::MessageTooLarge)?;
        let prospective =
            ProspectiveDecodedCustody::reserve(custody.owner.clone(), bytes)?;
        let mut values = Vec::with_capacity(capacity);
        values.extend_from_slice(&self.values);
        if values.capacity() != capacity {
            drop(values);
            return Err(InvalidMessage::MessageTooLarge);
        }
        Ok(Self {
            values,
            custody: Some(prospective.commit()),
        })
    }
}

#[cfg(feature = "std")]
impl<T> Deref for DecodedVec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target { &self.values }
}

#[cfg(feature = "std")]
impl<T> DerefMut for DecodedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.values }
}

#[cfg(feature = "std")]
impl<T: Clone> Clone for DecodedVec<T> {
    fn clone(&self) -> Self {
        assert!(self.custody.is_none(), "charged decoded backing requires fallible clone");
        Self { values: self.values.clone(), custody: None }
    }
}

#[cfg(feature = "std")]
impl<T: PartialEq> PartialEq for DecodedVec<T> {
    fn eq(&self, other: &Self) -> bool { self.values == other.values }
}

#[cfg(feature = "std")]
impl<T: Eq> Eq for DecodedVec<T> {}

#[cfg(feature = "std")]
struct ListChargeGuard {
    owner: Option<Arc<DecodedOwner>>,
    capacity: usize,
    element_size: usize,
    armed: bool,
}

#[cfg(feature = "std")]
impl ListChargeGuard {
    fn new<T>(reader: &Reader<'_>) -> Self {
        Self {
            owner: reader.decoded_owner.clone(),
            capacity: 0,
            element_size: size_of::<T>(),
            armed: true,
        }
    }

    fn replace(&mut self, capacity: usize) {
        self.capacity = capacity;
    }

    fn commit(mut self) {
        self.armed = false;
    }
}

#[cfg(feature = "std")]
impl Drop for ListChargeGuard {
    fn drop(&mut self) {
        if self.armed {
            if let Some(owner) = &self.owner {
                owner.release(self.capacity * self.element_size);
            }
        }
    }
}

/// Wrapper over a slice of bytes that allows reading chunks from
/// with the current position state held using a cursor.
///
/// A new reader for a sub section of the buffer can be created
/// using the `sub` function or a section of a certain length can
/// be obtained using the `take` function
pub struct Reader<'a> {
    /// The underlying buffer storing the readers content
    buffer: &'a [u8],
    /// Stores the current reading position for the buffer
    cursor: usize,
    #[cfg(feature = "std")]
    decoded_owner: Option<Arc<DecodedOwner>>,
}

impl<'a> Reader<'a> {
    /// Creates a new Reader of the provided `bytes` slice with
    /// the initial cursor position of zero.
    pub fn init(bytes: &'a [u8]) -> Self {
        Reader {
            buffer: bytes,
            cursor: 0,
            #[cfg(feature = "std")]
            decoded_owner: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn init_with_owner(bytes: &'a [u8], owner: Arc<DecodedOwner>) -> Self {
        Self { buffer: bytes, cursor: 0, decoded_owner: Some(owner) }
    }

    /// Attempts to create a new Reader on a sub section of this
    /// readers bytes by taking a slice of the provided `length`
    /// will return None if there is not enough bytes
    pub fn sub(&mut self, length: usize) -> Result<Self, InvalidMessage> {
        match self.take(length) {
            Some(bytes) => Ok(Self {
                buffer: bytes,
                cursor: 0,
                #[cfg(feature = "std")]
                decoded_owner: self.decoded_owner.clone(),
            }),
            None => Err(InvalidMessage::MessageTooShort),
        }
    }

    /// Borrows a slice of all the remaining bytes
    /// that appear after the cursor position.
    ///
    /// Moves the cursor to the end of the buffer length.
    pub fn rest(&mut self) -> &'a [u8] {
        let rest = &self.buffer[self.cursor..];
        self.cursor = self.buffer.len();
        rest
    }

    /// Attempts to borrow a slice of bytes from the current
    /// cursor position of `length` if there is not enough
    /// bytes remaining after the cursor to take the length
    /// then None is returned instead.
    pub fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        if self.left() < length {
            return None;
        }
        let current = self.cursor;
        self.cursor += length;
        Some(&self.buffer[current..current + length])
    }

    /// Used to check whether the reader has any content left
    /// after the cursor (cursor has not reached end of buffer)
    pub fn any_left(&self) -> bool {
        self.cursor < self.buffer.len()
    }

    pub fn expect_empty(&self, name: &'static str) -> Result<(), InvalidMessage> {
        match self.any_left() {
            true => Err(InvalidMessage::TrailingData(name)),
            false => Ok(()),
        }
    }

    /// Returns the cursor position which is also the number
    /// of bytes that have been read from the buffer.
    pub fn used(&self) -> usize {
        self.cursor
    }

    /// Returns the number of bytes that are still able to be
    /// read (The number of remaining takes)
    pub fn left(&self) -> usize {
        self.buffer.len() - self.cursor
    }

    #[cfg(feature = "std")]
    fn reserve_vec_capacity<T>(&self, capacity: usize) -> Result<(), InvalidMessage> {
        let Some(owner) = &self.decoded_owner else { return Ok(()); };
        let bytes = capacity.checked_mul(size_of::<T>())
            .ok_or(InvalidMessage::MessageTooLarge)?;
        owner.reserve(bytes)
    }


    #[cfg(feature = "std")]
    fn release_vec_capacity<T>(&self, capacity: usize) {
        let Some(owner) = &self.decoded_owner else { return; };
        owner.release(capacity * size_of::<T>());
    }


    /// Copy `bytes` into exactly-sized decoded backing, reserving before allocation.
    #[cfg(feature = "std")]
    pub(crate) fn copy_decoded(
        &self,
        bytes: &[u8],
    ) -> Result<(Vec<u8>, Option<DecodedCustody>), InvalidMessage> {
        let Some(owner) = &self.decoded_owner else { return Ok((bytes.to_vec(), None)); };
        // An empty `Vec` has no allocator backing.  In particular, do not
        // attach a zero-byte custody marker: payload `Clone` uses the
        // presence of custody to distinguish peer-owned backing from an
        // ordinary owner-free value, and valid empty TLS payloads may flow
        // through ordinary clone sites without allocating.
        if bytes.is_empty() {
            return Ok((Vec::new(), None));
        }
        owner.reserve(bytes.len())?;
        let mut copied = Vec::with_capacity(bytes.len());
        copied.extend_from_slice(bytes);
        if copied.capacity() != bytes.len() {
            let reserved = bytes.len();
            drop(copied);
            owner.release(reserved);
            return Err(InvalidMessage::MessageTooLarge);
        }
        Ok((copied, Some(DecodedCustody::exact(owner.clone(), bytes.len()))))
    }
}

/// Trait for implementing encoding and decoding functionality
/// on something.
pub trait Codec<'a>: Debug + Sized {
    /// Function for encoding itself by appending itself to
    /// the provided vec of bytes.
    fn encode(&self, bytes: &mut Vec<u8>);

    /// Function for decoding itself from the provided reader
    /// will return Some if the decoding was successful or
    /// None if it was not.
    fn read(_: &mut Reader<'a>) -> Result<Self, InvalidMessage>;

    /// Convenience function for encoding the implementation
    /// into a vec and returning it
    fn get_encoding(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.encode(&mut bytes);
        bytes
    }

    /// Function for wrapping a call to the read function in
    /// a Reader for the slice of bytes provided
    ///
    /// Returns `Err(InvalidMessage::ExcessData(_))` if
    /// `Self::read` does not read the entirety of `bytes`.
    fn read_bytes(bytes: &'a [u8]) -> Result<Self, InvalidMessage> {
        let mut reader = Reader::init(bytes);
        Self::read(&mut reader).and_then(|r| {
            reader.expect_empty("read_bytes")?;
            Ok(r)
        })
    }
}

impl Codec<'_> for u8 {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.push(*self);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        match r.take(1) {
            Some(&[byte]) => Ok(byte),
            _ => Err(InvalidMessage::MissingData("u8")),
        }
    }
}

pub(crate) fn put_u16(v: u16, out: &mut [u8]) {
    let out: &mut [u8; 2] = (&mut out[..2]).try_into().unwrap();
    *out = u16::to_be_bytes(v);
}

impl Codec<'_> for u16 {
    fn encode(&self, bytes: &mut Vec<u8>) {
        let mut b16 = [0u8; 2];
        put_u16(*self, &mut b16);
        bytes.extend_from_slice(&b16);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        match r.take(2) {
            Some(&[b1, b2]) => Ok(Self::from_be_bytes([b1, b2])),
            _ => Err(InvalidMessage::MissingData("u16")),
        }
    }
}

// Make a distinct type for u24, even though it's a u32 underneath
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub struct u24(pub u32);

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<u24> for usize {
    #[inline]
    fn from(v: u24) -> Self {
        v.0 as Self
    }
}

impl Codec<'_> for u24 {
    fn encode(&self, bytes: &mut Vec<u8>) {
        let be_bytes = u32::to_be_bytes(self.0);
        bytes.extend_from_slice(&be_bytes[1..]);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        match r.take(3) {
            Some(&[a, b, c]) => Ok(Self(u32::from_be_bytes([0, a, b, c]))),
            _ => Err(InvalidMessage::MissingData("u24")),
        }
    }
}

impl Codec<'_> for u32 {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.extend(Self::to_be_bytes(*self));
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        match r.take(4) {
            Some(&[a, b, c, d]) => Ok(Self::from_be_bytes([a, b, c, d])),
            _ => Err(InvalidMessage::MissingData("u32")),
        }
    }
}

pub(crate) fn put_u64(v: u64, bytes: &mut [u8]) {
    let bytes: &mut [u8; 8] = (&mut bytes[..8]).try_into().unwrap();
    *bytes = u64::to_be_bytes(v);
}

impl Codec<'_> for u64 {
    fn encode(&self, bytes: &mut Vec<u8>) {
        let mut b64 = [0u8; 8];
        put_u64(*self, &mut b64);
        bytes.extend_from_slice(&b64);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        match r.take(8) {
            Some(&[a, b, c, d, e, f, g, h]) => Ok(Self::from_be_bytes([a, b, c, d, e, f, g, h])),
            _ => Err(InvalidMessage::MissingData("u64")),
        }
    }
}

/// Implement `Codec` for lists of elements that implement `TlsListElement`.
///
/// `TlsListElement` provides the size of the length prefix for the list.
impl<'a, T: Codec<'a> + TlsListElement + Debug> Codec<'a> for Vec<T> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        let nest = LengthPrefixedBuffer::new(T::SIZE_LEN, bytes);

        for i in self {
            i.encode(nest.buf);
        }
    }

    fn read(r: &mut Reader<'a>) -> Result<Self, InvalidMessage> {
        #[cfg(feature = "std")]
        let mut charge = ListChargeGuard::new::<T>(r);
        let mut ret = Self::new();
        for item in TlsListIter::<T>::new(r)? {
            let item = item?;
            if ret.len() == ret.capacity() {
                #[cfg(feature = "std")]
                let old_capacity = ret.capacity();
                #[cfg(feature = "std")]
                let prospective = {
                    // Match the pinned RawVec amortized-growth policy rather than
                    // forcing an input-controlled allocation for every element.
                    let minimum = if size_of::<T>() == 1 { 8 } else if size_of::<T>() <= 1024 { 4 } else { 1 };
                    let prospective = old_capacity.saturating_mul(2)
                        .max(minimum)
                        .max(ret.len().checked_add(1).ok_or(InvalidMessage::MessageTooLarge)?);
                    r.reserve_vec_capacity::<T>(prospective)?;
                    prospective
                };
                #[cfg(feature = "std")]
                ret.reserve_exact(prospective - ret.len());
                #[cfg(not(feature = "std"))]
                ret.reserve(1);
                #[cfg(feature = "std")]
                if r.decoded_owner.is_some() && ret.capacity() != prospective {
                    // The prospective backing must die before either its debit or
                    // the replaced backing's debit is released.
                    drop(ret);
                    r.release_vec_capacity::<T>(prospective);
                    r.release_vec_capacity::<T>(old_capacity);
                    return Err(InvalidMessage::MessageTooLarge);
                }
                #[cfg(feature = "std")]
                {
                    r.release_vec_capacity::<T>(old_capacity);
                    charge.replace(ret.capacity());
                }
            }
            ret.push(item);
        }

        #[cfg(feature = "std")]
        charge.commit();
        Ok(ret)
    }
}

#[cfg(feature = "std")]
impl<'a, T: Codec<'a> + TlsListElement + Debug> Codec<'a> for DecodedVec<T> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        let nest = LengthPrefixedBuffer::new(T::SIZE_LEN, bytes);
        for item in &self.values {
            item.encode(nest.buf);
        }
    }

    fn read(r: &mut Reader<'a>) -> Result<Self, InvalidMessage> {
        let mut decoded = Self { values: Vec::new(), custody: None };
        for item in TlsListIter::<T>::new(r)? {
            let item = item?;
            if decoded.values.len() == decoded.values.capacity() {
                let old_capacity = decoded.values.capacity();
                let minimum = if size_of::<T>() == 1 { 8 } else if size_of::<T>() <= 1024 { 4 } else { 1 };
                let prospective = old_capacity
                    .saturating_mul(2)
                    .max(minimum)
                    .max(decoded.values.len().checked_add(1).ok_or(InvalidMessage::MessageTooLarge)?);

                if let Some(owner) = &r.decoded_owner {
                    let bytes = prospective.checked_mul(size_of::<T>())
                        .ok_or(InvalidMessage::MessageTooLarge)?;
                    owner.reserve(bytes)?;
                    let mut replacement = Vec::with_capacity(prospective);
                    if replacement.capacity() != prospective {
                        drop(replacement);
                        owner.release(bytes);
                        return Err(InvalidMessage::MessageTooLarge);
                    }
                    replacement.append(&mut decoded.values);
                    let old = core::mem::replace(&mut decoded.values, replacement);
                    let old_custody = decoded.custody.take();
                    drop(old);
                    drop(old_custody);
                    decoded.custody = Some(DecodedCustody::exact(owner.clone(), bytes));
                } else {
                    decoded.values.reserve(1);
                }
            }
            decoded.values.push(item);
        }
        Ok(decoded)
    }
}

/// An iterator over a vector of `TlsListElements`.
///
/// All uses _MUST_ exhaust the iterator, as errors may be delayed
/// until the last element.
pub(crate) struct TlsListIter<'a, T: Codec<'a> + TlsListElement + Debug> {
    sub: Reader<'a>,
    _t: PhantomData<T>,
}

impl<'a, T: Codec<'a> + TlsListElement + Debug> TlsListIter<'a, T> {
    pub(crate) fn new(r: &mut Reader<'a>) -> Result<Self, InvalidMessage> {
        let len = T::SIZE_LEN.read(r)?;
        let sub = r.sub(len)?;
        Ok(Self {
            sub,
            _t: PhantomData,
        })
    }
}

impl<'a, T: Codec<'a> + TlsListElement + Debug> Iterator for TlsListIter<'a, T> {
    type Item = Result<T, InvalidMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.sub.any_left() {
            true => Some(T::read(&mut self.sub)),
            false => None,
        }
    }
}

impl Codec<'_> for () {
    fn encode(&self, _: &mut Vec<u8>) {}

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        r.expect_empty("Empty")
    }
}

/// A trait for types that can be encoded and decoded in a list.
///
/// This trait is used to implement `Codec` for `Vec<T>`. Lists in the TLS wire format are
/// prefixed with a length, the size of which depends on the type of the list elements.
/// As such, the `Codec` implementation for `Vec<T>` requires an implementation of this trait
/// for its element type `T`.
pub(crate) trait TlsListElement {
    const SIZE_LEN: ListLength;
}

/// The length of the length prefix for a list.
///
/// The types that appear in lists are limited to three kinds of length prefixes:
/// 1, 2, and 3 bytes. For the latter kind, we require a `TlsListElement` implementer
/// to specify a maximum length and error if the actual length is larger.
pub(crate) enum ListLength {
    /// U8 but non-empty
    NonZeroU8 { empty_error: InvalidMessage },

    /// U16, perhaps empty
    U16,

    /// U16 but non-empty
    NonZeroU16 { empty_error: InvalidMessage },

    /// U24 with imposed upper bound
    U24 { max: usize, error: InvalidMessage },
}

impl ListLength {
    pub(crate) fn read(&self, r: &mut Reader<'_>) -> Result<usize, InvalidMessage> {
        Ok(match self {
            Self::NonZeroU8 { empty_error } => match usize::from(u8::read(r)?) {
                0 => return Err(*empty_error),
                len => len,
            },
            Self::U16 => usize::from(u16::read(r)?),
            Self::NonZeroU16 { empty_error } => match usize::from(u16::read(r)?) {
                0 => return Err(*empty_error),
                len => len,
            },
            Self::U24 { max, error } => match usize::from(u24::read(r)?) {
                len if len > *max => return Err(*error),
                len => len,
            },
        })
    }
}

/// Tracks encoding a length-delimited structure in a single pass.
pub(crate) struct LengthPrefixedBuffer<'a> {
    pub(crate) buf: &'a mut Vec<u8>,
    len_offset: usize,
    size_len: ListLength,
}

impl<'a> LengthPrefixedBuffer<'a> {
    /// Inserts a dummy length into `buf`, and remembers where it went.
    ///
    /// After this, the body of the length-delimited structure should be appended to `LengthPrefixedBuffer::buf`.
    /// The length header is corrected in `LengthPrefixedBuffer::drop`.
    pub(crate) fn new(size_len: ListLength, buf: &'a mut Vec<u8>) -> Self {
        let len_offset = buf.len();
        buf.extend(match size_len {
            ListLength::NonZeroU8 { .. } => &[0xff][..],
            ListLength::U16 | ListLength::NonZeroU16 { .. } => &[0xff, 0xff],
            ListLength::U24 { .. } => &[0xff, 0xff, 0xff],
        });

        Self {
            buf,
            len_offset,
            size_len,
        }
    }
}

impl Drop for LengthPrefixedBuffer<'_> {
    /// Goes back and corrects the length previously inserted at the start of the structure.
    fn drop(&mut self) {
        match self.size_len {
            ListLength::NonZeroU8 { .. } => {
                let len = self.buf.len() - self.len_offset - 1;
                debug_assert!(len <= 0xff);
                self.buf[self.len_offset] = len as u8;
            }
            ListLength::U16 | ListLength::NonZeroU16 { .. } => {
                let len = self.buf.len() - self.len_offset - 2;
                debug_assert!(len <= 0xffff);
                let out: &mut [u8; 2] = (&mut self.buf[self.len_offset..self.len_offset + 2])
                    .try_into()
                    .unwrap();
                *out = u16::to_be_bytes(len as u16);
            }
            ListLength::U24 { .. } => {
                let len = self.buf.len() - self.len_offset - 3;
                debug_assert!(len <= 0xff_ffff);
                let len_bytes = u32::to_be_bytes(len as u32);
                let out: &mut [u8; 3] = (&mut self.buf[self.len_offset..self.len_offset + 3])
                    .try_into()
                    .unwrap();
                out.copy_from_slice(&len_bytes[1..]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::prelude::v1::*;
    use std::vec;

    use super::*;
    use crate::msgs::base::{MaybeEmpty, NonEmpty, PayloadU8, PayloadU16};
    use crate::msgs::handshake::CertificateRequestPayloadTls13;

    #[derive(Debug)]
    struct TestOwner { limit: usize, used: AtomicUsize }

    impl DeframerBufferOwner for TestOwner {
        fn try_reserve(&self, bytes: usize) -> Result<(), crate::DeframerBufferError> {
            self.used.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |used| {
                used.checked_add(bytes).filter(|next| *next <= self.limit)
            }).map(|_| ()).map_err(|_| crate::DeframerBufferError)
        }
        fn release(&self, bytes: usize) { self.used.fetch_sub(bytes, Ordering::SeqCst); }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Tiny(u8);
    impl Codec<'_> for Tiny {
        fn encode(&self, out: &mut Vec<u8>) { out.push(self.0); }
        fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> { Ok(Self(u8::read(r)?)) }
    }
    impl TlsListElement for Tiny { const SIZE_LEN: ListLength = ListLength::U16; }

    #[test]
    fn decoded_custody_moves_without_recharging_and_releases_once() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 8, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let checkpoint = decoded.checkpoint();
        decoded.reserve(8).unwrap();
        let custody = DecodedCustody::since(decoded.clone(), checkpoint);
        let moved = custody;

        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 8);
        drop(moved);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_list_reserves_before_each_exact_capacity() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 8, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let values = Vec::<Tiny>::read(&mut reader).unwrap();
        assert_eq!(values.iter().map(|v| v.0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + values.capacity());
        drop(values);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 8, "custody follows decode scope");
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_payload_reserves_actual_capacity_before_copy() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 3, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[3, 1, 2, 3], decoded.clone());

        let payload = PayloadU8::<MaybeEmpty>::read(&mut reader).unwrap();
        assert_eq!(payload.0, [1, 2, 3]);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + payload.0.capacity());

        drop(payload);
        assert_eq!(
            owner.used.load(Ordering::SeqCst),
            arc_bytes,
            "payload backing releases before reader or connection owner drop"
        );
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_empty_certificate_request_context_has_no_custody_or_debit() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 128, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        // Empty context followed by a signature_algorithms extension carrying
        // rsa_pss_rsae_sha256.  A non-empty context is still rejected by the
        // TLS 1.3 client handler before its client-auth context copy.
        let encoded = [0, 0, 8, 0, 13, 0, 4, 0, 2, 8, 4];
        let mut reader = Reader::init_with_owner(&encoded, decoded.clone());

        let request = CertificateRequestPayloadTls13::read(&mut reader).unwrap();
        assert!(request.context.0.is_empty());
        let after_decode = owner.used.load(Ordering::SeqCst);

        let cloned = request.context.clone();
        assert!(cloned.0.is_empty());
        assert_eq!(owner.used.load(Ordering::SeqCst), after_decode);

        drop(cloned);
        drop(request);
        assert_eq!(owner.used.load(Ordering::SeqCst), after_decode);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_payload_denies_before_copy() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 2, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[3, 1, 2, 3], decoded.clone());

        assert_eq!(
            PayloadU8::<MaybeEmpty>::read(&mut reader),
            Err(InvalidMessage::MessageTooLarge)
        );
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_payload_owner_aware_clone_has_independent_custody() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 6, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let source = PayloadU16::<NonEmpty>::read(&mut reader).unwrap();

        let destination = source.try_clone_with_resource_owner().unwrap();
        assert_eq!(source.0, destination.0);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 6);

        drop(source);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 3);
        drop(destination);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_vec_copy_max_minus_one_denies_before_destination_allocation() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 15, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let source = DecodedVec::<Tiny>::read(&mut reader).unwrap();
        let source_backing = source.values.as_ptr();

        assert_eq!(
            source.try_copy_with_resource_owner(),
            Err(InvalidMessage::MessageTooLarge)
        );
        assert_eq!(source.values.as_ptr(), source_backing);
        assert_eq!(source.iter().map(|value| value.0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 8);

        drop(source);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_filtered_destination_has_early_drop_custody() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 3, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();

        let filtered = DecodedVec::try_copy_filtered(
            Some(decoded.clone()),
            &[1u8, 2, 3, 4],
            |value| value % 2 == 0,
        )
        .unwrap();
        assert_eq!(&*filtered, [2, 4]);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 2);
        drop(filtered);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);

        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_filtered_destination_max_minus_one_denies_before_allocation() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 1, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();

        assert_eq!(
            DecodedVec::try_copy_filtered(
                Some(decoded.clone()),
                &[1u8, 2, 3, 4],
                |value| value % 2 == 0,
            ),
            Err(InvalidMessage::MessageTooLarge)
        );
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);

        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_payload_owner_aware_clone_denies_before_destination_allocation() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 5, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let source = PayloadU16::<NonEmpty>::read(&mut reader).unwrap();

        assert_eq!(
            source.try_clone_with_resource_owner(),
            Err(InvalidMessage::MessageTooLarge)
        );
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 3);

        drop(source);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn owner_free_payload_clone_preserves_upstream_behavior() {
        let source = PayloadU16::<NonEmpty>::new(vec![1, 2, 3]);
        let destination = source.clone();
        assert_eq!(source, destination);
    }

    #[test]
    fn decoded_list_max_minus_one_denies_before_growth() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 7, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        assert_eq!(Vec::<Tiny>::read(&mut reader), Err(InvalidMessage::MessageTooLarge));
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[derive(Debug, PartialEq)]
    struct LaterError(u8);
    impl Codec<'_> for LaterError {
        fn encode(&self, _: &mut Vec<u8>) {}
        fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
            match u8::read(r)? {
                0 => Err(InvalidMessage::MissingData("later element")),
                value => Ok(Self(value)),
            }
        }
    }
    impl TlsListElement for LaterError {
        const SIZE_LEN: ListLength = ListLength::U16;
    }

    #[test]
    fn decoded_list_later_error_destroys_backing_before_rollback() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: usize::MAX, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 2, 1, 0], decoded.clone());
        assert_eq!(
            Vec::<LaterError>::read(&mut reader),
            Err(InvalidMessage::MissingData("later element"))
        );
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_vec_custody_follows_backing_and_move() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 8, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let values = DecodedVec::<Tiny>::read(&mut reader).unwrap();
        assert_eq!(values.iter().map(|value| value.0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 8);

        let moved = values;
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 8);
        drop(moved);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_vec_copy_is_separately_charged() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: arc_bytes + 16, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let source = DecodedVec::<Tiny>::read(&mut reader).unwrap();
        let destination = source.try_copy_with_resource_owner().unwrap();
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 16);
        drop(source);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes + 8);
        drop(destination);
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_vec_later_error_releases_partial_backing() {
        let arc_bytes = DecodedOwner::arc_layout().unwrap();
        let owner = Arc::new(TestOwner { limit: usize::MAX, used: AtomicUsize::new(0) });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 2, 1, 0], decoded.clone());
        assert!(DecodedVec::<LaterError>::read(&mut reader).is_err());
        assert_eq!(owner.used.load(Ordering::SeqCst), arc_bytes);
        drop(reader);
        drop(decoded);
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_owner_denies_before_arc_allocation_and_releases_after_arc() {
        let bytes = DecodedOwner::arc_layout().unwrap();
        let denied = Arc::new(TestOwner { limit: bytes - 1, used: AtomicUsize::new(0) });
        assert!(DecodedOwner::new(denied.clone()).is_err());
        assert_eq!(denied.used.load(Ordering::SeqCst), 0);

        let funded = Arc::new(TestOwner { limit: bytes, used: AtomicUsize::new(0) });
        let (decoded, charge) = DecodedOwner::new(funded.clone()).unwrap();
        let last = decoded.clone();
        drop(decoded);
        assert_eq!(funded.used.load(Ordering::SeqCst), bytes);
        drop(last);
        assert_eq!(funded.used.load(Ordering::SeqCst), bytes);
        drop(charge);
        assert_eq!(funded.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn interrupted_length_prefixed_buffer_leaves_maximum_length() {
        let mut buf = Vec::new();
        let nested = LengthPrefixedBuffer::new(ListLength::U16, &mut buf);
        nested.buf.push(0xaa);
        assert_eq!(nested.buf, &vec![0xff, 0xff, 0xaa]);
        // <- if the buffer is accidentally read here, there is no possibility
        //    that the contents of the length-prefixed buffer are interpreted
        //    as a subsequent encoding (perhaps allowing injection of a different
        //    extension)
        drop(nested);
        assert_eq!(buf, vec![0x00, 0x01, 0xaa]);
    }
}
