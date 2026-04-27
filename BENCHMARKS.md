# Benchmarks

Detailed performance benchmarks for PardusDB.

## Test Configuration

Unless otherwise noted, all benchmarks use:
- Vector dimension: 128
- Number of vectors: 10,000
- Number of queries: 100
- Top-K: 10

## Performance Summary (Apple Silicon M-series)

| Operation                  | Time          |
|----------------------------|---------------|
| Single insert              | ~160 µs/doc   |
| Batch insert (1,000 docs)  | ~6 ms         |
| Query (k=10)               | ~3 µs         |

## Batch Insert Speedup

PardusDB supports batch inserts for massive performance gains:

| Batch Size | Insert (10K vecs) | Speedup vs Individual |
|------------|-------------------|----------------------|
| Individual | 1.52s | 1.0x |
| 100 | 33ms | 45x |
| 500 | 10ms | 149x |
| 1000 | 6ms | **220x** |

## PardusDB vs Neo4j

Real-world benchmark comparing PardusDB against Neo4j 5.15 for vector similarity operations.

### Results

| Database   | Insert (10K vectors) | Search (100 queries) | Single Search |
|------------|---------------------|----------------------|---------------|
| PardusDB   | 18ms (543K/s)       | 355µs (281K/s)       | 3µs           |
| Neo4j      | 35.70s (280/s)      | 153ms (650/s)        | 1ms           |

### Speedup

| Operation | PardusDB Advantage |
|-----------|-------------------|
| Insert    | **1983x faster**  |
| Search    | **431x faster**   |

### Feature Comparison

| Feature         | PardusDB              | Neo4j                |
|-----------------|-----------------------|----------------------|
| Architecture    | Embedded (SQLite-like)| Client-Server        |
| Implementation  | Rust (native)         | Java (JVM)           |
| Setup Time      | 0 seconds             | 5-10 minutes         |
| Memory Overhead | Minimal (~50MB)       | High (JVM ~1GB+)     |
| Deployment      | Single binary/file    | Server + Docker/K8s  |
| Query Language  | SQL-like              | Cypher               |

### Search Accuracy

Accuracy comparison against brute-force exact search (ground truth).

**PardusDB Results:**

| Metric      | K=10  | K=5   | K=1   | Description              |
|-------------|-------|-------|-------|--------------------------|
| Recall@K    | 99.2% | 94.8% | 68.0% | True neighbors found     |
| Precision@K | 99.2% | 94.8% | 68.0% | Correct results ratio    |
| MRR         | 0.292 | 0.439 | 0.680 | Mean Reciprocal Rank     |

**PardusDB vs Neo4j Accuracy Comparison:**

| Metric      | PardusDB | Neo4j  | Winner    |
|-------------|----------|--------|-----------|
| Recall@10   | 99.2%    | 3.0%   | PardusDB  |
| Recall@5    | 94.8%    | 2.8%   | PardusDB  |
| Recall@1    | 68.0%    | 2.0%   | PardusDB  |
| MRR         | 0.292    | 0.010  | PardusDB  |

### Running the Neo4j Benchmark

```bash
# Without Neo4j (PardusDB only)
cargo run --release --bin benchmark_neo4j

# With Neo4j comparison (requires Neo4j running)
docker run -d -p 7687:7687 -e NEO4J_AUTH=neo4j/password123 neo4j:5.15
cargo run --release --features neo4j --bin benchmark_neo4j
```

## PardusDB vs HelixDB

Comparison against HelixDB, an open-source graph-vector database built in Rust.

### Results

| Database   | Insert (10K vectors) | Search (100 queries) | Single Search |
|------------|---------------------|----------------------|---------------|
| PardusDB   | 14ms (696K/s)       | 280µs (357K/s)       | 2µs           |
| HelixDB    | 2.87s (3.5K/s)      | 17ms (5.8K/s)        | 172µs         |

### Speedup

| Operation | PardusDB Advantage |
|-----------|-------------------|
| Insert    | **200x faster**   |
| Search    | **62x faster**    |

### Feature Comparison

| Feature         | PardusDB              | HelixDB                |
|-----------------|-----------------------|------------------------|
| Architecture    | Embedded (SQLite-like)| Server (Docker)        |
| Implementation  | Rust (native)         | Rust (native)          |
| Vector Index    | HNSW (optimized)      | HNSW                   |
| Graph Support   | No                    | Yes                    |
| Deployment      | Single binary/file    | Docker + CLI           |
| Setup Time      | 0 seconds             | 5-10 minutes           |
| Memory Overhead | Minimal (~50MB)       | Docker container       |
| Query Language  | SQL-like              | HelixQL                |
| Network Latency | None (in-process)     | HTTP API overhead      |
| Persistence     | Single file (.pardus) | LMDB                   |
| License         | MIT                   | AGPL-3.0               |

### Running the HelixDB Benchmark

```bash
# Without HelixDB (PardusDB only)
cargo run --release --bin benchmark_helix

# With HelixDB comparison (requires HelixDB running)
curl -sSL "https://install.helix-db.com" | bash
mkdir helix_bench && cd helix_bench
helix init
# Add schema.hx and queries.hx for vectors
helix push dev
cargo run --release --features helix --bin benchmark_helix
```