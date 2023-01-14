var sourcesIndex = JSON.parse('{\
"kvc":["",[],["client.rs"]],\
"kvc_cli":["",[],["client-cli.rs"]],\
"kvs":["",[],["server.rs"]],\
"simple_kv":["",[["network",[["compress",[],["gzip.rs","lz4_comp.rs","mod.rs","zstd_comp.rs"]],["multiplex",[],["mod.rs","yamux_mplex.rs"]]],["frame.rs","mod.rs","stream.rs","stream_result.rs","tls.rs"]],["pb",[],["abi.rs","mod.rs"]],["service",[],["command_service.rs","mod.rs","topic.rs","topic_service.rs"]],["storage",[],["memory.rs","mod.rs","sleddb.rs"]]],["config.rs","error.rs","lib.rs"]]\
}');
createSourceSidebar();
