//! Comprehensive test suite for file storage service
//!
//! This module contains unit tests, integration tests, and benchmarks
//! for all components of the file storage service.

use chrono::Utc;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;
use uuid::Uuid;

use crate::{
    config_types::{
        FileStorageConfig, ProcessingConfig, S3Config, SecurityConfig, StorageConfig,
        VirusScannerConfig,
    },
    error::{FileStorageError, FileStorageResult},
    models::{
        BatchOperation, BatchRequest, FileMetadata, FilePermissions, FileStatus, PaginationOptions,
        Permission, ProcessingStatus, ScanStatus, SearchQuery, StorageStats, ThumbnailInfo,
    },
    services::{
        AccessControlService, MediaProcessor, MetadataService, StorageService, VirusScanner,
    },
    utils::{content, file_type, hash, path, security, size, validation},
    AppState,
};

/// Test configuration builder
pub struct TestConfigBuilder {
    config: FileStorageConfig,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: FileStorageConfig::default(),
        }
    }

    pub fn with_storage_type(mut self, storage_type: &str) -> Self {
        self.config.storage.storage_type = storage_type.to_string();
        self
    }

    pub fn with_mongodb_uri(mut self, uri: &str) -> Self {
        self.config.database.mongodb_uri = uri.to_string();
        self
    }

    pub fn build(self) -> FileStorageConfig {
        self.config
    }
}

/// Test data builder for creating test files
pub struct TestFileBuilder {
    metadata: FileMetadata,
}

impl TestFileBuilder {
    pub fn new(owner_id: Uuid) -> Self {
        let file_id = Uuid::new_v4();
        let metadata = FileMetadata::new(
            "test_file.txt".to_string(),
            "text/plain".to_string(),
            1024,
            "test_hash".to_string(),
            owner_id,
            "s3".to_string(),
            "test-bucket".to_string(),
            format!("files/{}", file_id),
        );

        Self { metadata }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.metadata.original_name = name.to_string();
        self.metadata.storage_name = name.to_string();
        self
    }

    pub fn with_mime_type(mut self, mime_type: &str) -> Self {
        self.metadata.mime_type = mime_type.to_string();
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.metadata.size = size;
        self
    }

    pub fn with_status(mut self, status: FileStatus) -> Self {
        self.metadata.status = status;
        self
    }

    pub fn with_public_read(mut self, public: bool) -> Self {
        self.metadata.permissions.public_read = public;
        self
    }

    pub fn build(self) -> FileMetadata {
        self.metadata
    }
}

/// Mock storage service for testing
pub struct MockStorageService {
    pub files: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    pub should_fail: std::sync::Mutex<bool>,
}

impl MockStorageService {
    pub fn new() -> Self {
        Self {
            files: std::sync::Mutex::new(HashMap::new()),
            should_fail: std::sync::Mutex::new(false),
        }
    }

    pub fn set_failure_mode(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }

    pub async fn upload_file(
        &self,
        key: &str,
        data: bytes::Bytes,
        _content_type: &str,
        _metadata: Option<HashMap<String, String>>,
    ) -> FileStorageResult<String> {
        if *self.should_fail.lock().unwrap() {
            return Err(FileStorageError::StorageError {
                message: "Mock upload failure".to_string(),
            });
        }

        self.files
            .lock()
            .unwrap()
            .insert(key.to_string(), data.to_vec());
        Ok(format!("mock://bucket/{}", key))
    }

    pub async fn download_file(&self, key: &str) -> FileStorageResult<bytes::Bytes> {
        if *self.should_fail.lock().unwrap() {
            return Err(FileStorageError::StorageError {
                message: "Mock download failure".to_string(),
            });
        }

        let files = self.files.lock().unwrap();
        match files.get(key) {
            Some(data) => Ok(bytes::Bytes::from(data.clone())),
            None => Err(FileStorageError::FileNotFound {
                file_id: key.to_string(),
            }),
        }
    }

    pub async fn delete_file(&self, key: &str) -> FileStorageResult<()> {
        if *self.should_fail.lock().unwrap() {
            return Err(FileStorageError::StorageError {
                message: "Mock delete failure".to_string(),
            });
        }

        self.files.lock().unwrap().remove(key);
        Ok(())
    }

    pub async fn file_exists(&self, key: &str) -> FileStorageResult<bool> {
        Ok(self.files.lock().unwrap().contains_key(key))
    }

    pub async fn health_check(&self) -> FileStorageResult<()> {
        if *self.should_fail.lock().unwrap() {
            return Err(FileStorageError::ServiceUnavailable {
                service: "Mock storage".to_string(),
            });
        }
        Ok(())
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = TestConfigBuilder::new()
            .with_storage_type("local")
            .with_mongodb_uri("mongodb://localhost:27017/test")
            .build();

        assert_eq!(config.storage.storage_type, "local");
        assert_eq!(
            config.database.mongodb_uri,
            "mongodb://localhost:27017/test"
        );
    }

    #[test]
    fn test_file_builder() {
        let owner_id = Uuid::new_v4();
        let file = TestFileBuilder::new(owner_id)
            .with_name("custom.jpg")
            .with_mime_type("image/jpeg")
            .with_size(2048)
            .with_status(FileStatus::Available)
            .with_public_read(true)
            .build();

        assert_eq!(file.original_name, "custom.jpg");
        assert_eq!(file.mime_type, "image/jpeg");
        assert_eq!(file.size, 2048);
        assert_eq!(file.status, FileStatus::Available);
        assert_eq!(file.owner_id, owner_id);
        assert!(file.permissions.public_read);
    }

    #[test]
    fn test_file_metadata_creation() {
        let owner_id = Uuid::new_v4();
        let metadata = FileMetadata::new(
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            5120,
            "abcdef123456".to_string(),
            owner_id,
            "s3".to_string(),
            "documents".to_string(),
            "files/test.pdf".to_string(),
        );

        assert_eq!(metadata.original_name, "test.pdf");
        assert_eq!(metadata.mime_type, "application/pdf");
        assert_eq!(metadata.size, 5120);
        assert_eq!(metadata.hash, "abcdef123456");
        assert_eq!(metadata.owner_id, owner_id);
        assert_eq!(metadata.storage_type, "s3");
        assert_eq!(metadata.bucket, "documents");
        assert_eq!(metadata.storage_key, "files/test.pdf");
        assert_eq!(metadata.status, FileStatus::Uploading);
        assert_eq!(metadata.download_count, 0);
        assert!(metadata.last_accessed.is_none());
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_file_permissions() {
        let owner_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mut metadata = TestFileBuilder::new(owner_id).build();

        // Owner should have all permissions
        assert!(metadata.has_permission(&owner_id, &Permission::Read));
        assert!(metadata.has_permission(&owner_id, &Permission::Write));
        assert!(metadata.has_permission(&owner_id, &Permission::Delete));

        // Other users should not have permissions by default
        assert!(!metadata.has_permission(&user_id, &Permission::Read));

        // Grant specific permission
        metadata
            .permissions
            .user_permissions
            .insert(user_id, vec![Permission::Read, Permission::Write]);

        assert!(metadata.has_permission(&user_id, &Permission::Read));
        assert!(metadata.has_permission(&user_id, &Permission::Write));
        assert!(!metadata.has_permission(&user_id, &Permission::Delete));
    }

    #[test]
    fn test_file_expiration() {
        let owner_id = Uuid::new_v4();
        let mut file = TestFileBuilder::new(owner_id).build();

        // File should not be expired by default
        assert!(!file.is_expired());

        // Set expiration in the past
        file.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(file.is_expired());

        // Set expiration in the future
        file.expires_at = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!file.is_expired());
    }

    #[test]
    fn test_access_recording() {
        let owner_id = Uuid::new_v4();
        let mut file = TestFileBuilder::new(owner_id).build();

        assert_eq!(file.download_count, 0);
        assert!(file.last_accessed.is_none());

        file.record_access();

        assert_eq!(file.download_count, 1);
        assert!(file.last_accessed.is_some());

        let first_access_time = file.last_accessed.unwrap();

        // Wait a bit and record another access
        std::thread::sleep(std::time::Duration::from_millis(10));
        file.record_access();

        assert_eq!(file.download_count, 2);
        assert!(file.last_accessed.unwrap() > first_access_time);
    }

    #[tokio::test]
    async fn test_mock_storage_service() {
        let storage = MockStorageService::new();
        let test_data = bytes::Bytes::from("test file content");

        // Test upload
        let result = storage
            .upload_file("test.txt", test_data.clone(), "text/plain", None)
            .await;
        assert!(result.is_ok());

        // Test download
        let downloaded = storage.download_file("test.txt").await;
        assert!(downloaded.is_ok());
        assert_eq!(downloaded.unwrap(), test_data);

        // Test file exists
        let exists = storage.file_exists("test.txt").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());

        // Test delete
        let delete_result = storage.delete_file("test.txt").await;
        assert!(delete_result.is_ok());

        // File should no longer exist
        let exists_after_delete = storage.file_exists("test.txt").await;
        assert!(exists_after_delete.is_ok());
        assert!(!exists_after_delete.unwrap());

        // Test health check
        let health = storage.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_failure_mode() {
        let storage = MockStorageService::new();
        let test_data = bytes::Bytes::from("test data");

        // Enable failure mode
        storage.set_failure_mode(true);

        // All operations should fail
        let upload_result = storage
            .upload_file("test.txt", test_data, "text/plain", None)
            .await;
        assert!(upload_result.is_err());

        let download_result = storage.download_file("test.txt").await;
        assert!(download_result.is_err());

        let delete_result = storage.delete_file("test.txt").await;
        assert!(delete_result.is_err());

        let health_result = storage.health_check().await;
        assert!(health_result.is_err());

        // Disable failure mode
        storage.set_failure_mode(false);

        // Health check should now pass
        let health_result = storage.health_check().await;
        assert!(health_result.is_ok());
    }
}

// ============================================================================
// Utility Tests
// ============================================================================

#[cfg(test)]
mod utility_tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        // Test MIME type detection from magic bytes
        let jpeg_header = b"\xFF\xD8\xFF\xE0";
        assert_eq!(
            file_type::detect_from_magic_bytes(jpeg_header),
            Some("image/jpeg".to_string())
        );

        let png_header = b"\x89PNG\r\n\x1A\n";
        assert_eq!(
            file_type::detect_from_magic_bytes(png_header),
            Some("image/png".to_string())
        );

        let pdf_header = b"%PDF-1.4";
        assert_eq!(
            file_type::detect_from_magic_bytes(pdf_header),
            Some("application/pdf".to_string())
        );

        // Test allowed file types
        let allowed_types = vec![
            "image/jpeg".to_string(),
            "image/*".to_string(),
            "text/plain".to_string(),
        ];

        assert!(file_type::is_allowed_file_type(
            "image/jpeg",
            &allowed_types
        ));
        assert!(file_type::is_allowed_file_type("image/png", &allowed_types));
        assert!(file_type::is_allowed_file_type(
            "text/plain",
            &allowed_types
        ));
        assert!(!file_type::is_allowed_file_type(
            "application/exe",
            &allowed_types
        ));

        // Empty allowed list should allow everything
        assert!(file_type::is_allowed_file_type("anything", &[]));
    }

    #[test]
    fn test_path_utilities() {
        // Test filename sanitization
        assert_eq!(
            path::sanitize_filename("hello world.txt"),
            "hello_world.txt"
        );
        assert_eq!(
            path::sanitize_filename("file<>:\"|?*.txt"),
            "file________.txt"
        );
        assert_eq!(
            path::sanitize_filename("normal-file_name.jpg"),
            "normal-file_name.jpg"
        );

        // Test file extension extraction
        assert_eq!(
            path::get_file_extension("test.jpg"),
            Some("jpg".to_string())
        );
        assert_eq!(
            path::get_file_extension("test.tar.gz"),
            Some("gz".to_string())
        );
        assert_eq!(path::get_file_extension("no_extension"), None);
        assert_eq!(path::get_file_extension(""), None);

        // Test blocked extensions
        let blocked = vec!["exe".to_string(), "bat".to_string(), "scr".to_string()];
        assert!(path::has_blocked_extension("malware.exe", &blocked));
        assert!(path::has_blocked_extension("script.BAT", &blocked));
        assert!(!path::has_blocked_extension("document.pdf", &blocked));

        // Test storage path generation
        let file_id = Uuid::new_v4();
        let path_with_date = path::generate_storage_path(&file_id, "test.jpg", true);
        let path_without_date = path::generate_storage_path(&file_id, "test.jpg", false);

        assert!(path_with_date.contains(&file_id.to_string()));
        assert!(path_with_date.contains("/"));
        assert!(path_with_date.ends_with(".jpg"));

        assert!(path_without_date.contains(&file_id.to_string()));
        assert!(!path_without_date.contains("/"));
        assert!(path_without_date.ends_with(".jpg"));
    }

    #[test]
    fn test_validation() {
        // Test filename validation
        assert!(validation::validate_filename("valid_file.txt").is_ok());
        assert!(validation::validate_filename("").is_err());
        assert!(validation::validate_filename("file../test.txt").is_err());
        assert!(validation::validate_filename("CON.txt").is_err());
        assert!(validation::validate_filename("file:with|illegal<chars>.txt").is_err());

        // Test UUID validation
        let valid_uuid = Uuid::new_v4().to_string();
        assert!(validation::validate_uuid(&valid_uuid).is_ok());
        assert!(validation::validate_uuid("invalid-uuid").is_err());
        assert!(validation::validate_uuid("").is_err());

        // Test file size validation
        assert!(validation::validate_file_size(1024, 2048).is_ok());
        assert!(validation::validate_file_size(2048, 1024).is_err());
        assert!(validation::validate_file_size(0, 1024).is_ok());
    }

    #[test]
    fn test_size_formatting() {
        assert_eq!(size::format_bytes(0), "0 B");
        assert_eq!(size::format_bytes(512), "512 B");
        assert_eq!(size::format_bytes(1024), "1.02 KB");
        assert_eq!(size::format_bytes(1_000_000), "1.00 MB");
        assert_eq!(size::format_bytes(1_500_000_000), "1.50 GB");
        assert_eq!(size::format_bytes(2_000_000_000_000), "2.00 TB");

        // Test size parsing
        assert_eq!(size::parse_size("10MB"), Some(10_000_000));
        assert_eq!(size::parse_size("1.5GB"), Some(1_500_000_000));
        assert_eq!(size::parse_size("100"), Some(100));
        assert_eq!(size::parse_size("2KB"), Some(2_000));
        assert_eq!(size::parse_size("invalid"), None);
        assert_eq!(size::parse_size(""), None);
        assert_eq!(size::parse_size("10XB"), None);
    }

    #[test]
    fn test_hash_generation() {
        let test_data = b"test data for hashing";
        let (sha256, blake3) = hash::generate_file_hashes(test_data);

        assert!(!sha256.is_empty());
        assert!(!blake3.is_empty());
        assert_ne!(sha256, blake3);

        // SHA-256 should be 64 characters (32 bytes * 2 hex chars)
        assert_eq!(sha256.len(), 64);
        // Blake3 should be 64 characters as well
        assert_eq!(blake3.len(), 64);

        // Same input should produce same hashes
        let (sha256_2, blake3_2) = hash::generate_file_hashes(test_data);
        assert_eq!(sha256, sha256_2);
        assert_eq!(blake3, blake3_2);

        // Different input should produce different hashes
        let different_data = b"different test data";
        let (sha256_diff, blake3_diff) = hash::generate_file_hashes(different_data);
        assert_ne!(sha256, sha256_diff);
        assert_ne!(blake3, blake3_diff);
    }

    #[test]
    fn test_security_utilities() {
        // Test token generation
        let token1 = security::generate_token(32);
        let token2 = security::generate_token(32);

        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2);

        // Test different lengths
        let short_token = security::generate_token(8);
        let long_token = security::generate_token(64);
        assert_eq!(short_token.len(), 8);
        assert_eq!(long_token.len(), 64);

        // Test IP address checking
        let allowed_ranges = vec![
            "192.168.1.*".to_string(),
            "10.0.0.1".to_string(),
            "*".to_string(),
        ];

        assert!(security::is_allowed_ip("192.168.1.1", &allowed_ranges));
        assert!(security::is_allowed_ip("192.168.1.100", &allowed_ranges));
        assert!(security::is_allowed_ip("10.0.0.1", &allowed_ranges));
        assert!(security::is_allowed_ip("1.2.3.4", &allowed_ranges)); // * allows all

        // Test with specific allowed ranges
        let specific_ranges = vec!["192.168.1.1".to_string(), "10.0.0.*".to_string()];
        assert!(security::is_allowed_ip("192.168.1.1", &specific_ranges));
        assert!(security::is_allowed_ip("10.0.0.50", &specific_ranges));
        assert!(!security::is_allowed_ip("192.168.2.1", &specific_ranges));

        // Empty ranges should allow all
        assert!(security::is_allowed_ip("any.ip.address", &[]));
    }

    #[test]
    fn test_content_analysis() {
        // Test suspicious content detection
        let exe_content = b"MZ\x90\x00\x03\x00";
        assert!(content::is_suspicious_content(exe_content, "file.exe"));

        let elf_content = b"\x7fELF";
        assert!(content::is_suspicious_content(elf_content, "program"));

        let script_content = b"eval('malicious code')";
        assert!(content::is_suspicious_content(script_content, "script.js"));

        let vbs_content = b"CreateObject(\"WScript.Shell\")";
        assert!(content::is_suspicious_content(vbs_content, "script.vbs"));

        // Clean content should not be flagged
        let clean_content = b"This is just normal text content";
        assert!(!content::is_suspicious_content(
            clean_content,
            "document.txt"
        ));

        let clean_image = b"\xFF\xD8\xFF\xE0"; // JPEG header
        assert!(!content::is_suspicious_content(clean_image, "image.jpg"));

        // Test metadata extraction
        let metadata = content::extract_basic_metadata(clean_content, "text/plain");
        assert_eq!(metadata.get("size"), Some(&"33".to_string()));
        assert_eq!(metadata.get("mime_type"), Some(&"text/plain".to_string()));
    }
}

// ============================================================================
// Service Tests
// ============================================================================

#[cfg(test)]
mod service_tests {
    use super::*;

    #[tokio::test]
    async fn test_virus_scanner() {
        let config = VirusScannerConfig::default();
        let scanner = VirusScanner::new(&config).await.unwrap();

        // Test with clean data
        let clean_data = b"This is clean text data";
        let result = scanner.scan_file("test.txt", clean_data).await.unwrap();
        assert_eq!(result.status, ScanStatus::Clean);
        assert!(result.threats.is_empty());
        assert_eq!(result.scanner, "ai-core-scanner");

        // Test with executable data (should be detected as threat)
        let exe_data = b"MZ\x90\x00\x03\x00"; // PE header
        let result = scanner.scan_file("test.exe", exe_data).await.unwrap();
        assert_eq!(result.status, ScanStatus::Infected);
        assert!(!result.threats.is_empty());
        assert_eq!(result.threats[0].name, "Executable.Generic");

        // Test health check
        let health = scanner.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_media_processor() {
        let config = ProcessingConfig::default();
        let processor = MediaProcessor::new(&config);

        // Test hash generation
        let test_data = b"test image data";
        let (sha256, blake3) = processor.generate_hash(test_data).await;
        assert!(!sha256.is_empty());
        assert!(!blake3.is_empty());
        assert_ne!(sha256, blake3);

        // Test with invalid image data
        let invalid_data = b"not an image";
        let result = processor.process_image(invalid_data, "test.jpg").await;
        assert!(result.is_ok());
        let processing_result = result.unwrap();
        assert_eq!(processing_result.status, ProcessingStatus::Failed);
        assert!(!processing_result.errors.is_empty());
    }

    #[test]
    fn test_access_control_service() {
        let service = AccessControlService::new("test-secret").unwrap();

        let owner_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let file_metadata = TestFileBuilder::new(owner_id)
            .with_public_read(false)
            .build();

        // Test owner access
        tokio_test::block_on(async {
            assert!(
                service
                    .can_access_file(&owner_id, &file_metadata, "read")
                    .await
            );
            assert!(
                service
                    .can_access_file(&owner_id, &file_metadata, "write")
                    .await
            );
            assert!(
                service
                    .can_access_file(&owner_id, &file_metadata, "delete")
                    .await
            );
        });

        // Test non-owner access (should be denied)
        tokio_test::block_on(async {
            assert!(
                !service
                    .can_access_file(&user_id, &file_metadata, "read")
                    .await
            );
            assert!(
                !service
                    .can_access_file(&user_id, &file_metadata, "write")
                    .await
            );
            assert!(
                !service
                    .can_access_file(&user_id, &file_metadata, "delete")
                    .await
            );
        });

        // Test public file access
        let public_file = TestFileBuilder::new(owner_id)
            .with_public_read(true)
            .build();

        tokio_test::block_on(async {
            assert!(
                service
                    .can_access_file(&user_id, &public_file, "read")
                    .await
            );
            assert!(
                !service
                    .can_access_file(&user_id, &public_file, "write")
                    .await
            );
        });
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;

    /// Create test app state for integration tests
    async fn create_test_app_state() -> AppState {
        let config = Arc::new(TestConfigBuilder::new().build());

        // Use mock services for testing
        let storage_service = Arc::new(MockStorageService::new());

        // For metadata service, we'd need to mock MongoDB
        // For now, we'll create a minimal test setup

        // Create a stub metadata service (in real tests, you'd use testcontainers)
        let metadata_service = Arc::new({
            // This would be replaced with actual MongoDB test setup
            struct StubMetadataService;
            impl StubMetadataService {
                async fn save_file_metadata(
                    &self,
                    _metadata: &FileMetadata,
                ) -> FileStorageResult<()> {
                    Ok(())
                }

                async fn get_file_metadata(
                    &self,
                    _file_id: &Uuid,
                ) -> FileStorageResult<Option<FileMetadata>> {
                    Ok(None)
                }

                async fn health_check(&self) -> FileStorageResult<()> {
                    Ok(())
                }

                async fn get_storage_stats(&self) -> FileStorageResult<StorageStats> {
                    Ok(StorageStats {
                        total_files: 0,
                        total_size_bytes: 0,
                        uploads_today: 0,
                        downloads_today: 0,
                        storage_usage_percent: 0.0,
                        by_mime_type: HashMap::new(),
                        by_user: HashMap::new(),
                    })
                }
            }
            StubMetadataService
        });

        let virus_scanner = Arc::new(
            VirusScanner::new(&config.security.virus_scanner)
                .await
                .unwrap(),
        );
        let media_processor = Arc::new(MediaProcessor::new(&config.processing));
        let access_control =
            Arc::new(AccessControlService::new(&config.security.jwt_secret).unwrap());

        // Note: This is a simplified setup for testing
        // In real integration tests, you'd properly implement or mock all services
        todo!("Complete integration test setup with proper service mocking")
    }

    #[tokio::test]
    #[ignore] // Ignored because it requires full service setup
    async fn test_file_upload_workflow() {
        // This would test the complete file upload workflow
        // from HTTP request to storage and metadata persistence
        let _app_state = create_test_app_state().await;

        // Test would:
        // 1. Create upload request
        // 2. Process through virus scanning
        // 3. Store in mock storage
        // 4. Save metadata
        // 5. Verify all steps completed correctly
    }

    #[tokio::test]
    #[ignore] // Ignored because it requires full service setup
    async fn test_file_download_workflow() {
        // This would test the complete file download workflow
        // including permission checking and access logging
    }

    #[tokio::test]
    #[ignore] // Ignored because it requires full service setup
    async fn test_batch_operations() {
        // This would test batch file operations
        // like bulk delete, move, permission updates
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        use crate::error::FileStorageError;
        use axum::http::StatusCode;

        assert_eq!(
            FileStorageError::file_not_found("test").status_code(),
            StatusCode::NOT_FOUND
        );

        assert_eq!(
            FileStorageError::AuthenticationRequired.status_code(),
            StatusCode::UNAUTHORIZED
        );

        assert_eq!(
            FileStorageError::permission_denied("read", "file123").status_code(),
            StatusCode::FORBIDDEN
        );

        assert_eq!(
            FileStorageError::quota_exceeded(1000, 500).status_code(),
            StatusCode::PAYLOAD_TOO_LARGE
        );

        assert_eq!(
            FileStorageError::internal_error("test").status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_codes() {
        use crate::error::FileStorageError;

        assert_eq!(
            FileStorageError::file_not_found("test").error_code(),
            "FILE_NOT_FOUND"
        );

        assert_eq!(
            FileStorageError::virus_detected("malware").error_code(),
            "VIRUS_DETECTED"
        );

        assert_eq!(
            FileStorageError::invalid_file_type("text/plain", vec!["image/jpeg".to_string()])
                .error_code(),
            "INVALID_FILE_TYPE"
        );
    }

    #[test]
    fn test_error_response_format() {
        use crate::error::FileStorageError;

        let error = FileStorageError::quota_exceeded(1000, 500);
        let response = error.to_error_response();

        assert_eq!(response.error, "QUOTA_EXCEEDED");
        assert!(response.message.contains("quota exceeded"));
        assert!(response.details.is_some());

        let details = response.details.unwrap();
        assert_eq!(details["current_usage"], 1000);
        assert_eq!(details["quota_limit"], 500);
    }
}

// ============================================================================
// Performance Tests
// ============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_hash_performance() {
        let data = vec![0u8; 1024 * 1024]; // 1MB of data
        let start = Instant::now();

        for _ in 0..100 {
            let _ = hash::generate_file_hashes(&data);
        }

        let duration = start.elapsed();
        println!("100 hash operations on 1MB took: {:?}", duration);

        // Should complete within reasonable time (adjust threshold as needed)
        assert!(duration.as_secs() < 10);
    }

    #[tokio::test]
    async fn test_virus_scan_performance() {
        let config = VirusScannerConfig::default();
        let scanner = VirusScanner::new(&config).await.unwrap();
        let data = vec![0u8; 10 * 1024]; // 10KB test file

        let start = Instant::now();
        for i in 0..100 {
            let filename = format!("test_file_{}.txt", i);
            let _ = scanner.scan_file(&filename, &data).await;
        }
        let duration = start.elapsed();

        println!("100 virus scans took: {:?}", duration);
        assert!(duration.as_secs() < 5); // Should complete quickly for stub implementation
    }

    #[test]
    fn test_filename_sanitization_performance() {
        let test_filenames = vec![
            "normal_file.txt",
            "file with spaces.pdf",
            "file<>:\"|?*with_many_illegal_chars.doc",
            "very_long_filename_that_might_need_truncation_because_it_exceeds_normal_limits.xlsx",
        ];

        let start = Instant::now();
        for _ in 0..10000 {
            for filename in &test_filenames {
                let _ = path::sanitize_filename(filename);
            }
        }
        let duration = start.elapsed();

        println!("40,000 filename sanitizations took: {:?}", duration);
        assert!(duration.as_millis() < 1000); // Should be very fast
    }
}
