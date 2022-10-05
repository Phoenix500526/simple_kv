use crate::{compress, decompress, CommandRequest, CommandResponse, KvError, GZIP};
use bytes::{Buf, BufMut, BytesMut};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::debug;

/// 长度整个占 4B
pub const LEN_LEN: usize = 4;

/// 最高 2 位 表示是否压缩，剩余 30 bit 表示长度，因此 frame 最大为 1G
const MAX_FRAME: usize = 1024 * 1024;
const COMPRESSION_BIT: usize = 30;
const COMPRESSION_MASK: usize = 3 << 30;

/// 如果 payload 超过了 1436，则进行压缩，这样可以避免分片
const COMPRESSION_LIMIT: usize = 1436;

pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    fn encode_frame_with_compressor(
        &self,
        buf: &mut BytesMut,
        compressor_type: usize,
    ) -> Result<(), KvError> {
        let size = self.encoded_len();
        if size > MAX_FRAME {
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
            let mut payload = buf.split_off(LEN_LEN);
            buf.clear();

            // step 2.3: 进行压缩
            let len = compress(compressor_type, &buf_tmp, &mut payload)?;
            debug!("Encode a frame size: {}({})", size, len);
            // step 2.5: 写入压缩后的长度
            buf.put_u32((len | (GZIP << COMPRESSION_BIT)) as _);

            // step 2.6: 把 BytesMut 合并回来
            buf.unsplit(payload);
            Ok(())
        } else {
            self.encode(buf)?;
            Ok(())
        }
    }

    /// 把一个 Message encode 成一个 frame, Message 可以是 CommandRequest
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), KvError> {
        self.encode_frame_with_compressor(buf, GZIP)
    }

    /// 把一个完整的 frame decode 成一个 Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, KvError> {
        // step 1: 读取 header，并取出相关的值(注意：get_u32 后，position 会向前步进 4)
        let header = buf.get_u32() as usize;
        let (len, compressed) = decode_header(header);
        debug!("Got a frame: msg len {}, compressed {}", len, compressed);

        let mut buf_tmp = Vec::with_capacity(len * 2);
        let src = buf.split_to(len);
        decompress(compressed, &src, &mut buf_tmp)?;

        Ok(Self::decode(&buf_tmp[..buf_tmp.len()])?)
    }
}

impl FrameCoder for CommandRequest {}
impl FrameCoder for CommandResponse {}

fn decode_header(header: usize) -> (usize, usize) {
    let len = header & !COMPRESSION_MASK;
    let compressed = (header & COMPRESSION_MASK) >> COMPRESSION_BIT;
    (len, compressed)
}

/// 从 stream 中读出一个完整的 frame
pub async fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<(), KvError>
where
    S: AsyncRead + Unpin + Send,
{
    let header = stream.read_u32().await? as usize;
    let (len, _compressed) = decode_header(header);

    // 先分配至少一个 frame 的内存
    buf.reserve(LEN_LEN + len);
    buf.put_u32(header as _);
    unsafe { buf.advance_mut(len) };
    stream.read_exact(&mut buf[LEN_LEN..]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use crate::{utils::DummyStream, CommandRequest};
    use bytes::Bytes;

    #[tokio::test]
    async fn read_frame_should_work() {
        let mut buf = BytesMut::new();
        let cmd = CommandRequest::new_hdel("t1", "k1");
        cmd.encode_frame(&mut buf).unwrap();

        let mut stream = DummyStream { buf };
        let mut data = BytesMut::new();
        read_frame(&mut stream, &mut data).await.unwrap();
        let cmd1 = CommandRequest::decode_frame(&mut data).unwrap();
        assert_eq!(cmd, cmd1);
    }

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
            (v >> 6) != 0
        } else {
            true
        }
    }
}
