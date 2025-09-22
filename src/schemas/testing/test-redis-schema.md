# AI-PLATFORM Test Redis Schema
**FAANG-Enhanced Testing Infrastructure - Database Agent Implementation T4.1**

Comprehensive Redis schema for:
- High-speed test session management
- Real-time test coordination and locking
- Test result caching and temporary storage
- Cross-browser test synchronization
- Performance metrics buffering
- Test environment state management
- AI model response caching
- Live test monitoring and notifications

## ============================================================================
## Redis Database Organization
## ============================================================================

```bash
# Database 0: Test Sessions and Authentication
# Database 1: Test Coordination and Locking
# Database 2: Test Results Cache
# Database 3: Performance Metrics Buffer
# Database 4: AI Model Cache
# Database 5: Real-time Notifications
# Database 6: Test Environment State
# Database 7: Temporary Test Data
```

## ============================================================================
## Database 0: Test Sessions and Authentication
## ============================================================================

### Test User Sessions
```redis
# Active test sessions with TTL
SET session:test:{session_id} '{
  "userId": "user_123",
  "username": "test_admin",
  "role": "admin",
  "permissions": ["user:create", "test:execute", "admin:access"],
  "environment": "testing",
  "browser": "chromium",
  "platform": "desktop",
  "testSuite": "authentication-suite",
  "loginTime": "2025-01-11T12:00:00Z",
  "lastActivity": "2025-01-11T12:15:00Z",
  "ipAddress": "127.0.0.1",
  "userAgent": "Mozilla/5.0...",
  "deviceFingerprint": "abc123def456",
  "mfaVerified": true,
  "trustedDevice": false
}' EX 3600

# Session expiration tracking
ZADD session:expiry {timestamp} session:test:{session_id}

# User active sessions (for multi-session management)
SADD user:sessions:test_admin session:test:{session_id}

# Session blacklist (for immediate invalidation)
SET session:blacklist:{session_id} "revoked" EX 86400

# MFA temporary codes
SET mfa:code:{user_id} "123456" EX 300
SET mfa:backup:{user_id}:{code} "used" EX 3600

# Remember me tokens
SET remember:token:{token} '{
  "userId": "user_123",
  "created": "2025-01-11T12:00:00Z",
  "deviceInfo": "Chrome on Windows"
}' EX 2592000  # 30 days

# Password reset tokens
SET pwd:reset:{token} '{
  "userId": "user_123",
  "email": "test@example.com",
  "expires": "2025-01-11T13:00:00Z"
}' EX 1800  # 30 minutes
```

### Rate Limiting and Security
```redis
# Login attempt tracking
INCR rate:login:{ip} EX 3600
INCR rate:login:{user} EX 3600

# Failed login attempts
INCR failed:login:{user} EX 900
SET lockout:{user} "locked" EX 900

# API rate limiting
INCR rate:api:{user_id}:{endpoint} EX 3600

# Security events
LPUSH security:events '{
  "type": "failed_login",
  "userId": "user_123",
  "ip": "192.168.1.100",
  "timestamp": "2025-01-11T12:00:00Z",
  "details": {"attempts": 3}
}'
LTRIM security:events 0 999  # Keep last 1000 events
```

## ============================================================================
## Database 1: Test Coordination and Locking
## ============================================================================

### Test Execution Coordination
```redis
# Test execution queue with priority
ZADD test:queue:high 100 '{
  "testId": "auth-001",
  "testName": "login-flow-test",
  "environment": "staging",
  "browser": "chromium",
  "priority": "P0",
  "estimatedDuration": 45000
}'

ZADD test:queue:medium 50 '{...}'
ZADD test:queue:low 25 '{...}'

# Currently running tests
HSET test:running:{test_id} "status" "executing"
HSET test:running:{test_id} "startTime" "2025-01-11T12:00:00Z"
HSET test:running:{test_id} "executor" "worker-node-1"
HSET test:running:{test_id} "progress" "3/10"
EXPIRE test:running:{test_id} 3600

# Test environment locks
SET lock:env:staging:chrome "worker-node-1" EX 300
SET lock:env:staging:firefox "worker-node-2" EX 300

# Resource allocation tracking
HSET resource:workers worker-node-1 '{
  "status": "busy",
  "currentTest": "auth-001",
  "capacity": 5,
  "running": 3,
  "load": 0.6
}'

# Test dependencies and blocking
SADD test:blocked:by:{test_id} dependency-test-1 dependency-test-2
SADD test:blocks:{test_id} dependent-test-1 dependent-test-2

# Cross-browser synchronization points
SET sync:point:{test_group}:{browser} "ready" EX 300
INCR sync:ready:{test_group}

# Test result aggregation
HSET test:aggregate:{suite_id} passed 15
HSET test:aggregate:{suite_id} failed 2
HSET test:aggregate:{suite_id} total 17
HSET test:aggregate:{suite_id} duration 45000
```

### Distributed Locking
```redis
# Test resource locks with automatic expiration
SET lock:test:user:{user_id} "test-execution-123" NX EX 300
SET lock:environment:{env_name} "worker-1" NX EX 600
SET lock:database:{db_name} "migration-task" NX EX 1800

# Lock renewal (for long-running tests)
EXPIRE lock:test:user:{user_id} 300

# Lock queues (for fairness)
RPUSH lock:queue:env:staging worker-2
RPUSH lock:queue:env:staging worker-3
LPOP lock:queue:env:staging  # Get next in queue
```

## ============================================================================
## Database 2: Test Results Cache
## ============================================================================

### Test Execution Results
```redis
# Individual test results
HSET test:result:{execution_id} status "passed"
HSET test:result:{execution_id} duration "2500"
HSET test:result:{execution_id} startTime "2025-01-11T12:00:00Z"
HSET test:result:{execution_id} endTime "2025-01-11T12:02:30Z"
HSET test:result:{execution_id} browser "chromium"
HSET test:result:{execution_id} environment "staging"
HSET test:result:{execution_id} steps_passed "8"
HSET test:result:{execution_id} steps_total "8"
HSET test:result:{execution_id} error_message ""
EXPIRE test:result:{execution_id} 86400  # 24 hours

# Aggregated results by test name
HSET test:stats:{test_name} total_runs 150
HSET test:stats:{test_name} passed_runs 147
HSET test:stats:{test_name} failed_runs 3
HSET test:stats:{test_name} avg_duration 2350
HSET test:stats:{test_name} last_success "2025-01-11T12:00:00Z"
HSET test:stats:{test_name} success_rate 98.0

# Recent test history
LPUSH test:history:{test_name} '{
  "executionId": "exec-123",
  "status": "passed",
  "duration": 2500,
  "timestamp": "2025-01-11T12:00:00Z"
}'
LTRIM test:history:{test_name} 0 99  # Keep last 100 results

# Test failure patterns
SADD failure:pattern:{error_type} test-1 test-2 test-3
HSET failure:analysis error_rate 2.5
HSET failure:analysis most_common_error "element_not_found"
HSET failure:analysis flaky_tests 5
```

### Performance Metrics Cache
```redis
# Page performance metrics
HSET perf:{test_id}:{page} load_time 1200
HSET perf:{test_id}:{page} first_paint 800
HSET perf:{test_id}:{page} largest_paint 1500
HSET perf:{test_id}:{page} cumulative_shift 0.05
HSET perf:{test_id}:{page} time_to_interactive 2000

# API response times
LPUSH api:response_times:{endpoint} 150
LPUSH api:response_times:{endpoint} 180
LPUSH api:response_times:{endpoint} 135
LTRIM api:response_times:{endpoint} 0 999  # Keep last 1000

# System resource usage
HSET system:metrics:{node} cpu_usage 45.2
HSET system:metrics:{node} memory_usage 68.5
HSET system:metrics:{node} disk_usage 23.1
HSET system:metrics:{node} network_io 1024
EXPIRE system:metrics:{node} 300

# Performance baselines
HSET baseline:{test_suite} avg_duration 2500
HSET baseline:{test_suite} p95_duration 4000
HSET baseline:{test_suite} success_rate 98.5
HSET baseline:{test_suite} last_updated "2025-01-11T12:00:00Z"
```

## ============================================================================
## Database 3: Performance Metrics Buffer
## ============================================================================

### Real-time Metrics Collection
```redis
# Time-series metrics buffer (before writing to ClickHouse)
LPUSH metrics:buffer '{
  "timestamp": "2025-01-11T12:00:00.123Z",
  "test_id": "auth-001",
  "metric_type": "duration",
  "value": 2500,
  "environment": "staging",
  "browser": "chromium"
}'

# Metrics aggregation windows
HSET metrics:1min:{timestamp} total_tests 15
HSET metrics:1min:{timestamp} passed_tests 14
HSET metrics:1min:{timestamp} avg_duration 2340
EXPIRE metrics:1min:{timestamp} 3600

HSET metrics:5min:{timestamp} total_tests 75
HSET metrics:5min:{timestamp} passed_tests 72
HSET metrics:5min:{timestamp} avg_duration 2380
EXPIRE metrics:5min:{timestamp} 21600

# Performance alerts
LPUSH alerts:performance '{
  "type": "duration_threshold_exceeded",
  "test": "login-flow",
  "threshold": 3000,
  "actual": 4500,
  "timestamp": "2025-01-11T12:00:00Z"
}'
LTRIM alerts:performance 0 99

# Resource utilization tracking
ZADD resource:cpu:usage {timestamp} 75.5
ZADD resource:memory:usage {timestamp} 68.2
ZADD resource:network:bandwidth {timestamp} 1024

# Cleanup old metrics (runs periodically)
ZREMRANGEBYSCORE resource:cpu:usage 0 {old_timestamp}
```

### Browser Performance Tracking
```redis
# Core Web Vitals by browser
HSET cwv:chrome:lcp avg 1200
HSET cwv:chrome:lcp p95 2000
HSET cwv:chrome:fid avg 50
HSET cwv:chrome:fid p95 100
HSET cwv:chrome:cls avg 0.05
HSET cwv:chrome:cls p95 0.15

HSET cwv:firefox:lcp avg 1350
HSET cwv:firefox:lcp p95 2200
# ... similar for other browsers

# Browser compatibility scores
HSET compat:scores chrome 98.5
HSET compat:scores firefox 97.2
HSET compat:scores safari 95.8
HSET compat:scores edge 96.4
```

## ============================================================================
## Database 4: AI Model Cache
## ============================================================================

### AI-Generated Test Data Cache
```redis
# Generated test scenarios (with TTL to ensure freshness)
SET ai:scenario:{prompt_hash} '{
  "scenario": "Complex user registration flow",
  "steps": [...],
  "testData": {...},
  "confidence": 0.95,
  "generatedAt": "2025-01-11T12:00:00Z",
  "model": "gemini-pro"
}' EX 86400

# AI model responses cache
SET ai:response:{input_hash} '{
  "response": "Generated test case content",
  "model": "gemini-pro",
  "tokens": 150,
  "cost": 0.0023,
  "timestamp": "2025-01-11T12:00:00Z"
}' EX 3600

# Model performance tracking
HSET ai:model:gemini-pro total_requests 1500
HSET ai:model:gemini-pro avg_response_time 850
HSET ai:model:gemini-pro success_rate 99.2
HSET ai:model:gemini-pro avg_cost 0.0045

# AI test quality scores
HSET ai:quality:{test_id} initial_score 0.95
HSET ai:quality:{test_id} current_score 0.87
HSET ai:quality:{test_id} human_feedback 0.9
HSET ai:quality:{test_id} modifications 3

# Prompt optimization cache
HSET prompt:optimization base_prompt "Generate a test case for..."
HSET prompt:optimization optimized_prompt "Create a comprehensive test scenario that..."
HSET prompt:optimization improvement_score 1.25
HSET prompt:optimization success_rate 0.94
```

### Model Rate Limiting and Quotas
```redis
# API usage tracking
INCR api:usage:gemini:daily EX 86400
INCR api:usage:gemini:hourly EX 3600
INCR api:usage:gemini:minute EX 60

# Cost tracking
INCRBYFLOAT cost:daily:gemini 0.0023 EX 86400
INCRBYFLOAT cost:monthly:gemini 0.0023 EX 2592000

# Request queue for rate limiting
RPUSH ai:queue '{
  "prompt": "Generate test data for...",
  "priority": "high",
  "requestId": "req-123",
  "timestamp": "2025-01-11T12:00:00Z"
}'
```

## ============================================================================
## Database 5: Real-time Notifications
## ============================================================================

### Live Test Monitoring
```redis
# Real-time test status updates
PUBLISH test:status '{
  "testId": "auth-001",
  "status": "running",
  "progress": 60,
  "currentStep": "Entering credentials",
  "timestamp": "2025-01-11T12:00:00Z"
}'

# Test completion notifications
PUBLISH test:completed '{
  "testId": "auth-001",
  "status": "passed",
  "duration": 2500,
  "environment": "staging",
  "browser": "chromium"
}'

# Alert notifications
PUBLISH alerts:critical '{
  "type": "high_failure_rate",
  "test_suite": "authentication",
  "failure_rate": 15.5,
  "threshold": 10.0,
  "timestamp": "2025-01-11T12:00:00Z"
}'

# Dashboard updates
PUBLISH dashboard:update '{
  "type": "metrics_update",
  "data": {
    "total_tests": 1500,
    "passed_tests": 1485,
    "running_tests": 8,
    "avg_duration": 2340
  }
}'

# User-specific notifications
PUBLISH user:notifications:{user_id} '{
  "type": "test_assigned",
  "test_name": "user-profile-validation",
  "priority": "high",
  "due_date": "2025-01-11T15:00:00Z"
}'
```

### WebSocket Connection Management
```redis
# Active WebSocket connections
SADD ws:connections:{user_id} connection-1 connection-2
HSET ws:connection:{connection_id} user_id user_123
HSET ws:connection:{connection_id} last_ping "2025-01-11T12:00:00Z"
EXPIRE ws:connection:{connection_id} 3600

# Subscription management
SADD ws:subscriptions:{connection_id} test:status alerts:critical
SADD topic:subscribers:test:status connection-1 connection-2
```

## ============================================================================
## Database 6: Test Environment State
## ============================================================================

### Environment Configuration
```redis
# Environment status
HSET env:staging status "healthy"
HSET env:staging last_check "2025-01-11T12:00:00Z"
HSET env:staging active_tests 5
HSET env:staging available_slots 10
HSET env:staging version "1.2.3"

# Service health checks
HSET health:auth-service status "up"
HSET health:auth-service response_time 120
HSET health:auth-service last_check "2025-01-11T12:00:00Z"
EXPIRE health:auth-service 300

HSET health:database status "up"
HSET health:database connections 25
HSET health:database max_connections 100
EXPIRE health:database 300

# Feature flags
HSET features:staging new_auth_flow "enabled"
HSET features:staging beta_dashboard "disabled"
HSET features:staging mfa_enforcement "enabled"

# Environment variables
HSET env:vars:staging BASE_URL "https://staging.aicore.dev"
HSET env:vars:staging API_URL "https://staging-api.aicore.dev"
HSET env:vars:staging DEBUG_MODE "true"

# Maintenance windows
SET maintenance:staging '{
  "scheduled": true,
  "start": "2025-01-12T02:00:00Z",
  "end": "2025-01-12T04:00:00Z",
  "reason": "Database maintenance"
}' EX 86400
```

### Test Data Lifecycle
```redis
# Test data cleanup schedules
ZADD cleanup:schedule {timestamp} "test_users:{user_batch_id}"
ZADD cleanup:schedule {timestamp} "test_sessions:{session_batch_id}"

# Data retention policies
HSET retention:test_users ttl_hours 24
HSET retention:test_sessions ttl_hours 8
HSET retention:test_artifacts ttl_hours 168  # 1 week
HSET retention:performance_metrics ttl_hours 720  # 30 days

# Cleanup status tracking
HSET cleanup:status last_run "2025-01-11T12:00:00Z"
HSET cleanup:status items_cleaned 1250
HSET cleanup:status next_run "2025-01-11T16:00:00Z"
```

## ============================================================================
## Database 7: Temporary Test Data
## ============================================================================

### Test Execution Context
```redis
# Test execution context (temporary)
HSET ctx:{execution_id} test_name "login-flow-test"
HSET ctx:{execution_id} user_id "test_user_123"
HSET ctx:{execution_id} browser "chromium"
HSET ctx:{execution_id} viewport "1920x1080"
HSET ctx:{execution_id} test_data '{"username": "testuser", "email": "test@example.com"}'
EXPIRE ctx:{execution_id} 3600

# Generated test data (temporary)
SET testdata:{execution_id}:user '{
  "username": "test_user_456",
  "email": "generated@test.com",
  "password": "SecurePass123!",
  "role": "user",
  "expires_at": "2025-01-11T13:00:00Z"
}' EX 3600

# Screenshot and artifact references
LPUSH artifacts:{execution_id} "screenshot-001.png"
LPUSH artifacts:{execution_id} "video-recording.mp4"
LPUSH artifacts:{execution_id} "console-logs.txt"
EXPIRE artifacts:{execution_id} 86400

# Test step tracking
HSET step:{execution_id}:1 name "Navigate to login"
HSET step:{execution_id}:1 status "completed"
HSET step:{execution_id}:1 duration 1200
HSET step:{execution_id}:1 screenshot "step-1-screenshot.png"

HSET step:{execution_id}:2 name "Enter credentials"
HSET step:{execution_id}:2 status "running"
HSET step:{execution_id}:2 start_time "2025-01-11T12:01:30Z"
EXPIRE step:{execution_id}:2 3600

# Error context
SET error:{execution_id} '{
  "type": "ElementNotFoundError",
  "message": "Login button not found",
  "selector": "[data-testid=login-button]",
  "screenshot": "error-screenshot.png",
  "dom_snapshot": "dom-at-error.html",
  "stack_trace": "..."
}' EX 86400
```

### Browser State Management
```redis
# Browser instance tracking
HSET browser:{instance_id} status "busy"
HSET browser:{instance_id} test_id "auth-001"
HSET browser:{instance_id} started_at "2025-01-11T12:00:00Z"
HSET browser:{instance_id} memory_usage 512
EXPIRE browser:{instance_id} 3600

# Page state cache
HSET page:{instance_id} url "https://staging.aicore.dev/auth/login"
HSET page:{instance_id} title "Login - AI-PLATFORM"
HSET page:{instance_id} load_state "networkidle"
HSET page:{instance_id} cookies '{"auth_token": "...", "session_id": "..."}'
EXPIRE page:{instance_id} 1800

# Element state cache
SET element:{instance_id}:{selector} '{
  "visible": true,
  "enabled": true,
  "text": "Login",
  "value": "",
  "attributes": {...}
}' EX 300
```

## ============================================================================
## Redis Configuration and Optimization
## ============================================================================

### Memory Management
```redis
# Configure memory policies
CONFIG SET maxmemory 2gb
CONFIG SET maxmemory-policy allkeys-lru

# Enable keyspace notifications for expiration
CONFIG SET notify-keyspace-events Ex

# Optimize for performance
CONFIG SET save ""  # Disable RDB snapshots for speed
CONFIG SET appendonly yes  # Use AOF for durability
CONFIG SET appendfsync everysec  # Balance between performance and durability
```

### Monitoring and Alerts
```redis
# Memory usage monitoring
INFO memory

# Connection monitoring
INFO clients

# Performance monitoring
SLOWLOG GET 10
CLIENT LIST
```

## ============================================================================
## Utility Scripts and Functions
## ============================================================================

### Lua Scripts for Atomic Operations

#### Test Queue Management Script
```lua
-- test_queue_manager.lua
-- Atomically dequeue test with resource checking
local queue_key = KEYS[1]
local resource_key = KEYS[2]
local max_resources = tonumber(ARGV[1])

local current_resources = redis.call('HGET', resource_key, 'running') or 0
if tonumber(current_resources) >= max_resources then
    return nil
end

local test = redis.call('ZPOPMIN', queue_key)
if next(test) == nil then
    return nil
end

redis.call('HINCRBY', resource_key, 'running', 1)
return test[2]  -- Return test data
```

#### Session Validation Script
```lua
-- session_validator.lua
-- Validate session and update last activity
local session_key = KEYS[1]
local blacklist_key = KEYS[2]
local current_time = ARGV[1]

-- Check if session is blacklisted
if redis.call('EXISTS', blacklist_key) == 1 then
    return {0, 'BLACKLISTED'}
end

-- Check if session exists
if redis.call('EXISTS', session_key) == 0 then
    return {0, 'NOT_FOUND'}
end

-- Update last activity
redis.call('HSET', session_key, 'lastActivity', current_time)
redis.call('EXPIRE', session_key, 3600)

return {1, 'VALID'}
```

### Cleanup Scripts
```bash
#!/bin/bash
# cleanup_expired_data.sh

# Clean up expired test sessions
redis-cli --scan --pattern "session:test:*" | while read key; do
    redis-cli TTL "$key" | grep -q "^-1$" && redis-cli DEL "$key"
done

# Clean up old metrics
redis-cli ZREMRANGEBYSCORE resource:cpu:usage 0 $(date -d "24 hours ago" +%s)
redis-cli ZREMRANGEBYSCORE resource:memory:usage 0 $(date -d "24 hours ago" +%s)

# Clean up old test results
redis-cli --scan --pattern "test:result:*" | while read key; do
    if [ $(redis-cli TTL "$key") -lt 0 ]; then
        redis-cli DEL "$key"
    fi
done
```

## ============================================================================
## Performance Optimization
## ============================================================================

### Connection Pooling Configuration
```yaml
# Redis connection pool settings
redis_pool:
  max_connections: 100
  min_idle_connections: 10
  max_idle_connections: 20
  connection_timeout: 5000
  socket_timeout: 3000
  retry_attempts: 3

# Per-database connection limits
databases:
  0: # Sessions
    max_connections: 30
  1: # Coordination
    max_connections: 20
  2: # Cache
    max_connections: 25
  3: # Metrics
    max_connections: 15
```

### Pipeline Usage Examples
```javascript
// Batch operations for better performance
const pipeline = redis.pipeline();
pipeline.hset('test:result:123', 'status', 'passed');
pipeline.hset('test:result:123', 'duration', '2500');
pipeline.expire('test:result:123', 86400);
pipeline.lpush('test:history:login', JSON.stringify(result));
pipeline.ltrim('test:history:login', 0, 99);
await pipeline.exec();

// Atomic increment with expiration
await redis.multi()
  .incr('rate:api:user123')
  .expire('rate:api:user123', 3600)
  .exec();
```

## ============================================================================
## Monitoring and Alerting
## ============================================================================

### Redis Health Metrics to Monitor
- Memory usage and fragmentation
- Connection count and limits
- Command execution time (SLOWLOG)
- Keyspace hit ratio
- Expired keys per second
- Network I/O
- Replication lag (if using clustering)

### Alert Thresholds
```yaml
alerts:
  memory_usage: >90%
  connection_count: >80 connections
  slow_queries: >10ms average
  keyspace_misses: >50% miss rate
  replication_lag: >5 seconds
```

---

**Redis Schema Status**: ✅ READY FOR IMPLEMENTATION
**Integration Points**: PostgreSQL, ClickHouse, MongoDB, API Services
**Performance Target**: <1ms read operations, <5ms write operations
**Scalability**: Supports 10,000+ concurrent test sessions

**Next Steps**:
1. Implement connection pooling and failover
2. Set up monitoring and alerting
3. Configure backup and persistence strategies
4. Integrate with test execution framework
