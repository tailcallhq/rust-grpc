use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=news.proto");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo"));

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile_protos(&["news.proto"], &["."])
        .expect("Failed to compile news.proto");
}
