use serde::{Deserialize, Serialize};

/// Main configuration structure for the file storage service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Storage backend configuration
    pub storage: StorageConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// File processing configuration
    pub processing: ProcessingConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Feature flags
    pub features: FeatureFlags,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    #[serde(default = "default_host")]
    pub host: String,
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Maximum request size in bytes
    #[serde(default = "default_max_request_size")]
    pub max_request_size: usize,
    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    /// Maximum concurrent uploads
    #[serde(default = "default_max_concurrent_uploads")]
    pub max_concurrent_uploads: usize,
    /// Enable metrics endpoint
    #[serde(default = "default_true")]
    pub enable_metrics: bool,
}

/// Storage backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage type (s3, minio, local)
    #[serde(default = "default_storage_type")]
    pub storage_type: String,
    /// S3/MinIO configuration
    pub s3: S3Config,
    /// Local storage configuration
    pub local: LocalStorageConfig,
    /// Default bucket/container name
    #[serde(default = "default_bucket")]
    pub default_bucket: String,
    /// Enable multipart upload
    #[serde(default = "default_true")]
    pub enable_multipart: bool,
    /// Multipart chunk size in bytes
    #[serde(default = "default_multipart_chunk_size")]
    pub multipart_chunk_size: usize,
    /// Maximum file size in bytes
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,
}

/// S3/MinIO specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 endpoint URL (for MinIO or custom S3-compatible services)
    pub endpoint: Option<String>,
    /// AWS region
    #[serde(default = "default_region")]
    pub region: String,
    /// Access key ID
    pub access_key: String,
    /// Secret access key
    pub secret_key: String,
    /// Use path-style addressing
    #[serde(default = "default_false")]
    pub path_style: bool,
    /// Enable SSL/TLS
    #[serde(default = "default_true")]
    pub use_ssl: bool,
}

/// Local storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    /// Base directory for file storage
    #[serde(default = "default_storage_path")]
    pub base_path: String,
    /// Enable directory structure by date
    #[serde(default = "default_true")]
    pub organize_by_date: bool,
    /// Maximum directory depth
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// MongoDB connection URI
    pub mongodb_uri: String,
    /// MongoDB database name
    #[serde(default = "default_mongodb_database")]
    pub mongodb_database: String,
    /// PostgreSQL connection URI (for metadata caching)
    pub postgres_uri: Option<String>,
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT secret key
    pub jwt_secret: String,
    /// Token expiration time in seconds
    #[serde(default = "default_token_expiry")]
    pub token_expiry: u64,
    /// Virus scanner configuration
    pub virus_scanner: VirusScannerConfig,
    /// Enable file encryption at rest
    #[serde(default = "default_false")]
    pub enable_encryption: bool,
    /// Encryption key (base64 encoded)
    pub encryption_key: Option<String>,
    /// Allowed file types (MIME types)
    #[serde(default = "default_allowed_types")]
    pub allowed_file_types: Vec<String>,
    /// Blocked file extensions
    #[serde(default = "default_blocked_extensions")]
    pub blocked_extensions: Vec<String>,
    /// Maximum files per user
    #[serde(default = "default_max_files_per_user")]
    pub max_files_per_user: Option<usize>,
    /// Maximum storage per user in bytes
    #[serde(default = "default_max_storage_per_user")]
    pub max_storage_per_user: Option<usize>,
}

/// Virus scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusScannerConfig {
    /// Enable virus scanning
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// ClamAV daemon host
    #[serde(default = "default_clamd_host")]
    pub clamd_host: String,
    /// ClamAV daemon port
    #[serde(default = "default_clamd_port")]
    pub clamd_port: u16,
    /// Scan timeout in seconds
    #[serde(default = "default_scan_timeout")]
    pub scan_timeout: u64,
    /// Maximum file size to scan in bytes
    #[serde(default = "default_max_scan_size")]
    pub max_scan_size: usize,
    /// Quarantine infected files
    #[serde(default = "default_true")]
    pub quarantine_infected: bool,
}

/// File processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Enable image processing
    #[serde(default = "default_true")]
    pub enable_image_processing: bool,
    /// Enable video processing
    #[serde(default = "default_false")]
    pub enable_video_processing: bool,
    /// Image quality for compression (1-100)
    #[serde(default = "default_image_quality")]
    pub image_quality: u8,
    /// Thumbnail sizes to generate
    #[serde(default = "default_thumbnail_sizes")]
    pub thumbnail_sizes: Vec<ThumbnailSize>,
    /// Enable image optimization
    #[serde(default = "default_true")]
    pub enable_optimization: bool,
    /// Maximum processing time in seconds
    #[serde(default = "default_processing_timeout")]
    pub processing_timeout: u64,
    /// Supported image formats
    #[serde(default = "default_image_formats")]
    pub supported_image_formats: Vec<String>,
    /// Supported video formats
    #[serde(default = "default_video_formats")]
    pub supported_video_formats: Vec<String>,
}

/// Thumbnail size configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailSize {
    /// Size name (e.g., "small", "medium", "large")
    pub name: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Maintain aspect ratio
    #[serde(default = "default_true")]
    pub maintain_aspect_ratio: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Redis connection URI
    pub redis_uri: Option<String>,
    /// Enable file metadata caching
    #[serde(default = "default_true")]
    pub enable_metadata_cache: bool,
    /// Metadata cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub metadata_cache_ttl: u64,
    /// Enable thumbnail caching
    #[serde(default = "default_true")]
    pub enable_thumbnail_cache: bool,
    /// Thumbnail cache TTL in seconds
    #[serde(default = "default_thumbnail_cache_ttl")]
    pub thumbnail_cache_ttl: u64,
    /// Maximum cache size in bytes
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Enable JSON logging
    #[serde(default = "default_false")]
    pub json_format: bool,
    /// Log file path (optional)
    pub log_file: Option<String>,
    /// Enable access logging
    #[serde(default = "default_true")]
    pub enable_access_log: bool,
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub enable_audit_log: bool,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable file deduplication
    #[serde(default = "default_true")]
    pub enable_deduplication: bool,
    /// Enable file versioning
    #[serde(default = "default_false")]
    pub enable_versioning: bool,
    /// Enable file sharing
    #[serde(default = "default_true")]
    pub enable_sharing: bool,
    /// Enable batch operations
    #[serde(default = "default_true")]
    pub enable_batch_operations: bool,
    /// Enable file compression
    #[serde(default = "default_true")]
    pub enable_compression: bool,
    /// Enable file preview generation
    #[serde(default = "default_true")]
    pub enable_previews: bool,
    /// Enable background processing
    #[serde(default = "default_true")]
    pub enable_background_processing: bool,
}

// Default value functions

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8084
}

fn default_max_request_size() -> usize {
    100 * 1024 * 1024 // 100MB
}

fn default_request_timeout() -> u64 {
    300 // 5 minutes
}

fn default_max_concurrent_uploads() -> usize {
    10
}

fn default_storage_type() -> String {
    "s3".to_string()
}

fn default_bucket() -> String {
    "ai-core-files".to_string()
}

fn default_multipart_chunk_size() -> usize {
    5 * 1024 * 1024 // 5MB
}

fn default_max_file_size() -> usize {
    1024 * 1024 * 1024 // 1GB
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_storage_path() -> String {
    "./storage".to_string()
}

fn default_max_depth() -> u32 {
    3
}

fn default_mongodb_database() -> String {
    "ai_core_files".to_string()
}

fn default_pool_size() -> u32 {
    10
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_token_expiry() -> u64 {
    3600 // 1 hour
}

fn default_clamd_host() -> String {
    "localhost".to_string()
}

fn default_clamd_port() -> u16 {
    3310
}

fn default_scan_timeout() -> u64 {
    60
}

fn default_max_scan_size() -> usize {
    100 * 1024 * 1024 // 100MB
}

fn default_image_quality() -> u8 {
    85
}

fn default_processing_timeout() -> u64 {
    300 // 5 minutes
}

fn default_cache_ttl() -> u64 {
    3600 // 1 hour
}

fn default_thumbnail_cache_ttl() -> u64 {
    86400 // 24 hours
}

fn default_max_cache_size() -> usize {
    1024 * 1024 * 1024 // 1GB
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_files_per_user() -> Option<usize> {
    Some(10000)
}

fn default_max_storage_per_user() -> Option<usize> {
    Some(10 * 1024 * 1024 * 1024) // 10GB
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_allowed_types() -> Vec<String> {
    vec![
        "image/jpeg".to_string(),
        "image/png".to_string(),
        "image/webp".to_string(),
        "image/gif".to_string(),
        "video/mp4".to_string(),
        "video/webm".to_string(),
        "application/pdf".to_string(),
        "text/plain".to_string(),
        "application/json".to_string(),
        "application/xml".to_string(),
        "application/zip".to_string(),
    ]
}

fn default_blocked_extensions() -> Vec<String> {
    vec![
        "exe".to_string(),
        "bat".to_string(),
        "cmd".to_string(),
        "com".to_string(),
        "pif".to_string(),
        "scr".to_string(),
        "vbs".to_string(),
        "js".to_string(),
    ]
}

fn default_thumbnail_sizes() -> Vec<ThumbnailSize> {
    vec![
        ThumbnailSize {
            name: "small".to_string(),
            width: 150,
            height: 150,
            maintain_aspect_ratio: true,
        },
        ThumbnailSize {
            name: "medium".to_string(),
            width: 300,
            height: 300,
            maintain_aspect_ratio: true,
        },
        ThumbnailSize {
            name: "large".to_string(),
            width: 600,
            height: 600,
            maintain_aspect_ratio: true,
        },
    ]
}

fn default_image_formats() -> Vec<String> {
    vec![
        "jpeg".to_string(),
        "jpg".to_string(),
        "png".to_string(),
        "webp".to_string(),
        "gif".to_string(),
        "bmp".to_string(),
        "tiff".to_string(),
    ]
}

fn default_video_formats() -> Vec<String> {
    vec![
        "mp4".to_string(),
        "webm".to_string(),
        "avi".to_string(),
        "mov".to_string(),
        "wmv".to_string(),
        "flv".to_string(),
    ]
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
            processing: ProcessingConfig::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureFlags::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            max_request_size: default_max_request_size(),
            request_timeout: default_request_timeout(),
            max_concurrent_uploads: default_max_concurrent_uploads(),
            enable_metrics: default_true(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: default_storage_type(),
            s3: S3Config::default(),
            local: LocalStorageConfig::default(),
            default_bucket: default_bucket(),
            enable_multipart: default_true(),
            multipart_chunk_size: default_multipart_chunk_size(),
            max_file_size: default_max_file_size(),
        }
    }
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: None,
            region: default_region(),
            access_key: "".to_string(),
            secret_key: "".to_string(),
            path_style: default_false(),
            use_ssl: default_true(),
        }
    }
}

impl Default for LocalStorageConfig {
    fn default() -> Self {
        Self {
            base_path: default_storage_path(),
            organize_by_date: default_true(),
            max_depth: default_max_depth(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            mongodb_uri: "mongodb://localhost:27017".to_string(),
            mongodb_database: default_mongodb_database(),
            postgres_uri: None,
            pool_size: default_pool_size(),
            connection_timeout: default_connection_timeout(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production".to_string(),
            token_expiry: default_token_expiry(),
            virus_scanner: VirusScannerConfig::default(),
            enable_encryption: default_false(),
            encryption_key: None,
            allowed_file_types: default_allowed_types(),
            blocked_extensions: default_blocked_extensions(),
            max_files_per_user: default_max_files_per_user(),
            max_storage_per_user: default_max_storage_per_user(),
        }
    }
}

impl Default for VirusScannerConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            clamd_host: default_clamd_host(),
            clamd_port: default_clamd_port(),
            scan_timeout: default_scan_timeout(),
            max_scan_size: default_max_scan_size(),
            quarantine_infected: default_true(),
        }
    }
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            enable_image_processing: default_true(),
            enable_video_processing: default_false(),
            image_quality: default_image_quality(),
            thumbnail_sizes: default_thumbnail_sizes(),
            enable_optimization: default_true(),
            processing_timeout: default_processing_timeout(),
            supported_image_formats: default_image_formats(),
            supported_video_formats: default_video_formats(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_uri: None,
            enable_metadata_cache: default_true(),
            metadata_cache_ttl: default_cache_ttl(),
            enable_thumbnail_cache: default_true(),
            thumbnail_cache_ttl: default_thumbnail_cache_ttl(),
            max_cache_size: default_max_cache_size(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: default_false(),
            log_file: None,
            enable_access_log: default_true(),
            enable_audit_log: default_true(),
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_deduplication: default_true(),
            enable_versioning: default_false(),
            enable_sharing: default_true(),
            enable_batch_operations: default_true(),
            enable_compression: default_true(),
            enable_previews: default_true(),
            enable_background_processing: default_true(),
        }
    }
}
