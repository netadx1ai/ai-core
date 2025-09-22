//! Threat Detection Module
//!
//! Provides threat detection and security monitoring capabilities.

use crate::errors::SecurityResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Threat level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatLevel {
    /// Low threat level
    Low,
    /// Medium threat level
    Medium,
    /// High threat level
    High,
    /// Critical threat level
    Critical,
}

/// Security alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAlert {
    /// Multiple failed login attempts
    BruteForce {
        ip: IpAddr,
        attempts: u32,
        time_window: Duration,
    },
    /// Suspicious user agent
    SuspiciousUserAgent { user_agent: String, ip: IpAddr },
    /// Geographic anomaly
    GeographicAnomaly {
        user_id: String,
        current_location: String,
        previous_location: String,
    },
    /// Rate limit exceeded repeatedly
    RateLimitAbuse { ip: IpAddr, violations: u32 },
    /// Potential SQL injection attempt
    SqlInjection { ip: IpAddr, pattern: String },
    /// Potential XSS attempt
    CrossSiteScripting { ip: IpAddr, pattern: String },
    /// Unusual access patterns
    AnomalousAccess { user_id: String, pattern: String },
}

/// Threat detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    /// Maximum login attempts before flagging
    pub max_login_attempts: u32,
    /// Time window for login attempts
    pub login_attempt_window: Duration,
    /// IP blacklist duration
    pub ip_blacklist_duration: Duration,
    /// Suspicious activity threshold
    pub suspicious_activity_threshold: u32,
    /// Enable geographic anomaly detection
    pub enable_geo_detection: bool,
    /// Enable pattern-based detection
    pub enable_pattern_detection: bool,
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            max_login_attempts: 5,
            login_attempt_window: Duration::from_secs(900), // 15 minutes
            ip_blacklist_duration: Duration::from_secs(3600), // 1 hour
            suspicious_activity_threshold: 10,
            enable_geo_detection: true,
            enable_pattern_detection: true,
        }
    }
}

/// Login attempt tracking
#[derive(Debug, Clone)]
struct LoginAttempt {
    timestamp: Instant,
    success: bool,
    user_agent: Option<String>,
}

/// IP threat information
#[derive(Debug, Clone)]
struct IpThreatInfo {
    login_attempts: Vec<LoginAttempt>,
    threat_level: ThreatLevel,
    blacklisted_until: Option<Instant>,
    violations: u32,
    first_seen: Instant,
    last_activity: Instant,
}

impl IpThreatInfo {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            login_attempts: Vec::new(),
            threat_level: ThreatLevel::Low,
            blacklisted_until: None,
            violations: 0,
            first_seen: now,
            last_activity: now,
        }
    }

    fn add_login_attempt(&mut self, success: bool, user_agent: Option<String>) {
        self.login_attempts.push(LoginAttempt {
            timestamp: Instant::now(),
            success,
            user_agent,
        });
        self.last_activity = Instant::now();

        // Keep only recent attempts
        let cutoff = Instant::now() - Duration::from_secs(3600);
        self.login_attempts
            .retain(|attempt| attempt.timestamp > cutoff);
    }

    fn get_failed_attempts_in_window(&self, window: Duration) -> u32 {
        let cutoff = Instant::now() - window;
        self.login_attempts
            .iter()
            .filter(|attempt| attempt.timestamp > cutoff && !attempt.success)
            .count() as u32
    }

    fn is_blacklisted(&self) -> bool {
        if let Some(blacklisted_until) = self.blacklisted_until {
            Instant::now() < blacklisted_until
        } else {
            false
        }
    }

    fn blacklist(&mut self, duration: Duration) {
        self.blacklisted_until = Some(Instant::now() + duration);
        self.threat_level = ThreatLevel::High;
    }

    fn update_threat_level(&mut self, config: &ThreatDetectionConfig) {
        let failed_attempts = self.get_failed_attempts_in_window(config.login_attempt_window);

        self.threat_level = if failed_attempts >= config.max_login_attempts {
            ThreatLevel::Critical
        } else if failed_attempts >= config.max_login_attempts / 2 {
            ThreatLevel::High
        } else if self.violations > 0 {
            ThreatLevel::Medium
        } else {
            ThreatLevel::Low
        };
    }
}

/// Threat detector service
pub struct ThreatDetector {
    config: ThreatDetectionConfig,
    ip_threats: Arc<RwLock<HashMap<IpAddr, IpThreatInfo>>>,
    user_activities: Arc<RwLock<HashMap<String, Vec<String>>>>,
    suspicious_patterns: Vec<String>,
}

impl ThreatDetector {
    /// Create new threat detector
    pub fn new(config: ThreatDetectionConfig) -> Self {
        let suspicious_patterns = vec![
            // SQL injection patterns
            "union select".to_string(),
            "drop table".to_string(),
            "insert into".to_string(),
            "delete from".to_string(),
            // XSS patterns
            "<script>".to_string(),
            "javascript:".to_string(),
            "onload=".to_string(),
            "onerror=".to_string(),
            // Path traversal
            "../".to_string(),
            "..\\".to_string(),
            "/etc/passwd".to_string(),
        ];

        Self {
            config,
            ip_threats: Arc::new(RwLock::new(HashMap::new())),
            user_activities: Arc::new(RwLock::new(HashMap::new())),
            suspicious_patterns,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ThreatDetectionConfig::default())
    }

    /// Record login attempt
    pub async fn record_login_attempt(
        &self,
        ip: IpAddr,
        success: bool,
        user_agent: Option<String>,
    ) -> SecurityResult<Option<SecurityAlert>> {
        let mut ip_threats = self.ip_threats.write().await;
        let threat_info = ip_threats.entry(ip).or_insert_with(IpThreatInfo::new);

        threat_info.add_login_attempt(success, user_agent.clone());
        threat_info.update_threat_level(&self.config);

        let failed_attempts =
            threat_info.get_failed_attempts_in_window(self.config.login_attempt_window);

        if failed_attempts >= self.config.max_login_attempts {
            threat_info.blacklist(self.config.ip_blacklist_duration);

            return Ok(Some(SecurityAlert::BruteForce {
                ip,
                attempts: failed_attempts,
                time_window: self.config.login_attempt_window,
            }));
        }

        // Check for suspicious user agent
        if let Some(ref ua) = user_agent {
            if self.is_suspicious_user_agent(ua) {
                return Ok(Some(SecurityAlert::SuspiciousUserAgent {
                    user_agent: ua.clone(),
                    ip,
                }));
            }
        }

        Ok(None)
    }

    /// Check if IP is blacklisted
    pub async fn is_ip_blacklisted(&self, ip: IpAddr) -> bool {
        let ip_threats = self.ip_threats.read().await;
        ip_threats
            .get(&ip)
            .map(|info| info.is_blacklisted())
            .unwrap_or(false)
    }

    /// Get threat level for IP
    pub async fn get_ip_threat_level(&self, ip: IpAddr) -> ThreatLevel {
        let ip_threats = self.ip_threats.read().await;
        ip_threats
            .get(&ip)
            .map(|info| info.threat_level)
            .unwrap_or(ThreatLevel::Low)
    }

    /// Record suspicious activity
    pub async fn record_suspicious_activity(
        &self,
        ip: IpAddr,
        _activity: String,
    ) -> SecurityResult<Option<SecurityAlert>> {
        let mut ip_threats = self.ip_threats.write().await;
        let threat_info = ip_threats.entry(ip).or_insert_with(IpThreatInfo::new);

        threat_info.violations += 1;
        threat_info.last_activity = Instant::now();
        threat_info.update_threat_level(&self.config);

        if threat_info.violations >= self.config.suspicious_activity_threshold {
            return Ok(Some(SecurityAlert::RateLimitAbuse {
                ip,
                violations: threat_info.violations,
            }));
        }

        Ok(None)
    }

    /// Analyze input for malicious patterns
    pub async fn analyze_input(
        &self,
        ip: IpAddr,
        input: &str,
    ) -> SecurityResult<Vec<SecurityAlert>> {
        if !self.config.enable_pattern_detection {
            return Ok(vec![]);
        }

        let mut alerts = Vec::new();
        let input_lower = input.to_lowercase();

        for pattern in &self.suspicious_patterns {
            if input_lower.contains(pattern) {
                let alert = if pattern.contains("select")
                    || pattern.contains("drop")
                    || pattern.contains("insert")
                {
                    SecurityAlert::SqlInjection {
                        ip,
                        pattern: pattern.clone(),
                    }
                } else if pattern.contains("script") || pattern.contains("javascript") {
                    SecurityAlert::CrossSiteScripting {
                        ip,
                        pattern: pattern.clone(),
                    }
                } else {
                    SecurityAlert::AnomalousAccess {
                        user_id: "unknown".to_string(),
                        pattern: pattern.clone(),
                    }
                };
                alerts.push(alert);
            }
        }

        Ok(alerts)
    }

    /// Record user activity for anomaly detection
    pub async fn record_user_activity(
        &self,
        user_id: String,
        activity: String,
    ) -> SecurityResult<Option<SecurityAlert>> {
        let mut activities = self.user_activities.write().await;
        let user_activities = activities.entry(user_id.clone()).or_insert_with(Vec::new);

        user_activities.push(activity.clone());

        // Keep only recent activities
        if user_activities.len() > 100 {
            user_activities.drain(0..50);
        }

        // Check for unusual patterns
        if self.is_unusual_activity_pattern(user_activities) {
            return Ok(Some(SecurityAlert::AnomalousAccess {
                user_id,
                pattern: "unusual_activity_pattern".to_string(),
            }));
        }

        Ok(None)
    }

    /// Clean up old threat data
    pub async fn cleanup_old_data(&self) -> SecurityResult<u32> {
        let cutoff = Instant::now() - Duration::from_secs(86400); // 24 hours
        let mut removed_count = 0;

        let mut ip_threats = self.ip_threats.write().await;
        let initial_count = ip_threats.len();

        ip_threats.retain(|_, info| info.last_activity > cutoff && !info.is_blacklisted());

        removed_count += (initial_count - ip_threats.len()) as u32;

        Ok(removed_count)
    }

    /// Get threat statistics
    pub async fn get_threat_stats(&self) -> HashMap<String, u32> {
        let ip_threats = self.ip_threats.read().await;
        let mut stats = HashMap::new();

        let mut low_count = 0;
        let mut medium_count = 0;
        let mut high_count = 0;
        let mut critical_count = 0;
        let mut blacklisted_count = 0;

        for threat_info in ip_threats.values() {
            match threat_info.threat_level {
                ThreatLevel::Low => low_count += 1,
                ThreatLevel::Medium => medium_count += 1,
                ThreatLevel::High => high_count += 1,
                ThreatLevel::Critical => critical_count += 1,
            }

            if threat_info.is_blacklisted() {
                blacklisted_count += 1;
            }
        }

        stats.insert("total_ips".to_string(), ip_threats.len() as u32);
        stats.insert("low_threat".to_string(), low_count);
        stats.insert("medium_threat".to_string(), medium_count);
        stats.insert("high_threat".to_string(), high_count);
        stats.insert("critical_threat".to_string(), critical_count);
        stats.insert("blacklisted".to_string(), blacklisted_count);

        stats
    }

    /// Check if user agent is suspicious
    fn is_suspicious_user_agent(&self, user_agent: &str) -> bool {
        let suspicious_agents = [
            "bot", "crawler", "spider", "scraper", "curl", "wget", "python", "php", "sql",
        ];

        let ua_lower = user_agent.to_lowercase();
        suspicious_agents
            .iter()
            .any(|&agent| ua_lower.contains(agent))
    }

    /// Check if activity pattern is unusual
    fn is_unusual_activity_pattern(&self, activities: &[String]) -> bool {
        if activities.len() < 10 {
            return false;
        }

        // Check for rapid repetitive actions
        let recent_activities = &activities[activities.len().saturating_sub(10)..];
        let unique_activities: std::collections::HashSet<_> = recent_activities.iter().collect();

        // If less than 3 unique activities in last 10, it's suspicious
        unique_activities.len() < 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_brute_force_detection() {
        let mut config = ThreatDetectionConfig::default();
        config.max_login_attempts = 3;

        let detector = ThreatDetector::new(config);
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // First few attempts should not trigger alert
        let result = detector
            .record_login_attempt(ip, false, None)
            .await
            .unwrap();
        assert!(result.is_none());

        let result = detector
            .record_login_attempt(ip, false, None)
            .await
            .unwrap();
        assert!(result.is_none());

        // Third attempt should trigger brute force alert
        let result = detector
            .record_login_attempt(ip, false, None)
            .await
            .unwrap();
        assert!(matches!(result, Some(SecurityAlert::BruteForce { .. })));

        // IP should now be blacklisted
        assert!(detector.is_ip_blacklisted(ip).await);
    }

    #[tokio::test]
    async fn test_suspicious_user_agent_detection() {
        let detector = ThreatDetector::with_defaults();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        let result = detector
            .record_login_attempt(ip, false, Some("python-requests/2.25.1".to_string()))
            .await
            .unwrap();

        assert!(matches!(
            result,
            Some(SecurityAlert::SuspiciousUserAgent { .. })
        ));
    }

    #[tokio::test]
    async fn test_malicious_pattern_detection() {
        let detector = ThreatDetector::with_defaults();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        let alerts = detector
            .analyze_input(ip, "'; DROP TABLE users; --")
            .await
            .unwrap();

        assert!(!alerts.is_empty());
        assert!(matches!(alerts[0], SecurityAlert::SqlInjection { .. }));
    }

    #[tokio::test]
    async fn test_threat_level_escalation() {
        let detector = ThreatDetector::with_defaults();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // Initial threat level should be low
        assert_eq!(detector.get_ip_threat_level(ip).await, ThreatLevel::Low);

        // Record some failed attempts
        detector
            .record_login_attempt(ip, false, None)
            .await
            .unwrap();
        detector
            .record_login_attempt(ip, false, None)
            .await
            .unwrap();

        // Threat level should increase
        let level = detector.get_ip_threat_level(ip).await;
        assert!(matches!(level, ThreatLevel::Medium | ThreatLevel::High));
    }

    #[tokio::test]
    async fn test_cleanup_old_data() {
        let detector = ThreatDetector::with_defaults();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();

        // Record some activity
        detector
            .record_login_attempt(ip, false, None)
            .await
            .unwrap();

        // Stats should show 1 IP
        let stats = detector.get_threat_stats().await;
        assert_eq!(stats.get("total_ips"), Some(&1));

        // Cleanup shouldn't remove recent data
        let removed = detector.cleanup_old_data().await.unwrap();
        assert_eq!(removed, 0);
    }

    #[tokio::test]
    async fn test_unusual_activity_detection() {
        let detector = ThreatDetector::with_defaults();
        let user_id = "test_user".to_string();

        // Record repetitive activity
        for _ in 0..10 {
            let result = detector
                .record_user_activity(user_id.clone(), "login".to_string())
                .await
                .unwrap();

            if result.is_some() {
                assert!(matches!(
                    result,
                    Some(SecurityAlert::AnomalousAccess { .. })
                ));
                break;
            }
        }
    }
}
