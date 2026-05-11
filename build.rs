use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut news = manifest_dir.clone();
    news.push("news.proto");

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("Failed to locate vendored protoc");
    std::env::set_var("PROTOC", protoc);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile(&[news], &[manifest_dir])
        .expect("Failed to compile protos");
}
