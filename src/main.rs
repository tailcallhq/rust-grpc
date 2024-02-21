mod server;

use std::fs::File;
use std::io::Read;
use tonic::transport::Server as TonicServer;

use crate::server::Builder;
use anyhow::Result;
use prost_types::FileDescriptorSet;
use tower::make::Shared;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = ([127, 0, 0, 1], 50051).into();

    let mut file = File::open("news.proto")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let news = protox_parse::parse("news.proto", &content)?;
    let mut news_descriptor_set = FileDescriptorSet::default();
    news_descriptor_set.file.push(news);

    let service = Builder::configure()
        .register_file_descriptor_set(news_descriptor_set)
        .build()?;

    println!("NewsService server listening on {}", addr);

    let tonic_service = TonicServer::builder()
        .add_service(service)
        .into_service();
    let make_svc = Shared::new(tonic_service);
    println!("Server listening on http://{}", addr);
    let server = hyper::Server::bind(&addr).serve(make_svc);
    server.await?;

    Ok(())
}
