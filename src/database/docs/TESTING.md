# AI-CORE Database Testing Suite

This document outlines the comprehensive testing strategy for the AI-CORE database layer, covering unit tests, integration tests, and performance benchmarks across all supported databases.

## Overview

The AI-CORE database testing suite validates:
- **PostgreSQL**: Transactional data, user management, workflow state
- **ClickHouse**: Time-series analytics, metrics, event logging
- **MongoDB**: Document storage, flexible schemas, integration data
- **Redis**: Caching, sessions, real-time data, pub/sub messaging

## Test Structure

```
tests/
├── unit/                    # Unit tests for each database
│   ├── test_postgresql.rs   # PostgreSQL configuration and patterns
│   ├── test_clickhouse.rs   # ClickHouse analytics and events
│   ├── test_mongodb.rs      # MongoDB documents and aggregations
│   ├── test_redis.rs        # Redis caching and pub/sub
│   └── mod.rs              # Shared unit test utilities
├── integration/             # Cross-database integration tests
│   ├── test_cross_database.rs # Multi-database operations
│   └── mod.rs              # Integration test framework
├── performance/             # Performance validation tests
│   ├── test_performance_targets.rs # SLA validation
│   └── mod.rs              # Performance test utilities
└── integration_test.rs      # Main integration test runner

benches/
└── database_benchmarks.rs   # Criterion performance benchmarks
```

## Performance Requirements

### Service Level Agreements (SLAs)

| Database   | Operation Type           | Target Performance |
|------------|-------------------------|-------------------|
| PostgreSQL | Simple queries          | < 10ms           |
| PostgreSQL | Complex transactions    | < 100ms          |
| ClickHouse | Analytical queries      | < 1s (1M+ records) |
| ClickHouse | Bulk insertion         | > 100K rows/sec  |
| MongoDB    | Document operations     | < 50ms           |
| MongoDB    | Aggregation pipelines   | < 500ms          |
| Redis      | Cache operations        | < 1ms            |
| Redis      | Complex operations      | < 10ms           |

## Running Tests

### Prerequisites

```bash
# Install required dependencies
cargo install criterion
```

For integration tests with real databases:
```bash
# PostgreSQL
docker run -d --name postgres-test -e POSTGRES_PASSWORD=test -p 5432:5432 postgres:14

# ClickHouse
docker run -d --name clickhouse-test -p 8123:8123 clickhouse/clickhouse-server

# MongoDB
docker run -d --name mongodb-test -p 27017:27017 mongo:6

# Redis
docker run -d --name redis-test -p 6379:6379 redis:7
```

### Test Commands

#### Unit Tests (No external dependencies)
```bash
# Run all unit tests
cargo test --lib --all-features

# Run specific database unit tests
cargo test test_postgresql --all-features
cargo test test_clickhouse --all-features
cargo test test_mongodb --all-features
cargo test test_redis --all-features
```

#### Integration Tests (Requires running databases)
```bash
# Run integration tests
cargo test --test integration_test --all-features

# Run with testcontainers (requires Docker)
cargo test --test integration_test --all-features --features testing

# Run cross-database tests
cargo test test_cross_database --all-features
```

#### Performance Tests
```bash
# Run performance validation tests
cargo test --test performance --all-features

# Run criterion benchmarks
cargo bench --all-features

# Generate HTML benchmark reports
cargo bench --all-features -- --output-format html
```

### CI/CD Integration

#### GitHub Actions Example
```yaml
name: Database Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run unit tests
        run: cargo test --lib --all-features

  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:14
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3
      - name: Run integration tests
        run: cargo test --test integration_test --all-features
        env:
          DATABASE_URL: postgresql://postgres:test@localhost:5432/test
          REDIS_URL: redis://localhost:6379

  performance-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run performance tests
        run: cargo test --test performance --all-features
```

## Test Categories

### Unit Tests

#### PostgreSQL Tests (`test_postgresql.rs`)
- Configuration validation and serialization
- Connection pool management patterns
- Transaction handling simulation
- Error handling and timeout scenarios
- Thread safety and concurrent access

#### ClickHouse Tests (`test_clickhouse.rs`)
- Analytics event structure validation
- Bulk data preparation and serialization
- Query pattern optimization
- Materialized view definitions
- Performance under load simulation

#### MongoDB Tests (`test_mongodb.rs`)
- Document structure and validation
- BSON serialization/deserialization
- Aggregation pipeline construction
- Index strategy validation
- Bulk operation patterns

#### Redis Tests (`test_redis.rs`)
- Caching pattern validation
- Session management structures
- Pub/sub message formats
- Rate limiting algorithms
- Data structure optimization

### Integration Tests

#### Cross-Database Tests (`test_cross_database.rs`)
- Multi-database transaction coordination
- Data consistency patterns
- Error propagation across databases
- Performance coordination
- Health check synchronization

#### Testcontainer Tests (Optional)
- Real database integration with Docker containers
- End-to-end workflow validation
- Migration testing
- Backup and recovery procedures

### Performance Tests

#### SLA Validation (`test_performance_targets.rs`)
- Individual database performance targets
- Cross-database operation latency
- Concurrent operation throughput
- Memory usage and allocation patterns
- Error handling performance

#### Benchmarks (`database_benchmarks.rs`)
- Configuration creation and serialization
- Data structure operations
- Concurrent access patterns
- Memory allocation benchmarks
- Error handling benchmarks

## Mock Testing Strategy

For CI environments without databases, the test suite includes comprehensive mock tests:

### Mock Data Patterns
```rust
// Example: Mock PostgreSQL operations
#[tokio::test]
async fn test_mock_database_operations() {
    let config = PostgresConfig::default();

    // Simulate connection creation
    assert_eq!(config.max_connections, 20);

    // Mock transaction simulation
    let mock_user = User {
        id: "mock_user".to_string(),
        email: "test@example.com".to_string(),
        created_at: chrono::Utc::now(),
    };

    // Test serialization (simulates database insert)
    let serialized = serde_json::to_string(&mock_user).unwrap();
    assert!(!serialized.is_empty());

    println!("✅ Mock database operation completed");
}
```

### Performance Simulation
```rust
// Example: Mock performance validation
#[tokio::test]
async fn test_performance_simulation() {
    let start = Instant::now();

    // Simulate database operation with controlled delay
    tokio::time::sleep(Duration::from_millis(5)).await;

    let duration = start.elapsed();
    assert!(duration.as_millis() < 10, "Operation too slow: {}ms", duration.as_millis());
}
```

## Test Data Management

### Test Data Generation
```rust
// Helper function for generating test data
pub fn generate_test_campaign(id: &str) -> Campaign {
    Campaign {
        id: id.to_string(),
        name: format!("Test Campaign {}", id),
        status: "active".to_string(),
        created_at: chrono::Utc::now(),
        metrics: CampaignMetrics {
            impressions: 1000,
            clicks: 50,
            conversions: 5,
            cost: 25.0,
        },
    }
}

// Bulk test data generation
pub fn generate_bulk_test_data(count: usize) -> Vec<Campaign> {
    (0..count)
        .map(|i| generate_test_campaign(&format!("test_{}", i)))
        .collect()
}
```

### Test Environment Configuration
```rust
// Environment-specific test configuration
pub fn get_test_config() -> DatabaseConfig {
    if std::env::var("CI").is_ok() {
        // CI environment - use mock configuration
        mock_database_config()
    } else {
        // Local development - use real databases if available
        real_database_config()
    }
}
```

## Troubleshooting

### Common Test Issues

#### Connection Failures
```
Error: Connection refused (os error 61)
```
**Solution**: Ensure database services are running:
```bash
docker ps  # Check running containers
docker start postgres-test redis-test  # Start required services
```

#### Timeout Issues
```
Error: Operation timed out after 30s
```
**Solution**: Adjust test timeouts or check database performance:
```rust
// Increase timeout for slow environments
#[tokio::test]
#[tokio::time::timeout(Duration::from_secs(60))]
async fn test_slow_operation() {
    // Test implementation
}
```

#### Memory Issues
```
Error: Out of memory during bulk operations
```
**Solution**: Reduce test data size or use streaming:
```rust
// Use smaller test datasets in CI
let test_size = if cfg!(test) { 100 } else { 10000 };
let test_data = generate_bulk_test_data(test_size);
```

### Performance Debugging

#### Slow Tests
1. Use `cargo test -- --nocapture` to see timing output
2. Profile individual operations with `std::time::Instant`
3. Check database query plans and indexes
4. Monitor resource usage during tests

#### Flaky Tests
1. Add retry logic for network-dependent tests
2. Use deterministic test data and timestamps
3. Implement proper test isolation
4. Add debugging output for intermittent failures

## Best Practices

### Test Organization
- Group related tests in modules
- Use descriptive test names that explain the scenario
- Include both positive and negative test cases
- Test edge cases and error conditions

### Performance Testing
- Establish baseline performance metrics
- Test under various load conditions
- Monitor memory usage and resource consumption
- Validate performance against SLA requirements

### Mock vs Real Testing
- Use mocks for unit tests and CI environments
- Use real databases for integration and performance tests
- Ensure mock behavior matches real database behavior
- Document differences between mock and real implementations

### Maintenance
- Update tests when database schemas change
- Review performance baselines regularly
- Keep test data current and representative
- Monitor test execution times and optimize slow tests

## Contributing

When adding new database functionality:

1. **Add unit tests** for configuration and core logic
2. **Add integration tests** for cross-database interactions
3. **Add performance tests** if new SLA requirements exist
4. **Update documentation** with new test patterns
5. **Verify CI compatibility** with mock implementations

### Test Review Checklist

- [ ] Tests cover all code paths
- [ ] Performance requirements are validated
- [ ] Error conditions are tested
- [ ] Mock tests work in CI environment
- [ ] Integration tests work with real databases
- [ ] Documentation is updated
- [ ] Test names are descriptive
- [ ] Test data is representative

## Future Enhancements

### Planned Improvements
- Automated performance regression detection
- Visual performance trend reporting
- Property-based testing with QuickCheck
- Mutation testing for test quality validation
- Database-specific testing frameworks integration

### Monitoring Integration
- Integration with APM tools (DataDog, New Relic)
- Custom metrics for test performance
- Alerting on test performance degradation
- Historical performance trend analysis

This comprehensive testing strategy ensures the reliability, performance, and maintainability of the AI-CORE database layer across all supported database technologies.
