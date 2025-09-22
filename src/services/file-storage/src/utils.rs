//! Utility functions for file storage service
//!
//! This module contains common utility functions used throughout the file storage service
//! including file type detection, path manipulation, validation helpers, and more.

use crate::error::{FileStorageError, FileStorageResult};
use chrono::{DateTime, Utc};
use mime_guess::from_path;

use std::path::Path;
use uuid::Uuid;

/// File type detection and validation utilities
pub mod file_type {
    use super::*;

    /// Detect MIME type from file extension and/or content
    pub fn detect_mime_type(filename: &str, content: Option<&[u8]>) -> String {
        // First try to detect from filename
        let mime_from_filename = from_path(filename).first_or_octet_stream();

        // If we have content, try to detect from magic bytes
        if let Some(data) = content {
            if let Some(detected_type) = detect_from_magic_bytes(data) {
                return detected_type;
            }
        }

        mime_from_filename.to_string()
    }

    /// Detect file type from magic bytes (file signature)
    pub fn detect_from_magic_bytes(data: &[u8]) -> Option<String> {
        if data.len() < 8 {
            return None;
        }

        match data {
            // Images
            data if data.starts_with(b"\xFF\xD8\xFF") => Some("image/jpeg".to_string()),
            data if data.starts_with(b"\x89PNG\r\n\x1A\n") => Some("image/png".to_string()),
            data if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") => {
                Some("image/gif".to_string())
            }
            data if data.starts_with(b"RIFF") && data[8..12] == *b"WEBP" => {
                Some("image/webp".to_string())
            }
            data if data.starts_with(b"BM") => Some("image/bmp".to_string()),

            // Videos
            data if data[4..8] == *b"ftyp" => Some("video/mp4".to_string()),
            data if data.starts_with(b"\x1A\x45\xDF\xA3") => Some("video/webm".to_string()),
            data if data.starts_with(b"RIFF") && data[8..12] == *b"AVI " => {
                Some("video/avi".to_string())
            }

            // Audio
            data if data.starts_with(b"ID3") || data.starts_with(b"\xFF\xFB") => {
                Some("audio/mpeg".to_string())
            }
            data if data.starts_with(b"OggS") => Some("audio/ogg".to_string()),
            data if data.starts_with(b"RIFF") && data[8..12] == *b"WAVE" => {
                Some("audio/wav".to_string())
            }

            // Documents
            data if data.starts_with(b"%PDF") => Some("application/pdf".to_string()),
            data if data.starts_with(b"PK\x03\x04") => Some("application/zip".to_string()),
            data if data.starts_with(b"\xD0\xCF\x11\xE0") => {
                Some("application/vnd.ms-office".to_string())
            }

            // Text files
            data if is_text_content(data) => Some("text/plain".to_string()),

            _ => None,
        }
    }

    /// Check if content appears to be text
    fn is_text_content(data: &[u8]) -> bool {
        if data.is_empty() {
            return true;
        }

        // Check first 1024 bytes for text content
        let sample = &data[..std::cmp::min(1024, data.len())];

        // Count non-printable characters
        let non_printable_count = sample
            .iter()
            .filter(|&&b| b < 32 && b != 9 && b != 10 && b != 13) // Tab, LF, CR are OK
            .count();

        // If less than 5% non-printable, consider it text
        (non_printable_count as f64 / sample.len() as f64) < 0.05
    }

    /// Check if file type is allowed
    pub fn is_allowed_file_type(mime_type: &str, allowed_types: &[String]) -> bool {
        if allowed_types.is_empty() {
            return true; // No restrictions
        }

        allowed_types.iter().any(|allowed| {
            // Exact match
            if allowed == mime_type {
                return true;
            }

            // Wildcard match (e.g., "image/*")
            if allowed.ends_with("/*") {
                let prefix = &allowed[..allowed.len() - 1];
                return mime_type.starts_with(prefix);
            }

            false
        })
    }
}

/// Path and filename utilities
pub mod path {
    use super::*;

    /// Sanitize filename for safe storage
    pub fn sanitize_filename(filename: &str) -> String {
        // Replace unsafe characters
        let safe_chars = regex::Regex::new(r"[^\w\-_\.]").unwrap();
        let sanitized = safe_chars.replace_all(filename, "_");

        // Limit length
        let max_length = 255;
        if sanitized.len() > max_length {
            let sanitized_string = sanitized.to_string();
            let extension = Path::new(&sanitized_string)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            let name_without_ext = &sanitized[..sanitized.len() - extension.len() - 1];
            let truncated_name = &name_without_ext[..max_length - extension.len() - 1];

            format!("{}.{}", truncated_name, extension)
        } else {
            sanitized.to_string()
        }
    }

    /// Generate unique storage path
    pub fn generate_storage_path(file_id: &Uuid, filename: &str, organize_by_date: bool) -> String {
        let sanitized_filename = sanitize_filename(filename);
        let extension = Path::new(&sanitized_filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{}", ext))
            .unwrap_or_default();

        if organize_by_date {
            let now = Utc::now();
            let date_path = now.format("%Y/%m/%d").to_string();
            format!("{}/{}{}", date_path, file_id, extension)
        } else {
            format!("{}{}", file_id, extension)
        }
    }

    /// Extract file extension
    pub fn get_file_extension(filename: &str) -> Option<String> {
        Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Check if filename has blocked extension
    pub fn has_blocked_extension(filename: &str, blocked_extensions: &[String]) -> bool {
        if let Some(extension) = get_file_extension(filename) {
            blocked_extensions.contains(&extension)
        } else {
            false
        }
    }
}

/// Validation utilities
pub mod validation {
    use super::*;

    /// Validate filename
    pub fn validate_filename(filename: &str) -> FileStorageResult<()> {
        if filename.is_empty() {
            return Err(FileStorageError::InvalidFileName {
                name: filename.to_string(),
            });
        }

        if filename.len() > 255 {
            return Err(FileStorageError::InvalidFileName {
                name: "Filename too long".to_string(),
            });
        }

        // Check for dangerous patterns
        let dangerous_patterns = [
            "..", "/", "\\", ":", "*", "?", "\"", "<", ">", "|", "CON", "PRN", "AUX",
            "NUL", // Windows reserved names
        ];

        for pattern in &dangerous_patterns {
            if filename.contains(pattern) {
                return Err(FileStorageError::InvalidFileName {
                    name: format!("Contains dangerous pattern: {}", pattern),
                });
            }
        }

        Ok(())
    }

    /// Validate UUID
    pub fn validate_uuid(uuid_str: &str) -> FileStorageResult<Uuid> {
        Uuid::parse_str(uuid_str).map_err(|_| FileStorageError::InvalidParameter {
            parameter: "uuid".to_string(),
            value: uuid_str.to_string(),
        })
    }

    /// Validate file size
    pub fn validate_file_size(size: usize, max_size: usize) -> FileStorageResult<()> {
        if size > max_size {
            return Err(FileStorageError::FileTooLarge { size, max_size });
        }
        Ok(())
    }
}

/// Size formatting utilities
pub mod size {
    /// Format bytes into human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let unit_index = (bytes as f64).log10() as usize / 3;
        let unit_index = unit_index.min(UNITS.len() - 1);

        let size = bytes as f64 / 1000_f64.powi(unit_index as i32);

        if size >= 100.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else if size >= 10.0 {
            format!("{:.1} {}", size, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    /// Parse size string (e.g., "10MB", "1.5GB") to bytes
    pub fn parse_size(size_str: &str) -> Option<u64> {
        let size_regex = regex::Regex::new(r"^(\d+(?:\.\d+)?)\s*([A-Za-z]*)$").ok()?;
        let captures = size_regex.captures(size_str.trim())?;

        let number: f64 = captures.get(1)?.as_str().parse().ok()?;
        let unit = captures.get(2)?.as_str().to_uppercase();

        let multiplier = match unit.as_str() {
            "" | "B" => 1,
            "KB" | "K" => 1_000,
            "MB" | "M" => 1_000_000,
            "GB" | "G" => 1_000_000_000,
            "TB" | "T" => 1_000_000_000_000,
            "KIB" => 1_024_u64,
            "MIB" => 1_024_u64 * 1_024,
            "GIB" => 1_024_u64 * 1_024 * 1_024,
            "TIB" => 1_024_u64 * 1_024 * 1_024 * 1_024,
            _ => return None,
        };

        Some((number * multiplier as f64) as u64)
    }
}

/// Time utilities
pub mod time {
    use super::*;

    /// Format duration in human-readable format
    pub fn format_duration(milliseconds: u64) -> String {
        let total_seconds = milliseconds / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let ms = milliseconds % 1000;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else if seconds > 0 {
            format!("{}.{}s", seconds, ms / 100)
        } else {
            format!("{}ms", milliseconds)
        }
    }

    /// Get current timestamp as ISO string
    pub fn current_timestamp() -> String {
        Utc::now().to_rfc3339()
    }

    /// Parse ISO timestamp
    pub fn parse_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    }
}

/// Hash utilities
pub mod hash {
    use blake3::Hasher as Blake3Hasher;
    use sha2::{Digest, Sha256};

    /// Generate SHA-256 hash
    pub fn sha256_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Generate Blake3 hash
    pub fn blake3_hash(data: &[u8]) -> String {
        let mut hasher = Blake3Hasher::new();
        hasher.update(data);
        hasher.finalize().to_hex().to_string()
    }

    /// Generate both hashes for deduplication
    pub fn generate_file_hashes(data: &[u8]) -> (String, String) {
        (sha256_hash(data), blake3_hash(data))
    }
}

/// Security utilities
pub mod security {
    use rand::{thread_rng, Rng};

    /// Generate secure random token
    pub fn generate_token(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = thread_rng();

        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Check if IP address is in allowed range
    pub fn is_allowed_ip(ip: &str, allowed_ranges: &[String]) -> bool {
        if allowed_ranges.is_empty() {
            return true; // No restrictions
        }

        // Simple implementation - in production you'd use proper CIDR matching
        allowed_ranges.iter().any(|range| {
            if range == "*" || range == ip {
                true
            } else if range.ends_with(".*") {
                let prefix = &range[..range.len() - 1];
                ip.starts_with(prefix)
            } else {
                false
            }
        })
    }
}

/// Content analysis utilities
pub mod content {
    /// Check if content appears to be suspicious
    pub fn is_suspicious_content(data: &[u8], filename: &str) -> bool {
        // Check for executable signatures
        if data.starts_with(b"MZ") || data.starts_with(b"\x7fELF") {
            return true;
        }

        // Check for script extensions with executable content
        if filename.ends_with(".js") || filename.ends_with(".vbs") || filename.ends_with(".bat") {
            // Check for suspicious patterns in script files
            let content_str = String::from_utf8_lossy(data);
            let suspicious_patterns = [
                "eval(",
                "exec(",
                "system(",
                "shell_exec(",
                "CreateObject",
                "WScript.Shell",
                "cmd.exe",
                "powershell",
                "download",
                "http://",
                "https://",
            ];

            return suspicious_patterns
                .iter()
                .any(|pattern| content_str.to_lowercase().contains(&pattern.to_lowercase()));
        }

        false
    }

    /// Extract metadata from common file types
    pub fn extract_basic_metadata(
        data: &[u8],
        mime_type: &str,
    ) -> std::collections::HashMap<String, String> {
        let mut metadata = std::collections::HashMap::new();

        metadata.insert("size".to_string(), data.len().to_string());
        metadata.insert("mime_type".to_string(), mime_type.to_string());

        // Add specific metadata based on file type
        if mime_type.starts_with("image/") {
            if let Ok(img) = image::load_from_memory(data) {
                metadata.insert("width".to_string(), img.width().to_string());
                metadata.insert("height".to_string(), img.height().to_string());
                metadata.insert("format".to_string(), format!("{:?}", img.color()));
            }
        }

        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_detection() {
        // Test JPEG detection
        let jpeg_header = b"\xFF\xD8\xFF\xE0";
        assert_eq!(
            file_type::detect_from_magic_bytes(jpeg_header),
            Some("image/jpeg".to_string())
        );

        // Test PNG detection
        let png_header = b"\x89PNG\r\n\x1A\n";
        assert_eq!(
            file_type::detect_from_magic_bytes(png_header),
            Some("image/png".to_string())
        );

        // Test PDF detection
        let pdf_header = b"%PDF-1.4";
        assert_eq!(
            file_type::detect_from_magic_bytes(pdf_header),
            Some("application/pdf".to_string())
        );
    }

    #[test]
    fn test_filename_sanitization() {
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
    }

    #[test]
    fn test_file_extension() {
        assert_eq!(
            path::get_file_extension("test.jpg"),
            Some("jpg".to_string())
        );
        assert_eq!(
            path::get_file_extension("test.tar.gz"),
            Some("gz".to_string())
        );
        assert_eq!(path::get_file_extension("no_extension"), None);
    }

    #[test]
    fn test_filename_validation() {
        assert!(validation::validate_filename("valid_file.txt").is_ok());
        assert!(validation::validate_filename("").is_err());
        assert!(validation::validate_filename("file../test.txt").is_err());
        assert!(validation::validate_filename("CON.txt").is_err());
    }

    #[test]
    fn test_size_formatting() {
        assert_eq!(size::format_bytes(0), "0 B");
        assert_eq!(size::format_bytes(1024), "1.02 KB");
        assert_eq!(size::format_bytes(1_000_000), "1.00 MB");
        assert_eq!(size::format_bytes(1_500_000_000), "1.50 GB");
    }

    #[test]
    fn test_size_parsing() {
        assert_eq!(size::parse_size("10MB"), Some(10_000_000));
        assert_eq!(size::parse_size("1.5GB"), Some(1_500_000_000));
        assert_eq!(size::parse_size("100"), Some(100));
        assert_eq!(size::parse_size("invalid"), None);
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(time::format_duration(500), "500ms");
        assert_eq!(time::format_duration(1500), "1.5s");
        assert_eq!(time::format_duration(65000), "1m 5s");
        assert_eq!(time::format_duration(3661000), "1h 1m 1s");
    }

    #[test]
    fn test_hash_generation() {
        let data = b"test data";
        let (sha256, blake3) = hash::generate_file_hashes(data);

        assert!(!sha256.is_empty());
        assert!(!blake3.is_empty());
        assert_ne!(sha256, blake3);

        // SHA-256 should be 64 characters (32 bytes * 2 hex chars)
        assert_eq!(sha256.len(), 64);

        // Blake3 should be 64 characters as well
        assert_eq!(blake3.len(), 64);
    }

    #[test]
    fn test_token_generation() {
        let token1 = security::generate_token(32);
        let token2 = security::generate_token(32);

        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2); // Should be different
    }

    #[test]
    fn test_suspicious_content_detection() {
        // Executable content should be flagged
        let exe_content = b"MZ\x90\x00\x03\x00";
        assert!(content::is_suspicious_content(exe_content, "file.exe"));

        // Script with suspicious patterns
        let script_content = b"eval('malicious code')";
        assert!(content::is_suspicious_content(script_content, "script.js"));

        // Clean text should not be flagged
        let clean_content = b"This is just normal text content";
        assert!(!content::is_suspicious_content(
            clean_content,
            "document.txt"
        ));
    }

    #[test]
    fn test_allowed_file_types() {
        let allowed_types = vec![
            "image/jpeg".to_string(),
            "image/png".to_string(),
            "text/*".to_string(),
        ];

        assert!(file_type::is_allowed_file_type(
            "image/jpeg",
            &allowed_types
        ));
        assert!(file_type::is_allowed_file_type(
            "text/plain",
            &allowed_types
        ));
        assert!(!file_type::is_allowed_file_type(
            "application/exe",
            &allowed_types
        ));

        // Empty list should allow everything
        assert!(file_type::is_allowed_file_type("anything", &[]));
    }
}
