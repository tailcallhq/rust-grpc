fn main() {
    tonic_prost_build::configure()
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"))
                .join("news_descriptor.bin"),
        )
        .compile_protos(&["news.proto"], &["."])
        .expect("failed to compile news.proto");
}
