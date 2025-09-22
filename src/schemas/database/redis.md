# Redis Schema: High-Performance Cache and Real-time Data
## Handles 15% of data: Sessions, rate limiting, notifications, hot data access

Redis serves as the high-performance cache and real-time data layer for the Intelligent Automation Platform, providing sub-millisecond access to frequently accessed data and real-time features.

## Key Patterns and Data Structures

### 1. User Sessions and Authentication

```redis
# User sessions - Hash structure
HSET session:uuid:abc123def456 user_id "user_789" 
HSET session:uuid:abc123def456 username "john.doe"
HSET session:uuid:abc123def456 email "john@example.com"
HSET session:uuid:abc123def456 subscription_tier "pro"
HSET session:uuid:abc123def456 created_at "2024-01-15T10:30:00Z"
HSET session:uuid:abc123def456 last_accessed "2024-01-15T14:22:00Z"
HSET session:uuid:abc123def456 ip_address "192.168.1.100"
HSET session:uuid:abc123def456 permissions '["read:workflows", "write:content", "admin:campaigns"]'

# Session expiration (24 hours default)
EXPIRE session:uuid:abc123def456 86400

# User session index for fast lookup
SET user_session:user_789 abc123def456
EXPIRE user_session:user_789 86400

# API key sessions
HSET api_session:key_prefix12345 user_id "user_789"
HSET api_session:key_prefix12345 key_name "Production API Key"
HSET api_session:key_prefix12345 permissions '["read:workflows","write:automation"]'
HSET api_session:key_prefix12345 rate_limit_per_minute 1000
HSET api_session:key_prefix12345 last_used "2024-01-15T14:22:00Z"
EXPIRE api_session:key_prefix12345 3600
```

### 2. Rate Limiting

```redis
# API rate limiting - Sliding window approach
# Format: rate_limit:{user_id|api_key}:{window}:{timestamp_bucket}
INCR rate_limit:user_789:minute:202401151422
EXPIRE rate_limit:user_789:minute:202401151422 60

INCR rate_limit:user_789:hour:2024011514
EXPIRE rate_limit:user_789:hour:2024011514 3600

INCR rate_limit:user_789:day:20240115
EXPIRE rate_limit:user_789:day:20240115 86400

# Global rate limiting for system protection
INCR rate_limit:global:minute:202401151422
EXPIRE rate_limit:global:minute:202401151422 60

# Provider-specific rate limiting
INCR rate_limit:provider:openai:minute:202401151422
EXPIRE rate_limit:provider:openai:minute:202401151422 60

# Rate limit configuration per user
HSET rate_limits:user_789 per_minute 60
HSET rate_limits:user_789 per_hour 1000
HSET rate_limits:user_789 per_day 10000
HSET rate_limits:user_789 burst_limit 100
```

### 3. Real-time Workflow Progress

```redis
# Workflow progress tracking
HSET workflow:progress:workflow_123 status "running"
HSET workflow:progress:workflow_123 progress_percentage 65
HSET workflow:progress:workflow_123 current_step "content_generation"
HSET workflow:progress:workflow_123 completed_steps 3
HSET workflow:progress:workflow_123 total_steps 5
HSET workflow:progress:workflow_123 estimated_completion "2024-01-15T15:30:00Z"
HSET workflow:progress:workflow_123 cost_so_far 12.50
HSET workflow:progress:workflow_123 last_updated "2024-01-15T14:22:00Z"

# Workflow step queue (List for ordered processing)
LPUSH workflow:steps:workflow_123 '{"step_id":"step_4","type":"publish_content","provider":"facebook"}'
LPUSH workflow:steps:workflow_123 '{"step_id":"step_5","type":"analytics_setup","provider":"internal"}'

# Active workflows by user (Set for fast membership tests) 
SADD user:active_workflows:user_789 workflow_123
SADD user:active_workflows:user_789 workflow_456

# Workflow notifications queue
LPUSH workflow:notifications:workflow_123 '{"type":"step_completed","step":"content_generation","timestamp":"2024-01-15T14:20:00Z"}'
EXPIRE workflow:notifications:workflow_123 86400
```

### 4. Real-time Notifications

```redis
# User notification channels (for WebSocket/SSE)
SADD user:subscriptions:user_789 "workflow:workflow_123"
SADD user:subscriptions:user_789 "campaign:campaign_abc"
SADD user:subscriptions:user_789 "system:maintenance"

# Notification queues per user
LPUSH notifications:user_789 '{"id":"notif_001","type":"workflow_completed","title":"Content Campaign Complete","message":"Your social media campaign has been successfully published","priority":"medium","workflow_id":"workflow_123","timestamp":"2024-01-15T14:25:00Z"}'
EXPIRE notifications:user_789 604800  # 7 days

# Unread notification count
INCR notifications:unread:user_789
EXPIRE notifications:unread:user_789 604800

# System-wide announcements
LPUSH notifications:system '{"type":"maintenance","title":"Scheduled Maintenance","message":"Platform maintenance scheduled for tonight 2-4 AM EST","priority":"high","scheduled_for":"2024-01-16T07:00:00Z"}'

# Push notification tokens for mobile
SADD push_tokens:user_789 "fcm_token_android_abc123"
SADD push_tokens:user_789 "apns_token_ios_def456"
```

### 5. Content Scheduling and Publishing

```redis
# Scheduled posts queue (Sorted Set by publish time)
ZADD scheduled:posts 1705329600 '{"post_id":"post_001","content_id":"content_123","platform":"facebook","scheduled_time":"2024-01-15T16:00:00Z"}'
ZADD scheduled:posts 1705333200 '{"post_id":"post_002","content_id":"content_124","platform":"instagram","scheduled_time":"2024-01-15T17:00:00Z"}'

# Platform-specific publishing queues
LPUSH publish:facebook '{"post_id":"post_001","content_id":"content_123","priority":"high"}'
LPUSH publish:instagram '{"post_id":"post_002","content_id":"content_124","priority":"medium"}'
LPUSH publish:twitter '{"post_id":"post_003","content_id":"content_125","priority":"high"}'

# Publishing retry queue
ZADD publish:retry 1705329900 '{"post_id":"post_004","retry_count":1,"max_retries":3,"next_retry":"2024-01-15T16:05:00Z"}'

# Content approval queue
LPUSH approval:queue '{"content_id":"content_126","submitted_by":"user_789","submitted_at":"2024-01-15T14:30:00Z","priority":"high"}'

# Publishing status cache
HSET publish:status:post_001 status "completed"
HSET publish:status:post_001 platform_post_id "fb_post_789456"
HSET publish:status:post_001 published_at "2024-01-15T16:00:15Z"
HSET publish:status:post_001 platform_url "https://facebook.com/posts/789456"
EXPIRE publish:status:post_001 86400
```

### 6. Campaign and Analytics Cache

```redis
# Campaign performance cache (refreshed every 5 minutes)
HSET campaign:stats:campaign_abc impressions 15420
HSET campaign:stats:campaign_abc clicks 892
HSET campaign:stats:campaign_abc conversions 43
HSET campaign:stats:campaign_abc cost 156.75
HSET campaign:stats:campaign_abc engagement_rate 5.78
HSET campaign:stats:campaign_abc last_updated "2024-01-15T14:25:00Z"
EXPIRE campaign:stats:campaign_abc 300  # 5 minutes

# Real-time metrics for dashboard
HSET metrics:realtime:user_789 active_workflows 3
HSET metrics:realtime:user_789 pending_posts 12
HSET metrics:realtime:user_789 monthly_cost 245.67
HSET metrics:realtime:user_789 success_rate 94.2
EXPIRE metrics:realtime:user_789 60  # 1 minute

# Platform performance cache
HSET platform:performance:facebook avg_engagement 4.2
HSET platform:performance:facebook success_rate 98.5
HSET platform:performance:facebook avg_cost_per_post 2.15
EXPIRE platform:performance:facebook 300

# Trending hashtags cache
ZADD trending:hashtags 156 "#coffee"
ZADD trending:hashtags 142 "#sustainable"
ZADD trending:hashtags 98 "#newproduct"
EXPIRE trending:hashtags 3600  # 1 hour
```

### 7. System Health and Monitoring

```redis
# Service health status
HSET service:health:api_gateway status "healthy"
HSET service:health:api_gateway last_check "2024-01-15T14:22:00Z"
HSET service:health:api_gateway response_time_ms 45
HSET service:health:api_gateway error_rate 0.02
EXPIRE service:health:api_gateway 120

HSET service:health:intent_parser status "healthy"
HSET service:health:intent_parser last_check "2024-01-15T14:22:00Z"
HSET service:health:intent_parser queue_size 3
HSET service:health:intent_parser avg_processing_time_ms 1250

# System metrics (updated every minute)
HSET system:metrics cpu_usage_percent 23.5
HSET system:metrics memory_usage_percent 67.2
HSET system:metrics active_connections 142
HSET system:metrics queue_depth 8
HSET system:metrics cache_hit_rate 94.7
EXPIRE system:metrics 120

# Error tracking
INCR errors:api_gateway:429_rate_limit
EXPIRE errors:api_gateway:429_rate_limit 300
INCR errors:workflow_orchestrator:temporal_timeout
EXPIRE errors:workflow_orchestrator:temporal_timeout 300

# Performance metrics
LPUSH perf:api_response_times 45
LTRIM perf:api_response_times 0 999  # Keep last 1000 measurements
LPUSH perf:workflow_durations 15420
LTRIM perf:workflow_durations 0 999
```

### 8. Federation and Client Management

```redis
# Federated client status
HSET client:status:client_123 health "healthy"
HSET client:status:client_123 last_ping "2024-01-15T14:20:00Z"
HSET client:status:client_123 active_connections 5
HSET client:status:client_123 response_time_ms 180
HSET client:status:client_123 success_rate 99.1
EXPIRE client:status:client_123 300

# MCP server capabilities cache
HSET mcp:capabilities:content_server tools '["create_blog","create_social_post","generate_image"]'
HSET mcp:capabilities:content_server cost_per_request 0.05
HSET mcp:capabilities:content_server avg_response_time_ms 2400
EXPIRE mcp:capabilities:content_server 3600

# Client request queue
LPUSH client:requests:client_123 '{"request_id":"req_001","workflow_id":"workflow_456","type":"content_generation","priority":"high"}'

# Provider selection cache
HSET provider:selection:content_generation best_provider "openai_gpt4"
HSET provider:selection:content_generation cost_score 8.5
HSET provider:selection:content_generation performance_score 9.2
HSET provider:selection:content_generation availability_score 9.8
EXPIRE provider:selection:content_generation 1800  # 30 minutes
```

### 9. Caching Strategies

```redis
# LLM response cache (expensive API calls)
SET llm:cache:sha256:abc123def456 '{"model":"gpt-4","response":"Generated content here...","tokens_used":450,"cost":0.018}'
EXPIRE llm:cache:sha256:abc123def456 3600  # 1 hour

# Database query cache
SET db:query:user_workflows:user_789 '{"workflows":[{"id":"workflow_123","status":"running"},{"id":"workflow_456","status":"completed"}]}'
EXPIRE db:query:user_workflows:user_789 300  # 5 minutes

# API response cache
SET api:response:GET:/campaigns/campaign_abc '{"campaign":{"id":"campaign_abc","name":"Coffee Launch","status":"active"}}'
EXPIRE api:response:GET:/campaigns/campaign_abc 60  # 1 minute

# Configuration cache
HSET config:system max_concurrent_workflows 1000
HSET config:system default_timeout_seconds 3600
HSET config:system maintenance_mode false
```

### 10. Distributed Locks and Coordination

```redis
# Distributed locks for critical operations
SET lock:publish:facebook:user_789 "worker_node_2" NX EX 300
SET lock:billing:user_789 "billing_service_1" NX EX 60
SET lock:campaign:campaign_abc "orchestrator_3" NX EX 1800

# Workflow coordination
SET workflow:lock:workflow_123 "worker_1" NX EX 3600
INCR workflow:step_counter:workflow_123
EXPIRE workflow:step_counter:workflow_123 3600

# Resource allocation
INCR resource:usage:llm_tokens
EXPIRE resource:usage:llm_tokens 3600
INCR resource:usage:storage_mb
EXPIRE resource:usage:storage_mb 86400
```

## Data Organization Patterns

### Key Naming Conventions
- **User Data**: `user:{user_id}:{data_type}`
- **Workflow Data**: `workflow:{workflow_id}:{data_type}`
- **Campaign Data**: `campaign:{campaign_id}:{data_type}`
- **System Data**: `system:{metric_type}`
- **Cache Data**: `cache:{data_type}:{key}`
- **Queue Data**: `queue:{queue_type}`
- **Lock Data**: `lock:{resource_type}:{resource_id}`

### TTL Strategy
- **Sessions**: 24 hours (auto-renewed on activity)
- **Rate Limits**: Window-specific (1 min, 1 hour, 1 day)
- **Notifications**: 7 days
- **Cache**: 1-60 minutes based on data volatility
- **Locks**: Operation-specific (1-30 minutes)
- **Metrics**: 2-5 minutes for real-time data

### Memory Optimization
- Use appropriate data types (Hash for objects, Set for collections, Sorted Set for ordered data)
- Implement TTL on all keys to prevent memory leaks
- Use Redis data compression for large values where applicable
- Monitor memory usage and implement eviction policies

### High Availability Configuration
```redis
# Master-Slave replication
REPLICAOF master.redis.internal 6379

# Cluster configuration for horizontal scaling
CLUSTER NODES
CLUSTER INFO

# Sentinel for automatic failover
SENTINEL masters
SENTINEL slaves master1
```

### Performance Monitoring
```redis
# Monitor key metrics
INFO memory
INFO stats
INFO replication
SLOWLOG GET 10
CLIENT LIST
```

This Redis schema provides:
- **Sub-millisecond access** to frequently used data
- **Real-time capabilities** for notifications and progress tracking
- **Efficient rate limiting** and session management
- **Distributed coordination** for microservices
- **High-performance caching** for expensive operations
- **Scalable architecture** supporting horizontal growth

The design ensures optimal performance while maintaining data consistency and system reliability across the entire intelligent automation platform.