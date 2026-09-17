pub mod cache;
pub mod db;
pub mod error;
pub mod grpc;
pub mod models;
pub mod rest;

/// Generated protobuf stubs (proto/ecom.proto). Server handlers and client
/// stubs come from the same file, keeping both sides in lockstep
/// (claude.md rule #1).
pub mod pb {
    tonic::include_proto!("ecom");
}
