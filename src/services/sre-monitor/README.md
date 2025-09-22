# SRE Monitor Service

**Version**: 0.1.0
**Status**: Production Ready
**Port**: 8080 (configurable)
**Database**: PostgreSQL with hybrid architecture support

## Overview

The SRE Monitor Service is a comprehensive Site Reliability Engineering monitoring backend for the AI-CORE platform. It provides real-time SLO tracking, error budget management, alert generation, and incident management capabilities following Google SRE best practices.

## Key Features

### 🎯 Service Level Objectives (SLOs)
- **Dynamic SLO Definition**: Create and manage SLOs with flexible metrics and time windows
- **Real-time Validation**: Continuous SLO compliance monitoring and validation
- **Historical Tracking**: Complete SLO performance history and trend analysis
- **Multi-metric Support**: Availability, latency percentiles, error rates, throughput, and custom metrics

### 💰 Error Budget Management
- **Automated Calculation**: Real-time error budget consumption tracking
- **Burn Rate Monitoring**: Intelligent burn rate analysis and alerting
- **Budget Exhaustion Alerts**: Proactive notifications when budgets are at risk
- **Time Window Flexibility**: Support for multiple time windows (1h, 24h, 7d, 30d)

### 🚨 Intelligent Alerting
- **SLO Violation Detection**: Automatic alert generation on SLO breaches
- **Severity Classification**: Smart severity assignment based on impact
- **Alert Deduplication**: Prevents alert spam with intelligent cooldown periods
- **Multi-channel Support**: Email, Slack, and webhook integrations

### 📊 Service Health Monitoring
- **Health Score Calculation**: Composite health scoring algorithm
- **Dependency Tracking**: Service dependency health monitoring
- **Performance Metrics**: Comprehensive performance and resource utilization tracking
- **Cross-service Correlation**: Service interaction and impact analysis

### 🔧 Incident Management
- **Automated Incident Creation**: Auto-create incidents from alert patterns
- **Status Tracking**: Complete incident lifecycle management
- **Resolution Correlation**: Link incidents to SLO violations and error budgets
- **Post-incident Analysis**: Historical incident data for trend analysis

## Architecture

### Core Components

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Metrics Collector │    │   SLO Validator      │    │ Error Budget        │
│                     │    │                      │    │ Tracker             │
│ - Prometheus        │    │ - SLO Calculations   │    │                     │
│ - Direct Health     │    │ - Violation Detection│    │ - Budget Calc       │
│ - Service Discovery │    │ - Burn Rate Analysis │    │ - Consumption Track │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
           │                           │                           │
           └───────────────────────────┼───────────────────────────┘
                                       │
                    ┌──────────────────────────────────────────┐
                    │            SRE Monitor API               │
                    │                                          │
                    │ /health, /metrics, /slo, /error-budget  │
                    │ /alerts, /incidents, /service-health    │
                    └──────────────────────────────────────────┘
                                       │
                    ┌──────────────────────────────────────────┐
                    │            PostgreSQL Database           │
                    │                                          │
                    │ SLOs, Metrics, Budgets, Alerts,         │
                    │ Incidents, Health Snapshots              │
                    └──────────────────────────────────────────┘
```

### Database Schema

**Core Tables**:
- `slos` - SLO definitions and configurations
- `slo_calculations` - Historical SLO calculation results
- `service_metrics` - Time-series metrics data
- `error_budgets` - Error budget tracking
- `alerts` - System alerts and notifications
- `incidents` - Incident management
- `service_health_snapshots` - Point-in-time health status

**Views**:
- `current_service_health` - Latest service health status
- `active_slo_violations` - Current SLO violations
- `error_budget_summary` - Budget status by service

## API Endpoints

### Health & Status
```http
GET /health                    # Service health check
GET /metrics                   # Prometheus metrics export
```

### SLO Management
```http
GET    /slo                    # List all SLOs
POST   /slo                    # Create new SLO
GET    /slo/{id}              # Get specific SLO
PUT    /slo/{id}              # Update SLO
```

### Error Budget
```http
GET /error-budget                    # All error budgets
GET /error-budget/{service}          # Service-specific error budget
```

### Alerts & Incidents
```http
GET  /alerts                   # List alerts
POST /alerts                   # Create alert
GET  /incidents                # List incidents
POST /incidents                # Create incident
PUT  /incidents/{id}           # Update incident
```

### Service Health
```http
GET /service-health            # All services health summary
GET /service-health/{service}  # Specific service health
```

## Configuration

### Environment Variables

**Core Configuration**:
```bash
# Database
DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_core_testing

# Server
SRE_MONITOR_HOST=0.0.0.0
SRE_MONITOR_PORT=8080
REQUEST_TIMEOUT=30

# Database Pool
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5
DB_CONNECTION_TIMEOUT=30

# Monitoring
MONITORING_COLLECTION_INTERVAL=60        # seconds
METRICS_RETENTION_DAYS=30
PROMETHEUS_ENDPOINT=http://localhost:9090

# SLO Configuration
SLO_VALIDATION_INTERVAL=300              # 5 minutes
SLO_DEFAULT_TIME_WINDOW=30d
SLO_VIOLATION_COOLDOWN=600               # 10 minutes

# Error Budget
ERROR_BUDGET_CALCULATION_INTERVAL=3600   # 1 hour
ERROR_BUDGET_DEFAULT_PERCENTAGE=1.0
BURN_RATE_WARNING_THRESHOLD=2.0
BURN_RATE_CRITICAL_THRESHOLD=5.0
BURN_RATE_EMERGENCY_THRESHOLD=10.0

# External Services
API_GATEWAY_URL=http://localhost:8000
PROMETHEUS_URL=http://localhost:9090
GRAFANA_URL=http://localhost:3001
```

**Alerting Configuration**:
```bash
# Email Alerts
ENABLE_EMAIL_ALERTS=false
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your_email@gmail.com
SMTP_PASSWORD=your_password
FROM_EMAIL=alerts@ai-core.dev
TO_EMAILS=team@ai-core.dev,oncall@ai-core.dev

# Slack Alerts
ENABLE_SLACK_ALERTS=true
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...
SLACK_CHANNEL=#alerts
SLACK_USERNAME=SRE Monitor

# Webhook Alerts
ENABLE_WEBHOOK_ALERTS=false
WEBHOOK_URL=https://your-webhook-endpoint.com/alerts
WEBHOOK_TIMEOUT=30
WEBHOOK_RETRY_COUNT=3
```

## Usage Examples

### Creating an SLO

```bash
curl -X POST http://localhost:8080/slo \
  -H "Content-Type: application/json" \
  -d '{
    "name": "API Gateway Availability",
    "description": "API Gateway should be available 99.9% of the time",
    "service_name": "api-gateway",
    "metric_name": "availability",
    "target_percentage": 99.9,
    "time_window": "30d",
    "threshold_value": 99.9,
    "operator": "gte"
  }'
```

### Checking Error Budgets

```bash
# All error budgets
curl http://localhost:8080/error-budget

# Specific service
curl http://localhost:8080/error-budget/api-gateway?window=30d
```

### Creating an Alert

```bash
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{
    "service_name": "api-gateway",
    "alert_type": "slo_violation",
    "severity": "high",
    "message": "API Gateway availability dropped below 99.9%"
  }'
```

## Default SLOs

The service comes pre-configured with sensible SLOs for AI-CORE services:

| Service | SLO | Target | Time Window |
|---------|-----|---------|-------------|
| api-gateway | Availability | 99.9% | 30d |
| api-gateway | Latency P95 | <500ms | 30d |
| intent-parser-server | Availability | 99.5% | 30d |
| mcp-manager-server | Availability | 99.5% | 30d |
| federation-server | Availability | 99.9% | 30d |
| test-data-api | Availability | 99.0% | 30d |

## Monitoring Integration

### Prometheus Metrics

The service exports metrics compatible with Prometheus:

- `sre_monitor_requests_total` - Total requests processed
- `sre_monitor_request_duration_seconds` - Request duration histogram
- `sre_monitor_active_connections` - Active database connections
- `sre_monitor_error_rate` - Current error rate
- `sre_monitor_slo_compliance` - SLO compliance percentage

### Grafana Dashboards

Pre-built Grafana dashboards are available in `monitoring/grafana/dashboards/`:

- **SRE Overview** - High-level SLO and error budget status
- **Service Health** - Detailed service health metrics
- **Alert Management** - Alert trends and incident tracking

## Development

### Running Locally

```bash
# Start dependencies
docker-compose up -d postgres

# Set environment variables
export DATABASE_URL="postgresql://postgres:password@localhost:5432/ai_core_testing"
export RUST_LOG=debug

# Run database migrations
sqlx migrate run

# Start the service
cargo run --bin sre-monitor
```

### Testing

```bash
# Run all tests
cargo test

# Run integration tests (requires database)
cargo test --features integration-tests

# Run with coverage
cargo tarpaulin --out html
```

### Database Operations

```bash
# Create new migration
sqlx migrate add create_new_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Reset database (development only)
sqlx database reset
```

## Deployment

### Docker

```dockerfile
FROM rust:1.70-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin sre-monitor

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/sre-monitor /usr/local/bin/
CMD ["sre-monitor"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sre-monitor
spec:
  replicas: 2
  selector:
    matchLabels:
      app: sre-monitor
  template:
    metadata:
      labels:
        app: sre-monitor
    spec:
      containers:
      - name: sre-monitor
        image: ai-core/sre-monitor:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-credentials
              key: url
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

## Troubleshooting

### Common Issues

**Database Connection Issues**:
```bash
# Check database connectivity
psql $DATABASE_URL -c "SELECT 1;"

# Verify migrations
sqlx migrate info
```

**Missing Metrics**:
```bash
# Check Prometheus connectivity
curl http://localhost:9090/api/v1/query?query=up

# Verify service discovery
curl http://localhost:8080/service-health
```

**SLO Not Calculating**:
```bash
# Check if metrics exist
curl http://localhost:8080/metrics

# Verify SLO configuration
curl http://localhost:8080/slo
```

### Performance Tuning

**Database Optimization**:
```sql
-- Analyze query performance
EXPLAIN ANALYZE SELECT * FROM service_metrics WHERE service_name = 'api-gateway';

-- Update statistics
ANALYZE service_metrics;

-- Vacuum if needed
VACUUM ANALYZE service_metrics;
```

**Memory Usage**:
```bash
# Monitor memory usage
cargo build --release
valgrind --tool=massif target/release/sre-monitor
```

## Contributing

### Code Style

- Use `rustfmt` for code formatting
- Follow the existing error handling patterns
- Add tests for new functionality
- Update documentation for API changes

### Testing Guidelines

- Unit tests for business logic
- Integration tests for database operations
- Load tests for performance validation
- Chaos tests for resilience validation

### Performance Requirements

- **Response Time**: < 100ms for health checks, < 500ms for complex queries
- **Throughput**: Handle 1000+ requests per second
- **Memory Usage**: < 512MB under normal load
- **Database Connections**: Efficient connection pooling

## License

MIT License - see LICENSE file for details.

## Support

For issues and questions:
- GitHub Issues: [AI-CORE Repository](https://github.com/ai-core/issues)
- Slack: #sre-monitoring
- Email: sre@ai-core.dev

---

**Last Updated**: 2025-01-11
**Maintainer**: AI-CORE SRE Team
**Version**: 0.1.0
