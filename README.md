# simple_kv
[![codecov](https://codecov.io/gh/Phoenix500526/simple_kv/branch/main/graph/badge.svg?token=UHC5AS6QV8)](https://codecov.io/gh/Phoenix500526/simple_kv)
## 功能性需求
1. 根据不同的命令进行数据的存储、读取等。
2. 客户端能够通过网络访问 KV Server，发送包含命令的请求，并得到相应的结果。
3. 可以根据需要，灵活选择存储在内存中还是持久化到磁盘上

## 非功能性需求：
1. 网络层要灵活，优先考虑 TCP 协议，但是未来可能需要支持更多的协议，如 HTTP2/gRPC，未来还可能有安全性的要求，需要支持 TLS 安全协议
2. 处理流程中要加入一些 hook：收到客户端的命令后 OnRequestReceived、处理完客户端的命令后 OnRequestExecuted、发送响应之前 BeforeResponseSend、发送响应之后 AfterResponseSend。
3. 支持配置，容易部署，优先级低，排在最后


## 实现上的思考：
针对功能性需求：
1. 提供哪些命令：参考 Redis，提供 HGET、HGETALL、HMGET、HSET、HMSET、HDEL、HMDEL、HEXIST、HMEXIST
2. 命令跟请求怎么做抽象：用 protobuf 定义好协议，并生成相应的 rust 代码文件（一系列的 Command Object，同时还有 CommandRequest + CommandResponse），好处是灵活，可向后兼容，解析效率高，同时节省带宽
3. 命令和请求如何处理：抽象出 CommandService 这个 Trait，每个 CommandObject 获取到后，就进入 dispatch 流程，将相应的走各自实现的处理流程
4. 为了避免 CommandService Trait 和具体的存储后端耦合，抽象出一个 Storage  Trait，针对不同具体的存储后端实现不同 Storage Trait。重点关注的是 Storage 如何同外界打交道(行为)，基本要有的行为包括 get、set、del、get_all、contains、get_iter

需要用到并了解的 crate：
1. prost_build ： protobuf 编译


## Todo
- [x] Redis 还支持 PSUBSCRIBE，也就是说除了可以 subscribe “chat” 这样固定的 topic，还可以是 “chat.*”，一并订阅所有 “chat”、“chat.rust”、“chat.elixir” 。想想看，如果要支持 PSUBSCRIBE，你该怎么设计 Broadcaster 里的两张表？
- [ ] 除了 TLS，另外一个被广泛使用的处理应用层安全的协议是 noise protocol。Rust 下有 snow 这个很优秀的库处理 noise protocol。对于有余力的同学，你们可以看看它的文档，尝试着写段类似 tls.rs 的代码，让我们的 kvs / kvc 可以使用 noise protocol。
- [x] 在设计 frame 的时候，如果我们的压缩方法不止 gzip 一种，而是服务器或客户端都会根据各自的情况，在需要的时候做某种算法的压缩。假设服务器和客户端都支持 gzip、lz4 和 zstd 这三种压缩算法。那么 frame 该如何设计呢？需要用几个 bit 来存放压缩算法的信息？
- [x] 目前我们的 client 只适合测试，你可以将其修改成一个完整的命令行程序么？小提示，可以使用 clap 或 structopt，用户可以输入不同的命令；或者做一个交互式的命令行，使用 shellfish 或 rustyline，就像 redis-cli 那样。
- [ ] 试着使用 LengthDelimitedCodec 来重写 frame 这一层。
- [x] 如果希望通过如下配置记录日志，应该怎么做？
```bash
[log]
enable_log_file = true
enable_jaeger = false
log_level = 'info'
path = '/tmp/kv-log'
rotation = 'Daily'
```
