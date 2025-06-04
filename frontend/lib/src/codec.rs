use bytes::buf::{Buf, BufMut};
use futures_codec::{BytesMut, Decoder, Encoder};
use rmp_serde::decode::Error as RmpDecError;
use rmp_serde::encode::Error as RmpEncError;
use serde::Deserialize;
use std::io::{Cursor, Error as IoError};
use tetris_core::net::Message;

pub struct MessagePackCodec;

#[allow(unused)]
pub enum CodecError {
    Io(IoError),
    RmpDec(RmpDecError),
    RmpEnc(RmpEncError),
}

impl From<RmpDecError> for CodecError {
    fn from(err: RmpDecError) -> Self {
        Self::RmpDec(err)
    }
}

impl From<RmpEncError> for CodecError {
    fn from(err: RmpEncError) -> Self {
        Self::RmpEnc(err)
    }
}
impl From<IoError> for CodecError {
    fn from(err: IoError) -> Self {
        Self::Io(err)
    }
}

impl Decoder for MessagePackCodec {
    type Item = Message;

    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut cursor = Cursor::new(src);

        let mut de = rmp_serde::Deserializer::new(&mut cursor);

        let res = match Deserialize::deserialize(&mut de) {
            Ok(val) => Ok(Some(val)),
            Err(e) => Err(e.into()),
        };

        let pos = cursor.position();
        let after = cursor.into_inner();

        after.advance(pos as usize);

        res
    }
}

impl Encoder for MessagePackCodec {
    type Item = Message;

    type Error = CodecError;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut futures_codec::BytesMut,
    ) -> Result<(), Self::Error> {
        let data = rmp_serde::to_vec(&item)?;

        dst.reserve(data.len());
        dst.put_slice(&data);

        Ok(())
    }
}
