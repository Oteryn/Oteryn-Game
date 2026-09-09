use crate::enums::{AlertDescription, ContentType, HandshakeType, ProtocolVersion};
use crate::error::InvalidMessage;
use crate::msgs::alert::AlertMessagePayload;
use crate::msgs::base::Payload;
use crate::msgs::ccs::ChangeCipherSpecPayload;
use crate::msgs::codec::{Codec, Reader};
#[cfg(feature = "std")]
use crate::msgs::codec::DecodedVec;
#[cfg(all(feature = "std", not(test)))]
use crate::msgs::codec::DecodedCustody;
#[cfg(feature = "std")]
use crate::msgs::codec::DecodedOwner;
#[cfg(feature = "std")]
use crate::sync::Arc;
use crate::msgs::enums::{AlertLevel, KeyUpdateRequest};
use crate::msgs::handshake::{HandshakeMessagePayload, HandshakePayload};

mod inbound;
pub use inbound::{BorrowedPayload, InboundOpaqueMessage, InboundPlainMessage};

mod outbound;
use alloc::vec::Vec;

pub(crate) use outbound::read_opaque_message_header;
pub use outbound::{OutboundChunks, OutboundOpaqueMessage, OutboundPlainMessage, PrefixedPayload};

#[derive(Debug)]
pub enum MessagePayload<'a> {
    Alert(AlertMessagePayload),
    // one handshake message, parsed
    Handshake {
        parsed: HandshakeMessagePayload<'a>,
        encoded: Payload<'a>,
    },
    // (potentially) multiple handshake messages, unparsed
    HandshakeFlight(Payload<'a>),
    ChangeCipherSpec(ChangeCipherSpecPayload),
    ApplicationData(Payload<'a>),
}

impl<'a> MessagePayload<'a> {
    pub fn encode(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Alert(x) => x.encode(bytes),
            Self::Handshake { encoded, .. } => bytes.extend(encoded.bytes()),
            Self::HandshakeFlight(x) => bytes.extend(x.bytes()),
            Self::ChangeCipherSpec(x) => x.encode(bytes),
            Self::ApplicationData(x) => x.encode(bytes),
        }
    }

    pub fn handshake(parsed: HandshakeMessagePayload<'a>) -> Self {
        Self::Handshake {
            encoded: Payload::new(parsed.get_encoding()),
            parsed,
        }
    }

    pub fn new(
        typ: ContentType,
        vers: ProtocolVersion,
        payload: &'a [u8],
    ) -> Result<Self, InvalidMessage> {
        let mut r = Reader::init(payload);
        match typ {
            ContentType::ApplicationData => Ok(Self::ApplicationData(Payload::Borrowed(payload))),
            ContentType::Alert => AlertMessagePayload::read(&mut r).map(MessagePayload::Alert),
            ContentType::Handshake => {
                HandshakeMessagePayload::read_version(&mut r, vers).map(|parsed| Self::Handshake {
                    parsed,
                    encoded: Payload::Borrowed(payload),
                })
            }
            ContentType::ChangeCipherSpec => {
                ChangeCipherSpecPayload::read(&mut r).map(MessagePayload::ChangeCipherSpec)
            }
            _ => Err(InvalidMessage::InvalidContentType),
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn new_with_resource_owner(
        typ: ContentType,
        vers: ProtocolVersion,
        payload: &'a [u8],
        owner: Arc<DecodedOwner>,
    ) -> Result<Self, InvalidMessage> {
        let checkpoint = owner.checkpoint();
        let mut r = Reader::init_with_owner(payload, owner.clone());
        let decoded = match typ {
            ContentType::ApplicationData => Ok(Self::ApplicationData(Payload::Borrowed(payload))),
            ContentType::Alert => AlertMessagePayload::read(&mut r).map(MessagePayload::Alert),
            ContentType::Handshake => {
                HandshakeMessagePayload::read_version(&mut r, vers).map(|parsed| Self::Handshake {
                    parsed,
                    encoded: Payload::Borrowed(payload),
                })
            }
            ContentType::ChangeCipherSpec => {
                ChangeCipherSpecPayload::read(&mut r).map(MessagePayload::ChangeCipherSpec)
            }
            _ => Err(InvalidMessage::InvalidContentType),
        };
        match decoded {
            Ok(decoded) => Ok(decoded),
            Err(error) => {
                // The failed parser has already destroyed every partially built
                // value before the corresponding debit is returned.
                owner.rollback(checkpoint);
                Err(error)
            }
        }
    }

    pub fn content_type(&self) -> ContentType {
        match self {
            Self::Alert(_) => ContentType::Alert,
            Self::Handshake { .. } | Self::HandshakeFlight(_) => ContentType::Handshake,
            Self::ChangeCipherSpec(_) => ContentType::ChangeCipherSpec,
            Self::ApplicationData(_) => ContentType::ApplicationData,
        }
    }

    pub(crate) fn into_owned(self) -> MessagePayload<'static> {
        use MessagePayload::*;
        match self {
            Alert(x) => Alert(x),
            Handshake { parsed, encoded } => Handshake {
                parsed: parsed.into_owned(),
                encoded: encoded.into_owned(),
            },
            HandshakeFlight(x) => HandshakeFlight(x.into_owned()),
            ChangeCipherSpec(x) => ChangeCipherSpec(x),
            ApplicationData(x) => ApplicationData(x.into_owned()),
        }
    }
}

impl From<Message<'_>> for PlainMessage {
    fn from(msg: Message<'_>) -> Self {
        let typ = msg.payload.content_type();
        let payload = match msg.payload {
            MessagePayload::ApplicationData(payload) => payload.into_owned(),
            _ => {
                let mut buf = Vec::new();
                msg.payload.encode(&mut buf);
                Payload::Owned(buf)
            }
        };

        Self {
            typ,
            version: msg.version,
            payload,
        }
    }
}

/// A decrypted TLS frame
///
/// This type owns all memory for its interior parts. It can be decrypted from an OpaqueMessage
/// or encrypted into an OpaqueMessage, and it is also used for joining and fragmenting.
#[derive(Clone, Debug)]
pub struct PlainMessage {
    pub typ: ContentType,
    pub version: ProtocolVersion,
    pub payload: Payload<'static>,
}

impl PlainMessage {
    pub fn into_unencrypted_opaque(self) -> OutboundOpaqueMessage {
        OutboundOpaqueMessage {
            version: self.version,
            typ: self.typ,
            payload: PrefixedPayload::from(self.payload.bytes()),
        }
    }

    pub fn borrow_inbound(&self) -> InboundPlainMessage<'_> {
        InboundPlainMessage {
            version: self.version,
            typ: self.typ,
            payload: self.payload.bytes(),
        }
    }

    pub fn borrow_outbound(&self) -> OutboundPlainMessage<'_> {
        OutboundPlainMessage {
            version: self.version,
            typ: self.typ,
            payload: self.payload.bytes().into(),
        }
    }
}

/// A message with decoded payload
#[derive(Debug)]
pub struct Message<'a> {
    pub version: ProtocolVersion,
    pub payload: MessagePayload<'a>,
    #[cfg(all(feature = "std", not(test)))]
    pub(crate) decoded_custody: Option<DecodedCustody>,
}

impl<'a> Message<'a> {
    pub(crate) fn new(version: ProtocolVersion, payload: MessagePayload<'a>) -> Self {
        Self {
            version,
            payload,
            #[cfg(all(feature = "std", not(test)))]
            decoded_custody: None,
        }
    }
    pub fn is_handshake_type(&self, hstyp: HandshakeType) -> bool {
        // Bit of a layering violation, but OK.
        if let MessagePayload::Handshake { parsed, .. } = &self.payload {
            parsed.0.handshake_type() == hstyp
        } else {
            false
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn copy_decoded_filter<T, F>(
        &self,
        source: &[T],
        keep: F,
    ) -> Result<DecodedVec<T>, InvalidMessage>
    where
        T: Copy,
        F: Fn(&T) -> bool,
    {
        #[cfg(not(test))]
        let owner = self
            .decoded_custody
            .as_ref()
            .map(DecodedCustody::owner);
        #[cfg(test)]
        let owner = None;
        DecodedVec::try_copy_filtered(owner, source, keep)
    }

    pub fn build_alert(level: AlertLevel, desc: AlertDescription) -> Self {
        Self::new(
            ProtocolVersion::TLSv1_2,
            MessagePayload::Alert(AlertMessagePayload {
                level,
                description: desc,
            }),
        )
    }

    pub fn build_key_update_notify() -> Self {
        Self::new(
            ProtocolVersion::TLSv1_3,
            MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::KeyUpdate(KeyUpdateRequest::UpdateNotRequested),
            )),
        )
    }

    pub fn build_key_update_request() -> Self {
        Self::new(
            ProtocolVersion::TLSv1_3,
            MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::KeyUpdate(KeyUpdateRequest::UpdateRequested),
            )),
        )
    }

    #[cfg(feature = "std")]
    pub(crate) fn into_owned(self) -> Message<'static> {
        let Self {
            version,
            payload,
            #[cfg(not(test))]
            decoded_custody,
        } = self;
        Message {
            version,
            payload: payload.into_owned(),
            #[cfg(not(test))]
            decoded_custody,
        }
    }

    #[cfg(test)]
    pub(crate) fn into_wire_bytes(self) -> Vec<u8> {
        PlainMessage::from(self)
            .into_unencrypted_opaque()
            .encode()
    }
}

impl TryFrom<PlainMessage> for Message<'static> {
    type Error = InvalidMessage;

    fn try_from(plain: PlainMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            version: plain.version,
            payload: MessagePayload::new(plain.typ, plain.version, plain.payload.bytes())?
                .into_owned(),
            #[cfg(all(feature = "std", not(test)))]
            decoded_custody: None,
        })
    }
}

/// Parses a plaintext message into a well-typed [`Message`].
///
/// A [`PlainMessage`] must contain plaintext content. Encrypted content should be stored in an
/// [`InboundOpaqueMessage`] and decrypted before being stored into a [`PlainMessage`].
impl<'a> TryFrom<InboundPlainMessage<'a>> for Message<'a> {
    type Error = InvalidMessage;

    fn try_from(plain: InboundPlainMessage<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            version: plain.version,
            payload: MessagePayload::new(plain.typ, plain.version, plain.payload)?,
            #[cfg(all(feature = "std", not(test)))]
            decoded_custody: None,
        })
    }
}

#[cfg(feature = "std")]
impl<'a> Message<'a> {
    pub(crate) fn try_from_with_resource_owner(
        value: InboundPlainMessage<'a>,
        owner: Arc<DecodedOwner>,
    ) -> Result<Self, InvalidMessage> {
        #[cfg(not(test))]
        let checkpoint = owner.checkpoint();
        let payload = MessagePayload::new_with_resource_owner(
            value.typ,
            value.version,
            value.payload,
            owner.clone(),
        )?;
        Ok(Self {
            version: value.version,
            payload,
            #[cfg(not(test))]
            decoded_custody: Some(DecodedCustody::since(owner, checkpoint)),
        })
    }
}

#[derive(Debug)]
pub enum MessageError {
    TooShortForHeader,
    TooShortForLength,
    InvalidEmptyPayload,
    MessageTooLarge,
    InvalidContentType,
    UnknownProtocolVersion,
}

/// Content type, version and size.
pub(crate) const HEADER_SIZE: usize = 1 + 2 + 2;

/// Maximum message payload size.
/// That's 2^14 payload bytes and a 2KB allowance for ciphertext overheads.
const MAX_PAYLOAD: u16 = 16_384 + 2048;

/// Maximum on-the-wire message size.
#[cfg(feature = "std")]
pub(crate) const MAX_WIRE_SIZE: usize = MAX_PAYLOAD as usize + HEADER_SIZE;
