# simple_kv
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
