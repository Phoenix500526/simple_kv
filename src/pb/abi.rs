/// 来自客户端的命令请求
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct CommandRequest {
    #[prost(
        oneof = "command_request::RequestData",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14"
    )]
    pub request_data: ::core::option::Option<command_request::RequestData>,
}
/// Nested message and enum types in `CommandRequest`.
pub mod command_request {
    #[derive(PartialOrd, Clone, PartialEq, ::prost::Oneof)]
    pub enum RequestData {
        #[prost(message, tag = "1")]
        Hget(super::Hget),
        #[prost(message, tag = "2")]
        Hmget(super::Hmget),
        #[prost(message, tag = "3")]
        Hgetall(super::Hgetall),
        #[prost(message, tag = "4")]
        Hset(super::Hset),
        #[prost(message, tag = "5")]
        Hmset(super::Hmset),
        #[prost(message, tag = "6")]
        Hdel(super::Hdel),
        #[prost(message, tag = "7")]
        Hmdel(super::Hmdel),
        #[prost(message, tag = "8")]
        Hexist(super::Hexist),
        #[prost(message, tag = "9")]
        Hmexist(super::Hmexist),
        #[prost(message, tag = "10")]
        Subscribe(super::Subscribe),
        #[prost(message, tag = "11")]
        Unsubscribe(super::Unsubscribe),
        #[prost(message, tag = "12")]
        Publish(super::Publish),
        #[prost(message, tag = "13")]
        Psubscribe(super::PSubscribe),
        #[prost(message, tag = "14")]
        Punsubscribe(super::PUnsubscribe),
    }
}
/// 服务端的命令响应
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct CommandResponse {
    /// 状态码
    #[prost(uint32, tag = "1")]
    pub status: u32,
    /// 如果状态非 2XX，则需要在 message 中放入具体的错误信息
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    /// 成功返回 values
    #[prost(message, repeated, tag = "3")]
    pub values: ::prost::alloc::vec::Vec<Value>,
    /// 成功返回 Kv pairs，针对 getall 等命令
    #[prost(message, repeated, tag = "4")]
    pub kvpairs: ::prost::alloc::vec::Vec<Kvpair>,
}
/// get 相关命令
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hget {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub key: ::prost::alloc::string::String,
}
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hmget {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "2")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hgetall {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
}
/// set 相关命令
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct Hset {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub pair: ::core::option::Option<Kvpair>,
}
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct Hmset {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub pairs: ::prost::alloc::vec::Vec<Kvpair>,
}
/// del 相关命令
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hdel {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub key: ::prost::alloc::string::String,
}
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hmdel {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "2")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// 存在性判断
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hexist {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub key: ::prost::alloc::string::String,
}
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Hmexist {
    #[prost(string, tag = "1")]
    pub table: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "2")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// 订阅某个主题
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Subscribe {
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
}
/// 退订某个主题
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct Unsubscribe {
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub id: u32,
}
/// 订阅某个模式
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct PSubscribe {
    #[prost(string, tag = "1")]
    pub pattern: ::prost::alloc::string::String,
}
/// 退订某个模式
#[derive(PartialOrd, Eq, Clone, PartialEq, ::prost::Message)]
pub struct PUnsubscribe {
    #[prost(string, tag = "1")]
    pub pattern: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub id: u32,
}
/// 发布数据到某个主题
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct Publish {
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "2")]
    pub value: ::prost::alloc::vec::Vec<Value>,
}
/// 返回键值
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct Value {
    #[prost(oneof = "value::Value", tags = "1, 2, 3, 4, 5")]
    pub value: ::core::option::Option<value::Value>,
}
/// Nested message and enum types in `Value`.
pub mod value {
    #[derive(PartialOrd, Clone, PartialEq, ::prost::Oneof)]
    pub enum Value {
        #[prost(string, tag = "1")]
        String(::prost::alloc::string::String),
        #[prost(bytes, tag = "2")]
        Binary(::prost::bytes::Bytes),
        #[prost(int64, tag = "3")]
        Integer(i64),
        #[prost(double, tag = "4")]
        Float(f64),
        #[prost(bool, tag = "5")]
        Bool(bool),
    }
}
/// 返回键值对
#[derive(PartialOrd, Clone, PartialEq, ::prost::Message)]
pub struct Kvpair {
    #[prost(string, tag = "1")]
    pub key: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub value: ::core::option::Option<Value>,
}
