use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to locate vendored protoc");

    let mut prost_config = tonic_prost_build::Config::new();
    prost_config.protoc_executable(protoc);

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile_with_config(prost_config, &["news.proto"], &["."])
        .expect("failed to compile news.proto");
}
