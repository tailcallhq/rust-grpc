# Sample gRPC Rust News Server

## Overview

This repository contains a gRPC-based Rust server implementing CRUD operations for a news list. It features a batched `GetNews` API, allowing efficient retrieval of multiple news items.

### Features

- **CRUD Operations**: Create, Read, Update, and Delete news items.
- **Batched News Retrieval**: Fetch multiple news items in a single request.
- **gRPC Interface**: Efficient and modern protocol for inter-service communication.

## Prerequisites

Before you begin, ensure you have installed:

- [Rust](https://www.rust-lang.org/tools/install)

## Installation

## Running the Server

Start the server with:

```bash
cargo run
```

## Deploying to Shuttle.dev

To deploy this grpc service to shuttle.dev, follow the steps below;

- Fork/clone this repo onto your machine and run the command below. Don't worry. It'll install all the project dependencies as well

```shell
cargo install cargo-shuttle
```

- When the installation of dependencies is done, you'll need to create a shuttle project. To do this, you'll need to also have a shuttle.dev account. When you've created your account, run the next command below.

You'll be redirected to your browser to login.

```shell
shuttle init
```

The command above will create a shuttle project for you, you can specify where you want to install it. Make sure you have installed the shuttle-runtime dependency too. If you haven't, you'll encounter this error when you try deploying the service

```shell
thread 'main' panicked at /home/.cargo/registry/src/index.crates.io-6f17d22bba15001f/cargo-shuttle-0.49.0/src/lib.rs:2476:18:
Expected at least one crate with shuttle-runtime in the workspace
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

- When the project is initialized, you can go into the project directory, and run the command below to deploy.

```shell
shuttle deploy
```

You can also test this project locall on your maching by swicthing to the `shuttle-rust-grpc` folder, and run

```shell
cargo shuttle run
```

You'll see something like this in your terminal

```shell
Starting shuttle-rust-grpc on http://127.0.0.1:8010

2024-11-28T20:57:11.037+01:00 [Runtime] Starting on 127.0.0.1:8010
2024-11-28T20:57:11.038+01:00 [Runtime]  INFO actix_server::builder: starting 4 workers
2024-11-28T20:57:11.038+01:00 [Runtime]  INFO actix_server::server: Tokio runtime found; starting in existing Tokio runtime
2024-11-28T20:57:11.038+01:00 [Runtime]  INFO actix_server::server: starting service: "actix-web-service-127.0.0.1:8010", workers: 4, listening on: 127.0.0.1:8010
```

`ctrl` + Click on the link to open it in your browser

## gRPC API

The server uses the following gRPC API defined in `news.proto`:

```protobuf
syntax = "proto3";
import "google/protobuf/empty.proto";

package news;

message News {
  int32 id = 1;
  string title = 2;
  string body = 3;
  string postImage = 4;
}

service NewsService {
  rpc GetAllNews (google.protobuf.Empty) returns (NewsList) {}
  rpc GetNews (NewsId) returns (News) {}
  rpc GetMultipleNews (MultipleNewsId) returns (NewsList) {}
  rpc DeleteNews (NewsId) returns (google.protobuf.Empty) {}
  rpc EditNews (News) returns (News) {}
  rpc AddNews (News) returns (News) {}
}

message NewsId {
  int32 id = 1;
}

message MultipleNewsId {
  repeated NewsId ids = 1;
}

message NewsList {
  repeated News news = 1;
}
```

## Reflection api

The server supports reflection api by default

### example

`grpcurl -plaintext localhost:50051 list`

## License

This project is licensed under the MIT License.

---

© 2024 @ Tailcall
