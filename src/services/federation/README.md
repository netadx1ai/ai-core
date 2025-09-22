# Federation Service

The Federation Service is a comprehensive multi-tenant client management and provider orchestration system for the AI-CORE Intelligent Automation Platform. It provides centralized federation capabilities, cost optimization, schema translation, workflow execution, and MCP server integration.

## Overview

The Federation Service enables:

- **Multi-tenant Client Management**: Registration, authentication, and lifecycle management
- **Provider Selection & Cost Optimization**: Intelligent provider selection with cost-quality balance
- **Schema Translation**: Automatic compatibility layer between different client requirements
- **Workflow Execution**: Federated workflow orchestration with Temporal.io integration
- **MCP Server Integration**: Proxy and protocol translation for client MCP servers
- **Budget Management**: Cost tracking, budget enforcement, and optimization

## Architecture

```
Federation Service
├── Client Registry (multi-tenant management)
├── Provider Registry (provider discovery and selection)
├── Schema Translator (compatibility layer)
├── Workflow Engine (Temporal.io integration)
├── Proxy Layer (MCP server integration)
└── Cost Optimizer (intelligent provider selection)
```

## Features

### Client Management
- Client registration with tier-based resource allocation
- JWT and API key authentication
- Rate limiting and resource usage tracking
- Multi-tenant isolation and security

### Provider Selection
- Intelligent cost-optimized provider selection
- Quality metrics and performance tracking
- Multiple selection strategies (cost, quality, balanced)
- Real-time provider health monitoring

### Schema Translation
- Automatic schema compatibility translation
- Version-specific translation rules
- Caching for performance optimization
- Validation and error handling

### Workflow Execution
- Temporal.io integration for distributed workflows
- Federated workflow execution across providers
- Step-by-step execution with retry policies
- Resource usage and cost tracking

### MCP Integration
- Proxy layer for client MCP servers
- Protocol translation and compatibility
- Connection pooling and management
- Request routing and load balancing

## Configuration

The service uses YAML configuration with environment variable overrides:

```yaml
# Server configuration
server:
  host: "0.0.0.0"
  port: 8084
  requestTimeout: 30

# Database configuration
database:
  url: "postgresql://federation:federation@localhost:5432/federation"
  maxConnections: 20

# Redis configuration
redis:
  url: "redis://localhost:6379"
  keyPrefix: "federation:"

# Cost optimization
costOptimization:
  enabled: true
  strategy: "balanced"
  scoringWeights:
    cost: 0.3
    quality: 0.3
    performance: 0.2
    reliability: 0.2
```

## API Endpoints

### Client Management
- `POST /clients` - Register new client
- `GET /clients` - List clients with filtering
- `GET /clients/{id}` - Get client details
- `PUT /clients/{id}` - Update client configuration
- `DELETE /clients/{id}` - Delete client
- `GET /clients/{id}/usage` - Get client usage statistics

### Provider Management
- `POST /providers` - Register new provider
- `GET /providers` - List providers
- `GET /providers/{id}` - Get provider details
- `PUT /providers/{id}` - Update provider
- `DELETE /providers/{id}` - Delete provider
- `POST /providers/select` - Select optimal provider

### Schema Translation
- `POST /schema/translate` - Translate schema data
- `GET /schema/translations` - List available translations
- `GET /schema/translations/{id}` - Get translation details

### Workflow Execution
- `POST /workflows` - Create workflow
- `GET /workflows` - List workflows
- `POST /workflows/{id}/execute` - Execute workflow
- `GET /workflows/{id}/status` - Get workflow status
- `POST /workflows/{id}/cancel` - Cancel workflow

### MCP Proxy
- `POST /proxy/mcp/{server_id}/*path` - Proxy MCP requests
- `GET /proxy/mcp/{server_id}/*path` - Proxy MCP requests

### Cost Optimization
- `POST /cost/optimize` - Optimize provider selection
- `GET /cost/reports` - Get cost reports
- `GET /cost/reports/{client_id}` - Get client cost report

### System
- `GET /health` - Basic health check
- `GET /health/detailed` - Detailed health report
- `GET /status` - Service status
- `GET /metrics` - Prometheus metrics

## Client Tiers

### Free Tier
- 10 requests/minute, 600/hour, 14,400/day
- 2 concurrent connections
- 100MB daily data transfer
- 1GB storage

### Professional Tier
- 100 requests/minute, 6,000/hour, 144,000/day
- 10 concurrent connections
- 10GB daily data transfer
- 100GB storage

### Enterprise Tier
- 1,000 requests/minute, 60,000/hour, 1,440,000/day
- 100 concurrent connections
- 100GB daily data transfer
- 1TB storage

## Development

### Prerequisites
- Rust 1.75+
- PostgreSQL 14+
- Redis 6+
- Temporal server (optional)

### Building
```bash
# Development build
cargo build --package federation

# Release build
cargo build --package federation --release --features "metrics"

# Run with development settings
cargo run --package federation -- --dev
```

### Testing
```bash
# Run all tests
cargo test --package federation

# Run with output
cargo test --package federation -- --nocapture

# Run specific test
cargo test --package federation test_name
```

### Configuration
```bash
# Use custom config file
cargo run --package federation -- --config custom-config.yaml

# Override settings
cargo run --package federation -- --host 127.0.0.1 --port 9000

# Validate configuration
cargo run --package federation -- --validate-config
```

## Deployment

### Docker
```bash
# Build image
docker build -t federation-service .

# Run container
docker run -p 8084:8084 federation-service
```

### Environment Variables
```bash
export DATABASE_URL="postgresql://user:pass@host:5432/db"
export REDIS_URL="redis://host:6379"
export JWT_SECRET="your-secret-key"
export LOG_LEVEL="info"
```

### Docker Compose
```yaml
version: '3.8'
services:
  federation:
    build: .
    ports:
      - "8084:8084"
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/federation
      - REDIS_URL=redis://redis:6379
    depends_on:
      - db
      - redis
```

## Monitoring

### Metrics
The service exposes Prometheus metrics at `/metrics`:
- Request counts and durations
- Client registration and usage
- Provider selection and performance
- Schema translation statistics
- Workflow execution metrics
- Cost optimization data

### Health Checks
- `/health` - Basic liveness check
- `/health/detailed` - Comprehensive health report with component status

### Logging
Structured JSON logging with configurable levels:
```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "INFO",
  "message": "Client registered successfully",
  "client_id": "uuid",
  "client_name": "Example Client"
}
```

## Security

### Authentication
- JWT token authentication
- API key authentication
- OAuth 2.0 support (configurable)

### Authorization
- Role-based access control (RBAC)
- Resource-level permissions
- Tenant isolation

### Rate Limiting
- Per-client rate limits based on tier
- Global rate limits for system protection
- Configurable limits and windows

## Cost Optimization

### Strategies
- **Cost Minimizer**: Select cheapest available provider
- **Quality Preserver**: Maintain quality requirements while minimizing cost
- **Balanced**: Optimize for cost-quality ratio
- **Custom**: Configurable weights for different factors

### Budget Management
- Monthly and daily budget limits
- Real-time budget tracking
- Automatic alerts at configurable thresholds
- Cost reporting and analytics

## Troubleshooting

### Common Issues

#### Connection Errors
```bash
# Check database connectivity
psql $DATABASE_URL -c "SELECT 1;"

# Check Redis connectivity
redis-cli -u $REDIS_URL ping
```

#### Configuration Issues
```bash
# Validate configuration
cargo run --package federation -- --validate-config

# Check environment variables
env | grep -E "(DATABASE_URL|REDIS_URL|JWT_SECRET)"
```

#### Performance Issues
- Monitor `/metrics` endpoint for bottlenecks
- Check database connection pool utilization
- Review rate limiting and client usage patterns
- Analyze cost optimization effectiveness

### Logs Analysis
```bash
# Filter by log level
docker logs federation-service | jq 'select(.level == "ERROR")'

# Filter by client
docker logs federation-service | jq 'select(.client_id == "uuid")'

# Monitor health checks
docker logs federation-service | jq 'select(.message | contains("health"))'
```

## Contributing

### Code Style
- Follow Rust standard conventions
- Use `cargo fmt` for formatting
- Run `cargo clippy` for linting
- Ensure tests pass with `cargo test`

### Documentation
- Update README for new features
- Document API changes in OpenAPI specs
- Add inline documentation for public APIs

### Testing
- Write unit tests for new functionality
- Add integration tests for API endpoints
- Include performance tests for critical paths

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## Support

For support and questions:
- Create an issue in the repository
- Check the [documentation](../../docs/)
- Review the [API contracts](../../api-contracts/)
