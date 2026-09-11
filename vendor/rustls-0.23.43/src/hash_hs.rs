use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem;

use crate::crypto::hash;
#[cfg(feature = "std")]
use crate::error::InvalidMessage;
use crate::msgs::codec::Codec;
#[cfg(feature = "std")]
use crate::msgs::codec::exact_vec_copy;
use crate::msgs::enums::HashAlgorithm;
use crate::msgs::handshake::HandshakeMessagePayload;
use crate::msgs::message::{Message, MessagePayload};
#[cfg(feature = "std")]
use crate::sync::Arc;
#[cfg(feature = "std")]
use crate::DeframerBufferOwner;

#[cfg(feature = "std")]
#[derive(Debug)]
struct TranscriptCustody {
    owner: Arc<dyn DeframerBufferOwner>,
    bytes: usize,
}

#[cfg(feature = "std")]
impl TranscriptCustody {
    fn reserve(
        owner: Arc<dyn DeframerBufferOwner>,
        bytes: usize,
    ) -> Result<Self, InvalidMessage> {
        if bytes != 0 {
            owner
                .try_reserve(bytes)
                .map_err(|_| InvalidMessage::MessageTooLarge)?;
        }
        Ok(Self { owner, bytes })
    }
}

#[cfg(feature = "std")]
impl Drop for TranscriptCustody {
    fn drop(&mut self) {
        if self.bytes != 0 {
            self.owner.release(self.bytes);
        }
    }
}

#[cfg(feature = "std")]
fn geometric_capacity(current: usize, needed: usize) -> Result<usize, InvalidMessage> {
    if needed <= current {
        return Ok(current);
    }
    let mut target = current.max(8);
    while target < needed {
        target = target
            .checked_mul(2)
            .ok_or(InvalidMessage::MessageTooLarge)?;
    }
    Ok(target)
}

#[cfg(feature = "std")]
fn exact_grown_bytes(
    old: &[u8],
    extra: &[u8],
    capacity: usize,
) -> Result<Vec<u8>, InvalidMessage> {
    let len = old
        .len()
        .checked_add(extra.len())
        .ok_or(InvalidMessage::MessageTooLarge)?;
    if len > capacity {
        return Err(InvalidMessage::MessageTooLarge);
    }

    // The reservation is acquired by the caller before this allocation. On
    // pinned Rust 1.94 `Vec::with_capacity` requests the exact u8 capacity;
    // fail closed if the allocator-facing capacity ever drifts.
    let mut values = Vec::with_capacity(capacity);
    if values.capacity() != capacity {
        return Err(InvalidMessage::MessageTooLarge);
    }
    values.extend_from_slice(old);
    values.extend_from_slice(extra);
    if values.capacity() != capacity {
        return Err(InvalidMessage::MessageTooLarge);
    }
    Ok(values)
}

#[cfg(feature = "std")]
fn qualified_hash_context_bytes(
    provider: &'static dyn hash::Hash,
) -> Result<usize, InvalidMessage> {
    #[cfg(all(
        feature = "aws_lc_rs",
        not(feature = "fips"),
        target_os = "linux",
        target_arch = "x86_64"
    ))]
    {
        let sha256: &'static dyn hash::Hash = &crate::crypto::aws_lc_rs::hash::SHA256;
        let sha384: &'static dyn hash::Hash = &crate::crypto::aws_lc_rs::hash::SHA384;
        // Trait-object vtable identities are not stable across codegen units.
        // Compare only the data address of the exact static provider object.
        if core::ptr::addr_eq(provider, sha256) {
            // Rust 1.94 Box<ring_like::hash::Context>: 72-byte Rust context
            // plus AWS-LC OPENSSL_malloc(sizeof(SHA256_CTX)=112) => 120.
            return Ok(192);
        }
        if core::ptr::addr_eq(provider, sha384) {
            // 72-byte Rust context plus
            // OPENSSL_malloc(sizeof(SHA512_CTX)=216) => 224.
            return Ok(296);
        }
    }

    let _ = provider;
    Err(InvalidMessage::MessageTooLarge)
}

#[cfg(feature = "std")]
fn qualified_hash_fork_finish_bytes(
    provider: &'static dyn hash::Hash,
) -> Result<usize, InvalidMessage> {
    #[cfg(all(
        feature = "aws_lc_rs",
        not(feature = "fips"),
        target_os = "linux",
        target_arch = "x86_64"
    ))]
    {
        let sha256: &'static dyn hash::Hash = &crate::crypto::aws_lc_rs::hash::SHA256;
        let sha384: &'static dyn hash::Hash = &crate::crypto::aws_lc_rs::hash::SHA384;
        // Keep provider recognition bound to the exact static AWS-LC objects,
        // while ignoring trait-object vtable metadata for identity.
        if core::ptr::addr_eq(provider, sha256) {
            return Ok(120);
        }
        if core::ptr::addr_eq(provider, sha384) {
            return Ok(224);
        }
    }

    let _ = provider;
    Err(InvalidMessage::MessageTooLarge)
}

/// Early stage buffering of handshake payloads.
///
/// Before we know the hash algorithm to use to verify the handshake, we just buffer the messages.
/// During the handshake, we may restart the transcript due to a HelloRetryRequest, reverting
/// from the `HandshakeHash` to a `HandshakeHashBuffer` again.
pub(crate) struct HandshakeHashBuffer {
    buffer: Vec<u8>,
    client_auth_enabled: bool,
    #[cfg(feature = "std")]
    resource_owner: Option<Arc<dyn DeframerBufferOwner>>,
    #[cfg(feature = "std")]
    buffer_custody: Option<TranscriptCustody>,
}

impl Clone for HandshakeHashBuffer {
    fn clone(&self) -> Self {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none() && self.buffer_custody.is_none(),
            "charged transcript buffer requires a fallible owner-aware clone"
        );
        Self {
            buffer: self.buffer.clone(),
            client_auth_enabled: self.client_auth_enabled,
            #[cfg(feature = "std")]
            resource_owner: None,
            #[cfg(feature = "std")]
            buffer_custody: None,
        }
    }
}

impl HandshakeHashBuffer {
    pub(crate) fn new() -> Self {
        Self {
            buffer: Vec::new(),
            client_auth_enabled: false,
            #[cfg(feature = "std")]
            resource_owner: None,
            #[cfg(feature = "std")]
            buffer_custody: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn install_resource_owner(
        &mut self,
        owner: Arc<dyn DeframerBufferOwner>,
    ) -> Result<(), InvalidMessage> {
        if let Some(existing) = &self.resource_owner {
            return if Arc::ptr_eq(existing, &owner) {
                Ok(())
            } else {
                Err(InvalidMessage::MessageTooLarge)
            };
        }
        if self.buffer.capacity() != 0 || self.buffer_custody.is_some() {
            return Err(InvalidMessage::MessageTooLarge);
        }
        self.resource_owner = Some(owner);
        Ok(())
    }

    /// We might be doing client auth, so need to keep a full
    /// log of the handshake.
    pub(crate) fn set_client_auth_enabled(&mut self) {
        self.client_auth_enabled = true;
    }

    /// Hash/buffer a handshake message.
    pub(crate) fn add_message(&mut self, m: &Message<'_>) {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_add_message"
        );
        match &m.payload {
            MessagePayload::Handshake { encoded, .. } => self.add_raw(encoded.bytes()),
            MessagePayload::HandshakeFlight(payload) => self.add_raw(payload.bytes()),
            _ => {}
        };
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_add_message(&mut self, m: &Message<'_>) -> Result<(), InvalidMessage> {
        match &m.payload {
            MessagePayload::Handshake { encoded, .. } => self.try_add_raw(encoded.bytes()),
            MessagePayload::HandshakeFlight(payload) => self.try_add_raw(payload.bytes()),
            _ => Ok(()),
        }
    }

    /// Hash or buffer a byte slice.
    fn add_raw(&mut self, buf: &[u8]) {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_add_raw"
        );
        self.buffer.extend_from_slice(buf);
    }

    #[cfg(feature = "std")]
    fn try_add_raw(&mut self, buf: &[u8]) -> Result<(), InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            self.buffer.extend_from_slice(buf);
            return Ok(());
        };
        let needed = self
            .buffer
            .len()
            .checked_add(buf.len())
            .ok_or(InvalidMessage::MessageTooLarge)?;
        if needed <= self.buffer.capacity() {
            self.buffer.extend_from_slice(buf);
            return Ok(());
        }

        let target = geometric_capacity(self.buffer.capacity(), needed)?;
        let prospective = TranscriptCustody::reserve(owner, target)?;
        let replacement = exact_grown_bytes(&self.buffer, buf, target)?;
        let old_buffer = mem::replace(&mut self.buffer, replacement);
        let old_custody = self.buffer_custody.replace(prospective);
        drop(old_buffer);
        drop(old_custody);
        Ok(())
    }

    /// Get the hash value if we were to hash `extra` too.
    pub(crate) fn hash_given(
        &self,
        provider: &'static dyn hash::Hash,
        extra: &[u8],
    ) -> hash::Output {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_hash_given"
        );
        let mut ctx = provider.start();
        ctx.update(&self.buffer);
        ctx.update(extra);
        ctx.finish()
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_hash_given(
        &self,
        provider: &'static dyn hash::Hash,
        extra: &[u8],
    ) -> Result<hash::Output, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            return Ok(self.hash_given(provider, extra));
        };
        let custody = TranscriptCustody::reserve(owner, qualified_hash_context_bytes(provider)?)?;
        let mut ctx = provider.start();
        ctx.update(&self.buffer);
        ctx.update(extra);
        let output = ctx.finish();
        drop(custody);
        Ok(output)
    }

    /// We now know what hash function the verify_data will use.
    pub(crate) fn start_hash(self, provider: &'static dyn hash::Hash) -> HandshakeHash {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_start_hash"
        );
        let mut ctx = provider.start();
        ctx.update(&self.buffer);
        HandshakeHash {
            provider,
            ctx,
            client_auth: match self.client_auth_enabled {
                true => Some(self.buffer),
                false => None,
            },
            #[cfg(feature = "std")]
            resource_owner: None,
            #[cfg(feature = "std")]
            ctx_custody: None,
            #[cfg(feature = "std")]
            client_auth_custody: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_start_hash(
        self,
        provider: &'static dyn hash::Hash,
    ) -> Result<HandshakeHash, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            return Ok(self.start_hash(provider));
        };
        let ctx_custody = TranscriptCustody::reserve(
            owner.clone(),
            qualified_hash_context_bytes(provider)?,
        )?;
        let mut ctx = provider.start();
        ctx.update(&self.buffer);

        let HandshakeHashBuffer {
            buffer,
            client_auth_enabled,
            resource_owner: _,
            buffer_custody,
        } = self;
        let (client_auth, client_auth_custody) = if client_auth_enabled {
            (Some(buffer), buffer_custody)
        } else {
            drop(buffer);
            drop(buffer_custody);
            (None, None)
        };

        Ok(HandshakeHash {
            provider,
            ctx,
            client_auth,
            resource_owner: Some(owner),
            ctx_custody: Some(ctx_custody),
            client_auth_custody,
        })
    }
}

/// This deals with keeping a running hash of the handshake
/// payloads.  This is computed by buffering initially.  Once
/// we know what hash function we need to use we switch to
/// incremental hashing.
///
/// For client auth, we also need to buffer all the messages.
/// This is disabled in cases where client auth is not possible.
pub(crate) struct HandshakeHash {
    provider: &'static dyn hash::Hash,
    ctx: Box<dyn hash::Context>,

    /// buffer for client-auth.
    client_auth: Option<Vec<u8>>,
    #[cfg(feature = "std")]
    resource_owner: Option<Arc<dyn DeframerBufferOwner>>,
    #[cfg(feature = "std")]
    ctx_custody: Option<TranscriptCustody>,
    #[cfg(feature = "std")]
    client_auth_custody: Option<TranscriptCustody>,
}

impl HandshakeHash {
    /// We decided not to do client auth after all, so discard
    /// the transcript.
    pub(crate) fn abandon_client_auth(&mut self) {
        self.client_auth = None;
        #[cfg(feature = "std")]
        {
            self.client_auth_custody = None;
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn resource_owner(&self) -> Option<Arc<dyn DeframerBufferOwner>> {
        self.resource_owner.clone()
    }

    /// Hash/buffer a handshake message.
    pub(crate) fn add_message(&mut self, m: &Message<'_>) -> &mut Self {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_add_message"
        );
        match &m.payload {
            MessagePayload::Handshake { encoded, .. } => self.add_raw(encoded.bytes()),
            MessagePayload::HandshakeFlight(payload) => self.add_raw(payload.bytes()),
            _ => self,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_add_message(&mut self, m: &Message<'_>) -> Result<&mut Self, InvalidMessage> {
        match &m.payload {
            MessagePayload::Handshake { encoded, .. } => {
                self.try_add_raw(encoded.bytes())?;
            }
            MessagePayload::HandshakeFlight(payload) => {
                self.try_add_raw(payload.bytes())?;
            }
            _ => {}
        }
        Ok(self)
    }

    /// Hash/buffer an encoded handshake message.
    pub(crate) fn add(&mut self, bytes: &[u8]) {
        self.add_raw(bytes);
    }

    /// Hash or buffer a byte slice.
    fn add_raw(&mut self, buf: &[u8]) -> &mut Self {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_add_raw"
        );
        self.ctx.update(buf);

        if let Some(buffer) = &mut self.client_auth {
            buffer.extend_from_slice(buf);
        }

        self
    }

    #[cfg(feature = "std")]
    fn try_add_raw(&mut self, buf: &[u8]) -> Result<&mut Self, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            self.add_raw(buf);
            return Ok(self);
        };

        if let Some(buffer) = &self.client_auth {
            let needed = buffer
                .len()
                .checked_add(buf.len())
                .ok_or(InvalidMessage::MessageTooLarge)?;
            if needed > buffer.capacity() {
                let target = geometric_capacity(buffer.capacity(), needed)?;
                let prospective = TranscriptCustody::reserve(owner, target)?;
                let replacement = exact_grown_bytes(buffer, buf, target)?;
                let buffer = self.client_auth.as_mut().unwrap();
                let old_buffer = mem::replace(buffer, replacement);
                let old_custody = self.client_auth_custody.replace(prospective);
                drop(old_buffer);
                drop(old_custody);
                self.ctx.update(buf);
                return Ok(self);
            }
        }

        if let Some(buffer) = &mut self.client_auth {
            buffer.extend_from_slice(buf);
        }
        self.ctx.update(buf);
        Ok(self)
    }

    /// Get the hash value if we were to hash `extra` too,
    /// using hash function `hash`.
    pub(crate) fn hash_given(&self, extra: &[u8]) -> hash::Output {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_hash_given"
        );
        let mut ctx = self.ctx.fork();
        ctx.update(extra);
        ctx.finish()
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_hash_given(&self, extra: &[u8]) -> Result<hash::Output, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            return Ok(self.hash_given(extra));
        };
        let custody = TranscriptCustody::reserve(
            owner,
            qualified_hash_context_bytes(self.provider)?,
        )?;
        let mut ctx = self.ctx.fork();
        ctx.update(extra);
        let output = ctx.finish();
        drop(custody);
        Ok(output)
    }

    pub(crate) fn into_hrr_buffer(self) -> HandshakeHashBuffer {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_into_hrr_buffer"
        );
        let old_hash = self.ctx.finish();
        let old_handshake_hash_msg =
            HandshakeMessagePayload::build_handshake_hash(old_hash.as_ref());

        HandshakeHashBuffer {
            client_auth_enabled: self.client_auth.is_some(),
            buffer: old_handshake_hash_msg.get_encoding(),
            #[cfg(feature = "std")]
            resource_owner: None,
            #[cfg(feature = "std")]
            buffer_custody: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_into_hrr_buffer(self) -> Result<HandshakeHashBuffer, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            return Ok(self.into_hrr_buffer());
        };
        let HandshakeHash {
            provider: _,
            ctx,
            client_auth,
            resource_owner: _,
            ctx_custody,
            client_auth_custody,
        } = self;
        let client_auth_enabled = client_auth.is_some();
        let old_hash = ctx.finish();
        drop(ctx_custody);
        drop(client_auth);
        drop(client_auth_custody);

        let old_handshake_hash_msg =
            HandshakeMessagePayload::build_handshake_hash(old_hash.as_ref());
        let bytes = old_hash
            .as_ref()
            .len()
            .checked_add(4)
            .ok_or(InvalidMessage::MessageTooLarge)?;
        let custody = TranscriptCustody::reserve(owner.clone(), bytes)?;
        let mut buffer = Vec::with_capacity(bytes);
        if buffer.capacity() != bytes {
            return Err(InvalidMessage::MessageTooLarge);
        }
        old_handshake_hash_msg.encode(&mut buffer);
        if buffer.len() != bytes || buffer.capacity() != bytes {
            drop(buffer);
            drop(custody);
            return Err(InvalidMessage::MessageTooLarge);
        }

        Ok(HandshakeHashBuffer {
            buffer,
            client_auth_enabled,
            resource_owner: Some(owner),
            buffer_custody: Some(custody),
        })
    }

    /// Take the current hash value, and encapsulate it in a
    /// 'handshake_hash' handshake message.  Start this hash
    /// again, with that message at the front.
    pub(crate) fn rollup_for_hrr(&mut self) {
        let ctx = &mut self.ctx;

        let old_ctx = mem::replace(ctx, self.provider.start());
        let old_hash = old_ctx.finish();
        let old_handshake_hash_msg =
            HandshakeMessagePayload::build_handshake_hash(old_hash.as_ref());

        self.add_raw(&old_handshake_hash_msg.get_encoding());
    }

    /// Get the current hash value.
    pub(crate) fn current_hash(&self) -> hash::Output {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "owner-aware transcript must use try_current_hash"
        );
        self.ctx.fork_finish()
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_current_hash(&self) -> Result<hash::Output, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            return Ok(self.current_hash());
        };
        let custody = TranscriptCustody::reserve(
            owner,
            qualified_hash_fork_finish_bytes(self.provider)?,
        )?;
        let output = self.ctx.fork_finish();
        drop(custody);
        Ok(output)
    }

    /// Takes this object's buffer containing all handshake messages
    /// so far.  This method only works once; it resets the buffer
    /// to empty.
    #[cfg(feature = "tls12")]
    pub(crate) fn take_handshake_buf(&mut self) -> Option<Vec<u8>> {
        self.client_auth.take()
    }

    /// The hashing algorithm
    pub(crate) fn algorithm(&self) -> HashAlgorithm {
        self.provider.algorithm()
    }

    #[cfg(feature = "std")]
    pub(crate) fn try_clone_with_resource_owner(&self) -> Result<Self, InvalidMessage> {
        let Some(owner) = self.resource_owner.clone() else {
            return Ok(self.clone());
        };

        let ctx_custody = TranscriptCustody::reserve(
            owner.clone(),
            qualified_hash_context_bytes(self.provider)?,
        )?;
        let ctx = self.ctx.fork();

        let (client_auth, client_auth_custody) = match &self.client_auth {
            Some(source) if !source.is_empty() => {
                let custody = TranscriptCustody::reserve(owner.clone(), source.len())?;
                let values = exact_vec_copy(source);
                if values.capacity() != source.len() {
                    drop(values);
                    drop(custody);
                    return Err(InvalidMessage::MessageTooLarge);
                }
                (Some(values), Some(custody))
            }
            Some(_) => (Some(Vec::new()), None),
            None => (None, None),
        };

        Ok(Self {
            provider: self.provider,
            ctx,
            client_auth,
            resource_owner: Some(owner),
            ctx_custody: Some(ctx_custody),
            client_auth_custody,
        })
    }
}

impl Clone for HandshakeHash {
    fn clone(&self) -> Self {
        #[cfg(feature = "std")]
        assert!(
            self.resource_owner.is_none(),
            "charged transcript requires try_clone_with_resource_owner"
        );
        Self {
            provider: self.provider,
            ctx: self.ctx.fork(),
            client_auth: self.client_auth.clone(),
            #[cfg(feature = "std")]
            resource_owner: None,
            #[cfg(feature = "std")]
            ctx_custody: None,
            #[cfg(feature = "std")]
            client_auth_custody: None,
        }
    }
}

#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use super::provider::hash::SHA256;
    use super::*;
    use crate::crypto::hash::Hash;
    use crate::enums::ProtocolVersion;
    use crate::msgs::base::Payload;
    use crate::msgs::handshake::{HandshakeMessagePayload, HandshakePayload};

    #[test]
    fn hashes_correctly() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.add_raw(b"hello");
        assert_eq!(hhb.buffer.len(), 5);
        let mut hh = hhb.start_hash(&SHA256);
        assert!(hh.client_auth.is_none());
        hh.add_raw(b"world");
        let h = hh.current_hash();
        let h = h.as_ref();
        assert_eq!(h[0], 0x93);
        assert_eq!(h[1], 0x6a);
        assert_eq!(h[2], 0x18);
        assert_eq!(h[3], 0x5c);
    }

    #[test]
    fn hashes_message_types() {
        // handshake protocol encoding of 0x0e 00 00 00
        let server_hello_done_message = Message {
            version: ProtocolVersion::TLSv1_2,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ServerHelloDone,
            )),
        };

        let app_data_ignored = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::ApplicationData(Payload::Borrowed(b"hello")),
        };

        let end_of_early_data_flight = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::HandshakeFlight(Payload::Borrowed(b"\x05\x00\x00\x00")),
        };

        // buffered mode
        let mut hhb = HandshakeHashBuffer::new();
        hhb.add_message(&server_hello_done_message);
        hhb.add_message(&app_data_ignored);
        hhb.add_message(&end_of_early_data_flight);
        assert_eq!(
            hhb.start_hash(&SHA256)
                .current_hash()
                .as_ref(),
            SHA256
                .hash(b"\x0e\x00\x00\x00\x05\x00\x00\x00")
                .as_ref()
        );

        // non-buffered mode
        let mut hh = HandshakeHashBuffer::new().start_hash(&SHA256);
        hh.add_message(&server_hello_done_message);
        hh.add_message(&app_data_ignored);
        hh.add_message(&end_of_early_data_flight);
        assert_eq!(
            hh.current_hash().as_ref(),
            SHA256
                .hash(b"\x0e\x00\x00\x00\x05\x00\x00\x00")
                .as_ref()
        );
    }

    #[cfg(feature = "tls12")]
    #[test]
    fn buffers_correctly() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.set_client_auth_enabled();
        hhb.add_raw(b"hello");
        let mut hh = hhb.start_hash(&SHA256);
        assert_eq!(
            hh.client_auth
                .as_ref()
                .map(|buf| buf.len()),
            Some(5)
        );
        hh.add_raw(b"world");
        assert_eq!(
            hh.client_auth
                .as_ref()
                .map(|buf| buf.len()),
            Some(10)
        );
        let h = hh.current_hash();
        let h = h.as_ref();
        assert_eq!(h[0], 0x93);
        assert_eq!(h[1], 0x6a);
        assert_eq!(h[2], 0x18);
        assert_eq!(h[3], 0x5c);
        let buf = hh.take_handshake_buf();
        assert_eq!(Some(b"helloworld".to_vec()), buf);
    }

    #[test]
    fn abandon() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.set_client_auth_enabled();
        hhb.add_raw(b"hello");
        let mut hh = hhb.start_hash(&SHA256);
        hh.abandon_client_auth();
        assert_eq!(hh.client_auth, None);
        hh.add_raw(b"world");
        assert_eq!(hh.client_auth, None);
        let h = hh.current_hash();
        let h = h.as_ref();
        assert_eq!(h[0], 0x93);
        assert_eq!(h[1], 0x6a);
        assert_eq!(h[2], 0x18);
        assert_eq!(h[3], 0x5c);
    }

    #[test]
    fn clones_correctly() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.set_client_auth_enabled();
        hhb.add_raw(b"hello");

        let mut hhb_prime = hhb.clone();
        assert_eq!(hhb_prime.buffer, hhb.buffer);
        assert!(hhb_prime.client_auth_enabled);

        hhb_prime.add_raw(b"world");
        assert_eq!(hhb_prime.buffer.len(), 10);
        assert_ne!(hhb.buffer, hhb_prime.buffer);

        let hh = hhb.start_hash(&SHA256);
        let hh_hash = hh.current_hash();
        let hh_hash = hh_hash.as_ref();

        let mut hh_prime = hh.clone();
        let hh_prime_hash = hh_prime.current_hash();
        let hh_prime_hash = hh_prime_hash.as_ref();
        assert_eq!(hh_hash, hh_prime_hash);

        hh_prime.add_raw(b"goodbye");
        assert_eq!(hh.current_hash().as_ref(), hh_hash);
        assert_ne!(hh_prime.current_hash().as_ref(), hh_hash);
    }
}
