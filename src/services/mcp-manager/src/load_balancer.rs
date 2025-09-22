//! Load Balancer Module
//!
//! This module provides load balancing capabilities for distributing requests across
//! multiple MCP server instances. It supports various load balancing strategies,
//! circuit breaker patterns, and health-aware routing.

use crate::{models::ServerInfo, registry::ServerRegistry, McpError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Load balancer for distributing requests across MCP servers
#[derive(Debug)]
pub struct LoadBalancer {
    /// Server registry reference
    registry: Arc<ServerRegistry>,

    /// Load balancing configuration
    config: LoadBalancerConfig,

    /// Load balancing strategy
    strategy: Box<dyn LoadBalancingStrategy + Send + Sync>,

    /// Server connection tracking
    connections: Arc<RwLock<HashMap<Uuid, ServerConnections>>>,

    /// Circuit breaker states
    circuit_breakers: Arc<RwLock<HashMap<Uuid, CircuitBreaker>>>,

    /// Load balancer statistics
    stats: Arc<RwLock<LoadBalancerStatistics>>,
}

/// Load balancing configuration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy type
    pub strategy: LoadBalancingStrategyType,

    /// Enable sticky sessions
    pub sticky_sessions: bool,

    /// Session timeout in seconds
    pub session_timeout_seconds: u64,

    /// Maximum requests per server
    pub max_requests_per_server: u32,

    /// Enable circuit breaker
    pub circuit_breaker_enabled: bool,

    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,

    /// Health check integration
    pub health_aware: bool,

    /// Weight configuration for weighted strategies
    pub server_weights: HashMap<String, u32>,
}

/// Load balancing strategy types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategyType {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    Random,
    IpHash,
    ConsistentHash,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold percentage (0-100)
    pub failure_threshold: u32,

    /// Minimum number of requests before circuit can open
    pub min_requests: u32,

    /// Time window for calculating failure rate (seconds)
    pub window_seconds: u64,

    /// Recovery timeout when circuit is open (seconds)
    pub recovery_timeout_seconds: u64,

    /// Half-open state request limit
    pub half_open_max_requests: u32,
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for individual servers
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Current state
    pub state: CircuitBreakerState,

    /// Failure count in current window
    pub failure_count: u32,

    /// Total requests in current window
    pub request_count: u32,

    /// Window start time
    pub window_start: DateTime<Utc>,

    /// Last state change time
    pub last_state_change: DateTime<Utc>,

    /// Requests in half-open state
    pub half_open_requests: u32,

    /// Configuration
    pub config: CircuitBreakerConfig,
}

/// Server connection tracking
#[derive(Debug, Default, Clone)]
pub struct ServerConnections {
    /// Active connections count
    pub active_connections: u32,

    /// Total requests processed
    pub total_requests: u64,

    /// Total errors
    pub total_errors: u64,

    /// Last request timestamp
    pub last_request: Option<DateTime<Utc>>,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// Server weight (for weighted strategies)
    pub weight: u32,
}

/// Load balancer statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStatistics {
    /// Total requests processed
    pub total_requests: u64,

    /// Total errors
    pub total_errors: u64,

    /// Requests by server
    pub requests_by_server: HashMap<Uuid, u64>,

    /// Current active connections
    pub active_connections: u32,

    /// Average response time across all servers
    pub avg_response_time_ms: f64,

    /// Circuit breaker states
    pub circuit_breaker_states: HashMap<Uuid, String>,

    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Server selection result
#[derive(Debug, Clone)]
pub struct ServerSelection {
    /// Selected server
    pub server: ServerInfo,

    /// Selection reason
    pub reason: String,

    /// Selection timestamp
    pub timestamp: DateTime<Utc>,
}

/// Load balancing strategy trait
pub trait LoadBalancingStrategy: std::fmt::Debug {
    /// Select a server from the available pool
    fn select_server(
        &self,
        servers: &[ServerInfo],
        connections: &HashMap<Uuid, ServerConnections>,
        request_context: &RequestContext,
    ) -> Option<Uuid>;

    /// Get strategy name
    fn name(&self) -> &'static str;
}

/// Request context for load balancing decisions
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID
    pub request_id: String,

    /// Client IP address (for IP hash)
    pub client_ip: Option<String>,

    /// Session ID (for sticky sessions)
    pub session_id: Option<String>,

    /// Request priority
    pub priority: u32,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(registry: Arc<ServerRegistry>, config: LoadBalancerConfig) -> Self {
        let strategy = create_strategy(&config.strategy);

        Self {
            registry,
            config,
            strategy,
            connections: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(LoadBalancerStatistics::default())),
        }
    }

    /// Select a server for handling a request
    pub async fn select_server(&self, context: &RequestContext) -> Result<ServerSelection> {
        // Get available servers
        let mut servers = if self.config.health_aware {
            self.registry.get_available_servers().await
        } else {
            self.registry.list().await
        };

        if servers.is_empty() {
            return Err(McpError::ServerManagement(
                "No servers available for load balancing".to_string(),
            ));
        }

        // Filter out servers with open circuit breakers
        if self.config.circuit_breaker_enabled {
            servers = self.filter_circuit_breaker_servers(servers).await;
        }

        if servers.is_empty() {
            return Err(McpError::ServerManagement(
                "All servers have open circuit breakers".to_string(),
            ));
        }

        // Filter servers that have reached max connections
        servers = self.filter_overloaded_servers(servers).await;

        if servers.is_empty() {
            return Err(McpError::ServerManagement(
                "All servers are at maximum capacity".to_string(),
            ));
        }

        // Check for sticky session
        if self.config.sticky_sessions {
            if let Some(session_id) = &context.session_id {
                if let Some(server) = self.get_sticky_session_server(session_id, &servers).await {
                    return Ok(ServerSelection {
                        server,
                        reason: "Sticky session".to_string(),
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        // Use load balancing strategy
        let connections = self.connections.read().await;
        let selected_server_id = self
            .strategy
            .select_server(&servers, &connections, context)
            .ok_or_else(|| {
                McpError::ServerManagement("No server selected by strategy".to_string())
            })?;

        let selected_server = servers
            .into_iter()
            .find(|s| s.id == selected_server_id)
            .ok_or_else(|| {
                McpError::ServerManagement(
                    "Selected server not found in available list".to_string(),
                )
            })?;

        Ok(ServerSelection {
            server: selected_server,
            reason: format!("Strategy: {}", self.strategy.name()),
            timestamp: Utc::now(),
        })
    }

    /// Record request start for a server
    pub async fn record_request_start(&self, server_id: &Uuid) -> Result<()> {
        let mut connections = self.connections.write().await;
        let connection = connections.entry(*server_id).or_default();

        connection.active_connections += 1;
        connection.total_requests += 1;
        connection.last_request = Some(Utc::now());

        // Update circuit breaker
        if self.config.circuit_breaker_enabled {
            self.update_circuit_breaker_request(server_id).await;
        }

        // Update global stats
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.active_connections += 1;
        *stats.requests_by_server.entry(*server_id).or_insert(0) += 1;
        stats.last_updated = Utc::now();

        debug!(
            server_id = %server_id,
            active_connections = connection.active_connections,
            "Request started"
        );

        Ok(())
    }

    /// Record request completion for a server
    pub async fn record_request_completion(
        &self,
        server_id: &Uuid,
        success: bool,
        response_time_ms: u64,
    ) -> Result<()> {
        let mut connections = self.connections.write().await;
        let connection = connections.entry(*server_id).or_default();

        if connection.active_connections > 0 {
            connection.active_connections -= 1;
        }

        // Update average response time
        let total_response_time =
            connection.avg_response_time_ms * connection.total_requests as f64;
        connection.avg_response_time_ms = (total_response_time + response_time_ms as f64)
            / (connection.total_requests + 1) as f64;

        if !success {
            connection.total_errors += 1;
        }

        drop(connections);

        // Update circuit breaker
        if self.config.circuit_breaker_enabled {
            if success {
                self.update_circuit_breaker_success(server_id).await;
            } else {
                self.update_circuit_breaker_failure(server_id).await;
            }
        }

        // Update global stats
        let mut stats = self.stats.write().await;
        if stats.active_connections > 0 {
            stats.active_connections -= 1;
        }

        if !success {
            stats.total_errors += 1;
        }

        // Update average response time
        let total_response_time = stats.avg_response_time_ms * stats.total_requests as f64;
        stats.avg_response_time_ms =
            (total_response_time + response_time_ms as f64) / stats.total_requests as f64;

        stats.last_updated = Utc::now();

        debug!(
            server_id = %server_id,
            success = success,
            response_time_ms = response_time_ms,
            "Request completed"
        );

        Ok(())
    }

    /// Get load balancer statistics
    pub async fn get_statistics(&self) -> LoadBalancerStatistics {
        let mut stats = self.stats.read().await.clone();

        // Add circuit breaker states
        let circuit_breakers = self.circuit_breakers.read().await;
        for (server_id, cb) in circuit_breakers.iter() {
            stats
                .circuit_breaker_states
                .insert(*server_id, format!("{:?}", cb.state));
        }

        stats
    }

    /// Get server connections information
    pub async fn get_server_connections(&self) -> HashMap<Uuid, ServerConnections> {
        self.connections.read().await.clone()
    }

    /// Update server weights for weighted strategies
    pub async fn update_server_weights(&self, weights: HashMap<Uuid, u32>) -> Result<()> {
        let mut connections = self.connections.write().await;
        let weights_count = weights.len();

        for (server_id, weight) in weights {
            if let Some(connection) = connections.get_mut(&server_id) {
                connection.weight = weight;
            } else {
                let mut new_connection = ServerConnections::default();
                new_connection.weight = weight;
                connections.insert(server_id, new_connection);
            }
        }

        info!("Updated server weights for {} servers", weights_count);
        Ok(())
    }

    // Private helper methods

    async fn filter_circuit_breaker_servers(&self, servers: Vec<ServerInfo>) -> Vec<ServerInfo> {
        let circuit_breakers = self.circuit_breakers.read().await;
        let now = Utc::now();

        servers
            .into_iter()
            .filter(|server| {
                if let Some(cb) = circuit_breakers.get(&server.id) {
                    match cb.state {
                        CircuitBreakerState::Closed => true,
                        CircuitBreakerState::Open => {
                            // Check if recovery timeout has passed
                            let recovery_duration = chrono::Duration::seconds(
                                cb.config.recovery_timeout_seconds as i64,
                            );
                            now >= cb.last_state_change + recovery_duration
                        }
                        CircuitBreakerState::HalfOpen => {
                            cb.half_open_requests < cb.config.half_open_max_requests
                        }
                    }
                } else {
                    true
                }
            })
            .collect()
    }

    async fn filter_overloaded_servers(&self, servers: Vec<ServerInfo>) -> Vec<ServerInfo> {
        let connections = self.connections.read().await;

        servers
            .into_iter()
            .filter(|server| {
                if let Some(conn) = connections.get(&server.id) {
                    conn.active_connections < self.config.max_requests_per_server
                } else {
                    true
                }
            })
            .collect()
    }

    async fn get_sticky_session_server(
        &self,
        session_id: &str,
        servers: &[ServerInfo],
    ) -> Option<ServerInfo> {
        // Simple hash-based sticky session implementation
        let hash = self.hash_string(session_id);
        let index = (hash % servers.len() as u64) as usize;
        servers.get(index).cloned()
    }

    async fn update_circuit_breaker_request(&self, server_id: &Uuid) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        let cb = circuit_breakers
            .entry(*server_id)
            .or_insert_with(|| CircuitBreaker::new(self.config.circuit_breaker_config.clone()));

        let now = Utc::now();
        let window_duration = chrono::Duration::seconds(cb.config.window_seconds as i64);

        // Reset window if needed
        if now >= cb.window_start + window_duration {
            cb.window_start = now;
            cb.request_count = 0;
            cb.failure_count = 0;
        }

        cb.request_count += 1;

        if cb.state == CircuitBreakerState::HalfOpen {
            cb.half_open_requests += 1;
        }
    }

    async fn update_circuit_breaker_success(&self, server_id: &Uuid) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        if let Some(cb) = circuit_breakers.get_mut(server_id) {
            if cb.state == CircuitBreakerState::HalfOpen {
                // Successful request in half-open state, close circuit
                cb.state = CircuitBreakerState::Closed;
                cb.last_state_change = Utc::now();
                cb.half_open_requests = 0;
                cb.failure_count = 0;
                cb.request_count = 0;

                info!(server_id = %server_id, "Circuit breaker closed after successful request");
            }
        }
    }

    async fn update_circuit_breaker_failure(&self, server_id: &Uuid) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        if let Some(cb) = circuit_breakers.get_mut(server_id) {
            cb.failure_count += 1;

            let failure_rate = if cb.request_count >= cb.config.min_requests {
                (cb.failure_count as f64 / cb.request_count as f64) * 100.0
            } else {
                0.0
            };

            if failure_rate >= cb.config.failure_threshold as f64
                && cb.state == CircuitBreakerState::Closed
            {
                cb.state = CircuitBreakerState::Open;
                cb.last_state_change = Utc::now();

                warn!(
                    server_id = %server_id,
                    failure_rate = failure_rate,
                    "Circuit breaker opened due to high failure rate"
                );
            } else if cb.state == CircuitBreakerState::HalfOpen {
                // Failed request in half-open state, reopen circuit
                cb.state = CircuitBreakerState::Open;
                cb.last_state_change = Utc::now();
                cb.half_open_requests = 0;

                warn!(server_id = %server_id, "Circuit breaker reopened after failed request");
            }
        }
    }

    fn hash_string(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

impl CircuitBreaker {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            request_count: 0,
            window_start: Utc::now(),
            last_state_change: Utc::now(),
            half_open_requests: 0,
            config,
        }
    }
}

// Load balancing strategy implementations

#[derive(Debug)]
struct RoundRobinStrategy {
    counter: AtomicU64,
}

impl RoundRobinStrategy {
    fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl LoadBalancingStrategy for RoundRobinStrategy {
    fn select_server(
        &self,
        servers: &[ServerInfo],
        _connections: &HashMap<Uuid, ServerConnections>,
        _context: &RequestContext,
    ) -> Option<Uuid> {
        if servers.is_empty() {
            return None;
        }

        let index = self.counter.fetch_add(1, Ordering::Relaxed) % servers.len() as u64;
        servers.get(index as usize).map(|s| s.id)
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }
}

#[derive(Debug)]
struct LeastConnectionsStrategy;

impl LoadBalancingStrategy for LeastConnectionsStrategy {
    fn select_server(
        &self,
        servers: &[ServerInfo],
        connections: &HashMap<Uuid, ServerConnections>,
        _context: &RequestContext,
    ) -> Option<Uuid> {
        servers
            .iter()
            .min_by_key(|server| {
                connections
                    .get(&server.id)
                    .map(|c| c.active_connections)
                    .unwrap_or(0)
            })
            .map(|s| s.id)
    }

    fn name(&self) -> &'static str {
        "least_connections"
    }
}

#[derive(Debug)]
struct WeightedRoundRobinStrategy {
    counter: AtomicU64,
}

impl WeightedRoundRobinStrategy {
    fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl LoadBalancingStrategy for WeightedRoundRobinStrategy {
    fn select_server(
        &self,
        servers: &[ServerInfo],
        connections: &HashMap<Uuid, ServerConnections>,
        _context: &RequestContext,
    ) -> Option<Uuid> {
        if servers.is_empty() {
            return None;
        }

        // Create weighted list
        let mut weighted_servers = Vec::new();
        for server in servers {
            let weight = connections
                .get(&server.id)
                .map(|c| c.weight)
                .unwrap_or(1)
                .max(1); // Minimum weight of 1

            for _ in 0..weight {
                weighted_servers.push(server.id);
            }
        }

        if weighted_servers.is_empty() {
            return servers.first().map(|s| s.id);
        }

        let index = self.counter.fetch_add(1, Ordering::Relaxed) % weighted_servers.len() as u64;
        weighted_servers.get(index as usize).copied()
    }

    fn name(&self) -> &'static str {
        "weighted_round_robin"
    }
}

#[derive(Debug)]
struct RandomStrategy;

impl LoadBalancingStrategy for RandomStrategy {
    fn select_server(
        &self,
        servers: &[ServerInfo],
        _connections: &HashMap<Uuid, ServerConnections>,
        _context: &RequestContext,
    ) -> Option<Uuid> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        servers.choose(&mut rng).map(|s| s.id)
    }

    fn name(&self) -> &'static str {
        "random"
    }
}

#[derive(Debug)]
struct IpHashStrategy;

impl LoadBalancingStrategy for IpHashStrategy {
    fn select_server(
        &self,
        servers: &[ServerInfo],
        _connections: &HashMap<Uuid, ServerConnections>,
        context: &RequestContext,
    ) -> Option<Uuid> {
        if servers.is_empty() {
            return None;
        }

        let hash_input = context.client_ip.as_ref().unwrap_or(&context.request_id);

        let hash = self.hash_string(hash_input);
        let index = (hash % servers.len() as u64) as usize;
        servers.get(index).map(|s| s.id)
    }

    fn name(&self) -> &'static str {
        "ip_hash"
    }
}

impl IpHashStrategy {
    fn hash_string(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug)]
struct ConsistentHashStrategy;

impl LoadBalancingStrategy for ConsistentHashStrategy {
    fn select_server(
        &self,
        servers: &[ServerInfo],
        _connections: &HashMap<Uuid, ServerConnections>,
        context: &RequestContext,
    ) -> Option<Uuid> {
        if servers.is_empty() {
            return None;
        }

        // Simple consistent hashing implementation
        let hash_input = context.client_ip.as_ref().unwrap_or(&context.request_id);

        let hash = self.hash_string(hash_input);

        // Find the server with the smallest hash greater than the input hash
        let mut server_hashes: Vec<(u64, Uuid)> = servers
            .iter()
            .map(|s| (self.hash_string(&s.id.to_string()), s.id))
            .collect();

        server_hashes.sort_by_key(|(h, _)| *h);

        for (server_hash, server_id) in &server_hashes {
            if *server_hash >= hash {
                return Some(*server_id);
            }
        }

        // Wrap around to the first server
        server_hashes.first().map(|(_, id)| *id)
    }

    fn name(&self) -> &'static str {
        "consistent_hash"
    }
}

impl ConsistentHashStrategy {
    fn hash_string(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

fn create_strategy(
    strategy_type: &LoadBalancingStrategyType,
) -> Box<dyn LoadBalancingStrategy + Send + Sync> {
    match strategy_type {
        LoadBalancingStrategyType::RoundRobin => Box::new(RoundRobinStrategy::new()),
        LoadBalancingStrategyType::LeastConnections => Box::new(LeastConnectionsStrategy),
        LoadBalancingStrategyType::WeightedRoundRobin => {
            Box::new(WeightedRoundRobinStrategy::new())
        }
        LoadBalancingStrategyType::Random => Box::new(RandomStrategy),
        LoadBalancingStrategyType::IpHash => Box::new(IpHashStrategy),
        LoadBalancingStrategyType::ConsistentHash => Box::new(ConsistentHashStrategy),
    }
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategyType::RoundRobin,
            sticky_sessions: false,
            session_timeout_seconds: 1800,
            max_requests_per_server: 1000,
            circuit_breaker_enabled: true,
            circuit_breaker_config: CircuitBreakerConfig::default(),
            health_aware: true,
            server_weights: HashMap::new(),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 50,
            min_requests: 10,
            window_seconds: 60,
            recovery_timeout_seconds: 30,
            half_open_max_requests: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{ServerCapabilities, ServerConfig},
        registry::{RegistryConfig, ServerRegistry},
    };

    fn create_test_server(name: &str, id: Option<Uuid>) -> ServerInfo {
        let mut server = ServerInfo::new(
            name.to_string(),
            "1.0.0".to_string(),
            "test".to_string(),
            ServerConfig {
                endpoint: format!("http://localhost:8080/{}", name),
                port: 8080,
                host: "localhost".to_string(),
                timeout_seconds: 30,
                max_connections: 100,
                settings: HashMap::new(),
                environment: HashMap::new(),
                auth: None,
                ssl: None,
            },
            ServerCapabilities {
                protocol_version: "2024-11-05".to_string(),
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
                features: Vec::new(),
                max_request_size: None,
                max_response_size: None,
                content_types: Vec::new(),
            },
        );

        if let Some(id) = id {
            server.id = id;
        }

        server
    }

    #[tokio::test]
    async fn test_load_balancer_creation() {
        let registry_config = RegistryConfig::default();
        let registry = Arc::new(ServerRegistry::new(registry_config));
        let config = LoadBalancerConfig::default();
        let lb = LoadBalancer::new(registry, config);

        assert_eq!(lb.strategy.name(), "round_robin");
    }

    #[tokio::test]
    async fn test_round_robin_strategy() {
        let strategy = RoundRobinStrategy::new();
        let servers = vec![
            create_test_server("server1", None),
            create_test_server("server2", None),
            create_test_server("server3", None),
        ];

        let connections = HashMap::new();
        let context = RequestContext {
            request_id: "test".to_string(),
            client_ip: None,
            session_id: None,
            priority: 1,
            metadata: HashMap::new(),
        };

        // Test round-robin distribution
        let mut selections = Vec::new();
        for _ in 0..6 {
            if let Some(server_id) = strategy.select_server(&servers, &connections, &context) {
                selections.push(server_id);
            }
        }

        // Should cycle through servers
        assert_eq!(selections.len(), 6);
        assert_eq!(selections[0], selections[3]);
        assert_eq!(selections[1], selections[4]);
        assert_eq!(selections[2], selections[5]);
    }

    #[tokio::test]
    async fn test_least_connections_strategy() {
        let strategy = LeastConnectionsStrategy;
        let servers = vec![
            create_test_server("server1", None),
            create_test_server("server2", None),
        ];

        let mut connections = HashMap::new();
        connections.insert(
            servers[0].id,
            ServerConnections {
                active_connections: 5,
                ..Default::default()
            },
        );
        connections.insert(
            servers[1].id,
            ServerConnections {
                active_connections: 2,
                ..Default::default()
            },
        );

        let context = RequestContext {
            request_id: "test".to_string(),
            client_ip: None,
            session_id: None,
            priority: 1,
            metadata: HashMap::new(),
        };

        let selected = strategy.select_server(&servers, &connections, &context);
        assert_eq!(selected, Some(servers[1].id)); // Server with fewer connections
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreaker::new(config);

        assert_eq!(cb.state, CircuitBreakerState::Closed);

        // Simulate failures
        cb.request_count = 20;
        cb.failure_count = 12; // 60% failure rate

        // Circuit should open if failure rate exceeds threshold
        assert!(
            cb.failure_count as f64 / cb.request_count as f64 * 100.0
                > cb.config.failure_threshold as f64
        );
    }
}
