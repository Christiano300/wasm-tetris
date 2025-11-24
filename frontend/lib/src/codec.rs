//! Adapted from <https://github.com/matthunz/futures-codec/blob/master/src/codec/cbor.rs> to use
//! packed CBOR format
use std::io::Error as IoError;
use std::marker::PhantomData;

use bytes::{Buf, BufMut, BytesMut};

use futures_codec::{Decoder, Encoder};
use serde::{Deserialize, Serialize};
use serde_cbor::Error as CborError;

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct CborCodec<Enc, Dec> {
    enc: PhantomData<Enc>,
    dec: PhantomData<Dec>,
}

#[derive(Debug)]
pub enum CborCodecError {
    Io(IoError),
    Cbor(CborError),
}

impl std::fmt::Display for CborCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Cbor(e) => write!(f, "CBOR error: {e}"),
        }
    }
}

impl std::error::Error for CborCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(ref e) => Some(e),
            Self::Cbor(ref e) => Some(e),
        }
    }
}

impl From<IoError> for CborCodecError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<CborError> for CborCodecError {
    fn from(e: CborError) -> Self {
        Self::Cbor(e)
    }
}

impl<Enc, Dec> CborCodec<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    pub const fn new() -> Self {
        Self {
            enc: PhantomData,
            dec: PhantomData,
        }
    }
}

/// Decoder impl parses cbor objects from bytes
impl<Enc, Dec> Decoder for CborCodec<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    type Item = Dec;
    type Error = CborCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Build deserializer
        let mut de = serde_cbor::Deserializer::from_slice(buf);

        // Attempt deserialization
        let res: Result<Dec, _> = serde::de::Deserialize::deserialize(&mut de);

        // If we ran out before parsing, return none and try again later
        let res = match res {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.is_eof() => Ok(None),
            Err(e) => Err(e.into()),
        };

        // Update offset from iterator
        let offset = de.byte_offset();

        // Advance buffer
        buf.advance(offset);

        res
    }
}

/// Encoder impl encodes object streams to bytes
impl<Enc, Dec> Encoder for CborCodec<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    type Item = Enc;
    type Error = CborCodecError;

    fn encode(&mut self, data: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode cbor
        let j = serde_cbor::ser::to_vec_packed(&data)?;

        // Write to buffer
        buf.reserve(j.len());
        buf.put_slice(&j);

        Ok(())
    }
}
