use std::path::PathBuf;

fn main() {
    let mut news = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    news.push("news.proto");
    tonic_build::compile_protos(news).expect("Failed to compile protos");
}
