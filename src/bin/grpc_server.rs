//! gRPC server (specs.md #6, claude.md tier 4): tonic over HTTP/2 on 50051.
//! Serves OrderService and ShipmentService from the Redis cache.
//!
//! Run: cargo run --bin grpc_server

use tonic::transport::Server;

use ecomrust::cache::{Cache, DEFAULT_REDIS_URL};
use ecomrust::grpc::{OrderGrpc, ShipmentGrpc};
use ecomrust::pb::order_service_server::OrderServiceServer;
use ecomrust::pb::shipment_service_server::ShipmentServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let cache = Cache::connect(&redis_url).await?;

    let addr = "0.0.0.0:50051".parse()?;
    println!("gRPC server listening on {addr} (Redis: {redis_url})");

    Server::builder()
        .add_service(OrderServiceServer::new(OrderGrpc::new(cache.clone())))
        .add_service(ShipmentServiceServer::new(ShipmentGrpc::new(cache)))
        .serve(addr)
        .await?;
    Ok(())
}
