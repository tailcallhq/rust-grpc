use std::path::PathBuf;

fn main() {
    let mut news = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    news.push("news.proto");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", news.display());
    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile_protos(&[news], &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
        .expect("Failed to compile protos");
}
