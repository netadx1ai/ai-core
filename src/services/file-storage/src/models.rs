use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// File metadata stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Unique file identifier
    pub id: Uuid,
    /// MongoDB ObjectId for internal use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    /// Original filename
    pub original_name: String,
    /// Sanitized filename for storage
    pub storage_name: String,
    /// File MIME type
    pub mime_type: String,
    /// File extension
    pub extension: Option<String>,
    /// File size in bytes
    pub size: u64,
    /// SHA-256 hash for deduplication
    pub hash: String,
    /// Blake3 hash for additional verification
    pub blake3_hash: String,
    /// Storage backend type (s3, minio, local)
    pub storage_type: String,
    /// Storage bucket/container
    pub bucket: String,
    /// Storage key/path
    pub storage_key: String,
    /// File owner user ID
    pub owner_id: Uuid,
    /// Upload timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// File status
    pub status: FileStatus,
    /// Access permissions
    pub permissions: FilePermissions,
    /// File tags for organization
    pub tags: Vec<String>,
    /// Custom metadata
    pub custom_metadata: HashMap<String, serde_json::Value>,
    /// Virus scan result
    pub virus_scan: Option<VirusScanResult>,
    /// Processing results
    pub processing: Option<ProcessingResult>,
    /// Thumbnails and previews
    pub thumbnails: Vec<ThumbnailInfo>,
    /// File versions (if versioning enabled)
    pub versions: Vec<FileVersion>,
    /// Parent folder ID
    pub folder_id: Option<Uuid>,
    /// Expiration timestamp (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Download count
    pub download_count: u64,
    /// Last accessed timestamp
    pub last_accessed: Option<DateTime<Utc>>,
}

/// File status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    /// File is being uploaded
    Uploading,
    /// File upload completed, processing in progress
    Processing,
    /// File is available and ready
    Available,
    /// File is quarantined due to virus detection
    Quarantined,
    /// File processing failed
    Failed,
    /// File is archived
    Archived,
    /// File is marked for deletion
    Deleted,
}

/// File permissions structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    /// Public read access
    pub public_read: bool,
    /// Public write access (dangerous)
    pub public_write: bool,
    /// Specific user permissions
    pub user_permissions: HashMap<Uuid, Vec<Permission>>,
    /// Role-based permissions
    pub role_permissions: HashMap<String, Vec<Permission>>,
    /// Access control list
    pub acl: Vec<AccessControlEntry>,
}

/// Individual permission type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Write,
    Delete,
    Share,
    Admin,
}

/// Access control entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlEntry {
    /// Principal (user ID, role, or group)
    pub principal: String,
    /// Principal type
    pub principal_type: PrincipalType,
    /// Granted permissions
    pub permissions: Vec<Permission>,
    /// Grant timestamp
    pub granted_at: DateTime<Utc>,
    /// Granted by user ID
    pub granted_by: Uuid,
    /// Expiration (optional)
    pub expires_at: Option<DateTime<Utc>>,
}

/// Principal type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalType {
    User,
    Role,
    Group,
    Service,
}

/// Virus scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusScanResult {
    /// Scan status
    pub status: ScanStatus,
    /// Scanner used
    pub scanner: String,
    /// Scan timestamp
    pub scanned_at: DateTime<Utc>,
    /// Scan duration in milliseconds
    pub scan_duration_ms: u64,
    /// Threats detected
    pub threats: Vec<ThreatInfo>,
    /// Scanner version
    pub scanner_version: Option<String>,
    /// Signature database version
    pub signature_version: Option<String>,
}

/// Scan status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    /// Scan pending
    Pending,
    /// Scan in progress
    Scanning,
    /// File is clean
    Clean,
    /// Threats detected
    Infected,
    /// Scan failed
    Failed,
    /// File too large to scan
    Skipped,
}

/// Threat information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    /// Threat name/signature
    pub name: String,
    /// Threat type
    pub threat_type: String,
    /// Severity level
    pub severity: ThreatSeverity,
    /// Description
    pub description: Option<String>,
}

/// Threat severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Processing result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// Processing status
    pub status: ProcessingStatus,
    /// Processing start time
    pub started_at: DateTime<Utc>,
    /// Processing completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Processing duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Processing steps performed
    pub steps: Vec<ProcessingStep>,
    /// Any errors encountered
    pub errors: Vec<ProcessingError>,
    /// Generated artifacts
    pub artifacts: HashMap<String, serde_json::Value>,
}

/// Processing status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Processing step information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStep {
    /// Step name
    pub name: String,
    /// Step status
    pub status: ProcessingStatus,
    /// Step start time
    pub started_at: DateTime<Utc>,
    /// Step completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Step duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Step output
    pub output: Option<serde_json::Value>,
}

/// Processing error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Error timestamp
    pub timestamp: DateTime<Utc>,
    /// Error details
    pub details: Option<serde_json::Value>,
}

/// Thumbnail information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailInfo {
    /// Thumbnail size name
    pub name: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// File size in bytes
    pub size: u64,
    /// Storage key for thumbnail
    pub storage_key: String,
    /// MIME type
    pub mime_type: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// File version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    /// Version number
    pub version: u32,
    /// Storage key for this version
    pub storage_key: String,
    /// File size for this version
    pub size: u64,
    /// Version timestamp
    pub created_at: DateTime<Utc>,
    /// User who created this version
    pub created_by: Uuid,
    /// Version description/comment
    pub description: Option<String>,
    /// Hash for this version
    pub hash: String,
}

/// Folder/directory structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Unique folder identifier
    pub id: Uuid,
    /// MongoDB ObjectId for internal use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    /// Folder name
    pub name: String,
    /// Folder path
    pub path: String,
    /// Parent folder ID
    pub parent_id: Option<Uuid>,
    /// Folder owner
    pub owner_id: Uuid,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Folder permissions
    pub permissions: FilePermissions,
    /// Folder tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// File count in folder
    pub file_count: u64,
    /// Total size of files in folder
    pub total_size: u64,
}

/// File upload request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    /// Target filename
    pub filename: String,
    /// MIME type (optional, will be detected)
    pub mime_type: Option<String>,
    /// Target folder ID
    pub folder_id: Option<Uuid>,
    /// File tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Access permissions
    pub permissions: Option<FilePermissions>,
    /// Processing options
    pub processing_options: ProcessingOptions,
    /// Expiration time
    pub expires_at: Option<DateTime<Utc>>,
}

/// Processing options for uploaded files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    /// Generate thumbnails
    pub generate_thumbnails: bool,
    /// Thumbnail sizes to generate
    pub thumbnail_sizes: Vec<String>,
    /// Optimize images
    pub optimize_images: bool,
    /// Image quality (1-100)
    pub image_quality: Option<u8>,
    /// Enable virus scanning
    pub virus_scan: bool,
    /// Enable background processing
    pub background_processing: bool,
    /// Custom processing pipeline
    pub custom_pipeline: Vec<String>,
}

/// File download request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    /// File ID
    pub file_id: Uuid,
    /// Download as attachment
    pub as_attachment: bool,
    /// Custom filename for download
    pub download_name: Option<String>,
    /// Range header for partial downloads
    pub range: Option<String>,
}

/// File search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search text (filename, tags, metadata)
    pub query: Option<String>,
    /// MIME type filter
    pub mime_type: Option<String>,
    /// File extension filter
    pub extension: Option<String>,
    /// Owner filter
    pub owner_id: Option<Uuid>,
    /// Folder filter
    pub folder_id: Option<Uuid>,
    /// Tags filter
    pub tags: Vec<String>,
    /// Size range filter
    pub size_range: Option<SizeRange>,
    /// Date range filter
    pub date_range: Option<DateRange>,
    /// Status filter
    pub status: Option<FileStatus>,
    /// Sort options
    pub sort: Option<SortOptions>,
    /// Pagination
    pub pagination: PaginationOptions,
}

/// Size range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRange {
    /// Minimum size in bytes
    pub min: Option<u64>,
    /// Maximum size in bytes
    pub max: Option<u64>,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start date
    pub start: Option<DateTime<Utc>>,
    /// End date
    pub end: Option<DateTime<Utc>>,
}

/// Sort options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOptions {
    /// Field to sort by
    pub field: SortField,
    /// Sort direction
    pub direction: SortDirection,
}

/// Sort field enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Name,
    Size,
    CreatedAt,
    UpdatedAt,
    DownloadCount,
    LastAccessed,
}

/// Sort direction enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Pagination options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationOptions {
    /// Page number (0-based)
    pub page: u32,
    /// Items per page
    pub per_page: u32,
}

/// File list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    /// Files
    pub files: Vec<FileMetadata>,
    /// Total count
    pub total: u64,
    /// Current page
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total pages
    pub total_pages: u32,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total files
    pub total_files: u64,
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Files uploaded today
    pub uploads_today: u64,
    /// Files downloaded today
    pub downloads_today: u64,
    /// Storage usage percentage
    pub storage_usage_percent: f64,
    /// Stats by MIME type
    pub by_mime_type: HashMap<String, FileTypeStats>,
    /// Stats by user
    pub by_user: HashMap<Uuid, UserStats>,
}

/// File type statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeStats {
    /// File count
    pub count: u64,
    /// Total size in bytes
    pub size: u64,
    /// Average size in bytes
    pub average_size: f64,
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    /// File count
    pub file_count: u64,
    /// Total size in bytes
    pub total_size: u64,
    /// Uploads today
    pub uploads_today: u64,
    /// Downloads today
    pub downloads_today: u64,
    /// Storage quota usage percentage
    pub quota_usage_percent: f64,
}

/// Batch operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    /// File IDs to operate on
    pub file_ids: Vec<Uuid>,
    /// Operation to perform
    pub operation: BatchOperation,
    /// Operation parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Batch operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchOperation {
    Delete,
    Move,
    Copy,
    UpdateTags,
    UpdatePermissions,
    Archive,
    Restore,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Operation performed
    pub operation: BatchOperation,
    /// Total files processed
    pub total: u32,
    /// Successfully processed
    pub success: u32,
    /// Failed to process
    pub failed: u32,
    /// Individual file results
    pub results: Vec<FileOperationResult>,
}

/// Individual file operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationResult {
    /// File ID
    pub file_id: Uuid,
    /// Operation success
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

// Default implementations

impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            public_read: false,
            public_write: false,
            user_permissions: HashMap::new(),
            role_permissions: HashMap::new(),
            acl: Vec::new(),
        }
    }
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            generate_thumbnails: true,
            thumbnail_sizes: vec!["small".to_string(), "medium".to_string()],
            optimize_images: true,
            image_quality: Some(85),
            virus_scan: true,
            background_processing: true,
            custom_pipeline: Vec::new(),
        }
    }
}

impl Default for PaginationOptions {
    fn default() -> Self {
        Self {
            page: 0,
            per_page: 50,
        }
    }
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            field: SortField::CreatedAt,
            direction: SortDirection::Desc,
        }
    }
}

// Utility implementations

impl FileMetadata {
    /// Create new file metadata
    pub fn new(
        original_name: String,
        mime_type: String,
        size: u64,
        hash: String,
        owner_id: Uuid,
        storage_type: String,
        bucket: String,
        storage_key: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            _id: None,
            original_name: original_name.clone(),
            storage_name: original_name,
            mime_type,
            extension: None,
            size,
            hash: hash.clone(),
            blake3_hash: hash, // Will be updated with actual Blake3 hash
            storage_type,
            bucket,
            storage_key,
            owner_id,
            created_at: now,
            updated_at: now,
            status: FileStatus::Uploading,
            permissions: FilePermissions::default(),
            tags: Vec::new(),
            custom_metadata: HashMap::new(),
            virus_scan: None,
            processing: None,
            thumbnails: Vec::new(),
            versions: Vec::new(),
            folder_id: None,
            expires_at: None,
            download_count: 0,
            last_accessed: None,
        }
    }

    /// Check if file is public readable
    pub fn is_public_readable(&self) -> bool {
        self.permissions.public_read
    }

    /// Check if user has permission
    pub fn has_permission(&self, user_id: &Uuid, permission: &Permission) -> bool {
        // Owner has all permissions
        if &self.owner_id == user_id {
            return true;
        }

        // Check user-specific permissions
        if let Some(user_perms) = self.permissions.user_permissions.get(user_id) {
            if user_perms.contains(permission) {
                return true;
            }
        }

        // Check ACL
        for ace in &self.permissions.acl {
            if ace.principal == user_id.to_string() && ace.permissions.contains(permission) {
                // Check if not expired
                if let Some(expires_at) = ace.expires_at {
                    if expires_at < Utc::now() {
                        continue;
                    }
                }
                return true;
            }
        }

        false
    }

    /// Check if file is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Update download count and last accessed time
    pub fn record_access(&mut self) {
        self.download_count += 1;
        self.last_accessed = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

impl Folder {
    /// Create new folder
    pub fn new(name: String, parent_id: Option<Uuid>, owner_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            _id: None,
            name,
            path: String::new(), // Will be computed based on hierarchy
            parent_id,
            owner_id,
            created_at: now,
            updated_at: now,
            permissions: FilePermissions::default(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            file_count: 0,
            total_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metadata_creation() {
        let owner_id = Uuid::new_v4();
        let metadata = FileMetadata::new(
            "test.jpg".to_string(),
            "image/jpeg".to_string(),
            1024,
            "hash123".to_string(),
            owner_id,
            "s3".to_string(),
            "bucket".to_string(),
            "key".to_string(),
        );

        assert_eq!(metadata.original_name, "test.jpg");
        assert_eq!(metadata.mime_type, "image/jpeg");
        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.owner_id, owner_id);
        assert_eq!(metadata.status, FileStatus::Uploading);
    }

    #[test]
    fn test_permissions() {
        let owner_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let metadata = FileMetadata::new(
            "test.jpg".to_string(),
            "image/jpeg".to_string(),
            1024,
            "hash123".to_string(),
            owner_id,
            "s3".to_string(),
            "bucket".to_string(),
            "key".to_string(),
        );

        // Owner should have all permissions
        assert!(metadata.has_permission(&owner_id, &Permission::Read));
        assert!(metadata.has_permission(&owner_id, &Permission::Write));
        assert!(metadata.has_permission(&owner_id, &Permission::Delete));

        // Other users should not have permissions by default
        assert!(!metadata.has_permission(&other_user, &Permission::Read));
    }

    #[test]
    fn test_file_expiration() {
        let owner_id = Uuid::new_v4();
        let mut metadata = FileMetadata::new(
            "test.jpg".to_string(),
            "image/jpeg".to_string(),
            1024,
            "hash123".to_string(),
            owner_id,
            "s3".to_string(),
            "bucket".to_string(),
            "key".to_string(),
        );

        // File should not be expired by default
        assert!(!metadata.is_expired());

        // Set expiration in the past
        metadata.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(metadata.is_expired());

        // Set expiration in the future
        metadata.expires_at = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_access_recording() {
        let owner_id = Uuid::new_v4();
        let mut metadata = FileMetadata::new(
            "test.jpg".to_string(),
            "image/jpeg".to_string(),
            1024,
            "hash123".to_string(),
            owner_id,
            "s3".to_string(),
            "bucket".to_string(),
            "key".to_string(),
        );

        assert_eq!(metadata.download_count, 0);
        assert!(metadata.last_accessed.is_none());

        metadata.record_access();

        assert_eq!(metadata.download_count, 1);
        assert!(metadata.last_accessed.is_some());
    }
}
