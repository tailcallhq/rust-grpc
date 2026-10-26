use super::*;
use news::news_service_client::NewsServiceClient;
use std::time::Duration;
use tokio::sync::oneshot;
use tonic::transport::Channel;

async fn start_server() -> (
    Channel,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(MyNewsService::new().serve(listener, async move {
        let _ = stopped.await;
    }));
    let channel = Channel::from_shared(format!("http://{address}"))
        .unwrap()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(2))
        .connect()
        .await
        .unwrap();
    (channel, stop, task)
}

async fn stop_server(stop: oneshot::Sender<()>, task: tokio::task::JoinHandle<Result<()>>) {
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .expect("server did not shut down")
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn news_crud_survives_transport_upgrade() {
    let (channel, stop, task) = start_server().await;
    let mut client = NewsServiceClient::new(channel);
    let initial = client.get_all_news(()).await.unwrap().into_inner().news;
    assert_eq!(initial.len(), 5);
    assert_eq!(initial[0].title, "Note 1");

    let added = client
        .add_news(News {
            id: 999,
            title: "Added".into(),
            body: "Body".into(),
            post_image: "Image".into(),
            status: 1,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(added.id, 6);
    assert_eq!(
        client
            .get_news(NewsId { id: 6 })
            .await
            .unwrap()
            .into_inner(),
        added
    );

    let edited = News {
        title: "Edited".into(),
        body: "Edited body".into(),
        post_image: "Edited image".into(),
        ..added
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
        client.get_all_news(()).await.unwrap().into_inner().news,
        initial
    );
    assert_eq!(
        client.get_news(NewsId { id: 6 }).await.unwrap_err().code(),
        tonic::Code::NotFound
    );
    assert_eq!(
        client
            .delete_news(NewsId { id: 6 })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::NotFound
    );
    assert_eq!(
        client.edit_news(edited).await.unwrap_err().code(),
        tonic::Code::NotFound
    );
    drop(client);
    stop_server(stop, task).await;
}

#[tokio::test]
async fn batch_lookup_keeps_storage_order_and_omits_missing_or_duplicate_ids() {
    let (channel, stop, task) = start_server().await;
    let mut client = NewsServiceClient::new(channel);
    let request = MultipleNewsId {
        ids: [5, 1, 1, 999, 3]
            .into_iter()
            .map(|id| NewsId { id })
            .collect(),
    };
    let rows = client
        .get_multiple_news(request)
        .await
        .unwrap()
        .into_inner()
        .news;
    assert_eq!(
        rows.iter().map(|item| item.id).collect::<Vec<_>>(),
        [1, 3, 5]
    );
    assert!(client
        .get_multiple_news(MultipleNewsId { ids: vec![] })
        .await
        .unwrap()
        .into_inner()
        .news
        .is_empty());
    drop(client);
    stop_server(stop, task).await;
}

#[tokio::test]
async fn reflection_still_lists_news_and_returns_its_descriptor() {
    use tonic_reflection::pb::v1alpha::{
        server_reflection_client::ServerReflectionClient,
        server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
        ServerReflectionRequest,
    };

    let (channel, stop, task) = start_server().await;
    let mut client = ServerReflectionClient::new(channel);
    let requests = [
        MessageRequest::ListServices(String::new()),
        MessageRequest::FileContainingSymbol("news.NewsService".into()),
    ]
    .map(|message_request| ServerReflectionRequest {
        host: String::new(),
        message_request: Some(message_request),
    });
    let mut responses = client
        .server_reflection_info(tokio_stream::iter(requests))
        .await
        .unwrap()
        .into_inner();
    let listed = responses.message().await.unwrap().unwrap();
    match listed.message_response.unwrap() {
        MessageResponse::ListServicesResponse(list) => {
            assert!(list
                .service
                .iter()
                .any(|service| service.name == "news.NewsService"));
        }
        response => panic!("unexpected reflection list response: {response:?}"),
    }
    let descriptor = responses.message().await.unwrap().unwrap();
    match descriptor.message_response.unwrap() {
        MessageResponse::FileDescriptorResponse(files) => {
            assert!(!files.file_descriptor_proto.is_empty());
            assert!(files
                .file_descriptor_proto
                .iter()
                .all(|file| !file.is_empty()));
        }
        response => panic!("unexpected descriptor response: {response:?}"),
    }
    assert!(responses.message().await.unwrap().is_none());
    drop(responses);
    drop(client);
    stop_server(stop, task).await;
}

#[test]
fn honeycomb_metadata_is_preserved_and_invalid_values_are_rejected() {
    let metadata = telemetry_metadata("test-only-placeholder").unwrap();
    assert_eq!(
        metadata.get("x-honeycomb-team").unwrap(),
        "test-only-placeholder"
    );
    assert!(telemetry_metadata("invalid\nvalue").is_err());
}
