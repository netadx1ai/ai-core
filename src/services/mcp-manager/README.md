# MCP Manager Service

The MCP Manager Service is a core component of the AI-CORE Intelligent Automation Platform that provides centralized management and orchestration of Model Context Protocol (MCP) servers. It handles server registry, lifecycle management, health monitoring, load balancing, and protocol communication.

## Features

### Core Functionality
- **Server Registry**: Centralized registration and management of MCP server instances
- **Lifecycle Management**: Start, stop, restart, and monitor MCP servers
- **Health Monitoring**: Continuous health checks with automatic recovery
- **Load Balancing**: Distribute requests across healthy server instances
- **Protocol Communication**: Handle MCP protocol messages and routing
- **Circuit Breaker**: Automatic failover and recovery mechanisms

### Advanced Features
- **Multiple Load Balancing Strategies**: Round-robin, least connections, weighted, random, IP hash, consistent hash
- **Sticky Sessions**: Session affinity for stateful applications
- **Auto-Discovery**: Automatic detection and registration of MCP servers
- **Metrics Collection**: Comprehensive operational metrics and monitoring
- **Rate Limiting**: Configurable rate limits for API protection
- **Authentication**: JWT and API key authentication support

## Architecture

```text
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   API Gateway   │────│  MCP Manager    │────│  MCP Servers    │
│                 │    │   Service       │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                       ┌─────────────────┐
                       │ Intent Parser   │
                       │   Service       │
                       └─────────────────┘
```

The MCP Manager Service acts as an intermediary between the API Gateway and MCP servers, providing:
- Request routing and load balancing
- Health monitoring and failover
- Protocol translation and validation
- Metrics collection and reporting

## Configuration

### Basic Configuration

The service is configured via YAML files. See `config/mcp-manager.yaml` for a complete example:

```yaml
environment: "development"

server:
  host: "0.0.0.0"
  port: 8083
  max_connections: 1000

mcp:
  max_servers: 100
  protocol_version: "2024-11-05"
  auto_restart: true

health:
  enabled: true
  check_interval_seconds: 30
  failure_threshold: 3

load_balancer:
  strategy: "round_robin"
  circuit_breaker: true
```

### Environment Variables

Key settings can be overridden with environment variables:

- `MCP_MANAGER_PORT`: Service port (default: 8083)
- `MCP_MANAGER_LOG_LEVEL`: Logging level (default: info)
- `MCP_MANAGER_MAX_SERVERS`: Maximum number of servers (default: 100)
- `DATABASE_URL`: PostgreSQL connection string
- `REDIS_URL`: Redis connection string

## API Reference

### Server Management

#### Register Server
```http
POST /servers
Content-Type: application/json

{
  "name": "my-mcp-server",
  "version": "1.0.0",
  "server_type": "filesystem",
  "config": {
    "endpoint": "http://localhost:8080",
    "port": 8080,
    "host": "localhost"
  },
  "capabilities": {
    "protocol_version": "2024-11-05",
    "tools": [],
    "resources": [],
    "prompts": []
  }
}
```

#### List Servers
```http
GET /servers?status=running&page=1&page_size=20
```

#### Get Server Details
```http
GET /servers/{server_id}
```

#### Update Server
```http
PUT /servers/{server_id}
Content-Type: application/json

{
  "name": "updated-server-name",
  "config": { ... }
}
```

#### Delete Server
```http
DELETE /servers/{server_id}
```

### Health Monitoring

#### Health Check
```http
GET /health
```

#### Detailed Health
```http
GET /health/detailed
```

#### Server Health
```http
GET /servers/{server_id}/health
```

#### Manual Health Check
```http
POST /servers/{server_id}/health
```

### Load Balancer

#### Select Server
```http
POST /load-balancer/select
Content-Type: application/json

{
  "request_id": "unique-request-id",
  "client_ip": "192.168.1.100",
  "session_id": "session-123"
}
```

#### Load Balancer Statistics
```http
GET /load-balancer/stats
```

#### Update Server Weights
```http
PUT /load-balancer/weights
Content-Type: application/json

{
  "weights": {
    "server-id-1": 100,
    "server-id-2": 200
  }
}
```

### Protocol Communication

#### Send MCP Request
```http
POST /protocol/request
Content-Type: application/json

{
  "server_id": "server-uuid",
  "method": "tools/list",
  "params": {}
}
```

#### Send MCP Notification
```http
POST /protocol/notification
Content-Type: application/json

{
  "server_id": "server-uuid",
  "method": "logging/setLevel",
  "params": {
    "level": "debug"
  }
}
```

#### Batch Requests
```http
POST /protocol/batch
Content-Type: application/json

{
  "requests": [
    {
      "server_id": "server-uuid-1",
      "method": "tools/list"
    },
    {
      "server_id": "server-uuid-2",
      "method": "resources/list"
    }
  ],
  "parallel": true
}
```

### Metrics

#### Prometheus Metrics
```http
GET /metrics
```

#### Service Statistics
```http
GET /registry/stats
```

## Development

### Prerequisites

- Rust 1.75.0 or later
- PostgreSQL 13+ (optional, for persistent storage)
- Redis 6+ (optional, for caching and session storage)

### Building

```bash
# Build the service
cargo build --package mcp-manager

# Build with all features
cargo build --package mcp-manager --features "metrics"

# Run tests
cargo test --package mcp-manager

# Run with development configuration
cargo run --package mcp-manager -- --config config/mcp-manager.yaml --dev
```

### Development Mode

In development mode, the service provides additional features:
- Enhanced logging with debug information
- Relaxed authentication requirements
- Hot-reload configuration support
- Development-friendly CORS settings

```bash
cargo run --package mcp-manager -- --dev
```

### Testing

The service includes comprehensive test coverage:

```bash
# Run all tests
cargo test --package mcp-manager

# Run specific test modules
cargo test --package mcp-manager health::tests
cargo test --package mcp-manager registry::tests
cargo test --package mcp-manager load_balancer::tests

# Run with output
cargo test --package mcp-manager -- --nocapture
```

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package mcp-manager

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mcp-manager /usr/local/bin/
COPY config/mcp-manager.yaml /etc/mcp-manager/config.yaml
EXPOSE 8083
CMD ["mcp-manager", "--config", "/etc/mcp-manager/config.yaml"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mcp-manager
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mcp-manager
  template:
    metadata:
      labels:
        app: mcp-manager
    spec:
      containers:
      - name: mcp-manager
        image: ai-core/mcp-manager:latest
        ports:
        - containerPort: 8083
        env:
        - name: MCP_MANAGER_PORT
          value: "8083"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database-secret
              key: url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: redis-secret
              key: url
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8083
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8083
          initialDelaySeconds: 5
          periodSeconds: 5
```

### System Service

```ini
# /etc/systemd/system/mcp-manager.service
[Unit]
Description=MCP Manager Service
After=network.target

[Service]
Type=simple
User=mcp-manager
Group=mcp-manager
WorkingDirectory=/opt/mcp-manager
ExecStart=/opt/mcp-manager/bin/mcp-manager --config /opt/mcp-manager/config/mcp-manager.yaml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

## Monitoring

### Metrics

The service exposes Prometheus metrics on `/metrics`:

- `mcp_manager_servers_total`: Total number of registered servers
- `mcp_manager_servers_healthy`: Number of healthy servers
- `mcp_manager_requests_total`: Total HTTP requests processed
- `mcp_manager_request_duration_seconds`: Request duration histogram
- `mcp_manager_health_checks_total`: Total health checks performed
- `mcp_manager_load_balancer_requests_total`: Load balancer requests

### Logging

Structured JSON logging with configurable levels:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "INFO",
  "message": "Server registered successfully",
  "server_id": "550e8400-e29b-41d4-a716-446655440000",
  "server_name": "my-mcp-server",
  "target": "mcp_manager::registry"
}
```

### Health Checks

Multiple health check endpoints:

- `/health`: Basic service health
- `/health/detailed`: Comprehensive health information
- `/servers/{id}/health`: Individual server health

## Troubleshooting

### Common Issues

#### Server Registration Fails
```bash
# Check server configuration
curl -X POST http://localhost:8083/servers \
  -H "Content-Type: application/json" \
  -d '{"name": "test", "version": "1.0.0", ...}'

# Validate configuration
mcp-manager --validate-config --config config/mcp-manager.yaml
```

#### Health Checks Failing
```bash
# Check server connectivity
curl http://your-mcp-server:8080/health

# Manual health check
curl -X POST http://localhost:8083/servers/{server-id}/health
```

#### Load Balancer Issues
```bash
# Check load balancer statistics
curl http://localhost:8083/load-balancer/stats

# Verify server selection
curl -X POST http://localhost:8083/load-balancer/select \
  -H "Content-Type: application/json" \
  -d '{"request_id": "test-123"}'
```

### Debug Mode

Enable debug logging for troubleshooting:

```bash
RUST_LOG=debug mcp-manager --config config/mcp-manager.yaml
```

### Performance Tuning

#### Database Optimization
- Increase connection pool size for high load
- Configure appropriate timeouts
- Use connection pooling with pgbouncer

#### Memory Usage
- Adjust `max_servers` based on available memory
- Configure garbage collection settings
- Monitor heap usage with metrics

#### Network Performance
- Use HTTP/2 for better multiplexing
- Enable compression for large responses
- Configure appropriate timeout values

## Contributing

### Code Style

The project follows standard Rust conventions:
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow the project's error handling patterns

### Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Update documentation
6. Submit a pull request

### Testing Guidelines

- Write unit tests for all new functions
- Add integration tests for API endpoints
- Include error case testing
- Maintain >90% test coverage

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## Support

For support and questions:
- Create an issue in the repository
- Check the documentation at [docs.ai-core.dev](https://docs.ai-core.dev)
- Join our community discussions

## Related Services

- [Intent Parser Service](../intent-parser/README.md): Natural language processing for automation requests
- [API Gateway](../api-gateway/README.md): Main entry point for all API requests
- [Security Service](../security/README.md): Authentication and authorization
