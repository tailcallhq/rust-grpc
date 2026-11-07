use std::path::PathBuf;

// Generates the gRPC service bindings and binary file descriptor set required
// by the tonic reflection server from the canonical news.proto definition.
fn main() {
    let mut news = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    news.push("news.proto");
    tonic_prost_build::compile_protos(news).expect("Failed to compile protos");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile_protos(&["news.proto"], &["proto"])
        .unwrap();
}
