use std::path::PathBuf;

fn main() {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("grpc_descriptor.bin"))
        .compile(
            &["proto/news.proto", "proto/posts.proto", "proto/users.proto"],
            &["proto"],
        )
        .unwrap();
}
