//! Core services for file storage functionality
//!
//! This module contains the main business logic services including:
//! - StorageService: S3/MinIO integration
//! - MetadataService: MongoDB metadata management
//! - VirusScanner: Security scanning integration
//! - MediaProcessor: Image/video processing
//! - AccessControlService: Permission management

use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client as S3Client, Error as S3Error};
use blake3::Hasher as Blake3Hasher;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    options::{ClientOptions, FindOptions, UpdateOptions},
    Client as MongoClient, Collection, Database,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io::Cursor, path::Path, sync::Arc, time::Duration};
use tokio::{fs, io::AsyncReadExt, sync::RwLock, time::timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    config_types::{ProcessingConfig, S3Config, SecurityConfig, VirusScannerConfig},
    error::{FileStorageError, FileStorageResult},
    models::{
        FileMetadata, FileStatus, ProcessingResult, ProcessingStatus, ProcessingStep, ScanStatus,
        StorageStats, ThumbnailInfo, UserStats, VirusScanResult,
    },
};

/// Storage service for S3/MinIO operations
#[derive(Clone)]
pub struct StorageService {
    s3_client: S3Client,
    default_bucket: String,
    config: S3Config,
}

impl StorageService {
    /// Create new storage service
    pub async fn new(config: &crate::config_types::StorageConfig) -> FileStorageResult<Self> {
        let s3_config = &config.s3;

        // Configure AWS SDK
        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(s3_config.region.clone()));

        // Set endpoint for MinIO or custom S3-compatible services
        if let Some(endpoint) = &s3_config.endpoint {
            aws_config_builder = aws_config_builder.endpoint_url(endpoint);
        }

        let aws_config = aws_config_builder.load().await;
        let s3_client = S3Client::new(&aws_config);

        // Test connection
        match s3_client.list_buckets().send().await {
            Ok(_) => info!("Successfully connected to S3/MinIO"),
            Err(e) => {
                warn!("Failed to connect to S3/MinIO: {}", e);
                // Continue anyway for development
            }
        }

        Ok(Self {
            s3_client,
            default_bucket: config.default_bucket.clone(),
            config: s3_config.clone(),
        })
    }

    /// Upload file to storage
    pub async fn upload_file(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> FileStorageResult<String> {
        let mut request = self
            .s3_client
            .put_object()
            .bucket(&self.default_bucket)
            .key(key)
            .body(data.into())
            .content_type(content_type);

        // Add metadata if provided
        if let Some(meta) = metadata {
            for (k, v) in meta {
                request = request.metadata(&k, &v);
            }
        }

        match request.send().await {
            Ok(_) => {
                debug!("Successfully uploaded file: {}", key);
                Ok(format!("s3://{}/{}", self.default_bucket, key))
            }
            Err(e) => {
                error!("Failed to upload file {}: {}", key, e);
                Err(FileStorageError::StorageError {
                    message: format!("Upload failed: {}", e),
                })
            }
        }
    }

    /// Download file from storage
    pub async fn download_file(&self, key: &str) -> FileStorageResult<Bytes> {
        match self
            .s3_client
            .get_object()
            .bucket(&self.default_bucket)
            .key(key)
            .send()
            .await
        {
            Ok(response) => {
                let data =
                    response
                        .body
                        .collect()
                        .await
                        .map_err(|e| FileStorageError::StorageError {
                            message: format!("Failed to read response body: {}", e),
                        })?;
                debug!("Successfully downloaded file: {}", key);
                Ok(data.into_bytes())
            }
            Err(e) => {
                error!("Failed to download file {}: {}", key, e);
                Err(FileStorageError::StorageError {
                    message: format!("Download failed: {}", e),
                })
            }
        }
    }

    /// Delete file from storage
    pub async fn delete_file(&self, key: &str) -> FileStorageResult<()> {
        match self
            .s3_client
            .delete_object()
            .bucket(&self.default_bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => {
                debug!("Successfully deleted file: {}", key);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete file {}: {}", key, e);
                Err(FileStorageError::StorageError {
                    message: format!("Delete failed: {}", e),
                })
            }
        }
    }

    /// Check if file exists
    pub async fn file_exists(&self, key: &str) -> FileStorageResult<bool> {
        match self
            .s3_client
            .head_object()
            .bucket(&self.default_bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") || e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(FileStorageError::StorageError {
                        message: format!("Failed to check file existence: {}", e),
                    })
                }
            }
        }
    }

    /// Health check
    pub async fn health_check(&self) -> FileStorageResult<()> {
        match self.s3_client.list_buckets().send().await {
            Ok(_) => Ok(()),
            Err(e) => Err(FileStorageError::ServiceUnavailable {
                service: format!("S3/MinIO: {}", e),
            }),
        }
    }
}

/// Metadata service for MongoDB operations
#[derive(Clone)]
pub struct MetadataService {
    database: Database,
    files_collection: Collection<Document>,
    folders_collection: Collection<Document>,
}

impl MetadataService {
    /// Create new metadata service
    pub async fn new(mongodb_uri: &str) -> FileStorageResult<Self> {
        let client_options = ClientOptions::parse(mongodb_uri).await.map_err(|e| {
            FileStorageError::ConfigurationError {
                message: format!("Invalid MongoDB URI: {}", e),
            }
        })?;

        let client = MongoClient::with_options(client_options).map_err(|e| {
            FileStorageError::ConnectionError {
                service: format!("MongoDB: {}", e),
            }
        })?;

        let database = client.database("ai_core_files");
        let files_collection = database.collection("files");
        let folders_collection = database.collection("folders");

        // Test connection
        match database.run_command(doc! {"ping": 1}, None).await {
            Ok(_) => info!("Successfully connected to MongoDB"),
            Err(e) => {
                warn!("Failed to connect to MongoDB: {}", e);
                // Continue anyway for development
            }
        }

        Ok(Self {
            database,
            files_collection,
            folders_collection,
        })
    }

    /// Save file metadata
    pub async fn save_file_metadata(&self, metadata: &FileMetadata) -> FileStorageResult<()> {
        let doc = mongodb::bson::to_document(metadata).map_err(|e| {
            FileStorageError::SerializationError {
                message: format!("Failed to serialize metadata: {}", e),
            }
        })?;

        match self.files_collection.insert_one(doc, None).await {
            Ok(_) => {
                debug!("Saved metadata for file: {}", metadata.id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to save metadata: {}", e);
                Err(FileStorageError::DatabaseError {
                    message: format!("Insert failed: {}", e),
                })
            }
        }
    }

    /// Get file metadata by ID
    pub async fn get_file_metadata(
        &self,
        file_id: &Uuid,
    ) -> FileStorageResult<Option<FileMetadata>> {
        let filter = doc! {"id": file_id.to_string()};

        match self.files_collection.find_one(filter, None).await {
            Ok(Some(doc)) => {
                let metadata: FileMetadata = mongodb::bson::from_document(doc).map_err(|e| {
                    FileStorageError::SerializationError {
                        message: format!("Failed to deserialize metadata: {}", e),
                    }
                })?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                error!("Failed to get metadata: {}", e);
                Err(FileStorageError::DatabaseError {
                    message: format!("Query failed: {}", e),
                })
            }
        }
    }

    /// Update file metadata
    pub async fn update_file_metadata(&self, metadata: &FileMetadata) -> FileStorageResult<()> {
        let filter = doc! {"id": metadata.id.to_string()};
        let doc = mongodb::bson::to_document(metadata).map_err(|e| {
            FileStorageError::SerializationError {
                message: format!("Failed to serialize metadata: {}", e),
            }
        })?;

        let update = doc! {"$set": doc};
        let options = UpdateOptions::builder().upsert(true).build();

        match self
            .files_collection
            .update_one(filter, update, options)
            .await
        {
            Ok(_) => {
                debug!("Updated metadata for file: {}", metadata.id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to update metadata: {}", e);
                Err(FileStorageError::DatabaseError {
                    message: format!("Update failed: {}", e),
                })
            }
        }
    }

    /// Delete file metadata
    pub async fn delete_file_metadata(&self, file_id: &Uuid) -> FileStorageResult<()> {
        let filter = doc! {"id": file_id.to_string()};

        match self.files_collection.delete_one(filter, None).await {
            Ok(_) => {
                debug!("Deleted metadata for file: {}", file_id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete metadata: {}", e);
                Err(FileStorageError::DatabaseError {
                    message: format!("Delete failed: {}", e),
                })
            }
        }
    }

    /// List files with pagination
    pub async fn list_files(
        &self,
        owner_id: &Uuid,
        page: u32,
        per_page: u32,
    ) -> FileStorageResult<(Vec<FileMetadata>, u64)> {
        let filter = doc! {"owner_id": owner_id.to_string()};
        let skip = (page * per_page) as u64;

        // Get total count
        let total = self
            .files_collection
            .count_documents(filter.clone(), None)
            .await
            .map_err(|e| FileStorageError::DatabaseError {
                message: format!("Count failed: {}", e),
            })?;

        // Get files with pagination
        let options = FindOptions::builder()
            .skip(skip)
            .limit(per_page as i64)
            .sort(doc! {"created_at": -1})
            .build();

        let mut cursor = self
            .files_collection
            .find(filter, options)
            .await
            .map_err(|e| FileStorageError::DatabaseError {
                message: format!("Query failed: {}", e),
            })?;

        let mut files = Vec::new();
        while cursor
            .advance()
            .await
            .map_err(|e| FileStorageError::DatabaseError {
                message: format!("Cursor error: {}", e),
            })?
        {
            let doc = cursor.current();
            let metadata: FileMetadata =
                mongodb::bson::from_document(doc.try_into().map_err(|e| {
                    FileStorageError::SerializationError {
                        message: format!("Failed to convert raw document: {}", e),
                    }
                })?)
                .map_err(|e| FileStorageError::SerializationError {
                    message: format!("Failed to deserialize metadata: {}", e),
                })?;
            files.push(metadata);
        }

        Ok((files, total))
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> FileStorageResult<StorageStats> {
        // Aggregate statistics
        let pipeline = vec![doc! {
            "$group": {
                "_id": null,
                "total_files": {"$sum": 1},
                "total_size": {"$sum": "$size"},
            }
        }];

        let mut cursor = self
            .files_collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| FileStorageError::DatabaseError {
                message: format!("Aggregation failed: {}", e),
            })?;

        let mut total_files = 0u64;
        let mut total_size_bytes = 0u64;

        if cursor
            .advance()
            .await
            .map_err(|e| FileStorageError::DatabaseError {
                message: format!("Cursor error: {}", e),
            })?
        {
            let doc = cursor.current();
            total_files = doc.get_i64("total_files").unwrap_or(0) as u64;
            total_size_bytes = doc.get_i64("total_size").unwrap_or(0) as u64;
        }

        // For now, return basic stats
        // In production, you'd calculate these properly
        Ok(StorageStats {
            total_files,
            total_size_bytes,
            uploads_today: 0,           // Would calculate from today's uploads
            downloads_today: 0,         // Would calculate from today's downloads
            storage_usage_percent: 0.0, // Would calculate based on quota
            by_mime_type: HashMap::new(),
            by_user: HashMap::new(),
        })
    }

    /// Health check
    pub async fn health_check(&self) -> FileStorageResult<()> {
        match self.database.run_command(doc! {"ping": 1}, None).await {
            Ok(_) => Ok(()),
            Err(e) => Err(FileStorageError::ServiceUnavailable {
                service: format!("MongoDB: {}", e),
            }),
        }
    }
}

/// Virus scanner service (stub implementation)
pub struct VirusScanner {
    config: VirusScannerConfig,
    enabled: bool,
}

impl VirusScanner {
    /// Create new virus scanner
    pub async fn new(config: &VirusScannerConfig) -> FileStorageResult<Self> {
        Ok(Self {
            config: config.clone(),
            enabled: config.enabled,
        })
    }

    /// Scan file for viruses
    pub async fn scan_file(
        &self,
        _file_path: &str,
        data: &[u8],
    ) -> FileStorageResult<VirusScanResult> {
        if !self.enabled {
            return Ok(VirusScanResult {
                status: ScanStatus::Skipped,
                scanner: "disabled".to_string(),
                scanned_at: Utc::now(),
                scan_duration_ms: 0,
                threats: Vec::new(),
                scanner_version: None,
                signature_version: None,
            });
        }

        // Stub implementation - always returns clean
        // In production, this would integrate with ClamAV or similar
        let start_time = std::time::Instant::now();

        // Simulate scanning time
        tokio::time::sleep(Duration::from_millis(100)).await;

        let scan_duration_ms = start_time.elapsed().as_millis() as u64;

        // Basic heuristic checks
        let mut status = ScanStatus::Clean;
        let mut threats = Vec::new();

        // Check for suspicious patterns
        if data.len() > 0 {
            // Check for executable headers
            if data.starts_with(b"MZ") || data.starts_with(b"\x7fELF") {
                // Don't allow executables
                status = ScanStatus::Infected;
                threats.push(crate::models::ThreatInfo {
                    name: "Executable.Generic".to_string(),
                    threat_type: "Executable".to_string(),
                    severity: crate::models::ThreatSeverity::High,
                    description: Some("Executable file detected".to_string()),
                });
            }
        }

        Ok(VirusScanResult {
            status,
            scanner: "ai-core-scanner".to_string(),
            scanned_at: Utc::now(),
            scan_duration_ms,
            threats,
            scanner_version: Some("1.0.0".to_string()),
            signature_version: Some("20240101".to_string()),
        })
    }

    /// Health check
    pub async fn health_check(&self) -> FileStorageResult<()> {
        if !self.enabled {
            return Ok(());
        }

        // In production, this would check ClamAV daemon connectivity
        Ok(())
    }
}

/// Media processor for image/video processing
pub struct MediaProcessor {
    config: ProcessingConfig,
}

impl MediaProcessor {
    /// Create new media processor
    pub fn new(config: &ProcessingConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Process image file
    pub async fn process_image(
        &self,
        data: &[u8],
        original_name: &str,
    ) -> FileStorageResult<ProcessingResult> {
        let start_time = Utc::now();
        let mut steps = Vec::new();
        let mut artifacts = HashMap::new();

        if !self.config.enable_image_processing {
            return Ok(ProcessingResult {
                status: ProcessingStatus::Completed,
                started_at: start_time,
                completed_at: Some(Utc::now()),
                duration_ms: Some(0),
                steps,
                errors: Vec::new(),
                artifacts,
            });
        }

        // Load image
        let image = match image::load_from_memory(data) {
            Ok(img) => img,
            Err(e) => {
                return Ok(ProcessingResult {
                    status: ProcessingStatus::Failed,
                    started_at: start_time,
                    completed_at: Some(Utc::now()),
                    duration_ms: Some(0),
                    steps,
                    errors: vec![crate::models::ProcessingError {
                        code: "IMAGE_LOAD_ERROR".to_string(),
                        message: e.to_string(),
                        timestamp: Utc::now(),
                        details: None,
                    }],
                    artifacts,
                });
            }
        };

        // Generate thumbnails
        let thumbnail_step_start = Utc::now();
        let mut thumbnails = Vec::new();

        for thumbnail_size in &self.config.thumbnail_sizes {
            let resized = image.resize(
                thumbnail_size.width,
                thumbnail_size.height,
                FilterType::Lanczos3,
            );

            let mut buffer = Vec::new();
            let format = ImageFormat::Jpeg;

            if resized
                .write_to(&mut Cursor::new(&mut buffer), format)
                .is_ok()
            {
                thumbnails.push(ThumbnailInfo {
                    name: thumbnail_size.name.clone(),
                    width: thumbnail_size.width,
                    height: thumbnail_size.height,
                    size: buffer.len() as u64,
                    storage_key: format!("thumbnails/{}/{}", original_name, thumbnail_size.name),
                    mime_type: "image/jpeg".to_string(),
                    created_at: Utc::now(),
                });
            }
        }

        steps.push(ProcessingStep {
            name: "generate_thumbnails".to_string(),
            status: ProcessingStatus::Completed,
            started_at: thumbnail_step_start,
            completed_at: Some(Utc::now()),
            duration_ms: Some((Utc::now() - thumbnail_step_start).num_milliseconds() as u64),
            output: Some(serde_json::json!({
                "thumbnails_generated": thumbnails.len()
            })),
        });

        artifacts.insert(
            "thumbnails".to_string(),
            serde_json::to_value(&thumbnails).unwrap(),
        );

        let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;

        Ok(ProcessingResult {
            status: ProcessingStatus::Completed,
            started_at: start_time,
            completed_at: Some(Utc::now()),
            duration_ms: Some(duration_ms),
            steps,
            errors: Vec::new(),
            artifacts,
        })
    }

    /// Generate file hash
    pub async fn generate_hash(&self, data: &[u8]) -> (String, String) {
        // Generate SHA-256 hash
        let mut sha256_hasher = Sha256::new();
        sha256_hasher.update(data);
        let sha256_hash = format!("{:x}", sha256_hasher.finalize());

        // Generate Blake3 hash
        let mut blake3_hasher = Blake3Hasher::new();
        blake3_hasher.update(data);
        let blake3_hash = blake3_hasher.finalize().to_hex().to_string();

        (sha256_hash, blake3_hash)
    }
}

/// Access control service for permission management
pub struct AccessControlService {
    jwt_secret: String,
}

impl AccessControlService {
    /// Create new access control service
    pub fn new(jwt_secret: &str) -> FileStorageResult<Self> {
        Ok(Self {
            jwt_secret: jwt_secret.to_string(),
        })
    }

    /// Check if user can access file
    pub async fn can_access_file(
        &self,
        user_id: &Uuid,
        file_metadata: &FileMetadata,
        action: &str,
    ) -> bool {
        // Owner can do everything
        if file_metadata.owner_id == *user_id {
            return true;
        }

        // Check public permissions
        if action == "read" && file_metadata.permissions.public_read {
            return true;
        }

        if action == "write" && file_metadata.permissions.public_write {
            return true;
        }

        // Check user-specific permissions
        if let Some(permissions) = file_metadata.permissions.user_permissions.get(user_id) {
            match action {
                "read" => permissions.contains(&crate::models::Permission::Read),
                "write" => permissions.contains(&crate::models::Permission::Write),
                "delete" => permissions.contains(&crate::models::Permission::Delete),
                "share" => permissions.contains(&crate::models::Permission::Share),
                "admin" => permissions.contains(&crate::models::Permission::Admin),
                _ => false,
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_media_processor() {
        let config = ProcessingConfig::default();
        let processor = MediaProcessor::new(&config);

        // Test with a small image data (1x1 pixel PNG)
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x35, 0xA3, 0x6F, 0xDA, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        let result = processor
            .process_image(&png_data, "test.png")
            .await
            .unwrap();
        assert_eq!(result.status, ProcessingStatus::Completed);
    }

    #[tokio::test]
    async fn test_hash_generation() {
        let config = ProcessingConfig::default();
        let processor = MediaProcessor::new(&config);

        let data = b"test data";
        let (sha256, blake3) = processor.generate_hash(data).await;

        assert!(!sha256.is_empty());
        assert!(!blake3.is_empty());
        assert_ne!(sha256, blake3);
    }

    #[tokio::test]
    async fn test_virus_scanner() {
        let config = VirusScannerConfig::default();
        let scanner = VirusScanner::new(&config).await.unwrap();

        // Test with clean data
        let clean_data = b"This is clean text data";
        let result = scanner.scan_file("test.txt", clean_data).await.unwrap();
        assert_eq!(result.status, ScanStatus::Clean);

        // Test with executable data
        let exe_data = b"MZ\x90\x00\x03\x00\x00\x00"; // PE header
        let result = scanner.scan_file("test.exe", exe_data).await.unwrap();
        assert_eq!(result.status, ScanStatus::Infected);
        assert!(!result.threats.is_empty());
    }

    #[tokio::test]
    async fn test_access_control() {
        let service = AccessControlService::new("test-secret").unwrap();

        let owner_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();

        let file_metadata = FileMetadata::new(
            "test.txt".to_string(),
            "text/plain".to_string(),
            100,
            "hash123".to_string(),
            owner_id,
            "s3".to_string(),
            "bucket".to_string(),
            "key".to_string(),
        );

        // Owner should have access
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

        // Other user should not have access by default
        assert!(
            !service
                .can_access_file(&other_user_id, &file_metadata, "read")
                .await
        );
        assert!(
            !service
                .can_access_file(&other_user_id, &file_metadata, "write")
                .await
        );
        assert!(
            !service
                .can_access_file(&other_user_id, &file_metadata, "delete")
                .await
        );
    }
}
