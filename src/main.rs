use std::sync::{Arc, LazyLock, Mutex};

use anyhow::Result;
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use tonic::{metadata::MetadataMap, transport::Server as TonicServer, Response, Status};
use tonic_tracing_opentelemetry::middleware::server;

use news::news_service_server::NewsService;
use news::news_service_server::NewsServiceServer;
use news::{MultipleNewsId, News, NewsId, NewsList};
use shuttle_runtime::Service;
use tracing_subscriber::layer::SubscriberExt;

pub mod news {
    tonic::include_proto!("news");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("news_descriptor");
}

#[derive(Debug, Default)]
pub struct MyNewsService {
    news: Arc<Mutex<Vec<News>>>,
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
        Ok(Response::new(NewsList { news: lock.clone() }))
    }

    async fn get_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<Response<News>, Status> {
        let id = request.into_inner().id;
        let lock = self.news.lock().unwrap();
        lock.iter()
            .find(|news| news.id == id)
            .cloned()
            .map(Response::new)
            .ok_or_else(|| Status::not_found("News not found"))
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
        let news = lock
            .iter()
            .filter(|item| ids.contains(&item.id))
            .cloned()
            .collect();
        Ok(Response::new(NewsList { news }))
    }

    async fn delete_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<Response<()>, Status> {
        let id = request.into_inner().id;
        let mut lock = self.news.lock().unwrap();
        let len_before = lock.len();
        lock.retain(|news| news.id != id);
        if lock.len() == len_before {
            Err(Status::not_found("News not found"))
        } else {
            Ok(Response::new(()))
        }
    }

    async fn edit_news(
        &self,
        request: tonic::Request<News>,
    ) -> std::result::Result<Response<News>, Status> {
        let replacement = request.into_inner();
        let mut lock = self.news.lock().unwrap();
        let stored = lock
            .iter_mut()
            .find(|news| news.id == replacement.id)
            .ok_or_else(|| Status::not_found("News not found"))?;

        stored.title = replacement.title.clone();
        stored.body = replacement.body.clone();
        stored.post_image = replacement.post_image.clone();
        stored.status = replacement.status;
        Ok(Response::new(replacement))
    }

    async fn add_news(
        &self,
        request: tonic::Request<News>,
    ) -> std::result::Result<Response<News>, Status> {
        let mut news = request.into_inner();
        let mut lock = self.news.lock().unwrap();
        news.id = lock.iter().map(|item| item.id).max().unwrap_or(0) + 1;
        lock.push(news.clone());
        Ok(Response::new(news))
    }
}

static RESOURCE: LazyLock<Resource> = LazyLock::new(|| {
    Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", "rust-grpc"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])
        .build()
});

fn init_tracer() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let mut metadata = MetadataMap::new();
    metadata.insert(
        "x-honeycomb-team",
        std::env::var("HONEYCOMB_API_KEY")?.parse()?,
    );

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("https://api.honeycomb.io:443")
        .with_metadata(metadata)
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(RESOURCE.clone())
        .build();

    let tracer = provider.tracer("rust-grpc");
    let trace_layer = tracing_opentelemetry::layer()
        .with_location(false)
        .with_threads(false)
        .with_tracer(tracer);
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(trace_layer))?;
    global::set_tracer_provider(provider);

    Ok(())
}

#[shuttle_runtime::main]
async fn shuttle_main() -> Result<impl Service, shuttle_runtime::Error> {
    if std::env::var("HONEYCOMB_API_KEY").is_ok() {
        init_tracer()?;
    }

    Ok(MyNewsService::new())
}

#[shuttle_runtime::async_trait]
impl Service for MyNewsService {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let reflection = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(news::FILE_DESCRIPTOR_SET)
            .build_v1()
            .map_err(|error| shuttle_runtime::Error::Custom(anyhow::anyhow!(error)))?;

        println!("NewsService server listening on {addr}");

        TonicServer::builder()
            .layer(server::OtelGrpcLayer::default())
            .add_service(NewsServiceServer::new(self))
            .add_service(reflection)
            .serve(addr)
            .await
            .map_err(|error| shuttle_runtime::Error::Custom(anyhow::anyhow!(error)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::{Code, Request};

    fn news(title: &str) -> News {
        News {
            id: 0,
            title: title.into(),
            body: format!("{title} body"),
            post_image: format!("{title} image"),
            status: 1,
        }
    }

    #[tokio::test]
    async fn crud_and_batch_lookup_survive_the_dependency_migration() {
        let service = MyNewsService::new();

        let initial = service
            .get_all_news(Request::new(()))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(initial.news.len(), 5);

        let selected = service
            .get_multiple_news(Request::new(MultipleNewsId {
                ids: vec![NewsId { id: 1 }, NewsId { id: 3 }, NewsId { id: 999 }],
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            selected.news.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1, 3]
        );

        let created = service
            .add_news(Request::new(news("Created")))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(created.id, 6);

        let updated = service
            .edit_news(Request::new(News {
                title: "Updated".into(),
                body: "Updated body".into(),
                post_image: "Updated image".into(),
                status: 2,
                ..created
            }))
            .await
            .unwrap()
            .into_inner();

        let fetched = service
            .get_news(Request::new(NewsId { id: updated.id }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(fetched.title, "Updated");
        assert_eq!(fetched.body, "Updated body");
        assert_eq!(fetched.post_image, "Updated image");
        assert_eq!(fetched.status, 2);

        service
            .delete_news(Request::new(NewsId { id: updated.id }))
            .await
            .unwrap();
        let deleted = service
            .get_news(Request::new(NewsId { id: updated.id }))
            .await
            .unwrap_err();
        assert_eq!(deleted.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn missing_records_keep_not_found_semantics() {
        let service = MyNewsService::new();
        let id = i32::MAX;

        let get = service
            .get_news(Request::new(NewsId { id }))
            .await
            .unwrap_err();
        assert_eq!(get.code(), Code::NotFound);

        let edit = service
            .edit_news(Request::new(News {
                id,
                ..news("Missing")
            }))
            .await
            .unwrap_err();
        assert_eq!(edit.code(), Code::NotFound);

        let delete = service
            .delete_news(Request::new(NewsId { id }))
            .await
            .unwrap_err();
        assert_eq!(delete.code(), Code::NotFound);
    }
}
