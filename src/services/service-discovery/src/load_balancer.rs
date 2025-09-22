//! Load Balancer Module
//!
//! Provides various load balancing strategies for distributing requests across service instances.
//! Supports round-robin, least connections, weighted round-robin, consistent hash, random, and IP hash strategies.

use crate::config::ServiceDiscoveryConfig;
use crate::models::{LoadBalancerStats, LoadBalancingStrategy, ResponseTimeStats, ServiceInstance};

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Load balancer trait for dependency injection
#[async_trait]
pub trait LoadBalancer: Send + Sync {
    /// Select a service instance using the specified strategy
    async fn select_instance(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
        strategy: LoadBalancingStrategy,
        client_key: Option<&str>,
    ) -> Result<Option<ServiceInstance>>;

    /// Record request statistics for load balancing decisions
    async fn record_request(
        &self,
        service_id: Uuid,
        response_time: Duration,
        success: bool,
    ) -> Result<()>;

    /// Get load balancer statistics
    async fn get_stats(&self, service_name: &str) -> Result<LoadBalancerStats>;

    /// Reset statistics for a service
    async fn reset_stats(&self, service_name: &str) -> Result<()>;
}

/// Connection tracking information
#[derive(Debug)]
struct ConnectionInfo {
    /// Current active connections
    active_connections: AtomicU64,

    /// Total requests handled
    total_requests: AtomicU64,

    /// Total response time in milliseconds
    total_response_time_ms: AtomicU64,

    /// Number of failed requests
    failed_requests: AtomicU64,

    /// Last request timestamp
    last_request_time: RwLock<Option<Instant>>,

    /// Response time samples for percentile calculations
    response_time_samples: RwLock<Vec<u64>>,
}

impl ConnectionInfo {
    fn new() -> Self {
        Self {
            active_connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            last_request_time: RwLock::new(None),
            response_time_samples: RwLock::new(Vec::new()),
        }
    }

    /// Record a new request
    async fn record_request(&self, response_time: Duration, success: bool) {
        let response_time_ms = response_time.as_millis() as u64;

        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms
            .fetch_add(response_time_ms, Ordering::Relaxed);

        if !success {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        *self.last_request_time.write().await = Some(Instant::now());

        // Store response time sample (keep only last 1000 samples)
        let mut samples = self.response_time_samples.write().await;
        samples.push(response_time_ms);
        if samples.len() > 1000 {
            samples.remove(0);
        }
    }

    /// Get average response time
    fn avg_response_time_ms(&self) -> f64 {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        if total_requests == 0 {
            0.0
        } else {
            self.total_response_time_ms.load(Ordering::Relaxed) as f64 / total_requests as f64
        }
    }

    /// Get error rate
    fn error_rate(&self) -> f64 {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        if total_requests == 0 {
            0.0
        } else {
            self.failed_requests.load(Ordering::Relaxed) as f64 / total_requests as f64
        }
    }

    /// Calculate response time percentiles
    async fn get_response_time_stats(&self) -> ResponseTimeStats {
        let samples = self.response_time_samples.read().await;
        if samples.is_empty() {
            return ResponseTimeStats {
                avg_ms: 0.0,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                max_ms: 0.0,
            };
        }

        let mut sorted_samples = samples.clone();
        sorted_samples.sort_unstable();

        let avg_ms = self.avg_response_time_ms();
        let p50_ms = percentile(&sorted_samples, 50.0) as f64;
        let p95_ms = percentile(&sorted_samples, 95.0) as f64;
        let p99_ms = percentile(&sorted_samples, 99.0) as f64;
        let max_ms = *sorted_samples.last().unwrap_or(&0) as f64;

        ResponseTimeStats {
            avg_ms,
            p50_ms,
            p95_ms,
            p99_ms,
            max_ms,
        }
    }
}

/// Round-robin counter for services
#[derive(Debug)]
struct RoundRobinCounter {
    counter: AtomicUsize,
}

impl RoundRobinCounter {
    fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }

    fn next(&self, max: usize) -> usize {
        if max == 0 {
            0
        } else {
            self.counter.fetch_add(1, Ordering::Relaxed) % max
        }
    }
}

/// Weighted round-robin state
#[derive(Debug)]
struct WeightedRoundRobinState {
    /// Current weights for each instance
    current_weights: RwLock<HashMap<Uuid, i32>>,
}

impl WeightedRoundRobinState {
    fn new() -> Self {
        Self {
            current_weights: RwLock::new(HashMap::new()),
        }
    }

    /// Select next instance using weighted round-robin algorithm
    async fn select(&self, instances: &[ServiceInstance]) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        let mut weights = self.current_weights.write().await;

        // Initialize weights if needed
        for instance in instances {
            weights.entry(instance.id).or_insert(0);
        }

        // Find instance with highest current weight
        let mut selected_instance = &instances[0];
        let mut max_weight = i32::MIN;
        let mut total_weight = 0;

        for instance in instances {
            let current_weight = *weights.get(&instance.id).unwrap_or(&0);
            let effective_weight = current_weight + instance.weight as i32;

            weights.insert(instance.id, effective_weight);
            total_weight += instance.weight as i32;

            if effective_weight > max_weight {
                max_weight = effective_weight;
                selected_instance = instance;
            }
        }

        // Reduce the selected instance's weight by total weight
        if let Some(current_weight) = weights.get_mut(&selected_instance.id) {
            *current_weight -= total_weight;
        }

        Some(selected_instance.clone())
    }
}

/// Consistent hash ring for consistent hashing
#[derive(Debug)]
struct ConsistentHashRing {
    /// Virtual nodes on the ring
    ring: RwLock<std::collections::BTreeMap<u64, Uuid>>,

    /// Number of virtual nodes per instance
    virtual_nodes: usize,
}

impl ConsistentHashRing {
    fn new(virtual_nodes: usize) -> Self {
        Self {
            ring: RwLock::new(std::collections::BTreeMap::new()),
            virtual_nodes,
        }
    }

    /// Update the hash ring with new instances
    async fn update(&self, instances: &[ServiceInstance]) {
        let mut ring = self.ring.write().await;
        ring.clear();

        for instance in instances {
            for i in 0..self.virtual_nodes {
                let key = format!("{}:{}", instance.id, i);
                let hash = self.hash(&key);
                ring.insert(hash, instance.id);
            }
        }
    }

    /// Find the instance responsible for a given key
    async fn find(&self, key: &str, instances: &[ServiceInstance]) -> Option<ServiceInstance> {
        let ring = self.ring.read().await;
        if ring.is_empty() {
            return None;
        }

        let hash = self.hash(key);

        // Find the first node clockwise from the hash
        let instance_id = ring
            .range(hash..)
            .next()
            .or_else(|| ring.iter().next())
            .map(|(_, id)| *id)?;

        // Find the instance with this ID
        instances.iter().find(|i| i.id == instance_id).cloned()
    }

    /// Hash function for consistent hashing
    fn hash(&self, key: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

/// Load balancer implementation
pub struct LoadBalancerImpl {
    /// Configuration
    config: Arc<ServiceDiscoveryConfig>,

    /// Connection information per service instance
    connection_info: Arc<DashMap<Uuid, ConnectionInfo>>,

    /// Round-robin counters per service
    round_robin_counters: Arc<DashMap<String, RoundRobinCounter>>,

    /// Weighted round-robin state per service
    weighted_rr_state: Arc<DashMap<String, WeightedRoundRobinState>>,

    /// Consistent hash rings per service
    hash_rings: Arc<DashMap<String, ConsistentHashRing>>,

    /// Service statistics
    service_stats: Arc<DashMap<String, RwLock<LoadBalancerStats>>>,
}

impl LoadBalancerImpl {
    /// Create a new load balancer instance
    pub fn new(config: Arc<ServiceDiscoveryConfig>) -> Self {
        Self {
            config,
            connection_info: Arc::new(DashMap::new()),
            round_robin_counters: Arc::new(DashMap::new()),
            weighted_rr_state: Arc::new(DashMap::new()),
            hash_rings: Arc::new(DashMap::new()),
            service_stats: Arc::new(DashMap::new()),
        }
    }

    /// Select instance using round-robin strategy
    fn select_round_robin(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
    ) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        let counter = self
            .round_robin_counters
            .entry(service_name.to_string())
            .or_insert_with(RoundRobinCounter::new);

        let index = counter.next(instances.len());
        Some(instances[index].clone())
    }

    /// Select instance using least connections strategy
    fn select_least_connections(&self, instances: &[ServiceInstance]) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        let mut selected = &instances[0];
        let mut min_connections = u64::MAX;

        for instance in instances {
            let connections = self
                .connection_info
                .get(&instance.id)
                .map(|info| info.active_connections.load(Ordering::Relaxed))
                .unwrap_or(0);

            if connections < min_connections {
                min_connections = connections;
                selected = instance;
            }
        }

        Some(selected.clone())
    }

    /// Select instance using weighted round-robin strategy
    async fn select_weighted_round_robin(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
    ) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        let state = self
            .weighted_rr_state
            .entry(service_name.to_string())
            .or_insert_with(WeightedRoundRobinState::new);

        state.select(instances).await
    }

    /// Select instance using consistent hash strategy
    async fn select_consistent_hash(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
        key: &str,
    ) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        let virtual_nodes = self
            .config
            .load_balancer
            .strategies
            .consistent_hash
            .virtual_nodes as usize;

        let ring = self
            .hash_rings
            .entry(service_name.to_string())
            .or_insert_with(|| ConsistentHashRing::new(virtual_nodes));

        // Update ring with current instances
        ring.update(instances).await;

        // Find instance for key
        ring.find(key, instances).await
    }

    /// Select instance using random strategy
    fn select_random(&self, instances: &[ServiceInstance]) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        instances.choose(&mut rng).cloned()
    }

    /// Select instance using IP hash strategy
    fn select_ip_hash(
        &self,
        instances: &[ServiceInstance],
        client_ip: &str,
    ) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        client_ip.hash(&mut hasher);
        let hash = hasher.finish();

        let index = (hash as usize) % instances.len();
        Some(instances[index].clone())
    }

    /// Update service statistics
    async fn update_service_stats(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
    ) -> Result<()> {
        let stats_entry = self
            .service_stats
            .entry(service_name.to_string())
            .or_insert_with(|| {
                RwLock::new(LoadBalancerStats {
                    service_name: service_name.to_string(),
                    total_requests: 0,
                    active_connections: HashMap::new(),
                    response_times: HashMap::new(),
                    error_rates: HashMap::new(),
                    last_updated: chrono::Utc::now(),
                })
            });

        let mut stats = stats_entry.write().await;

        // Update active connections
        stats.active_connections.clear();
        for instance in instances {
            if let Some(info) = self.connection_info.get(&instance.id) {
                let connections = info.active_connections.load(Ordering::Relaxed);
                stats
                    .active_connections
                    .insert(instance.id, connections as u32);
            }
        }

        // Update response times and error rates
        stats.response_times.clear();
        stats.error_rates.clear();
        for instance in instances {
            if let Some(info) = self.connection_info.get(&instance.id) {
                let response_stats = info.get_response_time_stats().await;
                stats.response_times.insert(instance.id, response_stats);
                stats.error_rates.insert(instance.id, info.error_rate());
            }
        }

        // Update total requests
        stats.total_requests = self
            .connection_info
            .iter()
            .map(|entry| entry.total_requests.load(Ordering::Relaxed))
            .sum();

        stats.last_updated = chrono::Utc::now();

        Ok(())
    }
}

#[async_trait]
impl LoadBalancer for LoadBalancerImpl {
    async fn select_instance(
        &self,
        service_name: &str,
        instances: &[ServiceInstance],
        strategy: LoadBalancingStrategy,
        client_key: Option<&str>,
    ) -> Result<Option<ServiceInstance>> {
        if instances.is_empty() {
            return Ok(None);
        }

        // Filter healthy instances only
        let healthy_instances: Vec<_> = instances
            .iter()
            .filter(|instance| matches!(instance.status, crate::models::ServiceStatus::Healthy))
            .cloned()
            .collect();

        if healthy_instances.is_empty() {
            warn!(
                "No healthy instances available for service: {}",
                service_name
            );
            return Ok(None);
        }

        let selected = match strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.select_round_robin(service_name, &healthy_instances)
            }
            LoadBalancingStrategy::LeastConnections => {
                self.select_least_connections(&healthy_instances)
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(service_name, &healthy_instances)
                    .await
            }
            LoadBalancingStrategy::ConsistentHash => {
                if let Some(key) = client_key {
                    self.select_consistent_hash(service_name, &healthy_instances, key)
                        .await
                } else {
                    warn!(
                        "Consistent hash strategy requires client key, falling back to round-robin"
                    );
                    self.select_round_robin(service_name, &healthy_instances)
                }
            }
            LoadBalancingStrategy::Random => self.select_random(&healthy_instances),
            LoadBalancingStrategy::IpHash => {
                if let Some(client_ip) = client_key {
                    self.select_ip_hash(&healthy_instances, client_ip)
                } else {
                    warn!("IP hash strategy requires client IP, falling back to round-robin");
                    self.select_round_robin(service_name, &healthy_instances)
                }
            }
        };

        // Increment active connections for selected instance
        if let Some(ref instance) = selected {
            let info = self
                .connection_info
                .entry(instance.id)
                .or_insert_with(ConnectionInfo::new);
            info.active_connections.fetch_add(1, Ordering::Relaxed);
        }

        // Update service statistics
        self.update_service_stats(service_name, instances).await?;

        debug!(
            "Selected instance {:?} for service {} using {:?} strategy",
            selected.as_ref().map(|i| i.id),
            service_name,
            strategy
        );

        Ok(selected)
    }

    async fn record_request(
        &self,
        service_id: Uuid,
        response_time: Duration,
        success: bool,
    ) -> Result<()> {
        let info = self
            .connection_info
            .entry(service_id)
            .or_insert_with(ConnectionInfo::new);

        // Decrement active connections
        info.active_connections.fetch_sub(1, Ordering::Relaxed);

        // Record request statistics
        info.record_request(response_time, success).await;

        debug!(
            "Recorded request for service {}: {}ms, success: {}",
            service_id,
            response_time.as_millis(),
            success
        );

        Ok(())
    }

    async fn get_stats(&self, service_name: &str) -> Result<LoadBalancerStats> {
        let stats_entry = self
            .service_stats
            .get(service_name)
            .ok_or_else(|| anyhow::anyhow!("No statistics found for service: {}", service_name))?;

        let stats = stats_entry.read().await;
        Ok(stats.clone())
    }

    async fn reset_stats(&self, service_name: &str) -> Result<()> {
        // Remove service statistics
        self.service_stats.remove(service_name);

        // Reset counters
        self.round_robin_counters.remove(service_name);
        self.weighted_rr_state.remove(service_name);
        self.hash_rings.remove(service_name);

        info!(
            "Reset load balancer statistics for service: {}",
            service_name
        );
        Ok(())
    }
}

/// Calculate percentile from sorted samples
fn percentile(sorted_samples: &[u64], percentile: f64) -> u64 {
    if sorted_samples.is_empty() {
        return 0;
    }

    let index = (percentile / 100.0) * (sorted_samples.len() - 1) as f64;
    let lower_index = index.floor() as usize;
    let upper_index = index.ceil() as usize;

    if lower_index == upper_index {
        sorted_samples[lower_index]
    } else {
        let lower_value = sorted_samples[lower_index] as f64;
        let upper_value = sorted_samples[upper_index] as f64;
        let weight = index - lower_index as f64;
        (lower_value + weight * (upper_value - lower_value)) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServiceProtocol, ServiceStatus};
    use std::collections::HashMap;

    fn create_test_instances() -> Vec<ServiceInstance> {
        vec![
            ServiceInstance {
                id: Uuid::new_v4(),
                name: "test-service".to_string(),
                version: "1.0.0".to_string(),
                address: "127.0.0.1".to_string(),
                port: 8080,
                protocol: ServiceProtocol::Http,
                status: ServiceStatus::Healthy,
                weight: 100,
                metadata: HashMap::new(),
                last_health_check: None,
            },
            ServiceInstance {
                id: Uuid::new_v4(),
                name: "test-service".to_string(),
                version: "1.0.0".to_string(),
                address: "127.0.0.1".to_string(),
                port: 8081,
                protocol: ServiceProtocol::Http,
                status: ServiceStatus::Healthy,
                weight: 200,
                metadata: HashMap::new(),
                last_health_check: None,
            },
        ]
    }

    #[test]
    fn test_round_robin_counter() {
        let counter = RoundRobinCounter::new();

        assert_eq!(counter.next(3), 0);
        assert_eq!(counter.next(3), 1);
        assert_eq!(counter.next(3), 2);
        assert_eq!(counter.next(3), 0);
    }

    #[test]
    fn test_percentile_calculation() {
        let samples = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

        assert_eq!(percentile(&samples, 0.0), 10);
        assert_eq!(percentile(&samples, 50.0), 55);
        assert_eq!(percentile(&samples, 95.0), 95);
        assert_eq!(percentile(&samples, 100.0), 100);
    }

    #[tokio::test]
    async fn test_load_balancer_round_robin() {
        let config = Arc::new(ServiceDiscoveryConfig::default());
        let lb = LoadBalancerImpl::new(config);
        let instances = create_test_instances();

        let result1 = lb
            .select_instance(
                "test-service",
                &instances,
                LoadBalancingStrategy::RoundRobin,
                None,
            )
            .await
            .unwrap();

        let result2 = lb
            .select_instance(
                "test-service",
                &instances,
                LoadBalancingStrategy::RoundRobin,
                None,
            )
            .await
            .unwrap();

        assert!(result1.is_some());
        assert!(result2.is_some());
        assert_ne!(result1.as_ref().unwrap().id, result2.as_ref().unwrap().id);
    }

    #[tokio::test]
    async fn test_connection_info_stats() {
        let info = ConnectionInfo::new();

        info.record_request(Duration::from_millis(100), true).await;
        info.record_request(Duration::from_millis(200), false).await;
        info.record_request(Duration::from_millis(150), true).await;

        assert_eq!(info.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(info.failed_requests.load(Ordering::Relaxed), 1);
        assert_eq!(info.avg_response_time_ms(), 150.0);
        assert_eq!(info.error_rate(), 1.0 / 3.0);

        let stats = info.get_response_time_stats().await;
        assert_eq!(stats.avg_ms, 150.0);
        assert_eq!(stats.p50_ms, 150.0);
    }
}
