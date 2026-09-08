use super::*;
use news::news_service_client::NewsServiceClient;
use tokio::sync::oneshot;
use tonic::{transport::Channel, Code};

struct TestServer {
    channel: Channel,
    shutdown: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
}

impl TestServer {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown, receiver) = oneshot::channel();
        let task = tokio::spawn(MyNewsService::new().serve_until(listener, async {
            let _ = receiver.await;
        }));
        let channel = Channel::from_shared(format!("http://{addr}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self {
            channel,
            shutdown: Some(shutdown),
            task,
        }
    }

    async fn stop(mut self) {
        self.shutdown.take().unwrap().send(()).unwrap();
        drop(self.channel);
        tokio::time::timeout(std::time::Duration::from_secs(5), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn grpc_crud_round_trip() {
    let server = TestServer::start().await;
    let mut client = NewsServiceClient::new(server.channel.clone());
    assert_eq!(
        client
            .get_all_news(())
            .await
            .unwrap()
            .into_inner()
            .news
            .len(),
        5
    );
    let created = client
        .add_news(News {
            id: 999,
            title: "New title".into(),
            body: "New body".into(),
            post_image: "image.png".into(),
            status: 1,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(created.id, 6);
    assert_eq!(
        client
            .get_news(NewsId { id: 6 })
            .await
            .unwrap()
            .into_inner(),
        created
    );
    let edited = News {
        title: "Edited".into(),
        body: "Changed".into(),
        post_image: "other.png".into(),
        ..created
    };
    assert_eq!(
        client.edit_news(edited.clone()).await.unwrap().into_inner(),
        edited
    );
    assert_eq!(
        client
            .get_news(NewsId { id: 6 })
            .await
            .unwrap()
            .into_inner(),
        edited
    );
    client.delete_news(NewsId { id: 6 }).await.unwrap();
    assert_eq!(
        client.get_news(NewsId { id: 6 }).await.unwrap_err().code(),
        Code::NotFound
    );
    drop(client);
    server.stop().await;
}

#[tokio::test]
async fn grpc_batch_and_missing_items_preserve_semantics() {
    let server = TestServer::start().await;
    let mut client = NewsServiceClient::new(server.channel.clone());
    let result = client
        .get_multiple_news(MultipleNewsId {
            ids: vec![
                NewsId { id: 3 },
                NewsId { id: 1 },
                NewsId { id: 3 },
                NewsId { id: 999 },
            ],
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        result.news.iter().map(|n| n.id).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert!(client
        .get_multiple_news(MultipleNewsId { ids: vec![] })
        .await
        .unwrap()
        .into_inner()
        .news
        .is_empty());
    assert_eq!(
        client
            .delete_news(NewsId { id: 999 })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .edit_news(News {
                id: 999,
                ..Default::default()
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    drop(client);
    server.stop().await;
}

#[tokio::test]
async fn grpc_reflection_supports_v1_and_v1alpha() {
    let server = TestServer::start().await;
    {
        use tonic_reflection::pb::v1::{
            server_reflection_client::ServerReflectionClient,
            server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
            ServerReflectionRequest,
        };
        let mut client = ServerReflectionClient::new(server.channel.clone());
        let mut response = client
            .server_reflection_info(tokio_stream::iter([ServerReflectionRequest {
                host: String::new(),
                message_request: Some(MessageRequest::ListServices(String::new())),
            }]))
            .await
            .unwrap()
            .into_inner();
        let message = response.message().await.unwrap().unwrap();
        let Some(MessageResponse::ListServicesResponse(services)) = message.message_response else {
            panic!("Expected service list")
        };
        assert!(services
            .service
            .iter()
            .any(|s| s.name == "news.NewsService"));
    }
    {
        use tonic_reflection::pb::v1alpha::{
            server_reflection_client::ServerReflectionClient,
            server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
            ServerReflectionRequest,
        };
        let mut client = ServerReflectionClient::new(server.channel.clone());
        let mut response = client
            .server_reflection_info(tokio_stream::iter([ServerReflectionRequest {
                host: String::new(),
                message_request: Some(MessageRequest::FileContainingSymbol(
                    "news.NewsService".into(),
                )),
            }]))
            .await
            .unwrap()
            .into_inner();
        let message = response.message().await.unwrap().unwrap();
        let Some(MessageResponse::FileDescriptorResponse(descriptor)) = message.message_response
        else {
            panic!("Expected descriptor")
        };
        assert!(!descriptor.file_descriptor_proto.is_empty());
    }
    server.stop().await;
}
