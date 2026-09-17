//! Smoke-test gRPC client: exercises both services end-to-end using the same
//! generated stubs a Fastify client would use (claude.md rule #1).
//!
//! Run: cargo run --bin grpc_client   (with grpc_server running)

use ecomrust::pb::order_service_client::OrderServiceClient;
use ecomrust::pb::shipment_service_client::ShipmentServiceClient;
use ecomrust::pb::{GetRequest, ListRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut orders = OrderServiceClient::connect("http://localhost:50051").await?;

    let list = orders
        .list_orders(ListRequest {
            page: 1,
            page_size: 3,
            sort: "id".to_string(),
            order: "desc".to_string(),
        })
        .await?
        .into_inner();
    println!(
        "ListOrders(desc): page {}/{} of {} records, sort={} order={}",
        list.page, list.total_pages, list.total_records, list.sort, list.order
    );
    for o in &list.data {
        println!("  #{:<6} {} {:<12} {}", o.id, o.order_date, o.status, o.total_amount);
    }

    let one = orders.get_order(GetRequest { id: 1 }).await?.into_inner();
    println!("GetOrder(1): status={} total={}", one.status, one.total_amount);

    let mut shipments = ShipmentServiceClient::connect("http://localhost:50051").await?;
    let s = shipments.get_shipment(GetRequest { id: 1 }).await?.into_inner();
    println!(
        "GetShipment(1): status={} cost={}",
        s.shipment_status, s.shipment_cost
    );

    println!("gRPC smoke test: OK");
    Ok(())
}
