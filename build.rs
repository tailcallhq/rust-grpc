use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("Failed to locate vendored protoc");
    std::env::set_var("PROTOC", protoc);

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile_protos(&["news.proto"], &["."])
        .expect("Failed to compile protos");
}
