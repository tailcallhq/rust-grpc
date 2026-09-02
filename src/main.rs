use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap,
};
use once_cell::sync::Lazy;
use opentelemetry::{global, KeyValue};
#[allow(unused_imports)]
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{propagation::TraceContextPropagator, Resource};
use tonic::{metadata::MetadataMap, transport::Server as TonicServer, Response, Status};
use tonic_tracing_opentelemetry::middleware::server;
use tower::make::Shared;

use news::news_service_server::NewsService;
use news::news_service_server::NewsServiceServer;
use news::{MultipleNewsId, News, NewsId, NewsList};
use shuttle_runtime::Service;
use tracing_subscriber::layer::SubscriberExt;

pub mod news {
    tonic::include_proto!("news"); // The package name specified in your .proto
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("news_descriptor");
}

#[derive(Debug, Default)]
pub struct MyNewsService {
    news: Arc<Mutex<Vec<News>>>, // Using a simple vector to store news items in memory
}

impl MyNewsService {
    fn new() -> MyNewsService {
        let news = vec![
            News {
                id: 1,
                title: "Note 1".into(),
                body: "Content 1".into(),
                post_image: "Post image 1".into(),
                status: 0,
            },
            News {
                id: 2,
                title: "Note 2".into(),
                body: "Content 2".into(),
                post_image: "Post image 2".into(),
                status: 1,
            },
            News {
                id: 3,
                title: "Note 3".into(),
                body: "Content 3".into(),
                post_image: "Post image 3".into(),
                status: 1,
            },
            News {
                id: 4,
                title: "Note 4".into(),
                body: "Content 4".into(),
                post_image: "Post image 4".into(),
                status: 1,
            },
        ];
        MyNewsService {
            news: Arc::new(Mutex::new(news)),
        }
    }
}

#[tonic::async_trait]
impl NewsService for MyNewsService {
    async fn get_all_news(
        &self,
        _request: tonic::Request<()>,
    ) -> std::result::Result<Response<NewsList>, Status> {
        let lock = self.news.lock().unwrap();
        let news = NewsList {
            news: lock.clone(),
        };
        Ok(Response::new(news))
    }

    async fn get_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<Response<News>, Status> {
        let id = request.into_inner().id;
        let lock = self.news.lock().unwrap();
        if let Some(news) = lock.iter().find(|n| n.id == id) {
            return Ok(Response::new(news.clone()));
        }
        Err(Status::not_found("News not found"))
    }

    async fn get_multiple_news(
        &self,
        _request: tonic::Request<MultipleNewsId>,
    ) -> std::result::Result<Response<NewsList>, Status> {
        let lock = self.news.lock().unwrap();
        let news = NewsList {
            news: lock.clone(),
        };
        Ok(Response::new(news))
    }

    async fn delete_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<Response<()>, Status> {
        let id = request.into_inner().id;
        let mut lock = self.news.lock().unwrap();
        let initial_len = lock.len();
        lock.retain(|n| n.id != id);
        if lock.len() < initial_len {
            return Ok(Response::new(()));
        }
        Err(Status::not_found("News not found"))
    }

    async fn edit_news(
        &self,
        request: tonic::Request<News>,
    ) -> std::result::Result<Response<News>, Status> {
        let new_news = request.into_inner();
        let mut lock = self.news.lock().unwrap();
        if let Some(news) = lock.iter_mut().find(|n| n.id == new_news.id) {
            news.title = new_news.title.clone();
            news.body = new_news.body.clone();
            news.post_image = new_news.post_image.clone();
            return Ok(Response::new(new_news));
        }
        Err(Status::not_found("News not found"))
    }

    async fn add_news(
        &self,
        request: tonic::Request<News>,
    ) -> std::result::Result<Response<News>, Status> {
        let mut news = request.into_inner();
        let mut lock = self.news.lock().unwrap();
        let new_id = lock.iter().map(|n| n.id).max().unwrap_or(0) + 1; // Simple ID generation
        news.id = new_id;
        lock.push(news.clone());
        Ok(Response::new(news))
    }
}

static RESOURCE: Lazy<Resource> = Lazy::new(|| {
    Resource::builder()
        .with_attribute(KeyValue::new(opentelemetry_semantic_conventions::resource::SERVICE_NAME, "rust-grpc"))
        .with_attribute(KeyValue::new(opentelemetry_semantic_conventions::resource::SERVICE_VERSION, "test"))
        .build()
});

fn init_tracer() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    static TELEMETRY_URL: &str = "https://api.honeycomb.io:443";
    let headers = HeaderMap::from_iter([(
        HeaderName::from_static("x-honeycomb-team"),
        HeaderValue::from_str(&std::env::var("HONEYCOMB_API_KEY")?)?,
    )]);

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(TELEMETRY_URL)
        .with_metadata(MetadataMap::from_headers(headers.clone()))
        .build()?;

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(RESOURCE.clone())
        .build();

    use opentelemetry::trace::TracerProvider as _;
    let tracer = provider.tracer("tracing");
    let trace_layer = tracing_opentelemetry::layer()
        .with_location(false)
        .with_threads(false)
        .with_tracer(tracer);

    let subscriber = tracing_subscriber::registry().with(trace_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    global::set_tracer_provider(provider);

    Ok(())
}

#[shuttle_runtime::main]
async fn shuttle_main() -> Result<impl Service, shuttle_runtime::Error> {
    if std::env::var("HONEYCOMB_API_KEY").is_ok() {
        init_tracer()?;
    }

    let news_service = MyNewsService::new();

    Ok(news_service)
}

#[async_trait::async_trait]
impl Service for MyNewsService {
    async fn bind(mut self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(news::FILE_DESCRIPTOR_SET)
            .build_v1()
            .unwrap();

        println!("NewsService server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| shuttle_runtime::Error::Custom(anyhow::anyhow!(e)))?;
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        TonicServer::builder()
            .layer(server::OtelGrpcLayer::default())
            .add_service(NewsServiceServer::new(self))
            .add_service(service)
            .serve_with_incoming(incoming)
            .await
            .map_err(|e| shuttle_runtime::Error::Custom(anyhow::anyhow!(e)))?;

        Ok(())
    }
}
