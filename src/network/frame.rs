use crate::{CommandRequest, CommandResponse, KvError};
use bytes::{Buf, BufMut, BytesMut};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use prost::Message;
use std::io::{Read, Write};
use tracing::debug;

/// 长度整个占 4B
pub const LEN_LEN: usize = 4;

/// 最高 1 位 表示是否压缩，剩余 31 bit 表示长度，因此 frame 最大为 2G
const MAX_FRAME: usize = 2 * 1024 * 1024;
const COMPRESSION_BIT: usize = 1 << 31;

/// 如果 payload 超过了 1436，则进行压缩，这样可以避免分片
const COMPRESSION_LIMIT: usize = 1436;

pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// 把一个 Message encode 成一个 frame, Message 可以是 CommandRequest
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), KvError> {
        let size = self.encoded_len();
        if size >= MAX_FRAME {
            return Err(KvError::FrameError);
        }

        // step 1: 将长度信息写入头部
        buf.put_u32(size as _);
        // step 2: 判断是否需要进行压缩
        if size > COMPRESSION_LIMIT {
            // step 2.1: 现将数据 encode 到 buf_tmp 中暂存起来，等待压缩
            let mut buf_tmp = Vec::with_capacity(size);
            self.encode(&mut buf_tmp)?;

            // step 2.2: 跳过开头的长度信息
            let payload = buf.split_off(LEN_LEN);
            buf.clear();

            // step 2.3: 进行压缩
            let mut encoder = GzEncoder::new(payload.writer(), Compression::default());
            encoder.write_all(&buf_tmp[..])?;

            // step 2.4: 压缩完成狗，将 gzip encoder 中的 BytesMut 拿回来
            let payload = encoder.finish()?.into_inner();
            debug!("Encode a frame: size {}({})", size, payload.len());

            // step 2.5: 写入压缩后的长度
            buf.put_u32((payload.len() | COMPRESSION_BIT) as _);

            // step 2.6: 把 BytesMut 合并回来
            buf.unsplit(payload);
            Ok(())
        } else {
            self.encode(buf)?;
            Ok(())
        }
    }

    /// 把一个完整的 frame decode 成一个 Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, KvError> {
        // step 1: 读取 header，并取出相关的值
        let header = buf.get_u32() as usize;
        let (len, compressed) = decode_header(header);
        debug!("Got a frame: msg len {}, compressed {}", len, compressed);
        if compressed {
            // step 2：解压
            let mut decoder = GzDecoder::new(&buf[..len]);
            let mut buf_tmp = Vec::with_capacity(len * 2);
            decoder.read_to_end(&mut buf_tmp)?;
            buf.advance(len);

            // step 3: decode 成相应的消息
            Ok(Self::decode(&buf_tmp[..buf_tmp.len()])?)
        } else {
            let msg = Self::decode(&buf[..len])?;
            buf.advance(len);
            Ok(msg)
        }
    }
}

impl FrameCoder for CommandRequest {}
impl FrameCoder for CommandResponse {}

fn decode_header(header: usize) -> (usize, bool) {
    let len = header & !COMPRESSION_BIT;
    let compressed = header & COMPRESSION_BIT == COMPRESSION_BIT;
    (len, compressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use bytes::Bytes;

    #[test]
    fn command_request_encode_decode_should_work() {
        let mut buf = BytesMut::new();
        let cmd = CommandRequest::new_hdel("t1", "k1");
        cmd.encode_frame(&mut buf).unwrap();

        assert!(!is_compressed(&buf));

        let cmd1 = CommandRequest::decode_frame(&mut buf).unwrap();
        assert_eq!(cmd1, cmd);
    }

    #[test]
    fn command_response_encode_decode_should_work() {
        let mut buf = BytesMut::new();
        let values: Vec<Value> = vec![1.into(), "hello".into(), "world".into()];

        let res: CommandResponse = values.into();
        res.encode_frame(&mut buf).unwrap();

        assert!(!is_compressed(&buf));

        let res1 = CommandResponse::decode_frame(&mut buf).unwrap();
        assert_eq!(res, res1);
    }

    #[test]
    fn command_response_compressed_encode_decode_should_work() {
        let mut buf = BytesMut::new();
        let value: Value = Bytes::from(vec![0u8; COMPRESSION_LIMIT + 1]).into();
        let res: CommandResponse = value.into();
        res.encode_frame(&mut buf).unwrap();

        assert!(is_compressed(&buf));

        let res1 = CommandResponse::decode_frame(&mut buf).unwrap();
        assert_eq!(res, res1);
    }

    fn is_compressed(buf: &BytesMut) -> bool {
        if let &[v] = &buf[..1] {
            v >> 7 == 1
        } else {
            false
        }
    }
}
