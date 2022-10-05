use crate::{Compressor, KvError};
use bytes::{BufMut, BytesMut};
use lz4::{Decoder, EncoderBuilder};
use std::io::{Read, Write};
#[derive(Debug)]
pub struct Lz4 {}

impl Compressor for Lz4 {
    fn compress(src: &[u8], dst: &mut bytes::BytesMut) -> Result<usize, KvError> {
        let mut encoder = EncoderBuilder::new().level(4).build(dst.writer())?;
        encoder.write_all(src)?;
        let (_output, result) = encoder.finish();
        result.map_or_else(|e| Err(e.into()), |_| Ok(dst.len()))
    }

    fn decompress(src: &BytesMut, dst: &mut Vec<u8>) -> Result<(), KvError> {
        let mut decoder = Decoder::new(&src[..])?;
        decoder.read_to_end(dst)?;
        Ok(())
    }
}
