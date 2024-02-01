use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tonic::{transport::Server, Response, Status};

use news::news_service_server::{NewsService, NewsServiceServer};
use news::{News, NewsId, NewsList, MultipleNewsId};
use std::sync::{Arc, Mutex};
use crate::news::news_service_client::NewsServiceClient;

const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/descriptor_set.bin"
        ));

pub mod news {
    tonic::include_proto!("news"); // The package name specified in your .proto
}

#[derive(Debug, Default)]
pub struct MyNewsService {
    news: Arc<Mutex<Vec<News>>>, // Using a simple vector to store news items in memory
}

impl MyNewsService {
    fn new() -> MyNewsService {
        let news = vec![
            News { id: 1, title: "Note 1".into(), body: "Content 1".into(), post_image: "Post image 1".into() },
            News { id: 2, title: "Note 2".into(), body: "Content 2".into(), post_image: "Post image 2".into() },
            News { id: 3, title: "Note 3".into(), body: "Content 3".into(), post_image: "Post image 3".into() },
            News { id: 4, title: "Note 4".into(), body: "Content 4".into(), post_image: "Post image 4".into() },
            News { id: 5, title: "Note 5".into(), body: "Content 5".into(), post_image: "Post image 5".into() },
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
    ) -> std::result::Result<tonic::Response<NewsList>, tonic::Status> {
        println!("1");
        let lock = self.news.lock().unwrap();
        let reply = NewsList {
            news: lock.clone(),
        };
        Ok(Response::new(reply))
    }

    async fn get_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<tonic::Response<News>, tonic::Status> {
        println!("2");
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
    ) -> std::result::Result<tonic::Response<NewsList>, tonic::Status> {
        println!("3");
        let ids = request.into_inner().ids.into_iter().map(|id| id.id).collect::<Vec<_>>();
        let lock = self.news.lock().unwrap();
        let news_items: Vec<News> = lock.iter().filter(|n| ids.contains(&n.id)).cloned().collect();
        Ok(Response::new(NewsList { news: news_items }))
    }

    async fn delete_news(
        &self,
        request: tonic::Request<NewsId>,
    ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
        println!("4");
        let id = request.into_inner().id;
        let mut lock = self.news.lock().unwrap();
        let len_before = lock.len();
        lock.retain(|news| news.id != id);
        let len_after = lock.len();

        if len_before == len_after {
            Err(Status::not_found("News not found"))
        } else {
            let x = tonic::Response::new(());
            Ok(x)
        }
    }

    async fn edit_news(
        &self,
        request: tonic::Request<News>,
    ) -> std::result::Result<tonic::Response<News>, tonic::Status> {
        println!("5");
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
    ) -> std::result::Result<tonic::Response<News>, tonic::Status> {
        println!("6");
        let mut news = request.into_inner();
        let mut lock = self.news.lock().unwrap();
        let new_id = lock.iter().map(|n| n.id).max().unwrap_or(0) + 1; // Simple ID generation
        news.id = new_id;
        lock.push(news.clone());
        Ok(Response::new(news))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 50051);

    let news_service = MyNewsService::new();

    println!("NewsService server listening on {}", addr);

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap(); // idk if we need this

    let thread = tokio::task::spawn(async move {
        Server::builder()
            .add_service(NewsServiceServer::new(news_service))
            .add_service(reflection_service)
            .serve(addr).await.unwrap();
    });

    let mut client = NewsServiceClient::connect("http://localhost:50051").await?;

    let request = tonic::Request::new(());

    let response = client.get_all_news(request).await?;

    println!("RESPONSE={:#?}", response);

    thread.await.unwrap();

    Ok(())
}
