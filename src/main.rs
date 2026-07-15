use std::sync::{Arc, Mutex};

use anyhow::Result;
use once_cell::sync::Lazy;
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use tonic::{
    metadata::{MetadataMap, MetadataValue},
    transport::Server as TonicServer,
    Response, Status,
};
use tonic_tracing_opentelemetry::middleware::server;

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
            News {
                id: 5,
                title: "Note 5".into(),
                body: "Content 5".into(),
                post_image: "Post image 5".into(),
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
        let reply = NewsList { news: lock.clone() };
        Ok(Response::new(reply))
    }

    async fn get_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<Response<News>, Status> {
        let id = request.into_inner().id;
        let lock = self.news.lock().unwrap();
        let item = lock.iter().find(|&n| n.id == id).cloned();
        match item {
            Some(news) => Ok(Response::new(news)),
            None => Err(Status::not_found("News not found")),
        }
    }

    async fn get_multiple_news(
        &self,
        request: tonic::Request<MultipleNewsId>,
    ) -> std::result::Result<Response<NewsList>, Status> {
        let ids = request
            .into_inner()
            .ids
            .into_iter()
            .map(|id| id.id)
            .collect::<Vec<_>>();
        let lock = self.news.lock().unwrap();
        let news_items: Vec<News> = lock
            .iter()
            .filter(|n| ids.contains(&n.id))
            .cloned()
            .collect();
        Ok(Response::new(NewsList { news: news_items }))
    }

    async fn delete_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<Response<()>, Status> {
        let id = request.into_inner().id;
        let mut lock = self.news.lock().unwrap();
        let len_before = lock.len();
        lock.retain(|news| news.id != id);
        let len_after = lock.len();

        if len_before == len_after {
            Err(Status::not_found("News not found"))
        } else {
            let x = Response::new(());
            Ok(x)
        }
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
        .with_attributes([
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                "rust-grpc",
            ),
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                env!("CARGO_PKG_VERSION"),
            ),
        ])
        .build()
});

fn init_tracer() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    static TELEMETRY_URL: &str = "https://api.honeycomb.io:443";
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "x-honeycomb-team",
        MetadataValue::try_from(std::env::var("HONEYCOMB_API_KEY")?)?,
    );

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(TELEMETRY_URL)
        .with_metadata(metadata)
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(RESOURCE.clone())
        .build();

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

        TonicServer::builder()
            .layer(server::OtelGrpcLayer::default())
            .add_service(NewsServiceServer::new(self))
            .add_service(service)
            .serve(addr)
            .await
            .map_err(|e| shuttle_runtime::Error::Custom(anyhow::anyhow!(e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::{Code, Request};

    fn news(id: i32, title: &str) -> News {
        News {
            id,
            title: title.into(),
            body: format!("{title} body"),
            post_image: format!("{title} image"),
            status: 0,
        }
    }

    #[tokio::test]
    async fn preserves_crud_behavior_after_dependency_migration() {
        let service = MyNewsService::new();

        let initial = service
            .get_all_news(Request::new(()))
            .await
            .expect("initial list should be available")
            .into_inner();
        assert_eq!(initial.news.len(), 5);

        let selected = service
            .get_multiple_news(Request::new(MultipleNewsId {
                ids: vec![NewsId { id: 1 }, NewsId { id: 3 }, NewsId { id: 999 }],
            }))
            .await
            .expect("batch lookup should succeed")
            .into_inner();
        assert_eq!(
            selected.news.iter().map(|item| item.id).collect::<Vec<_>>(),
            [1, 3]
        );

        let created = service
            .add_news(Request::new(news(99, "Created")))
            .await
            .expect("create should succeed")
            .into_inner();
        assert_eq!(created.id, 6);

        let updated = service
            .edit_news(Request::new(News {
                title: "Updated".into(),
                ..created
            }))
            .await
            .expect("edit should succeed")
            .into_inner();
        assert_eq!(updated.title, "Updated");

        service
            .delete_news(Request::new(NewsId { id: updated.id }))
            .await
            .expect("delete should succeed");
        let deleted = service
            .get_news(Request::new(NewsId { id: updated.id }))
            .await
            .expect_err("deleted item should no longer be returned");
        assert_eq!(deleted.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn returns_not_found_for_unknown_news() {
        let service = MyNewsService::new();

        let error = service
            .get_news(Request::new(NewsId { id: i32::MAX }))
            .await
            .expect_err("unknown news should return a gRPC not-found status");

        assert_eq!(error.code(), Code::NotFound);
    }
}
