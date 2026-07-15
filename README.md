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
- [Shuttle CLI](https://docs.shuttle.rs/getting-started/installation)

The protobuf compiler is provided by `protoc-bin-vendored`, so contributors do not need to
install a platform-specific `protoc` package.

## Installation

Verify the generated protobuf code, service behavior, and CI workflow with:

```bash
cargo test --all-features --workspace
```

## Dependency baseline

The service uses one compatible current generation for each interconnected runtime stack:

| Stack | Previous baseline | Current baseline | Migration |
| --- | --- | --- | --- |
| gRPC/protobuf | Tonic 0.11, Prost 0.12 | Tonic 0.14, Prost 0.14 | Uses the split `tonic-prost` runtime and `tonic-prost-build` generator APIs. |
| HTTP/server | Hyper 0.14, Tower 0.4 as direct dependencies | Tonic 0.14 transport | Removes the obsolete custom Hyper server adapter and serves the Tonic router directly. |
| Telemetry | OpenTelemetry 0.22 | OpenTelemetry 0.32 | Uses the typed OTLP exporter and `SdkTracerProvider` builders. |
| Deployment | Shuttle Runtime 0.49 | Shuttle Runtime 0.57 | Keeps the `Service` integration on the current Shuttle interface. |
| CI generation | gh-workflow 0.5 | gh-workflow 0.8 | Regenerates the workflow with the current typed step API and `actions/checkout@v5`. |
| Protobuf compiler | System `protoc` | `protoc-bin-vendored` 3.2 | Makes local and CI builds reproducible without an extra package-manager step. |

## Running the Server Locally

Start the server with:

```bash
cargo shuttle run --port 50051
```

## Deploying to Shuttle.dev

Deploy the server with:

```bash
cargo shuttle init
```

Follow the prompts:
1. Select your project name
2. Choose 'Empty (no framework)' when prompted

Then deploy with:

```bash
cargo shuttle deploy
```

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
