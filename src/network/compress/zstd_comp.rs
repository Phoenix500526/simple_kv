use crate::{Compressor, KvError};
use bytes::{BufMut, BytesMut};
use std::io::{Read, Write};
use zstd::{Decoder, Encoder};
#[derive(Debug)]
pub struct Zstd {}

impl Compressor for Zstd {
    fn compress(src: &[u8], dst: &mut BytesMut) -> Result<usize, KvError> {
        let mut encoder = Encoder::new(dst.writer(), 1)?;
        encoder.write_all(src)?;
        encoder.finish()?;
        Ok(dst.len())
    }

    fn decompress(src: &BytesMut, dst: &mut Vec<u8>) -> Result<(), KvError> {
        let mut decoder = Decoder::new(&src[..])?;
        decoder.read_to_end(dst)?;
        Ok(())
    }
}
