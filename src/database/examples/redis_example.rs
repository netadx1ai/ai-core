//! Redis caching and pub/sub example
//!
//! This example demonstrates how to use Redis for caching operations,
//! session management, rate limiting, and pub/sub messaging in the AI-CORE platform.

use ai_core_database::{
    connections::{RedisConfig, RedisConnection},
    DatabaseError,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: u32,
    name: String,
    email: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationMessage {
    user_id: u32,
    title: String,
    body: String,
    notification_type: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Redis Example Demo");
    println!("==============================");

    // Create Redis configuration
    let config = RedisConfig {
        url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        default_ttl_seconds: 300, // 5 minutes
        ..Default::default()
    };

    // Create Redis connection
    let redis = RedisConnection::new(config)
        .await
        .expect("Failed to connect to Redis. Please ensure Redis is running on localhost:6379");

    println!("✅ Connected to Redis successfully");

    // Run all examples
    basic_caching_example(&redis).await?;
    session_management_example(&redis).await?;
    rate_limiting_example(&redis).await?;
    pubsub_example(&redis).await?;
    cache_aside_pattern_example(&redis).await?;
    batch_operations_example(&redis).await?;
    lua_scripting_example(&redis).await?;

    // Display final statistics
    display_statistics(&redis).await?;

    println!("\n🎉 Redis example completed successfully!");
    Ok(())
}

/// Demonstrate basic caching operations
async fn basic_caching_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n📦 Basic Caching Operations");
    println!("----------------------------");

    let user = User {
        id: 1,
        name: "Alice Johnson".to_string(),
        email: "alice@example.com".to_string(),
        created_at: chrono::Utc::now(),
    };

    // Set value with default TTL
    redis.set("user:1", &user).await?;
    println!("✅ Cached user:1");

    // Get value from cache
    let cached_user: Option<User> = redis.get("user:1").await?;
    println!("📖 Retrieved user:1: {:?}", cached_user);

    // Set value with custom TTL (10 seconds)
    redis.set_with_ttl("user:1:temp", &user, 10).await?;
    println!("✅ Cached user:1:temp with 10s TTL");

    // Check TTL
    let ttl = redis.ttl("user:1:temp").await?;
    println!("⏱️  TTL for user:1:temp: {} seconds", ttl);

    // Check if key exists
    let exists = redis.exists("user:1").await?;
    println!("🔍 user:1 exists: {}", exists);

    // Increment counter
    let count = redis.incr("page_views", 1).await?;
    println!("📊 Page views: {}", count);

    let count = redis.incr("page_views", 5).await?;
    println!("📊 Page views after adding 5: {}", count);

    Ok(())
}

/// Demonstrate session management
async fn session_management_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n🔐 Session Management");
    println!("----------------------");

    let session_id = "sess_123456789";
    let session_data = User {
        id: 42,
        name: "Bob Smith".to_string(),
        email: "bob@example.com".to_string(),
        created_at: chrono::Utc::now(),
    };

    // Create session (30 minutes)
    redis.set_session(session_id, &session_data, 1800).await?;
    println!("✅ Created session: {}", session_id);

    // Retrieve session
    let retrieved_session: Option<User> = redis.get_session(session_id).await?;
    println!("📖 Retrieved session: {:?}", retrieved_session);

    // Refresh session (extend to 1 hour)
    let refreshed = redis.refresh_session(session_id, 3600).await?;
    println!("🔄 Session refreshed: {}", refreshed);

    // Check TTL
    let session_key = format!("session:{}", session_id);
    let ttl = redis.ttl(&session_key).await?;
    println!("⏱️  Session TTL: {} seconds", ttl);

    Ok(())
}

/// Demonstrate rate limiting
async fn rate_limiting_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n🚦 Rate Limiting");
    println!("----------------");

    let user_id = "user_123";
    let window_seconds = 60; // 1 minute window
    let limit = 5; // 5 requests per minute

    println!(
        "Rate limit: {} requests per {} seconds",
        limit, window_seconds
    );

    // Simulate multiple requests
    for i in 1..=7 {
        let allowed = redis
            .check_rate_limit(user_id, window_seconds, limit)
            .await?;
        println!(
            "Request {}: {}",
            i,
            if allowed {
                "✅ ALLOWED"
            } else {
                "❌ RATE LIMITED"
            }
        );

        if i <= 3 {
            sleep(Duration::from_millis(100)).await;
        }
    }

    Ok(())
}

/// Demonstrate pub/sub messaging
async fn pubsub_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n📡 Pub/Sub Messaging");
    println!("--------------------");

    let channel = "notifications";

    // Subscribe to channel
    let mut receiver = redis.subscribe(channel).await?;
    println!("✅ Subscribed to channel: {}", channel);

    // Spawn a task to listen for messages
    let redis_clone = redis;
    tokio::spawn(async move {
        println!("👂 Listening for messages...");
        while let Ok(message) = receiver.recv().await {
            println!("📨 Received message: {}", message);
        }
    });

    // Give subscriber time to start
    sleep(Duration::from_millis(100)).await;

    // Publish some messages
    let messages = vec![
        NotificationMessage {
            user_id: 1,
            title: "Welcome!".to_string(),
            body: "Welcome to AI-CORE platform".to_string(),
            notification_type: "welcome".to_string(),
        },
        NotificationMessage {
            user_id: 1,
            title: "New Message".to_string(),
            body: "You have a new message".to_string(),
            notification_type: "message".to_string(),
        },
    ];

    for (i, msg) in messages.iter().enumerate() {
        let subscriber_count = redis_clone.publish(channel, msg).await?;
        println!(
            "📤 Published message {}: {} subscribers notified",
            i + 1,
            subscriber_count
        );
        sleep(Duration::from_millis(500)).await;
    }

    // Unsubscribe
    redis_clone.unsubscribe(channel).await?;
    println!("✅ Unsubscribed from channel: {}", channel);

    Ok(())
}

/// Demonstrate cache-aside pattern
async fn cache_aside_pattern_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n🔄 Cache-Aside Pattern");
    println!("----------------------");

    let user_id = "user:cache_aside:1";

    // Simulate expensive database operation
    let expensive_operation = || async {
        println!("💭 Simulating expensive database query...");
        sleep(Duration::from_millis(1000)).await; // Simulate 1 second delay
        Ok(User {
            id: 999,
            name: "Cache Aside User".to_string(),
            email: "cache@example.com".to_string(),
            created_at: chrono::Utc::now(),
        })
    };

    // First call - should execute expensive operation
    println!("🔍 First call (cache miss expected):");
    let start = std::time::Instant::now();
    let user1: User = redis.get_or_set(user_id, expensive_operation).await?;
    let duration1 = start.elapsed();
    println!("✅ Got user: {} (took {:?})", user1.name, duration1);

    // Second call - should return from cache
    println!("\n🔍 Second call (cache hit expected):");
    let start = std::time::Instant::now();
    let user2: User = redis
        .get_or_set(user_id, || async {
            println!("❌ This should not be called!");
            Err(DatabaseError::Connection(
                "Should not reach here".to_string(),
            ))
        })
        .await?;
    let duration2 = start.elapsed();
    println!("✅ Got user: {} (took {:?})", user2.name, duration2);

    println!("⚡ Cache speedup: {:?} vs {:?}", duration1, duration2);

    Ok(())
}

/// Demonstrate batch operations
async fn batch_operations_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n📦 Batch Operations");
    println!("-------------------");

    let users = vec![
        User {
            id: 10,
            name: "User 10".to_string(),
            email: "user10@example.com".to_string(),
            created_at: chrono::Utc::now(),
        },
        User {
            id: 11,
            name: "User 11".to_string(),
            email: "user11@example.com".to_string(),
            created_at: chrono::Utc::now(),
        },
        User {
            id: 12,
            name: "User 12".to_string(),
            email: "user12@example.com".to_string(),
            created_at: chrono::Utc::now(),
        },
    ];

    // Batch set
    let pairs: Vec<(&str, &User)> = vec![
        ("batch:user:10", &users[0]),
        ("batch:user:11", &users[1]),
        ("batch:user:12", &users[2]),
    ];
    redis.mset(&pairs).await?;
    println!("✅ Batch set {} users", users.len());

    // Batch get
    let keys = [
        "batch:user:10",
        "batch:user:11",
        "batch:user:12",
        "batch:user:13",
    ];
    let results: Vec<Option<User>> = redis.mget(&keys).await?;

    for (i, result) in results.iter().enumerate() {
        match result {
            Some(user) => println!("📖 {}: Found user {}", keys[i], user.name),
            None => println!("📖 {}: Not found", keys[i]),
        }
    }

    Ok(())
}

/// Demonstrate Lua scripting
async fn lua_scripting_example(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n🔧 Lua Scripting");
    println!("-----------------");

    // Atomic increment with maximum value script
    let script = r#"
        local key = KEYS[1]
        local increment = tonumber(ARGV[1])
        local max_value = tonumber(ARGV[2])

        local current = redis.call('GET', key)
        if not current then
            current = 0
        else
            current = tonumber(current)
        end

        local new_value = current + increment
        if new_value > max_value then
            return -1
        else
            redis.call('SET', key, new_value)
            return new_value
        end
    "#;

    let key = "lua_counter";

    // Test incrementing within limit
    let result: i64 = redis.eval(script, &[key], &["5", "100"]).await?;
    println!("✅ Incremented by 5: {}", result);

    let result: i64 = redis.eval(script, &[key], &["10", "100"]).await?;
    println!("✅ Incremented by 10: {}", result);

    // Test incrementing beyond limit
    let result: i64 = redis.eval(script, &[key], &["90", "100"]).await?;
    if result == -1 {
        println!("❌ Increment rejected (would exceed maximum)");
    } else {
        println!("✅ Incremented by 90: {}", result);
    }

    Ok(())
}

/// Display Redis statistics
async fn display_statistics(redis: &RedisConnection) -> Result<(), DatabaseError> {
    println!("\n📊 Redis Statistics");
    println!("-------------------");

    let stats = redis.get_stats().await;

    println!("Cache Operations:");
    println!("  - Sets: {}", stats.cache_sets);
    println!("  - Hits: {}", stats.cache_hits);
    println!("  - Misses: {}", stats.cache_misses);
    println!("  - Deletes: {}", stats.cache_deletes);

    if stats.cache_hits + stats.cache_misses > 0 {
        let hit_ratio =
            stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64 * 100.0;
        println!("  - Hit Ratio: {:.2}%", hit_ratio);
    }

    println!("Pub/Sub Operations:");
    println!("  - Published: {}", stats.pub_messages);
    println!("  - Received: {}", stats.sub_messages);

    println!("Connection Info:");
    println!("  - Uptime: {} seconds", stats.uptime_seconds);
    if let Some(memory) = stats.memory_usage_bytes {
        println!("  - Memory Usage: {} bytes", memory);
    }

    if let Some(error) = &stats.last_error {
        println!("  - Last Error: {}", error);
    }

    // Get database size
    let db_size = redis.db_size().await?;
    println!("  - Total Keys: {}", db_size);

    Ok(())
}
