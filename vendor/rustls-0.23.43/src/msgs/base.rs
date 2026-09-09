use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use pki_types::CertificateDer;
use zeroize::Zeroize;

use crate::error::InvalidMessage;
use crate::msgs::codec;
use crate::msgs::codec::{Codec, Reader};
#[cfg(feature = "std")]
use crate::msgs::codec::DecodedCustody;


/// An externally length'd payload
#[derive(Clone, Eq, PartialEq)]
pub enum Payload<'a> {
    Borrowed(&'a [u8]),
    Owned(Vec<u8>),
}

impl<'a> Codec<'a> for Payload<'a> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(self.bytes());
    }

    fn read(r: &mut Reader<'a>) -> Result<Self, InvalidMessage> {
        Ok(Self::read(r))
    }
}

impl<'a> Payload<'a> {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Borrowed(bytes) => bytes,
            Self::Owned(bytes) => bytes,
        }
    }

    pub fn into_owned(self) -> Payload<'static> {
        Payload::Owned(self.into_vec())
    }

    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Borrowed(bytes) => bytes.to_vec(),
            Self::Owned(bytes) => bytes,
        }
    }

    /// Convert borrowed payload bytes into independently charged backing.
    /// Owned payloads move allocation-free and therefore need no new debit.
    #[cfg(feature = "std")]
    #[allow(dead_code)] // Consumed by the next owner-aware Message conversion.
    pub(crate) fn into_owned_with_resource_owner(
        self,
        source_custody: &DecodedCustody,
    ) -> Result<(Payload<'static>, Option<DecodedCustody>), InvalidMessage> {
        match self {
            Self::Borrowed([]) => Ok((Payload::Owned(Vec::new()), None)),
            Self::Borrowed(bytes) => {
                let (bytes, custody) = source_custody.copy_bytes(bytes)?;
                Ok((Payload::Owned(bytes), Some(custody)))
            }
            Self::Owned(bytes) => Ok((Payload::Owned(bytes), None)),
        }
    }

    pub fn read(r: &mut Reader<'a>) -> Self {
        Self::Borrowed(r.rest())
    }
}

impl Payload<'static> {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Owned(bytes.into())
    }

    pub fn empty() -> Self {
        Self::Borrowed(&[])
    }
}

impl<'a> Codec<'a> for CertificateDer<'a> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        codec::u24(self.as_ref().len() as u32).encode(bytes);
        bytes.extend(self.as_ref());
    }

    fn read(r: &mut Reader<'a>) -> Result<Self, InvalidMessage> {
        let len = codec::u24::read(r)?.0 as usize;
        let mut sub = r.sub(len)?;
        let body = sub.rest();
        Ok(Self::from(body))
    }
}

impl fmt::Debug for Payload<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex(f, self.bytes())
    }
}

/// An arbitrary, unknown-content, u24-length-prefixed payload
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct PayloadU24<'a>(pub(crate) Payload<'a>);

impl PayloadU24<'_> {
    pub(crate) fn into_owned(self) -> PayloadU24<'static> {
        PayloadU24(self.0.into_owned())
    }
}

impl<'a> Codec<'a> for PayloadU24<'a> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        let inner = self.0.bytes();
        codec::u24(inner.len() as u32).encode(bytes);
        bytes.extend_from_slice(inner);
    }

    fn read(r: &mut Reader<'a>) -> Result<Self, InvalidMessage> {
        let len = codec::u24::read(r)?.0 as usize;
        let mut sub = r.sub(len)?;
        Ok(Self(Payload::read(&mut sub)))
    }
}

impl fmt::Debug for PayloadU24<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An arbitrary, unknown-content, u16-length-prefixed payload
///
/// The `C` type parameter controls whether decoded values may
/// be empty.
pub struct PayloadU16<C: Cardinality = MaybeEmpty>(pub(crate) Vec<u8>, PhantomData<C>, #[cfg(feature = "std")] Option<DecodedCustody>);

impl<C: Cardinality> Clone for PayloadU16<C> {
    fn clone(&self) -> Self {
        #[cfg(feature = "std")]
        assert!(self.2.is_none(), "charged payloads require an owner-aware deep copy");
        Self::new(self.0.clone())
    }
}
impl<C: Cardinality> PartialEq for PayloadU16<C> { fn eq(&self, other: &Self) -> bool { self.0 == other.0 } }
impl<C: Cardinality> Eq for PayloadU16<C> {}

impl<C: Cardinality> PayloadU16<C> {
    pub fn new(bytes: Vec<u8>) -> Self {
        debug_assert!(bytes.len() >= C::MIN);
        Self(bytes, PhantomData, #[cfg(feature = "std")] None)
    }

    pub(crate) fn try_clone_with_resource_owner(&self) -> Result<Self, InvalidMessage> {
        #[cfg(feature = "std")]
        if let Some(custody) = &self.2 {
            let (bytes, custody) = custody.copy_bytes(&self.0)?;
            return Ok(Self(bytes, PhantomData, Some(custody)));
        }

        Ok(Self::new(self.0.clone()))
    }
}

impl PayloadU16<MaybeEmpty> {
    pub(crate) fn empty() -> Self {
        Self::new(Vec::new())
    }
}

impl<C: Cardinality> Codec<'_> for PayloadU16<C> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        debug_assert!(self.0.len() >= C::MIN);
        (self.0.len() as u16).encode(bytes);
        bytes.extend_from_slice(&self.0);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        let len = u16::read(r)? as usize;
        if len < C::MIN {
            return Err(InvalidMessage::IllegalEmptyValue);
        }
        let mut sub = r.sub(len)?;
        #[cfg(feature = "std")]
        let body = {
            let bytes = sub.rest();
            sub.copy_decoded(bytes)?
        };
        #[cfg(not(feature = "std"))]
        let body = sub.rest().to_vec();
        #[cfg(feature = "std")]
        return Ok(Self(body.0, PhantomData, body.1));
        #[cfg(not(feature = "std"))]
        Ok(Self(body, PhantomData))
    }
}

impl<C: Cardinality> fmt::Debug for PayloadU16<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex(f, &self.0)
    }
}

/// An arbitrary, unknown-content, u8-length-prefixed payload
///
/// `C` controls the minimum length accepted when decoding.
pub(crate) struct PayloadU8<C: Cardinality = MaybeEmpty>(pub(crate) Vec<u8>, PhantomData<C>, #[cfg(feature = "std")] Option<DecodedCustody>);

impl<C: Cardinality> Clone for PayloadU8<C> {
    fn clone(&self) -> Self {
        #[cfg(feature = "std")]
        assert!(self.2.is_none(), "charged payloads require an owner-aware deep copy");
        Self::new(self.0.clone())
    }
}
impl<C: Cardinality> PartialEq for PayloadU8<C> { fn eq(&self, other: &Self) -> bool { self.0 == other.0 } }
impl<C: Cardinality> Eq for PayloadU8<C> {}

impl<C: Cardinality> PayloadU8<C> {
    pub(crate) fn encode_slice(slice: &[u8], bytes: &mut Vec<u8>) {
        (slice.len() as u8).encode(bytes);
        bytes.extend_from_slice(slice);
    }

    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        debug_assert!(bytes.len() >= C::MIN);
        Self(bytes, PhantomData, #[cfg(feature = "std")] None)
    }
}

impl PayloadU8<MaybeEmpty> {
    pub(crate) fn empty() -> Self {
        Self(Vec::new(), PhantomData, #[cfg(feature = "std")] None)
    }
}

impl<C: Cardinality> Codec<'_> for PayloadU8<C> {
    fn encode(&self, bytes: &mut Vec<u8>) {
        debug_assert!(self.0.len() >= C::MIN);
        (self.0.len() as u8).encode(bytes);
        bytes.extend_from_slice(&self.0);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        let len = u8::read(r)? as usize;
        if len < C::MIN {
            return Err(InvalidMessage::IllegalEmptyValue);
        }
        let mut sub = r.sub(len)?;
        #[cfg(feature = "std")]
        let body = {
            let bytes = sub.rest();
            sub.copy_decoded(bytes)?
        };
        #[cfg(not(feature = "std"))]
        let body = sub.rest().to_vec();
        #[cfg(feature = "std")]
        return Ok(Self(body.0, PhantomData, body.1));
        #[cfg(not(feature = "std"))]
        Ok(Self(body, PhantomData))
    }
}

impl<C: Cardinality> Zeroize for PayloadU8<C> {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl<C: Cardinality> fmt::Debug for PayloadU8<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex(f, &self.0)
    }
}

pub trait Cardinality: Clone + Eq + PartialEq {
    const MIN: usize;
}

#[derive(Clone, Eq, PartialEq)]
pub struct MaybeEmpty;

impl Cardinality for MaybeEmpty {
    const MIN: usize = 0;
}

#[derive(Clone, Eq, PartialEq)]
pub struct NonEmpty;

impl Cardinality for NonEmpty {
    const MIN: usize = 1;
}

// Format an iterator of u8 into a hex string
pub(super) fn hex<'a>(
    f: &mut fmt::Formatter<'_>,
    payload: impl IntoIterator<Item = &'a u8>,
) -> fmt::Result {
    for b in payload {
        write!(f, "{b:02x}")?;
    }
    Ok(())
}
