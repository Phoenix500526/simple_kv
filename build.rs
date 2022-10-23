use std::process::Command;
fn main() {
    let mut config = prost_build::Config::new();
    // 为 protobuf 的 bytes 类型生成 Bytes 而非缺省的 Vec<u8>。
    // . 表示通配符，即为所有的 bytes 都成成 bytes::Bytes。这样效率更高，解析的时候就需要使用 bytes 了
    config.bytes(["."]);
    // 为 protobuf 生成的协议添加偏序 Trait
    config.type_attribute(".", "#[derive(PartialOrd)]");
    for item in [
        "Hgetall",
        "Hget",
        "Hmget",
        "Hdel",
        "Hmdel",
        "Hexist",
        "Hmexist",
        "Subscribe",
        "Unsubscribe",
        "PSubscribe",
        "PUnsubscribe",
    ] {
        config.type_attribute(item, "#[derive(Eq)]");
    }
    config
        .out_dir("src/pb")
        .compile_protos(&["abi.proto"], &["."])
        .unwrap();

    Command::new("cargo")
        .args(["fmt", "--", "src/*.rs"])
        .status()
        .expect("cargo fmt failed");
}
