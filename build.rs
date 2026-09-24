use std::path::PathBuf;

fn main() {
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("Failed to locate vendored protoc"),
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR should be set by Cargo"));

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile_protos(&["news.proto"], &["."])
        .expect("Failed to compile protos");

    println!("cargo:rerun-if-changed=news.proto");
}
