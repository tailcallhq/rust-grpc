use std::fs;
use std::path::PathBuf;
use protox::prost::Message;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let descriptor_path = out_dir.join("news_descriptor.bin");

    // Pure-Rust protobuf compilation via protox: eliminates dependency on external protoc binary
    let file_descriptors = protox::compile(["news.proto"], ["."])
        .expect("Failed to compile protos via protox");

    let mut buf = Vec::new();
    file_descriptors.encode(&mut buf).expect("Failed to encode file descriptor set");
    fs::write(&descriptor_path, buf).expect("Failed to write news_descriptor.bin");

    tonic_build::configure()
        .file_descriptor_set_path(&descriptor_path)
        .skip_protoc_run()
        .compile(&["news.proto"], &["."])
        .expect("Failed to compile gRPC service via tonic_build");
}
