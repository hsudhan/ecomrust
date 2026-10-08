//! tonic gRPC service handlers (specs.md #6). Both services serve from the
//! Redis cache, sharing the same validated list/get logic as the REST tier.

use tonic::{Request, Response, Status};

use crate::cache::{Cache, ORDERS_PREFIX, ORDERS_SORT_FIELDS, SHIPMENTS_PREFIX, SHIPMENTS_SORT_FIELDS};
use crate::error::AppError;
use crate::models::{OrderJson, PageParams, ShipmentJson, SortDir};
use crate::pb;

// ------------------------------------------------------------ mapping -----

fn grpc_params(r: &pb::ListRequest) -> Result<PageParams, Status> {
    let dir = match r.order.as_str() {
        "" | "asc" => SortDir::Asc,
        "desc" => SortDir::Desc,
        other => return Err(AppError::BadRequest(format!("invalid order '{other}'")).into()),
    };
    Ok(PageParams {
        page: if r.page <= 0 { 1 } else { r.page as i64 },
        page_size: if r.page_size <= 0 { 50 } else { r.page_size as i64 },
        sort: if r.sort.is_empty() { "id".to_string() } else { r.sort.clone() },
        dir,
    })
}

fn dir_str(dir: SortDir) -> &'static str {
    match dir {
        SortDir::Asc => "asc",
        SortDir::Desc => "desc",
    }
}

fn total_pages(total: i64, page_size: i64) -> i32 {
    (((total + page_size - 1) / page_size).max(1)) as i32
}

fn order_to_pb(o: OrderJson) -> pb::Order {
    pb::Order {
        id: o.id,
        customer_id: o.customer_id,
        order_date: o.order_date,
        status: o.status,
        total_amount: o.total_amount,
        created_at: o.created_at,
        updated_at: o.updated_at,
    }
}

fn shipment_to_pb(s: ShipmentJson) -> pb::Shipment {
    pb::Shipment {
        id: s.id,
        order_id: s.order_id,
        shipment_date: s.shipment_date,
        shipment_status: s.shipment_status,
        shipment_cost: s.shipment_cost,
        created_at: s.created_at,
        updated_at: s.updated_at,
    }
}

// -------------------------------------------------------- order service ---

pub struct OrderGrpc {
    cache: Cache,
}

impl OrderGrpc {
    pub fn new(cache: Cache) -> Self {
        Self { cache }
    }
}

#[tonic::async_trait]
impl pb::order_service_server::OrderService for OrderGrpc {
    async fn list_orders(
        &self,
        request: Request<pb::ListRequest>,
    ) -> Result<Response<pb::OrderList>, Status> {
        let params = grpc_params(request.get_ref())?;
        let (rows, total) = self
            .cache
            .list(ORDERS_PREFIX, ORDERS_SORT_FIELDS, &params)
            .await
            .map_err(Status::from)?;
        let data = rows
            .into_iter()
            .map(serde_json::from_value::<OrderJson>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
            .map_err(Status::from)?;
        Ok(Response::new(pb::OrderList {
            data: data.into_iter().map(order_to_pb).collect(),
            page: params.page as i32,
            page_size: params.page_size as i32,
            total_records: total as i32,
            total_pages: total_pages(total, params.page_size),
            sort: params.sort.clone(),
            order: dir_str(params.dir).to_string(),
        }))
    }

    async fn get_order(
        &self,
        request: Request<pb::GetRequest>,
    ) -> Result<Response<pb::Order>, Status> {
        let id = request.into_inner().id;
        if id < 1 {
            return Err(AppError::BadRequest("id must be >= 1".to_string()).into());
        }
        let order: OrderJson = self
            .cache
            .get(ORDERS_PREFIX, id)
            .await
            .map_err(Status::from)?;
        Ok(Response::new(order_to_pb(order)))
    }
}

// ----------------------------------------------------- shipment service ---

pub struct ShipmentGrpc {
    cache: Cache,
}

impl ShipmentGrpc {
    pub fn new(cache: Cache) -> Self {
        Self { cache }
    }
}

#[tonic::async_trait]
impl pb::shipment_service_server::ShipmentService for ShipmentGrpc {
    async fn list_shipments(
        &self,
        request: Request<pb::ListRequest>,
    ) -> Result<Response<pb::ShipmentList>, Status> {
        let params = grpc_params(request.get_ref())?;
        let (rows, total) = self
            .cache
            .list(SHIPMENTS_PREFIX, SHIPMENTS_SORT_FIELDS, &params)
            .await
            .map_err(Status::from)?;
        let data = rows
            .into_iter()
            .map(serde_json::from_value::<ShipmentJson>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)
            .map_err(Status::from)?;
        Ok(Response::new(pb::ShipmentList {
            data: data.into_iter().map(shipment_to_pb).collect(),
            page: params.page as i32,
            page_size: params.page_size as i32,
            total_records: total as i32,
            total_pages: total_pages(total, params.page_size),
            sort: params.sort.clone(),
            order: dir_str(params.dir).to_string(),
        }))
    }

    async fn get_shipment(
        &self,
        request: Request<pb::GetRequest>,
    ) -> Result<Response<pb::Shipment>, Status> {
        let id = request.into_inner().id;
        if id < 1 {
            return Err(AppError::BadRequest("id must be >= 1".to_string()).into());
        }
        let shipment: ShipmentJson = self
            .cache
            .get(SHIPMENTS_PREFIX, id)
            .await
            .map_err(Status::from)?;
        Ok(Response::new(shipment_to_pb(shipment)))
    }
}
