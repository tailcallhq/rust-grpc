use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap,
};
use once_cell::sync::Lazy;
use opentelemetry::{global, trace::TraceError, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, runtime, Resource};
use shuttle_runtime::Service;
use tonic::{metadata::MetadataMap, transport::Server as TonicServer, Response, Status};
use tonic_tracing_opentelemetry::middleware::server;
use tower::make::Shared;
use tracing_subscriber::layer::SubscriberExt;

pub mod grpc {
    pub mod news {
        tonic::include_proto!("news");
    }
    pub mod posts {
        tonic::include_proto!("posts");
    }
    pub mod users {
        tonic::include_proto!("users");
    }
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("grpc_descriptor");
}

use grpc::news::news_service_server::{NewsService, NewsServiceServer};
use grpc::news::{MultipleNewsId, News, NewsId, NewsList};
use grpc::posts::post_service_server::{PostService, PostServiceServer};
use grpc::posts::{
    DeleteResponse as PostDeleteResponse, Filter as PostFilter, Post, PostList, PostRequest,
    PostResponse,
};
use grpc::users::user_service_server::{UserService, UserServiceServer};
use grpc::users::{
    DeleteResponse as UserDeleteResponse, Filter as UserFilter, PatchUserRequest, User, UserList,
    UserRequest, UserResponse,
};

#[derive(Debug, Default, Clone)]
pub struct MyGrpcService {
    news: Arc<Mutex<Vec<News>>>, // Using a simple vector to store news items in memory
    posts: Arc<Mutex<Vec<Post>>>,
    users: Arc<Mutex<Vec<User>>>,
}

impl MyGrpcService {
    fn new() -> MyGrpcService {
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
        let posts = vec![
            Post {
                user_id: 1,
                id: 1,
                title: "Post 1".into(),
                body: "Body 1".into(),
            },
            Post {
                user_id: 1,
                id: 2,
                title: "Post 2".into(),
                body: "Body 2".into(),
            },
        ];
        let users = vec![User {
            id: 1,
            name: "Leanne Graham".into(),
            username: "Bret".into(),
            email: "Sincere@april.biz".into(),
            address: None,
            phone: "1-770-736-8031 x56442".into(),
            website: "hildegard.org".into(),
            company: None,
        }];
        MyGrpcService {
            news: Arc::new(Mutex::new(news)),
            posts: Arc::new(Mutex::new(posts)),
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[tonic::async_trait]
impl NewsService for MyGrpcService {
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

#[tonic::async_trait]
impl PostService for MyGrpcService {
    async fn list_posts(
        &self,
        request: tonic::Request<PostFilter>,
    ) -> std::result::Result<Response<PostList>, Status> {
        let filter = request.into_inner();
        let lock = self.posts.lock().unwrap();
        let posts = match filter.user_id {
            Some(user_id) => lock
                .iter()
                .filter(|p| p.user_id == user_id)
                .cloned()
                .collect(),
            None => lock.clone(),
        };
        Ok(Response::new(PostList { posts }))
    }

    async fn get_post(
        &self,
        request: tonic::Request<PostRequest>,
    ) -> std::result::Result<Response<Post>, Status> {
        let id = request.into_inner().id;
        let lock = self.posts.lock().unwrap();
        let post = lock.iter().find(|p| p.id == id).cloned();
        match post {
            Some(post) => Ok(Response::new(post)),
            None => Err(Status::not_found("Post not found")),
        }
    }

    async fn create_post(
        &self,
        request: tonic::Request<Post>,
    ) -> std::result::Result<Response<PostResponse>, Status> {
        let mut post = request.into_inner();
        let mut lock = self.posts.lock().unwrap();
        let new_id = lock.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        post.id = new_id;
        lock.push(post.clone());
        Ok(Response::new(PostResponse { post: Some(post) }))
    }

    async fn update_post(
        &self,
        request: tonic::Request<Post>,
    ) -> std::result::Result<Response<PostResponse>, Status> {
        let post_update = request.into_inner();
        let mut lock = self.posts.lock().unwrap();
        if let Some(post) = lock.iter_mut().find(|p| p.id == post_update.id) {
            *post = post_update.clone();
            return Ok(Response::new(PostResponse {
                post: Some(post_update),
            }));
        }
        Err(Status::not_found("Post not found"))
    }

    async fn delete_post(
        &self,
        request: tonic::Request<PostRequest>,
    ) -> std::result::Result<Response<PostDeleteResponse>, Status> {
        let id = request.into_inner().id;
        let mut lock = self.posts.lock().unwrap();
        let len_before = lock.len();
        lock.retain(|p| p.id != id);
        if lock.len() < len_before {
            Ok(Response::new(PostDeleteResponse {
                success: true,
                message: "Post deleted".into(),
            }))
        } else {
            Err(Status::not_found("Post not found"))
        }
    }
}

#[tonic::async_trait]
impl UserService for MyGrpcService {
    async fn list_users(
        &self,
        request: tonic::Request<UserFilter>,
    ) -> std::result::Result<Response<UserList>, Status> {
        let filter = request.into_inner();
        let lock = self.users.lock().unwrap();
        let users = if filter.id.is_empty() {
            lock.clone()
        } else {
            lock.iter()
                .filter(|u| filter.id.contains(&u.id))
                .cloned()
                .collect()
        };
        Ok(Response::new(UserList { users }))
    }

    async fn get_user(
        &self,
        request: tonic::Request<UserRequest>,
    ) -> std::result::Result<Response<User>, Status> {
        let id = request.into_inner().id;
        let lock = self.users.lock().unwrap();
        let user = lock.iter().find(|u| u.id == id).cloned();
        match user {
            Some(user) => Ok(Response::new(user)),
            None => Err(Status::not_found("User not found")),
        }
    }

    async fn create_user(
        &self,
        request: tonic::Request<User>,
    ) -> std::result::Result<Response<UserResponse>, Status> {
        let mut user = request.into_inner();
        let mut lock = self.users.lock().unwrap();
        let new_id = lock.iter().map(|u| u.id).max().unwrap_or(0) + 1;
        user.id = new_id;
        lock.push(user.clone());
        Ok(Response::new(UserResponse { user: Some(user) }))
    }

    async fn patch_user(
        &self,
        request: tonic::Request<PatchUserRequest>,
    ) -> std::result::Result<Response<UserResponse>, Status> {
        let req = request.into_inner();
        let mut lock = self.users.lock().unwrap();
        if let Some(user) = lock.iter_mut().find(|u| u.id == req.id) {
            if let Some(name) = req.name {
                user.name = name;
            }
            if let Some(username) = req.username {
                user.username = username;
            }
            if let Some(email) = req.email {
                user.email = email;
            }
            return Ok(Response::new(UserResponse {
                user: Some(user.clone()),
            }));
        }
        Err(Status::not_found("User not found"))
    }

    async fn delete_user(
        &self,
        request: tonic::Request<UserRequest>,
    ) -> std::result::Result<Response<UserDeleteResponse>, Status> {
        let id = request.into_inner().id;
        let mut lock = self.users.lock().unwrap();
        let len_before = lock.len();
        lock.retain(|u| u.id != id);
        if lock.len() < len_before {
            Ok(Response::new(UserDeleteResponse {
                success: true,
                message: "User deleted".into(),
            }))
        } else {
            Err(Status::not_found("User not found"))
        }
    }
}

static RESOURCE: Lazy<Resource> = Lazy::new(|| {
    Resource::default().merge(&Resource::new(vec![
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            "rust-grpc",
        ),
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
            "test",
        ),
    ]))
});

fn init_tracer() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    static TELEMETRY_URL: &str = "https://api.honeycomb.io:443";
    let headers = HeaderMap::from_iter([(
        HeaderName::from_static("x-honeycomb-team"),
        HeaderValue::from_str(&std::env::var("HONEYCOMB_API_KEY")?)?,
    )]);

    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(TELEMETRY_URL)
        .with_metadata(MetadataMap::from_headers(headers));

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(RESOURCE.clone()))
        .install_batch(runtime::Tokio)?
        .provider()
        .ok_or(TraceError::Other(
            anyhow!("Failed to instantiate OTLP provider").into(),
        ))?;

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

    let grpc_service = MyGrpcService::new();

    Ok(grpc_service)
}

#[async_trait::async_trait]
impl Service for MyGrpcService {
    async fn bind(mut self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(grpc::FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        println!("NewsService server listening on {}", addr);

        let tonic_service = TonicServer::builder()
            .layer(server::OtelGrpcLayer::default())
            .add_service(NewsServiceServer::new(self.clone()))
            .add_service(PostServiceServer::new(self.clone()))
            .add_service(UserServiceServer::new(self))
            .add_service(service)
            .into_service();
        let make_svc = Shared::new(tonic_service);

        let server = hyper::Server::bind(&addr).serve(make_svc);
        server
            .await
            .map_err(|e| shuttle_runtime::Error::Custom(anyhow::anyhow!(e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_posts() {
        let service = MyGrpcService::new();
        let request = tonic::Request::new(PostFilter { user_id: None });
        let response = service.list_posts(request).await.unwrap();
        let posts = response.into_inner();
        assert_eq!(posts.posts.len(), 2);
    }

    #[tokio::test]
    async fn test_get_post() {
        let service = MyGrpcService::new();
        let request = tonic::Request::new(PostRequest { id: 1 });
        let response = service.get_post(request).await.unwrap();
        let post = response.into_inner();
        assert_eq!(post.title, "Post 1");
    }

    #[tokio::test]
    async fn test_create_post() {
        let service = MyGrpcService::new();
        let new_post = Post {
            user_id: 1,
            id: 0, // ID should be ignored/overwritten
            title: "New Post".into(),
            body: "New Body".into(),
        };
        let request = tonic::Request::new(new_post);
        let response = service.create_post(request).await.unwrap();
        let post = response.into_inner().post.unwrap();
        assert_eq!(post.title, "New Post");
        assert_eq!(post.id, 3); // Should be 3 as there are 2 existing posts
    }

    #[tokio::test]
    async fn test_list_users() {
        let service = MyGrpcService::new();
        let request = tonic::Request::new(UserFilter { id: vec![] });
        let response = service.list_users(request).await.unwrap();
        let users = response.into_inner();
        assert_eq!(users.users.len(), 1);
    }

    #[tokio::test]
    async fn test_get_user() {
        let service = MyGrpcService::new();
        let request = tonic::Request::new(UserRequest { id: 1 });
        let response = service.get_user(request).await.unwrap();
        let user = response.into_inner();
        assert_eq!(user.name, "Leanne Graham");
    }
}
