use anyhow::Result;
use prometheus::{Counter, Gauge, Histogram, Registry};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;
use tracing::{error, info, warn};

use crate::models::{ServiceHealth, ServiceMetrics, ServiceDependency};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusData {
    pub result: Vec<PrometheusResult>,
    #[serde(rename = "resultType")]
    pub result_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusResult {
    pub metric: HashMap<String, String>,
    pub value: Option<(f64, String)>,
    pub values: Option<Vec<(f64, String)>>,
}

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    http_client: Client,
    registry: Arc<Registry>,
    metrics: Arc<tokio::sync::RwLock<HashMap<String, ServiceMetrics>>>,
    service_health: Arc<tokio::sync::RwLock<HashMap<String, ServiceHealth>>>,

    // Prometheus metrics
    requests_total: Counter,
    request_duration: Histogram,
    active_connections: Gauge,
    error_rate: Gauge,
    slo_compliance: Gauge,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());

        let requests_total = Counter::new(
            "sre_monitor_requests_total",
            "Total number of requests processed by SRE monitor"
        ).unwrap();

        let request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "sre_monitor_request_duration_seconds",
                "Request duration in seconds"
            ).buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        ).unwrap();

        let active_connections = Gauge::new(
            "sre_monitor_active_connections",
            "Number of active database connections"
        ).unwrap();

        let error_rate = Gauge::new(
            "sre_monitor_error_rate",
            "Current error rate percentage"
        ).unwrap();

        let slo_compliance = Gauge::new(
            "sre_monitor_slo_compliance",
            "SLO compliance percentage"
        ).unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();
        registry.register(Box::new(active_connections.clone())).unwrap();
        registry.register(Box::new(error_rate.clone())).unwrap();
        registry.register(Box::new(slo_compliance.clone())).unwrap();

        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            registry,
            metrics: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            service_health: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            requests_total,
            request_duration,
            active_connections,
            error_rate,
            slo_compliance,
        }
    }

    pub async fn collect_metrics(&self) -> Result<()> {
        let start_time = Instant::now();

        info!("Starting metrics collection cycle");

        // Collect from all known services
        let services = vec![
            "api-gateway",
            "intent-parser-server",
            "mcp-manager-server",
            "federation-server",
            "test-data-api",
            "chaos-monkey"
        ];

        for service in services {
            if let Err(e) = self.collect_service_metrics(service).await {
                error!("Failed to collect metrics for service {}: {}", service, e);
            }

            if let Err(e) = self.update_service_health(service).await {
                error!("Failed to update health for service {}: {}", service, e);
            }
        }

        let collection_duration = start_time.elapsed();
        self.request_duration.observe(collection_duration.as_secs_f64());

        info!("Metrics collection completed in {:?}", collection_duration);
        Ok(())
    }

    async fn collect_service_metrics(&self, service_name: &str) -> Result<()> {
        let now = chrono::Utc::now();

        // Try to get metrics from Prometheus if available
        let metrics = if let Ok(prom_metrics) = self.collect_from_prometheus(service_name).await {
            prom_metrics
        } else {
            // Fallback to direct service health check
            self.collect_from_direct_check(service_name).await?
        };

        // Update metrics store
        let mut metrics_store = self.metrics.write().await;
        metrics_store.insert(service_name.to_string(), metrics);

        Ok(())
    }

    async fn collect_from_prometheus(&self, service_name: &str) -> Result<ServiceMetrics> {
        let base_url = "http://localhost:9090"; // TODO: Make configurable
        let now = chrono::Utc::now();

        // Query for various metrics
        let latency_p50 = self.query_prometheus_metric(
            base_url,
            &format!("histogram_quantile(0.5, rate(http_request_duration_seconds_bucket{{service=\"{}\"}}[5m]))", service_name)
        ).await.unwrap_or(0.0);

        let latency_p95 = self.query_prometheus_metric(
            base_url,
            &format!("histogram_quantile(0.95, rate(http_request_duration_seconds_bucket{{service=\"{}\"}}[5m]))", service_name)
        ).await.unwrap_or(0.0);

        let latency_p99 = self.query_prometheus_metric(
            base_url,
            &format!("histogram_quantile(0.99, rate(http_request_duration_seconds_bucket{{service=\"{}\"}}[5m]))", service_name)
        ).await.unwrap_or(0.0);

        let error_rate = self.query_prometheus_metric(
            base_url,
            &format!("rate(http_requests_total{{service=\"{}\", status=~\"5.*\"}}[5m]) / rate(http_requests_total{{service=\"{}\"}}[5m]) * 100", service_name, service_name)
        ).await.unwrap_or(0.0);

        let throughput = self.query_prometheus_metric(
            base_url,
            &format!("rate(http_requests_total{{service=\"{}\"}}[5m])", service_name)
        ).await.unwrap_or(0.0);

        let cpu_usage = self.query_prometheus_metric(
            base_url,
            &format!("rate(process_cpu_seconds_total{{service=\"{}\"}}[5m]) * 100", service_name)
        ).await.unwrap_or(0.0);

        let memory_usage = self.query_prometheus_metric(
            base_url,
            &format!("process_resident_memory_bytes{{service=\"{}\"}}", service_name)
        ).await.unwrap_or(0.0);

        Ok(ServiceMetrics {
            service_name: service_name.to_string(),
            timestamp: now,
            latency_p50: latency_p50 * 1000.0, // Convert to milliseconds
            latency_p95: latency_p95 * 1000.0,
            latency_p99: latency_p99 * 1000.0,
            error_rate,
            throughput,
            availability: if error_rate < 1.0 { 99.9 } else { 99.0 },
            cpu_usage,
            memory_usage: memory_usage / 1024.0 / 1024.0, // Convert to MB
            disk_usage: 0.0, // TODO: Add disk usage collection
        })
    }

    async fn query_prometheus_metric(&self, base_url: &str, query: &str) -> Result<f64> {
        let url = format!("{}/api/v1/query", base_url);
        let response = self.http_client
            .get(&url)
            .query(&[("query", query)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Prometheus query failed: {}", response.status()));
        }

        let prom_response: PrometheusResponse = response.json().await?;

        if let Some(result) = prom_response.data.result.first() {
            if let Some((_, value_str)) = &result.value {
                return Ok(value_str.parse::<f64>().unwrap_or(0.0));
            }
        }

        Ok(0.0)
    }

    async fn collect_from_direct_check(&self, service_name: &str) -> Result<ServiceMetrics> {
        let now = chrono::Utc::now();
        let start_time = Instant::now();

        // Determine service URL based on service name
        let service_url = self.get_service_url(service_name);
        let health_endpoint = format!("{}/health", service_url);

        // Perform health check
        let response = self.http_client
            .get(&health_endpoint)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        let (availability, response_time, error_rate) = match response {
            Ok(resp) if resp.status().is_success() => {
                let response_time = start_time.elapsed().as_millis() as f64;
                (99.9, response_time, 0.0)
            }
            Ok(resp) => {
                let response_time = start_time.elapsed().as_millis() as f64;
                warn!("Service {} returned non-success status: {}", service_name, resp.status());
                (95.0, response_time, 5.0)
            }
            Err(e) => {
                let response_time = start_time.elapsed().as_millis() as f64;
                error!("Failed to check health for service {}: {}", service_name, e);
                (0.0, response_time, 100.0)
            }
        };

        Ok(ServiceMetrics {
            service_name: service_name.to_string(),
            timestamp: now,
            latency_p50: response_time,
            latency_p95: response_time * 1.2,
            latency_p99: response_time * 1.5,
            error_rate,
            throughput: if availability > 95.0 { 10.0 } else { 0.0 },
            availability,
            cpu_usage: 0.0, // Cannot determine without proper monitoring
            memory_usage: 0.0,
            disk_usage: 0.0,
        })
    }

    async fn update_service_health(&self, service_name: &str) -> Result<()> {
        let metrics = {
            let metrics_store = self.metrics.read().await;
            metrics_store.get(service_name).cloned()
        };

        if let Some(metrics) = metrics {
            let health = ServiceHealth {
                service_name: service_name.to_string(),
                status: self.determine_health_status(&metrics),
                last_check: metrics.timestamp,
                uptime_percentage: metrics.availability,
                response_time: metrics.latency_p95,
                error_count: (metrics.error_rate * 100.0) as u64,
                health_score: self.calculate_health_score(&metrics),
                dependencies: self.get_service_dependencies(service_name).await,
            };

            let mut health_store = self.service_health.write().await;
            health_store.insert(service_name.to_string(), health);
        }

        Ok(())
    }

    fn determine_health_status(&self, metrics: &ServiceMetrics) -> String {
        if metrics.availability < 95.0 || metrics.error_rate > 5.0 {
            "critical".to_string()
        } else if metrics.availability < 99.0 || metrics.error_rate > 1.0 || metrics.latency_p95 > 2000.0 {
            "warning".to_string()
        } else if metrics.availability >= 99.9 && metrics.latency_p95 < 500.0 {
            "healthy".to_string()
        } else {
            "warning".to_string()
        }
    }

    fn calculate_health_score(&self, metrics: &ServiceMetrics) -> f64 {
        let availability_score = metrics.availability / 100.0;
        let latency_score = (2000.0 - metrics.latency_p95.min(2000.0)) / 2000.0;
        let error_score = (100.0 - metrics.error_rate.min(100.0)) / 100.0;
        let throughput_score = (metrics.throughput / 100.0).min(1.0);

        (availability_score * 0.4 + latency_score * 0.3 + error_score * 0.2 + throughput_score * 0.1) * 100.0
    }

    async fn get_service_dependencies(&self, service_name: &str) -> Vec<ServiceDependency> {
        let mut dependencies = Vec::new();

        // Define service dependencies
        let deps = match service_name {
            "api-gateway" => vec!["intent-parser-server", "mcp-manager-server", "federation-server"],
            "federation-server" => vec!["api-gateway"],
            "chaos-monkey" => vec!["api-gateway"],
            _ => vec!["api-gateway"], // Most services depend on the API gateway
        };

        for dep_service in deps {
            let dep_url = self.get_service_url(dep_service);
            let start_time = Instant::now();

            let status = match self.http_client
                .get(&format!("{}/health", dep_url))
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => "healthy",
                Ok(_) => "warning",
                Err(_) => "critical",
            };

            dependencies.push(ServiceDependency {
                name: dep_service.to_string(),
                status: status.to_string(),
                latency: start_time.elapsed().as_millis() as f64,
                last_check: chrono::Utc::now(),
            });
        }

        dependencies
    }

    fn get_service_url(&self, service_name: &str) -> String {
        match service_name {
            "api-gateway" => "http://localhost:8000".to_string(),
            "intent-parser-server" => "http://localhost:8001".to_string(),
            "mcp-manager-server" => "http://localhost:8002".to_string(),
            "federation-server" => "http://localhost:8003".to_string(),
            "test-data-api" => "http://localhost:8004".to_string(),
            "chaos-monkey" => "http://localhost:8005".to_string(),
            _ => format!("http://localhost:8080"), // Default port
        }
    }

    pub async fn get_all_metrics(&self) -> Result<HashMap<String, ServiceMetrics>> {
        let metrics_store = self.metrics.read().await;
        Ok(metrics_store.clone())
    }

    pub async fn get_service_metrics(&self, service_name: &str) -> Result<Option<ServiceMetrics>> {
        let metrics_store = self.metrics.read().await;
        Ok(metrics_store.get(service_name).cloned())
    }

    pub async fn get_service_health_summary(&self) -> Result<HashMap<String, ServiceHealth>> {
        let health_store = self.service_health.read().await;
        Ok(health_store.clone())
    }

    pub async fn get_service_specific_health(&self, service_name: &str) -> Result<ServiceHealth> {
        let health_store = self.service_health.read().await;
        health_store.get(service_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Service {} not found", service_name))
    }

    pub fn record_request(&self) {
        self.requests_total.inc();
    }

    pub fn record_request_duration(&self, duration: Duration) {
        self.request_duration.observe(duration.as_secs_f64());
    }

    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count as f64);
    }

    pub fn set_error_rate(&self, rate: f64) {
        self.error_rate.set(rate);
    }

    pub fn set_slo_compliance(&self, compliance: f64) {
        self.slo_compliance.set(compliance);
    }

    pub fn get_prometheus_registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }

    pub async fn export_metrics(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_else(|e| {
            error!("Failed to encode metrics: {}", e);
            String::new()
        })
    }

    pub async fn cleanup_old_metrics(&self, retention_hours: u64) -> Result<()> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(retention_hours as i64);

        let mut metrics_store = self.metrics.write().await;
        let mut health_store = self.service_health.write().await;

        // Remove old metrics
        metrics_store.retain(|_, metrics| metrics.timestamp > cutoff_time);
        health_store.retain(|_, health| health.last_check > cutoff_time);

        info!("Cleaned up metrics older than {} hours", retention_hours);
        Ok(())
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(!collector.registry.gather().is_empty());
    }

    #[test]
    fn test_determine_health_status() {
        let collector = MetricsCollector::new();

        let healthy_metrics = ServiceMetrics {
            service_name: "test".to_string(),
            timestamp: Utc::now(),
            latency_p50: 100.0,
            latency_p95: 200.0,
            latency_p99: 300.0,
            error_rate: 0.1,
            throughput: 50.0,
            availability: 99.95,
            cpu_usage: 20.0,
            memory_usage: 512.0,
            disk_usage: 10.0,
        };

        assert_eq!(collector.determine_health_status(&healthy_metrics), "healthy");

        let critical_metrics = ServiceMetrics {
            service_name: "test".to_string(),
            timestamp: Utc::now(),
            latency_p50: 100.0,
            latency_p95: 200.0,
            latency_p99: 300.0,
            error_rate: 10.0,
            throughput: 50.0,
            availability: 90.0,
            cpu_usage: 20.0,
            memory_usage: 512.0,
            disk_usage: 10.0,
        };

        assert_eq!(collector.determine_health_status(&critical_metrics), "critical");
    }

    #[test]
    fn test_calculate_health_score() {
        let collector = MetricsCollector::new();

        let perfect_metrics = ServiceMetrics {
            service_name: "test".to_string(),
            timestamp: Utc::now(),
            latency_p50: 50.0,
            latency_p95: 100.0,
            latency_p99: 150.0,
            error_rate: 0.0,
            throughput: 100.0,
            availability: 100.0,
            cpu_usage: 10.0,
            memory_usage: 256.0,
            disk_usage: 5.0,
        };

        let score = collector.calculate_health_score(&perfect_metrics);
        assert!(score > 95.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn test_service_url_mapping() {
        let collector = MetricsCollector::new();

        assert_eq!(collector.get_service_url("api-gateway"), "http://localhost:8000");
        assert_eq!(collector.get_service_url("intent-parser-server"), "http://localhost:8001");
        assert_eq!(collector.get_service_url("unknown-service"), "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_metrics_storage() {
        let collector = MetricsCollector::new();

        let test_metrics = ServiceMetrics {
            service_name: "test-service".to_string(),
            timestamp: Utc::now(),
            latency_p50: 100.0,
            latency_p95: 200.0,
            latency_p99: 300.0,
            error_rate: 1.0,
            throughput: 50.0,
            availability: 99.0,
            cpu_usage: 25.0,
            memory_usage: 512.0,
            disk_usage: 15.0,
        };

        {
            let mut metrics_store = collector.metrics.write().await;
            metrics_store.insert("test-service".to_string(), test_metrics.clone());
        }

        let retrieved = collector.get_service_metrics("test-service").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().service_name, "test-service");
    }
}
