use alloc::vec::Vec;
use core::fmt::Debug;
#[cfg(feature = "std")]
use core::alloc::Layout;
use core::marker::PhantomData;
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
}

/// Custody for the allocation containing an `Arc<DecodedOwner>`.
///
/// This is deliberately external to `DecodedOwner`: dropping the value stored
/// in an Arc happens before Arc deallocates its control block.
#[cfg(feature = "std")]
#[derive(Debug)]
pub(crate) struct DecodedOwnerArcCustody {
    owner: Arc<dyn DeframerBufferOwner>,
    bytes: usize,
}

#[cfg(feature = "std")]
impl Drop for DecodedOwnerArcCustody {
    fn drop(&mut self) {
        self.owner.release(self.bytes);
    }
}

#[cfg(feature = "std")]
impl DecodedOwner {
    pub(crate) fn new(
        owner: Arc<dyn DeframerBufferOwner>,
    ) -> Result<(Arc<Self>, DecodedOwnerArcCustody), InvalidMessage> {
        // Rust 1.94 alloc::sync::Arc requests the padded ArcInner<T> layout.
        let (layout, _) = Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<Self>())
            .map_err(|_| InvalidMessage::MessageTooLarge)?;
        let bytes = layout.pad_to_align().size();
        owner.try_reserve(bytes).map_err(|_| InvalidMessage::MessageTooLarge)?;
        let decoded = Arc::new(Self { owner: owner.clone(), charged: AtomicUsize::new(0) });
        Ok((decoded, DecodedOwnerArcCustody { owner, bytes }))
    }

    pub(crate) fn reserve(&self, bytes: usize) -> Result<(), InvalidMessage> {
        if bytes == 0 { return Ok(()); }
        self.owner.try_reserve(bytes).map_err(|_| InvalidMessage::MessageTooLarge)?;
        self.charged.fetch_add(bytes, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn release(&self, bytes: usize) {
        if bytes == 0 { return; }
        self.charged.fetch_sub(bytes, Ordering::Relaxed);
        self.owner.release(bytes);
    }

    pub(crate) fn checkpoint(&self) -> usize {
        self.charged.load(Ordering::Relaxed)
    }

    /// Roll back allocations made after `checkpoint`.
    ///
    /// Callers must first destroy all backing covered by those allocations.
    pub(crate) fn rollback(&self, checkpoint: usize) {
        let charged = self.charged.load(Ordering::Relaxed);
        debug_assert!(charged >= checkpoint);
        self.release(charged - checkpoint);
    }
}

#[cfg(feature = "std")]
impl Drop for DecodedOwner {
    fn drop(&mut self) {
        self.owner.release(self.charged.load(Ordering::Relaxed));
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
    pub(crate) fn copy_decoded(&self, bytes: &[u8]) -> Result<Vec<u8>, InvalidMessage> {
        let Some(owner) = &self.decoded_owner else { return Ok(bytes.to_vec()); };
        owner.reserve(bytes.len())?;
        let mut copied = Vec::with_capacity(bytes.len());
        copied.extend_from_slice(bytes);
        if copied.capacity() != bytes.len() {
            let reserved = bytes.len();
            drop(copied);
            owner.release(reserved);
            return Err(InvalidMessage::MessageTooLarge);
        }
        Ok(copied)
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
        let mut ret = Self::new();
        for item in TlsListIter::<T>::new(r)? {
            let item = item?;
            if ret.len() == ret.capacity() {
                #[cfg(feature = "std")]
                let old_capacity = ret.capacity();
                #[cfg(feature = "std")]
                let prospective = {
                    // Match the pinned Vec amortized-growth shape rather than
                    // forcing an input-controlled allocation per element.
                    let required = ret.len().checked_add(1).ok_or(InvalidMessage::MessageTooLarge)?;
                    let prospective = ret.capacity().saturating_mul(2)
                        .max(required)
                        .max(4);
                    r.reserve_vec_capacity::<T>(prospective)?;
                    prospective
                };
                #[cfg(feature = "std")]
                if r.decoded_owner.is_some() {
                    ret.reserve_exact(prospective - ret.len());
                }
                #[cfg(not(feature = "std"))]
                ret.reserve(1);
                #[cfg(feature = "std")]
                if r.decoded_owner.is_none() {
                    ret.reserve(1);
                }
                #[cfg(feature = "std")]
                if r.decoded_owner.is_some() && ret.capacity() != prospective {
                    drop(ret);
                    r.release_vec_capacity::<T>(old_capacity);
                    r.release_vec_capacity::<T>(prospective);
                    return Err(InvalidMessage::MessageTooLarge);
                }
                #[cfg(feature = "std")]
                r.release_vec_capacity::<T>(old_capacity);
            }
            ret.push(item);
        }

        Ok(ret)
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
    use core::alloc::Layout;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::prelude::v1::*;
    use std::vec;

    use super::*;
    use crate::msgs::base::{MaybeEmpty, PayloadU8};

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

    #[derive(Debug, PartialEq)]
    struct Tiny(u8);
    impl Codec<'_> for Tiny {
        fn encode(&self, out: &mut Vec<u8>) { out.push(self.0); }
        fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> { Ok(Self(u8::read(r)?)) }
    }
    impl TlsListElement for Tiny { const SIZE_LEN: ListLength = ListLength::U16; }

    fn decoded_arc_bytes() -> usize {
        Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<DecodedOwner>())
            .unwrap().0.pad_to_align().size()
    }

    #[test]
    fn decoded_list_reserves_before_each_exact_capacity() {
        let owner = Arc::new(TestOwner { limit: decoded_arc_bytes() + 4, used: AtomicUsize::new(0) });
        let (decoded, _arc_custody) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        let values = Vec::<Tiny>::read(&mut reader).unwrap();
        assert_eq!(values.iter().map(|v| v.0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(owner.used.load(Ordering::SeqCst), decoded_arc_bytes() + values.capacity());
        assert_eq!(values.capacity(), 4, "decoded lists retain geometric growth");
        drop(values);
        assert_eq!(owner.used.load(Ordering::SeqCst), decoded_arc_bytes() + 4, "custody follows decode scope");
        drop(_arc_custody);
        drop(decoded);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_payload_reserves_actual_capacity_before_copy() {
        let owner = Arc::new(TestOwner { limit: decoded_arc_bytes() + 3, used: AtomicUsize::new(0) });
        let (decoded, _arc_custody) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[3, 1, 2, 3], decoded.clone());

        let payload = PayloadU8::<MaybeEmpty>::read(&mut reader).unwrap();
        assert_eq!(payload.0, [1, 2, 3]);
        assert_eq!(owner.used.load(Ordering::SeqCst), decoded_arc_bytes() + payload.0.capacity());

        drop(payload);
        drop(reader);
        drop(decoded);
        drop(_arc_custody);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_payload_denies_before_copy() {
        let owner = Arc::new(TestOwner { limit: decoded_arc_bytes() + 2, used: AtomicUsize::new(0) });
        let (decoded, _arc_custody) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[3, 1, 2, 3], decoded.clone());

        assert_eq!(
            PayloadU8::<MaybeEmpty>::read(&mut reader),
            Err(InvalidMessage::MessageTooLarge)
        );
        assert_eq!(owner.used.load(Ordering::SeqCst), decoded_arc_bytes());
        drop(_arc_custody);
    }

    #[test]
    fn decoded_list_max_minus_one_denies_before_growth() {
        let owner = Arc::new(TestOwner { limit: decoded_arc_bytes() + 3, used: AtomicUsize::new(0) });
        let (decoded, _arc_custody) = DecodedOwner::new(owner.clone()).unwrap();
        let mut reader = Reader::init_with_owner(&[0, 3, 1, 2, 3], decoded.clone());
        assert_eq!(Vec::<Tiny>::read(&mut reader), Err(InvalidMessage::MessageTooLarge));
        drop(decoded);
        drop(_arc_custody);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn decoded_owner_denies_before_arc_allocation() {
        let bytes = decoded_arc_bytes();
        let owner = Arc::new(TestOwner { limit: bytes - 1, used: AtomicUsize::new(0) });
        assert!(DecodedOwner::new(owner.clone()).is_err());
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
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
