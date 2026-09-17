# CLAUDE.md - Multi-Tier Architecture & Development Guidelines

This document serves as the core instruction guide and architectural reference for AI assistants (like Claude) and developers working on this high-performance web application codebase.

---

## 🏛️ Architecture Overview

The system utilizes a multi-tiered architecture designed for high performance, type safety, and clear separation of concerns.

```
[ Browser (SolidJS) ]
        │  ▲
   HTTPS│  │ (HTML / JSON / Server Actions)
        ▼  │
[ SolidStart / Node.js Router ]
        │  ▲
    HTTP│  │ (Reverse Proxy / Sub-routing)
        ▼  │
[ Fastify (API Gateway) ]
        │  ▲
    gRPC│  │ (Protobuf over HTTP/2)
        ▼  │
[ Rust Microservice ]
   │         │
   ▼         ▼
[Redis]   [Tuned PostgreSQL]
(Cache)   (Primary Storage)
```

---

## ⚙️ Tier-by-Tier Specifications

### 1. Presentation Tier: Browser (SolidJS)
* **Core Framework:** SolidJS (Fine-grained reactivity without a virtual DOM).
* **State Management:** Signals, Stores, and Resources for asynchronous data fetching.
* **Standards:**
  * Strict adherence to component boundaries to prevent unnecessary layout shifts.
  * Consistent use of `<Show>` and `<For>` components instead of native JavaScript array mapping or ternary operators within JSX.
  * Accessibility (a11y) enforcement via semantic HTML elements and ARIA attributes where custom interactions are required.

### 2. BFF (Backend-for-Frontend) Tier: SolidStart & Node.js Router
* **Role:** Serving as the entry point for user requests, managing Session Auth, SSR (Server-Side Rendering), and Server Actions.
* **Protocol to Browser:** HTTPS delivering HTML, JSON payloads, or executing RPC-like Server Actions.
* **Standards:**
  * Keep server-only operations strictly encapsulated inside `server$` functions or dedicated router endpoints.
  * Minimize the client-side JavaScript bundle by offloading data-heavy transformations to this layer.
  * Enforce security headers (CSP, HSTS, X-Frame-Options) at this layer.

### 3. API Gateway Tier: Fastify
* **Role:** Reverse proxy, sub-routing, rate limiting, global authentication validation, and payload validation.
* **Protocol Input:** HTTP/1.1 or HTTP/2 from the SolidStart router layer.
* **Protocol Output:** gRPC (Protobuf over HTTP/2) to downstream microservices.
* **Standards:**
  * Leverage Fastify’s internal schema compiler using JSON Schema (Ajv) for high-performance request body and query validation.
  * Centralized logging using `pino` structured format serialization.
  * Dynamic route grouping and plugin separation for maintainable codebase scaling.

### 4. Service Tier: Rust Microservices
* **Role:** Core business logic execution, heavy computation, and data orchestration.
* **Protocol Input:** gRPC incoming requests handled via `tonic` framework.
* **Standards:**
  * Enforce strict type-safety by sharing `.proto` definition files between Fastify (via client stubs) and Rust handlers.
  * Asynchronous execution model powered by `tokio` runtime.
  * Zero-cost abstractions utilized to minimize processing overhead.

### 5. Storage Tier: Redis & PostgreSQL
* **Redis (Cache):**
  * Used for session stores, rate-limiting counters, and short-lived caching layer.
  * All keys must follow a standard namespace format: `domain:subdomain:identifier` (e.g., `user:session:12345`).
* **PostgreSQL (Primary Storage):**
  * Tuned for high read/write throughput (optimized shared buffers, effective cache size, and `work_mem`).
  * Strict enforcement of indexing strategies for query optimization.
  * Database schema migrations must be written in raw, predictable SQL using structured migration tools.

---

## 🔁 Communication & Payload Protocol Standards

| Link Segment | Protocol | Payload Type | Core Standard |
| :--- | :--- | :--- | :--- |
| **Browser ➔ SolidStart** | HTTPS | HTML / JSON / Server Actions | Use Server Actions for mutations; leverage HTTP streaming for rendering data. |
| **SolidStart ➔ Fastify** | HTTP | JSON / REST-like | Match JSON schemas exactly; establish internal JWT verification context headers. |
| **Fastify ➔ Rust** | gRPC (HTTP/2) | Protocol Buffers (Protobuf) | Enforce backward compatibility on Proto files. Use `proto3` syntax rules. |
| **Rust ➔ Storage Layers** | TCP / Native | SQL / Redis Commands | Connection pooling must be configured and monitored via health endpoints. |

---

## 🔒 Security & Performance Guidelines

### Cross-Cutting Security
* **Token Management:** Store session identifier tokens in `HttpOnly`, `Secure`, `SameSite=Strict` cookies issued by the SolidStart server tier.
* **Input Sanitization:** Fastify acts as the primary barrier against malformed inputs using strict JSON validation, while Rust services perform structural data validation.
* **gRPC Transport Security:** Ensure all internal service-to-service communication is secured via mutual TLS (mTLS) in production environments.

### Performance Targets
* **TTFB (Time to First Byte):** `< 100ms` via server-side rendering strategies in SolidStart.
* **Internal RPC Latency:** `< 5ms` for gRPC communications between Fastify and Rust.
* **Database Queries:** All OLTP database read queries must execute within `< 20ms` using optimized PostgreSQL indexes.

---

## 🛠️ Development & Coding Rules for AI Assistants

1. **Type Safety & Contracts:** Never modify `.proto` files without updating both Fastify client stubs and Rust tonic handlers simultaneously.
2. **SolidJS Reactivity:** Do not destructure props from component arguments as it breaks reactivity; always access props directly or use `mergeProps`. Use `<Show>` and `<For>` strictly.
3. **Fastify Validation:** All Fastify routes must include strict Ajv JSON schemas for query parameters, params, and body payloads.
4. **Rust Error Handling:** Use idiomatic Rust error handling (`Result<T, E>`) with explicit error propagation and zero panics in production handlers.
5. **Database Queries:** Ensure all new queries are analyzed for index coverage; avoid N+1 query patterns by utilizing batching or precise SQL joins.
