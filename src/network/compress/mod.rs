mod gzip;
mod lz4_comp;
mod zstd_comp;
use crate::KvError;
use bytes::BytesMut;
pub use gzip::Gzip;
pub use lz4_comp::Lz4;
pub use zstd_comp::Zstd;

pub const GZIP: usize = 1;
pub const LZ4: usize = 2;
pub const ZSTD: usize = 3;

pub trait Compressor {
    fn compress(src: &[u8], dst: &mut BytesMut) -> Result<usize, KvError>;
    fn decompress(src: &BytesMut, dst: &mut Vec<u8>) -> Result<(), KvError>;
}

pub fn compress(comp: usize, src: &Vec<u8>, dst: &mut BytesMut) -> Result<usize, KvError> {
    match comp {
        GZIP => Gzip::compress(src, dst),
        LZ4 => Lz4::compress(src, dst),
        ZSTD => Zstd::compress(src, dst),
        _ => {
            dst.extend_from_slice(&src[..]);
            Ok(src.len())
        }
    }
}

pub fn decompress(comp: usize, src: &BytesMut, dst: &mut Vec<u8>) -> Result<(), KvError> {
    match comp {
        GZIP => Gzip::decompress(src, dst),
        LZ4 => Lz4::decompress(src, dst),
        ZSTD => Zstd::decompress(src, dst),
        _ => {
            dst.extend_from_slice(&src[..]);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn compress_and_decompress_via_lz4_should_work() {
        let ctx = b"Hello World";
        let mut buf = BytesMut::with_capacity(32);
        Lz4::compress(ctx, &mut buf).unwrap();
        let mut res = Vec::with_capacity(ctx.len());
        Lz4::decompress(&buf, &mut res).unwrap();
        assert_eq!(res, ctx[..]);
    }

    #[test]
    fn compress_and_decompress_via_gzip_should_work() {
        let ctx = b"Hello World";
        let mut buf = BytesMut::with_capacity(32);
        Gzip::compress(ctx, &mut buf).unwrap();
        let mut res = Vec::with_capacity(ctx.len());
        Gzip::decompress(&buf, &mut res).unwrap();
        assert_eq!(res, ctx[..]);
    }

    #[test]
    fn compress_and_decompress_via_zstd_should_work() {
        let ctx = b"Hello World";
        let mut buf = BytesMut::with_capacity(32);
        Zstd::compress(ctx, &mut buf).unwrap();
        let mut res = Vec::with_capacity(ctx.len());
        Zstd::decompress(&buf, &mut res).unwrap();
        assert_eq!(res, ctx[..]);
    }

    #[test]
    fn compress_and_decompress_should_work() {
        let src: Vec<u8> = "Hello World".into();
        let mut buf_1 = BytesMut::with_capacity(32);
        let mut buf_2 = BytesMut::with_capacity(32);
        let mut buf_3 = BytesMut::with_capacity(32);
        let mut buf_0 = BytesMut::with_capacity(32);
        let mut dst_1 = Vec::with_capacity(src.len());
        let mut dst_2 = Vec::with_capacity(src.len());
        let mut dst_3 = Vec::with_capacity(src.len());
        let mut dst_0 = Vec::with_capacity(src.len());
        let len1 = compress(1, &src, &mut buf_1).unwrap();
        decompress(1, &buf_1, &mut dst_1).unwrap();
        let len2 = compress(2, &src, &mut buf_2).unwrap();
        decompress(2, &buf_2, &mut dst_2).unwrap();
        assert_ne!(len1, len2);
        assert_ne!(buf_1, buf_2);
        assert_eq!(dst_1, dst_2);

        let len3 = compress(3, &src, &mut buf_3).unwrap();
        decompress(3, &buf_3, &mut dst_3).unwrap();
        let len0 = compress(0, &src, &mut buf_0).unwrap();
        decompress(0, &buf_0, &mut dst_0).unwrap();
        assert_ne!(len0, len3);
        assert_ne!(buf_3, buf_0);
        assert_eq!(dst_3, dst_0);
        assert_eq!(src, dst_0);
        assert_eq!(len0, src.len());
    }
}
