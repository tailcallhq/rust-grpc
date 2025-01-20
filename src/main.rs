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
use tonic::{metadata::MetadataMap, transport::Server as TonicServer, Response, Status};
use tonic_tracing_opentelemetry::middleware::server;
use tower::make::Shared;

use news::news_service_server::NewsService;
use news::{MultipleNewsId, News, NewsId, NewsList};
use shuttle_runtime::Service;
use tracing_subscriber::layer::SubscriberExt;

pub mod news {
    tonic::include_proto!("news"); // The package name specified in your .proto
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("news_descriptor");
}

pub mod posts {
    tonic::include_proto!("posts");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("posts_descriptor");
}

pub mod users {
    tonic::include_proto!("users");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("users_descriptor");
}

#[derive(Debug, Default)]
pub struct MyNewsService {
    news: Arc<Mutex<Vec<News>>>, // Using a simple vector to store news items in memory
}

#[derive(Debug, Default)]
pub struct MyPostService {
    posts: Arc<Mutex<Vec<posts::Post>>>,
}

#[derive(Debug, Default)]
pub struct MyUserService {
    users: Arc<Mutex<Vec<users::User>>>,
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

impl MyPostService {
    fn new() -> Self {
        let posts = vec![
            posts::Post {
                id: 1,
                user_id: 1,
                title: "First post".into(),
                body: "This is the first post content".into(),
            },
            posts::Post {
                id: 2,
                user_id: 1,
                title: "Second post".into(),
                body: "This is the second post content".into(),
            },
        ];
        MyPostService {
            posts: Arc::new(Mutex::new(posts)),
        }
    }
}

#[tonic::async_trait]
impl posts::post_service_server::PostService for MyPostService {
    async fn list_posts(
        &self,
        request: tonic::Request<posts::Filter>,
    ) -> Result<Response<posts::PostList>, Status> {
        let filter = request.into_inner();
        let posts = self.posts.lock().unwrap();
        let mut filtered = posts.clone();

        if !filter.ids.is_empty() {
            filtered.retain(|p| filter.ids.contains(&p.id));
        }

        if let Some(user_id) = filter.user_id {
            filtered.retain(|p| p.user_id == user_id);
        }

        if let (Some(start), Some(limit)) = (filter.start, filter.limit) {
            filtered = filtered
                .into_iter()
                .skip(start as usize)
                .take(limit as usize)
                .collect();
        }

        Ok(Response::new(posts::PostList { posts: filtered }))
    }

    async fn get_post(
        &self,
        request: tonic::Request<posts::PostRequest>,
    ) -> Result<Response<posts::Post>, Status> {
        let id = request.into_inner().id;
        let posts = self.posts.lock().unwrap();
        if let Some(post) = posts.iter().find(|p| p.id == id) {
            Ok(Response::new(post.clone()))
        } else {
            Err(Status::not_found("Post not found"))
        }
    }

    async fn create_post(
        &self,
        request: tonic::Request<posts::Post>,
    ) -> Result<Response<posts::PostResponse>, Status> {
        let mut post = request.into_inner();
        let mut posts = self.posts.lock().unwrap();
        post.id = posts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        posts.push(post.clone());
        Ok(Response::new(posts::PostResponse { post: Some(post) }))
    }

    async fn update_post(
        &self,
        request: tonic::Request<posts::Post>,
    ) -> Result<Response<posts::PostResponse>, Status> {
        let new_post = request.into_inner();
        let mut posts = self.posts.lock().unwrap();
        if let Some(post) = posts.iter_mut().find(|p| p.id == new_post.id) {
            *post = new_post.clone();
            Ok(Response::new(posts::PostResponse {
                post: Some(new_post),
            }))
        } else {
            Err(Status::not_found("Post not found"))
        }
    }

    async fn delete_post(
        &self,
        request: tonic::Request<posts::PostRequest>,
    ) -> Result<Response<posts::DeleteResponse>, Status> {
        let id = request.into_inner().id;
        let mut posts = self.posts.lock().unwrap();
        let len_before = posts.len();
        posts.retain(|p| p.id != id);
        if posts.len() < len_before {
            Ok(Response::new(posts::DeleteResponse {
                success: true,
                message: format!("Post {} successfully deleted", id),
            }))
        } else {
            Ok(Response::new(posts::DeleteResponse {
                success: false,
                message: format!("Post {} not found", id),
            }))
        }
    }
}

impl MyUserService {
    fn new() -> Self {
        let users = vec![users::User {
            id: 1,
            name: "Leanne Graham".to_string(),
            username: "Bret".to_string(),
            email: "Sincere@april.biz".to_string(),
            address: Some(users::Address {
                street: "Kulas Light".to_string(),
                suite: "Apt. 556".to_string(),
                city: "Gwenborough".to_string(),
                zipcode: "92998-3874".to_string(),
                geo: Some(users::Geo {
                    lat: "-37.3159".to_string(),
                    lng: "81.1496".to_string(),
                }),
            }),
            phone: "1-770-736-8031 x56442".to_string(),
            website: "hildegard.org".to_string(),
            company: Some(users::Company {
                name: "Romaguera-Crona".to_string(),
                catch_phrase: "Multi-layered client-server neural-net".to_string(),
                bs: "harness real-time e-markets".to_string(),
            }),
        }];
        MyUserService {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[tonic::async_trait]
impl users::user_service_server::UserService for MyUserService {
    async fn list_users(
        &self,
        request: tonic::Request<users::Filter>,
    ) -> Result<Response<users::UserList>, Status> {
        let filter = request.into_inner();
        let users = self.users.lock().unwrap();

        let mut filtered = users.clone();

        if !filter.ids.is_empty() {
            filtered.retain(|u| filter.ids.contains(&u.id));
        }

        if let Some(username) = filter.username {
            filtered.retain(|u| u.username == username);
        }

        if let Some(email) = filter.email {
            filtered.retain(|u| u.email == email);
        }

        if let (Some(start), Some(limit)) = (filter.start, filter.limit) {
            filtered = filtered
                .into_iter()
                .skip(start as usize)
                .take(limit as usize)
                .collect();
        }

        Ok(Response::new(users::UserList { users: filtered }))
    }

    async fn get_user(
        &self,
        request: tonic::Request<users::UserRequest>,
    ) -> Result<Response<users::User>, Status> {
        let id = request.into_inner().id;
        let users = self.users.lock().unwrap();

        if let Some(user) = users.iter().find(|u| u.id == id) {
            Ok(Response::new(user.clone()))
        } else {
            Err(Status::not_found("User not found"))
        }
    }

    async fn create_user(
        &self,
        request: tonic::Request<users::User>,
    ) -> Result<Response<users::UserResponse>, Status> {
        let mut user = request.into_inner();
        let mut users = self.users.lock().unwrap();

        user.id = users.iter().map(|u| u.id).max().unwrap_or(0) + 1;
        users.push(user.clone());

        Ok(Response::new(users::UserResponse { user: Some(user) }))
    }

    async fn patch_user(
        &self,
        request: tonic::Request<users::PatchUserRequest>,
    ) -> Result<Response<users::UserResponse>, Status> {
        let patch_request = request.into_inner();
        let user_id = patch_request.id;
        let new_user_data = patch_request.user.ok_or_else(|| {
            Status::invalid_argument("User data must be provided for patch operation")
        })?;

        let mut users = self.users.lock().unwrap();

        if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
            if !new_user_data.name.is_empty() {
                user.name = new_user_data.name;
            }
            if !new_user_data.username.is_empty() {
                user.username = new_user_data.username;
            }
            if !new_user_data.email.is_empty() {
                user.email = new_user_data.email;
            }
            if !new_user_data.phone.is_empty() {
                user.phone = new_user_data.phone;
            }
            if !new_user_data.website.is_empty() {
                user.website = new_user_data.website;
            }

            if let Some(new_address) = new_user_data.address {
                if user.address.is_none() {
                    user.address = Some(new_address);
                } else if let Some(ref mut address) = user.address {
                    if !new_address.street.is_empty() {
                        address.street = new_address.street;
                    }
                    if !new_address.suite.is_empty() {
                        address.suite = new_address.suite;
                    }
                    if !new_address.city.is_empty() {
                        address.city = new_address.city;
                    }
                    if !new_address.zipcode.is_empty() {
                        address.zipcode = new_address.zipcode;
                    }
                    // Update geo location if provided
                    if let Some(new_geo) = new_address.geo {
                        if let Some(ref mut geo) = address.geo {
                            if !new_geo.lat.is_empty() {
                                geo.lat = new_geo.lat;
                            }
                            if !new_geo.lng.is_empty() {
                                geo.lng = new_geo.lng;
                            }
                        } else {
                            address.geo = Some(new_geo);
                        }
                    }
                }
            }

            if let Some(new_company) = new_user_data.company {
                if user.company.is_none() {
                    user.company = Some(new_company);
                } else if let Some(ref mut company) = user.company {
                    if !new_company.name.is_empty() {
                        company.name = new_company.name;
                    }
                    if !new_company.catch_phrase.is_empty() {
                        company.catch_phrase = new_company.catch_phrase;
                    }
                    if !new_company.bs.is_empty() {
                        company.bs = new_company.bs;
                    }
                }
            }

            Ok(Response::new(users::UserResponse {
                user: Some(user.clone()),
            }))
        } else {
            Err(Status::not_found("User not found"))
        }
    }

    async fn delete_user(
        &self,
        request: tonic::Request<users::UserRequest>,
    ) -> Result<Response<users::DeleteResponse>, Status> {
        let id = request.into_inner().id;
        let mut users = self.users.lock().unwrap();
        let len_before = users.len();

        users.retain(|u| u.id != id);

        if users.len() < len_before {
            Ok(Response::new(users::DeleteResponse {
                success: true,
                message: format!("User {} successfully deleted", id),
            }))
        } else {
            Ok(Response::new(users::DeleteResponse {
                success: false,
                message: format!("User {} not found", id),
            }))
        }
    }
}

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

#[derive(Debug)]
pub struct CompositeService {
    news_service: MyNewsService,
    post_service: MyPostService,
    user_service: MyUserService,
}

#[shuttle_runtime::main]
async fn shuttle_main() -> Result<impl Service, shuttle_runtime::Error> {
    if std::env::var("HONEYCOMB_API_KEY").is_ok() {
        init_tracer()?;
    }

    let composite_service = CompositeService {
        news_service: MyNewsService::new(),
        post_service: MyPostService::new(),
        user_service: MyUserService::new(),
    };

    Ok(composite_service)
}

#[async_trait::async_trait]
impl Service for CompositeService {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(news::FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(posts::FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(users::FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        println!("Server listening on {}", addr);

        let tonic_service = TonicServer::builder()
            .layer(server::OtelGrpcLayer::default())
            .add_service(news::news_service_server::NewsServiceServer::new(
                self.news_service,
            ))
            .add_service(posts::post_service_server::PostServiceServer::new(
                self.post_service,
            ))
            .add_service(users::user_service_server::UserServiceServer::new(
                self.user_service,
            ))
            .add_service(reflection_service)
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
    use posts::post_service_server::PostService;
    use tonic::Request;
    use users::user_service_server::UserService;

    fn create_test_post() -> posts::Post {
        posts::Post {
            id: 0,
            user_id: 1,
            title: "Test Post".to_string(),
            body: "This is a test post content".to_string(),
        }
    }

    fn create_test_user() -> users::User {
        users::User {
            id: 0,
            name: "Test User".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            address: Some(users::Address {
                street: "123 Test St".to_string(),
                suite: "Apt 1".to_string(),
                city: "Test City".to_string(),
                zipcode: "12345".to_string(),
                geo: Some(users::Geo {
                    lat: "40.7128".to_string(),
                    lng: "-74.0060".to_string(),
                }),
            }),
            phone: "123-456-7890".to_string(),
            website: "test.com".to_string(),
            company: Some(users::Company {
                name: "Test Company".to_string(),
                catch_phrase: "Testing is our business".to_string(),
                bs: "innovative testing solutions".to_string(),
            }),
        }
    }

    #[tokio::test]
    async fn test_post_service_crud() {
        let service = MyPostService::new();

        let new_post = create_test_post();
        let create_response = service
            .create_post(Request::new(new_post.clone()))
            .await
            .unwrap()
            .into_inner();

        let created_post = create_response.post.unwrap();
        assert!(
            created_post.id > 0,
            "Created post should have an ID assigned"
        );
        assert_eq!(created_post.title, "Test Post");
        assert_eq!(created_post.body, "This is a test post content");

        let get_response = service
            .get_post(Request::new(posts::PostRequest {
                id: created_post.id,
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(get_response.id, created_post.id);
        assert_eq!(get_response.title, created_post.title);

        let mut updated_post = created_post.clone();
        updated_post.title = "Updated Title".to_string();

        let update_response = service
            .update_post(Request::new(updated_post.clone()))
            .await
            .unwrap()
            .into_inner();

        let updated = update_response.post.unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.id, created_post.id);

        let filter = posts::Filter {
            ids: vec![created_post.id],
            user_id: Some(1),
            start: Some(0),
            limit: Some(10),
        };

        let list_response = service
            .list_posts(Request::new(filter))
            .await
            .unwrap()
            .into_inner();

        assert!(
            !list_response.posts.is_empty(),
            "Should find the created post"
        );
        assert_eq!(list_response.posts[0].id, created_post.id);

        let delete_response = service
            .delete_post(Request::new(posts::PostRequest {
                id: created_post.id,
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(delete_response.success, "Post deletion should succeed");
    }

    #[tokio::test]
    async fn test_user_service_crud() {
        let service = MyUserService::new();

        let new_user = create_test_user();
        let create_response = service
            .create_user(Request::new(new_user.clone()))
            .await
            .unwrap()
            .into_inner();

        let created_user = create_response.user.unwrap();
        assert!(
            created_user.id > 0,
            "Created user should have an ID assigned"
        );
        assert_eq!(created_user.name, "Test User");
        assert_eq!(created_user.email, "test@example.com");

        let mut patch_user = created_user.clone();
        patch_user.name = "Updated Name".to_string();

        let patch_request = users::PatchUserRequest {
            id: created_user.id,
            user: Some(patch_user),
        };

        let patch_response = service
            .patch_user(Request::new(patch_request))
            .await
            .unwrap()
            .into_inner();

        let patched_user = patch_response.user.unwrap();
        assert_eq!(patched_user.name, "Updated Name");
        assert_eq!(patched_user.email, created_user.email);

        let filter = users::Filter {
            ids: vec![created_user.id],
            username: Some("testuser".to_string()),
            email: None,
            start: Some(0),
            limit: Some(10),
        };

        let list_response = service
            .list_users(Request::new(filter))
            .await
            .unwrap()
            .into_inner();

        assert!(
            !list_response.users.is_empty(),
            "Should find the created user"
        );
        assert_eq!(list_response.users[0].id, created_user.id);

        let delete_response = service
            .delete_user(Request::new(users::UserRequest {
                id: created_user.id,
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(delete_response.success, "User deletion should succeed");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let post_service = MyPostService::new();
        let user_service = MyUserService::new();

        let post_result = post_service
            .get_post(Request::new(posts::PostRequest { id: 99999 }))
            .await;

        assert!(post_result.is_err(), "Should fail for non-existent post");
        assert_eq!(
            post_result.unwrap_err().code(),
            tonic::Code::NotFound,
            "Should return NotFound error"
        );

        let user_result = user_service
            .get_user(Request::new(users::UserRequest { id: 99999 }))
            .await;

        assert!(user_result.is_err(), "Should fail for non-existent user");
        assert_eq!(
            user_result.unwrap_err().code(),
            tonic::Code::NotFound,
            "Should return NotFound error"
        );

        let invalid_patch = users::PatchUserRequest {
            id: 99999,
            user: None,
        };

        let patch_result = user_service.patch_user(Request::new(invalid_patch)).await;

        assert!(
            patch_result.is_err(),
            "Should fail for invalid patch request"
        );
    }
}
