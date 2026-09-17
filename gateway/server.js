/**
 * ecom-gateway — Fastify API gateway tier (claude.md tier 3, specs.md #3/#7).
 *
 *   SolidStart (:8081) --HTTP/JSON--> gateway (:4000) --gRPC/HTTP2--> Rust (:50051)
 *                                                        |
 *                                                        +--SQL--> PostgreSQL (6 direct entities)
 *
 * - orders / shipments are served by invoking the Rust tonic services over
 *   gRPC, using client stubs loaded from the SAME proto/ecom.proto the Rust
 *   server was built from (claude.md rule #1).
 * - The remaining 6 entities have no Rust service yet; they are served
 *   directly from PostgreSQL so the e-commerce web app stays whole
 *   (specs.md #7: integrate response data with the web application).
 * - Ajv JSON-schema validation on every route; pino structured logging.
 */
import Fastify from "fastify";
import cors from "@fastify/cors";
import pg from "pg";
import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROTO_PATH = path.join(__dirname, "..", "proto", "ecom.proto");

const GRPC_ADDR = process.env.GRPC_ADDR ?? "localhost:50051";
const CONN_STRING =
  process.env.DATABASE_URL ?? "postgresql://harir@localhost:5432/ecomdb";
const PORT = Number(process.env.GATEWAY_PORT ?? 4000);

// ---------------------------------------------------------------------------
// gRPC client stubs (proto3 contract shared with Rust).
// keepCase: field names stay snake_case (page_size, total_records) so the
// REST envelope passes straight through to the web UI.
// ---------------------------------------------------------------------------
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: Number, // ids <= 100K: safe as JS numbers
  enums: String,
  defaults: true,
});
const proto = grpc.loadPackageDefinition(packageDefinition).ecom;

const orderClient = new proto.OrderService(GRPC_ADDR, grpc.credentials.createInsecure());
const shipmentClient = new proto.ShipmentService(GRPC_ADDR, grpc.credentials.createInsecure());

function rpc(client, method, request) {
  return new Promise((resolve, reject) => {
    client[method](request, (err, response) => (err ? reject(err) : resolve(response)));
  });
}

// gRPC status code -> HTTP
function grpcErrorToHttp(err, reply) {
  const msg = err.details ?? err.message ?? "gRPC error";
  switch (err.code) {
    case grpc.status.INVALID_ARGUMENT:
      return reply.code(400).send({ error: msg });
    case grpc.status.NOT_FOUND:
      return reply.code(404).send({ error: msg });
    case grpc.status.UNAVAILABLE:
      return reply.code(502).send({ error: `rust service unavailable: ${msg}` });
    default:
      return reply.code(500).send({ error: msg });
  }
}

// ---------------------------------------------------------------------------
const pool = new pg.Pool({ connectionString: CONN_STRING });

const app = Fastify({ logger: { level: "info" } });
await app.register(cors, { origin: true });

// Ajv schemas (claude.md rule #3): list pagination + :id params.
const listSchema = (sortEnum) => ({
  querystring: {
    type: "object",
    properties: {
      page: { type: "integer", minimum: 1, default: 1 },
      page_size: { type: "integer", minimum: 1, maximum: 200, default: 50 },
      sort: { type: "string", enum: sortEnum, default: sortEnum[0] },
      order: { type: "string", enum: ["asc", "desc"], default: "asc" },
    },
    additionalProperties: false,
  },
});

const idSchema = {
  params: {
    type: "object",
    properties: { id: { type: "integer", minimum: 1 } },
    required: ["id"],
    additionalProperties: false,
  },
};

app.get("/health", async () => ({ status: "ok", service: "gateway" }));

// ------------------------------------------------ gRPC-backed: orders -----
app.get("/orders", { schema: listSchema(["id", "order_date"]) }, async (request, reply) => {
  const { page, page_size, sort, order } = request.query;
  try {
    return await rpc(orderClient, "listOrders", { page, page_size, sort, order });
  } catch (err) {
    return grpcErrorToHttp(err, reply);
  }
});

app.get("/order/:id", { schema: idSchema }, async (request, reply) => {
  try {
    return await rpc(orderClient, "getOrder", { id: request.params.id });
  } catch (err) {
    return grpcErrorToHttp(err, reply);
  }
});

// --------------------------------------------- gRPC-backed: shipments -----
app.get("/shipments", { schema: listSchema(["id", "shipment_date"]) }, async (request, reply) => {
  const { page, page_size, sort, order } = request.query;
  try {
    return await rpc(shipmentClient, "listShipments", { page, page_size, sort, order });
  } catch (err) {
    return grpcErrorToHttp(err, reply);
  }
});

app.get("/shipment/:id", { schema: idSchema }, async (request, reply) => {
  try {
    return await rpc(shipmentClient, "getShipment", { id: request.params.id });
  } catch (err) {
    return grpcErrorToHttp(err, reply);
  }
});

// ------------------------------------- direct PostgreSQL: 6 other entities
const PG_ENTITIES = {
  users: {
    table: 'ecommerce."user"',
    columns: ["id", "username", "email", "created_at", "updated_at"],
  },
  logins: {
    table: "ecommerce.login",
    columns: ["id", "user_id", "login_date", "ip_address", "login_type", "device_name", "location"],
  },
  "shopping-carts": {
    table: "ecommerce.shopping_cart",
    columns: ["id", "order_id", "customer_id", "product_id", "quantity", "created_at", "updated_at"],
  },
  "payment-infos": {
    table: "ecommerce.payment_info",
    columns: ["id", "order_id", "payment_method", "payment_amount", "payment_status", "payment_date", "created_at", "updated_at"],
  },
  payments: {
    table: "ecommerce.payment",
    columns: ["id", "order_id", "payment_date", "payment_amount", "payment_status", "created_at", "updated_at"],
  },
  "shipment-trackings": {
    table: "ecommerce.shipment_tracking",
    columns: ["id", "shipment_id", "tracking_number", "status", "updated_at"],
  },
};

const pgListSchema = {
  querystring: {
    type: "object",
    properties: {
      page: { type: "integer", minimum: 1, default: 1 },
      page_size: { type: "integer", minimum: 1, maximum: 200, default: 50 },
    },
    additionalProperties: false,
  },
};

for (const [route, { table, columns }] of Object.entries(PG_ENTITIES)) {
  app.get(`/${route}`, { schema: pgListSchema }, async (request) => {
    const { page, page_size } = request.query;
    const offset = (page - 1) * page_size;
    const colList = columns.join(", ");
    const [dataResult, countResult] = await Promise.all([
      pool.query(`SELECT ${colList} FROM ${table} ORDER BY id LIMIT $1 OFFSET $2`, [page_size, offset]),
      pool.query(`SELECT COUNT(*)::int AS total FROM ${table}`),
    ]);
    const total = countResult.rows[0].total;
    return {
      data: dataResult.rows,
      page,
      page_size,
      total_records: total,
      total_pages: Math.max(1, Math.ceil(total / page_size)),
    };
  });
}

try {
  await app.listen({ port: PORT, host: "0.0.0.0" });
} catch (err) {
  app.log.error(err);
  process.exit(1);
}
