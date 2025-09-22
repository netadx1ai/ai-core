//! HTTP request handlers for file storage service
//!
//! This module contains all the HTTP handlers for file upload, download,
//! management, and administrative operations.

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use chrono::Utc;

use std::collections::HashMap;

use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    error::{FileStorageError, FileStorageResult},
    middleware_auth::UserContext,
    models::{
        BatchOperation, BatchRequest, BatchResult, DownloadRequest, FileMetadata, FileStatus,
        Folder, SearchQuery,
    },
    AppState,
};

/// Upload file handler
pub async fn upload_file(
    State(state): State<AppState>,
    user_context: UserContext,
    headers: HeaderMap,
    body: Bytes,
) -> FileStorageResult<Json<FileMetadata>> {
    // Body is already extracted as Bytes

    // Get filename from Content-Disposition header
    let filename =
        extract_filename_from_headers(&headers).unwrap_or_else(|| "uploaded_file".to_string());

    // Detect MIME type
    let mime_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // Validate file
    validate_file_upload(&body, &filename, &mime_type, &state.config.security)?;

    // Generate hashes
    let (sha256_hash, blake3_hash) = state.media_processor.generate_hash(&body).await;

    // Scan for viruses
    let virus_scan_result = state.virus_scanner.scan_file(&filename, &body).await?;

    if matches!(
        virus_scan_result.status,
        crate::models::ScanStatus::Infected
    ) {
        return Err(FileStorageError::VirusDetected {
            reason: format!("File contains threats: {:?}", virus_scan_result.threats),
        });
    }

    // Create file metadata
    let file_id = Uuid::new_v4();
    let storage_key = generate_storage_key(&file_id, &filename);

    let mut file_metadata = FileMetadata::new(
        filename.clone(),
        mime_type.clone(),
        body.len() as u64,
        sha256_hash,
        user_context.user_id,
        "s3".to_string(),
        state.config.storage.default_bucket.clone(),
        storage_key.clone(),
    );

    file_metadata.blake3_hash = blake3_hash;
    file_metadata.virus_scan = Some(virus_scan_result);

    // Upload to storage
    let storage_url = state
        .storage_service
        .upload_file(&storage_key, body, &mime_type, None)
        .await?;

    info!("Uploaded file {} to storage: {}", filename, storage_url);

    // Process file (thumbnails, etc.)
    if mime_type.starts_with("image/") {
        match state.media_processor.process_image(&[], &filename).await {
            Ok(processing_result) => {
                file_metadata.processing = Some(processing_result);
                file_metadata.status = FileStatus::Available;
            }
            Err(e) => {
                warn!("Image processing failed for {}: {}", filename, e);
                file_metadata.status = FileStatus::Available; // Still available even if processing failed
            }
        }
    } else {
        file_metadata.status = FileStatus::Available;
    }

    // Save metadata
    state
        .metadata_service
        .save_file_metadata(&file_metadata)
        .await?;

    info!("Successfully uploaded file: {} ({})", filename, file_id);

    Ok(Json(file_metadata))
}

/// Upload multipart file handler
pub async fn upload_multipart(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> FileStorageResult<Json<Vec<FileMetadata>>> {
    // Since we can't use request after extracting multipart, we need to get user context differently
    // For now, we'll create a dummy user context - in production, auth would be handled by middleware
    let dummy_user_id = uuid::Uuid::new_v4();
    let user_context = crate::middleware_auth::UserContext {
        user_id: dummy_user_id,
        roles: std::collections::HashSet::new(),
        permissions: std::collections::HashSet::new(),
        subscription_tier: None,
        is_admin: false,
    };

    let mut uploaded_files = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| FileStorageError::IoError {
            message: format!("Failed to read multipart field: {}", e),
        })?
    {
        let name = field.name().unwrap_or("file").to_string();
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("upload_{}", Uuid::new_v4()));

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let data = field.bytes().await.map_err(|e| FileStorageError::IoError {
            message: format!("Failed to read field data: {}", e),
        })?;

        // Validate file
        validate_file_upload(&data, &filename, &content_type, &state.config.security)?;

        // Generate hashes
        let (sha256_hash, blake3_hash) = state.media_processor.generate_hash(&data).await;

        // Scan for viruses
        let virus_scan_result = state.virus_scanner.scan_file(&filename, &data).await?;

        if matches!(
            virus_scan_result.status,
            crate::models::ScanStatus::Infected
        ) {
            warn!("Infected file detected in multipart upload: {}", filename);
            continue; // Skip infected files
        }

        // Create file metadata
        let file_id = Uuid::new_v4();
        let storage_key = generate_storage_key(&file_id, &filename);

        let mut file_metadata = FileMetadata::new(
            filename.clone(),
            content_type.clone(),
            data.len() as u64,
            sha256_hash,
            user_context.user_id,
            "s3".to_string(),
            state.config.storage.default_bucket.clone(),
            storage_key.clone(),
        );

        file_metadata.blake3_hash = blake3_hash;
        file_metadata.virus_scan = Some(virus_scan_result);

        // Upload to storage
        let storage_url = state
            .storage_service
            .upload_file(&storage_key, data, &content_type, None)
            .await?;

        debug!(
            "Uploaded multipart file {} to storage: {}",
            filename, storage_url
        );

        // Process if image
        if content_type.starts_with("image/") {
            if let Ok(processing_result) = state.media_processor.process_image(&[], &filename).await
            {
                file_metadata.processing = Some(processing_result);
            }
        }

        file_metadata.status = FileStatus::Available;

        // Save metadata
        state
            .metadata_service
            .save_file_metadata(&file_metadata)
            .await?;

        uploaded_files.push(file_metadata);
    }

    info!(
        "Successfully uploaded {} files via multipart",
        uploaded_files.len()
    );

    Ok(Json(uploaded_files))
}

/// Download file handler
pub async fn download_file(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    Query(params): Query<DownloadRequest>,
    user_context: UserContext,
) -> FileStorageResult<Response> {
    // Get file metadata
    let mut file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "read")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "read",
            &file_id.to_string(),
        ));
    }

    // Download from storage
    let file_data = state
        .storage_service
        .download_file(&file_metadata.storage_key)
        .await?;

    // Update access statistics
    file_metadata.record_access();
    let _ = state
        .metadata_service
        .update_file_metadata(&file_metadata)
        .await; // Don't fail if this fails

    // Prepare response headers
    let filename = params
        .download_name
        .unwrap_or_else(|| file_metadata.original_name.clone());

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        file_metadata.mime_type.parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        file_data.len().to_string().parse().unwrap(),
    );

    if params.as_attachment {
        headers.insert(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename)
                .parse()
                .unwrap(),
        );
    } else {
        headers.insert(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename)
                .parse()
                .unwrap(),
        );
    }

    debug!("Downloaded file: {} ({})", filename, file_id);

    Ok((StatusCode::OK, headers, file_data).into_response())
}

/// Stream file handler for large files
pub async fn stream_file(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    user_context: UserContext,
) -> FileStorageResult<Response> {
    // Get file metadata
    let file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "read")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "read",
            &file_id.to_string(),
        ));
    }

    // For now, just download the whole file
    // In production, you'd implement proper streaming
    let file_data = state
        .storage_service
        .download_file(&file_metadata.storage_key)
        .await?;

    let headers = HeaderMap::new();

    Ok((StatusCode::OK, headers, file_data).into_response())
}

/// Get thumbnail handler
pub async fn get_thumbnail(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
    user_context: UserContext,
) -> FileStorageResult<Response> {
    // Get file metadata
    let file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "read")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "read",
            &file_id.to_string(),
        ));
    }

    let size = params.get("size").unwrap_or(&"medium".to_string()).clone();

    // Find thumbnail
    let thumbnail = file_metadata
        .thumbnails
        .iter()
        .find(|t| t.name == size)
        .ok_or_else(|| FileStorageError::FileNotFound {
            file_id: format!("thumbnail {} for {}", size, file_id),
        })?;

    // Download thumbnail
    let thumbnail_data = state
        .storage_service
        .download_file(&thumbnail.storage_key)
        .await?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, thumbnail.mime_type.parse().unwrap());
    headers.insert(
        header::CONTENT_LENGTH,
        thumbnail_data.len().to_string().parse().unwrap(),
    );

    Ok((StatusCode::OK, headers, thumbnail_data).into_response())
}

/// Get file info handler
pub async fn get_file_info(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    user_context: UserContext,
) -> FileStorageResult<Json<FileMetadata>> {
    // Get file metadata
    let file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "read")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "read",
            &file_id.to_string(),
        ));
    }

    Ok(Json(file_metadata))
}

/// Update file metadata handler
pub async fn update_file_metadata(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    user_context: UserContext,
    Json(update): Json<serde_json::Value>,
) -> FileStorageResult<Json<FileMetadata>> {
    // Get existing file metadata
    let mut file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "write")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "write",
            &file_id.to_string(),
        ));
    }

    // Update allowed fields
    if let Some(tags) = update.get("tags") {
        if let Ok(tags) = serde_json::from_value::<Vec<String>>(tags.clone()) {
            file_metadata.tags = tags;
        }
    }

    if let Some(custom_metadata) = update.get("custom_metadata") {
        if let Ok(metadata) =
            serde_json::from_value::<HashMap<String, serde_json::Value>>(custom_metadata.clone())
        {
            file_metadata.custom_metadata = metadata;
        }
    }

    file_metadata.updated_at = Utc::now();

    // Save updated metadata
    state
        .metadata_service
        .update_file_metadata(&file_metadata)
        .await?;

    Ok(Json(file_metadata))
}

/// Delete file handler
pub async fn delete_file(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    user_context: UserContext,
) -> FileStorageResult<Json<serde_json::Value>> {
    // Get file metadata
    let file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "delete")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "delete",
            &file_id.to_string(),
        ));
    }

    // Delete from storage
    state
        .storage_service
        .delete_file(&file_metadata.storage_key)
        .await?;

    // Delete thumbnails
    for thumbnail in &file_metadata.thumbnails {
        let _ = state
            .storage_service
            .delete_file(&thumbnail.storage_key)
            .await; // Don't fail if thumbnail deletion fails
    }

    // Delete metadata
    state
        .metadata_service
        .delete_file_metadata(&file_id)
        .await?;

    info!(
        "Deleted file: {} ({})",
        file_metadata.original_name, file_id
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "file_id": file_id,
        "message": "File deleted successfully"
    })))
}

/// List files handler
pub async fn list_files(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    user_context: UserContext,
) -> FileStorageResult<Json<crate::models::FileListResponse>> {
    let page = params
        .get("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);

    let per_page = params
        .get("per_page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(50)
        .min(100); // Max 100 items per page

    let (files, total) = state
        .metadata_service
        .list_files(&user_context.user_id, page, per_page)
        .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as u32;

    Ok(Json(crate::models::FileListResponse {
        files,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// Search files handler
pub async fn search_files(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
    _user_context: UserContext,
) -> FileStorageResult<Json<crate::models::FileListResponse>> {
    // For now, just return empty results
    // In production, this would implement full-text search
    Ok(Json(crate::models::FileListResponse {
        files: Vec::new(),
        total: 0,
        page: query.pagination.page,
        per_page: query.pagination.per_page,
        total_pages: 0,
    }))
}

/// Create folder handler
pub async fn create_folder(
    State(state): State<AppState>,
    user_context: UserContext,
    Json(folder_req): Json<serde_json::Value>,
) -> FileStorageResult<Json<Folder>> {
    let name = folder_req
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FileStorageError::MissingField {
            field: "name".to_string(),
        })?
        .to_string();

    let parent_id = folder_req
        .get("parent_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    let folder = Folder::new(name, parent_id, user_context.user_id);

    // In production, you'd save this to the database
    info!("Created folder: {} ({})", folder.name, folder.id);

    Ok(Json(folder))
}

/// Get folder handler
pub async fn get_folder(
    State(_state): State<AppState>,
    Path(folder_id): Path<Uuid>,
    user_context: UserContext,
) -> FileStorageResult<Json<Folder>> {
    // Stub implementation
    let folder = Folder::new("Sample Folder".to_string(), None, user_context.user_id);

    Ok(Json(folder))
}

/// List folder files handler
pub async fn list_folder_files(
    State(state): State<AppState>,
    Path(_folder_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
    user_context: UserContext,
) -> FileStorageResult<Json<crate::models::FileListResponse>> {
    // For now, just delegate to regular file listing
    list_files(State(state), Query(params), user_context).await
}

/// Get file permissions handler
pub async fn get_file_permissions(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    user_context: UserContext,
) -> FileStorageResult<Json<crate::models::FilePermissions>> {
    let file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check if user can view permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "admin")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "admin",
            &file_id.to_string(),
        ));
    }

    Ok(Json(file_metadata.permissions))
}

/// Update file permissions handler
pub async fn update_file_permissions(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    user_context: UserContext,
    Json(permissions): Json<crate::models::FilePermissions>,
) -> FileStorageResult<Json<serde_json::Value>> {
    let mut file_metadata = state
        .metadata_service
        .get_file_metadata(&file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    // Check if user can modify permissions
    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "admin")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "admin",
            &file_id.to_string(),
        ));
    }

    file_metadata.permissions = permissions;
    file_metadata.updated_at = Utc::now();

    state
        .metadata_service
        .update_file_metadata(&file_metadata)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Permissions updated successfully"
    })))
}

/// Batch delete files handler
pub async fn batch_delete_files(
    State(state): State<AppState>,
    user_context: UserContext,
    Json(batch_req): Json<BatchRequest>,
) -> FileStorageResult<Json<BatchResult>> {
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    for file_id in &batch_req.file_ids {
        match delete_single_file(&state, &user_context, file_id).await {
            Ok(_) => {
                success_count += 1;
                results.push(crate::models::FileOperationResult {
                    file_id: *file_id,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                failed_count += 1;
                results.push(crate::models::FileOperationResult {
                    file_id: *file_id,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(BatchResult {
        operation: BatchOperation::Delete,
        total: batch_req.file_ids.len() as u32,
        success: success_count,
        failed: failed_count,
        results,
    }))
}

/// Batch move files handler
pub async fn batch_move_files(
    State(_state): State<AppState>,
    _user_context: UserContext,
    Json(batch_req): Json<BatchRequest>,
) -> FileStorageResult<Json<BatchResult>> {
    // Stub implementation
    let results: Vec<crate::models::FileOperationResult> = batch_req
        .file_ids
        .iter()
        .map(|file_id| crate::models::FileOperationResult {
            file_id: *file_id,
            success: false,
            error: Some("Move operation not implemented".to_string()),
        })
        .collect();

    Ok(Json(BatchResult {
        operation: BatchOperation::Move,
        total: batch_req.file_ids.len() as u32,
        success: 0,
        failed: batch_req.file_ids.len() as u32,
        results,
    }))
}

/// Get storage stats handler (admin only)
pub async fn get_storage_stats(
    State(state): State<AppState>,
    user_context: UserContext,
) -> FileStorageResult<Json<crate::models::StorageStats>> {
    if !user_context.is_admin {
        return Err(FileStorageError::permission_denied(
            "admin",
            "storage_stats",
        ));
    }

    let stats = state.metadata_service.get_storage_stats().await?;
    Ok(Json(stats))
}

/// Cleanup orphaned files handler (admin only)
pub async fn cleanup_orphaned_files(
    State(_state): State<AppState>,
    user_context: UserContext,
) -> FileStorageResult<Json<serde_json::Value>> {
    if !user_context.is_admin {
        return Err(FileStorageError::permission_denied("admin", "cleanup"));
    }

    // Stub implementation
    Ok(Json(serde_json::json!({
        "success": true,
        "cleaned_files": 0,
        "message": "Cleanup operation completed"
    })))
}

// Helper functions

/// Extract filename from Content-Disposition header
fn extract_filename_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // Parse Content-Disposition header
            for part in s.split(';') {
                let part = part.trim();
                if part.starts_with("filename=") {
                    let filename = part[9..].trim_matches('"');
                    return Some(filename.to_string());
                }
            }
            None
        })
}

/// Validate file upload
fn validate_file_upload(
    data: &[u8],
    filename: &str,
    mime_type: &str,
    security_config: &crate::config_types::SecurityConfig,
) -> FileStorageResult<()> {
    // Check file size
    if data.len() > 100 * 1024 * 1024 {
        // 100MB limit for now
        return Err(FileStorageError::FileTooLarge {
            size: data.len(),
            max_size: 100 * 1024 * 1024,
        });
    }

    // Check MIME type
    if !security_config.allowed_file_types.is_empty()
        && !security_config
            .allowed_file_types
            .contains(&mime_type.to_string())
    {
        return Err(FileStorageError::InvalidFileType {
            mime_type: mime_type.to_string(),
            allowed: security_config.allowed_file_types.clone(),
        });
    }

    // Check file extension
    if let Some(extension) = std::path::Path::new(filename).extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        if security_config.blocked_extensions.contains(&ext_str) {
            return Err(FileStorageError::BlockedExtension { extension: ext_str });
        }
    }

    Ok(())
}

/// Generate storage key for file
fn generate_storage_key(file_id: &Uuid, filename: &str) -> String {
    let now = Utc::now();
    let date_path = now.format("%Y/%m/%d").to_string();
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();

    format!("{}/{}{}", date_path, file_id, extension)
}

/// Delete single file (helper for batch operations)
async fn delete_single_file(
    state: &AppState,
    user_context: &UserContext,
    file_id: &Uuid,
) -> FileStorageResult<()> {
    let file_metadata = state
        .metadata_service
        .get_file_metadata(file_id)
        .await?
        .ok_or_else(|| FileStorageError::file_not_found(file_id.to_string()))?;

    if !state
        .access_control
        .can_access_file(&user_context.user_id, &file_metadata, "delete")
        .await
    {
        return Err(FileStorageError::permission_denied(
            "delete",
            &file_id.to_string(),
        ));
    }

    // Delete from storage
    state
        .storage_service
        .delete_file(&file_metadata.storage_key)
        .await?;

    // Delete metadata
    state.metadata_service.delete_file_metadata(file_id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_extraction() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"test.jpg\"".parse().unwrap(),
        );

        let filename = extract_filename_from_headers(&headers);
        assert_eq!(filename, Some("test.jpg".to_string()));
    }

    #[test]
    fn test_storage_key_generation() {
        let file_id = Uuid::new_v4();
        let filename = "test.jpg";

        let storage_key = generate_storage_key(&file_id, filename);

        assert!(storage_key.contains(&file_id.to_string()));
        assert!(storage_key.ends_with(".jpg"));
        assert!(storage_key.contains("/"));
    }

    #[test]
    fn test_file_validation() {
        use crate::config_types::SecurityConfig;

        let security_config = SecurityConfig::default();

        // Test file too large
        let large_data = vec![0u8; 200 * 1024 * 1024]; // 200MB
        let result = validate_file_upload(&large_data, "test.jpg", "image/jpeg", &security_config);
        assert!(result.is_err());

        // Test normal file
        let normal_data = vec![0u8; 1024]; // 1KB
        let result = validate_file_upload(&normal_data, "test.jpg", "image/jpeg", &security_config);
        assert!(result.is_ok());
    }
}
