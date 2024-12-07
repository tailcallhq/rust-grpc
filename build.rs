use std::path::PathBuf;

fn main() {
    fn configure_tonic() -> tonic_build::Builder {
        tonic_build::configure()
            .protoc_arg("--experimental_allow_proto3_optional")
            .build_server(true)
            .build_client(true)
    }

    // Compile news.proto
    let mut news = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    news.push("news.proto");

    // Configure and compile each proto file with descriptors
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Compile news.proto
    configure_tonic()
        .file_descriptor_set_path(out_dir.join("news_descriptor.bin"))
        .compile(&["news.proto"], &["."])
        .expect("Failed to compile news.proto");

    // Compile posts.proto
    configure_tonic()
        .file_descriptor_set_path(out_dir.join("posts_descriptor.bin"))
        .compile(&["posts.proto"], &["."])
        .expect("Failed to compile posts.proto");

    // Compile users.proto
    configure_tonic()
        .file_descriptor_set_path(out_dir.join("users_descriptor.bin"))
        .compile(&["users.proto"], &["."])
        .expect("Failed to compile users.proto");
}
