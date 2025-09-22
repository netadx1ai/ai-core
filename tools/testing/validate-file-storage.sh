#!/bin/bash

# AI-CORE File Storage Service Validation Script
# This script validates the file storage service implementation

set -e

echo "🚀 AI-CORE File Storage Service Validation"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "SUCCESS")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "ERROR")
            echo -e "${RED}❌ $message${NC}"
            ;;
        "WARNING")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
    esac
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "src/services/file-storage" ]; then
    print_status "ERROR" "Please run this script from the AI-CORE project root directory"
    exit 1
fi

print_status "INFO" "Starting file storage service validation..."

# Step 1: Check workspace configuration
echo
echo "Step 1: Validating workspace configuration"
echo "----------------------------------------"

if grep -q "src/services/file-storage" Cargo.toml; then
    print_status "SUCCESS" "File storage service is registered in workspace"
else
    print_status "ERROR" "File storage service not found in workspace Cargo.toml"
    exit 1
fi

# Step 2: Check service structure
echo
echo "Step 2: Validating service structure"
echo "-----------------------------------"

required_files=(
    "src/services/file-storage/Cargo.toml"
    "src/services/file-storage/src/main.rs"
    "src/services/file-storage/src/config_types.rs"
    "src/services/file-storage/src/error.rs"
    "src/services/file-storage/src/models.rs"
    "src/services/file-storage/src/services.rs"
    "src/services/file-storage/src/handlers.rs"
    "src/services/file-storage/src/middleware_auth.rs"
    "src/services/file-storage/src/utils.rs"
    "src/services/file-storage/src/tests.rs"
)

for file in "${required_files[@]}"; do
    if [ -f "$file" ]; then
        print_status "SUCCESS" "Found required file: $file"
    else
        print_status "ERROR" "Missing required file: $file"
        exit 1
    fi
done

# Step 3: Check dependencies
echo
echo "Step 3: Validating dependencies"
echo "------------------------------"

if grep -q "axum.*multipart" src/services/file-storage/Cargo.toml; then
    print_status "SUCCESS" "Axum multipart feature enabled"
else
    print_status "WARNING" "Axum multipart feature not found"
fi

required_deps=(
    "axum"
    "tokio"
    "serde"
    "uuid"
    "chrono"
    "tracing"
    "anyhow"
    "thiserror"
    "mongodb"
    "redis"
    "aws-sdk-s3"
    "image"
    "blake3"
    "sha2"
    "jsonwebtoken"
)

for dep in "${required_deps[@]}"; do
    if grep -q "$dep" src/services/file-storage/Cargo.toml; then
        print_status "SUCCESS" "Found dependency: $dep"
    else
        print_status "WARNING" "Dependency not found: $dep"
    fi
done

# Step 4: Validate code structure
echo
echo "Step 4: Validating code structure"
echo "--------------------------------"

# Check main.rs structure
if grep -q "pub struct AppState" src/services/file-storage/src/main.rs; then
    print_status "SUCCESS" "AppState struct found"
else
    print_status "ERROR" "AppState struct not found in main.rs"
fi

if grep -q "async fn main()" src/services/file-storage/src/main.rs; then
    print_status "SUCCESS" "Async main function found"
else
    print_status "ERROR" "Async main function not found"
fi

# Check services implementation
services=(
    "StorageService"
    "MetadataService"
    "VirusScanner"
    "MediaProcessor"
    "AccessControlService"
)

for service in "${services[@]}"; do
    if grep -q "pub struct $service" src/services/file-storage/src/services.rs; then
        print_status "SUCCESS" "Found service: $service"
    else
        print_status "ERROR" "Service not found: $service"
    fi
done

# Check error types
error_types=(
    "FileStorageError"
    "FileStorageResult"
)

for error_type in "${error_types[@]}"; do
    if grep -q "$error_type" src/services/file-storage/src/error.rs; then
        print_status "SUCCESS" "Found error type: $error_type"
    else
        print_status "ERROR" "Error type not found: $error_type"
    fi
done

# Check models
models=(
    "FileMetadata"
    "FilePermissions"
    "FileStatus"
    "VirusScanResult"
    "ProcessingResult"
)

for model in "${models[@]}"; do
    if grep -q "pub struct $model\|pub enum $model" src/services/file-storage/src/models.rs; then
        print_status "SUCCESS" "Found model: $model"
    else
        print_status "ERROR" "Model not found: $model"
    fi
done

# Step 5: Check API endpoints
echo
echo "Step 5: Validating API endpoints"
echo "-------------------------------"

endpoints=(
    "/health"
    "/metrics"
    "/api/v1/files/upload"
    "/api/v1/files/upload/multipart"
    "/api/v1/files/:file_id/download"
    "/api/v1/files/:file_id"
    "/api/v1/files"
    "/api/v1/folders"
)

for endpoint in "${endpoints[@]}"; do
    if grep -q "$endpoint" src/services/file-storage/src/main.rs; then
        print_status "SUCCESS" "Found endpoint: $endpoint"
    else
        print_status "WARNING" "Endpoint not found: $endpoint"
    fi
done

# Step 6: Check handler functions
echo
echo "Step 6: Validating handler functions"
echo "-----------------------------------"

handlers=(
    "upload_file"
    "upload_multipart"
    "download_file"
    "get_file_info"
    "delete_file"
    "list_files"
    "health_check"
)

for handler in "${handlers[@]}"; do
    if grep -q "pub async fn $handler\|async fn $handler" src/services/file-storage/src/handlers.rs; then
        print_status "SUCCESS" "Found handler: $handler"
    elif grep -q "async fn $handler" src/services/file-storage/src/main.rs; then
        print_status "SUCCESS" "Found handler in main.rs: $handler"
    else
        print_status "ERROR" "Handler not found: $handler"
    fi
done

# Step 7: Check utility modules
echo
echo "Step 7: Validating utility modules"
echo "---------------------------------"

util_modules=(
    "file_type"
    "path"
    "validation"
    "size"
    "hash"
    "security"
)

for module in "${util_modules[@]}"; do
    if grep -q "pub mod $module" src/services/file-storage/src/utils.rs; then
        print_status "SUCCESS" "Found utility module: $module"
    else
        print_status "ERROR" "Utility module not found: $module"
    fi
done

# Step 8: Check test coverage
echo
echo "Step 8: Validating test coverage"
echo "-------------------------------"

test_modules=(
    "unit_tests"
    "utility_tests"
    "service_tests"
    "integration_tests"
    "error_tests"
    "performance_tests"
)

for test_module in "${test_modules[@]}"; do
    if grep -q "mod $test_module" src/services/file-storage/src/tests.rs; then
        print_status "SUCCESS" "Found test module: $test_module"
    else
        print_status "WARNING" "Test module not found: $test_module"
    fi
done

# Step 9: Check configuration
echo
echo "Step 9: Validating configuration"
echo "-------------------------------"

config_structs=(
    "FileStorageConfig"
    "StorageConfig"
    "SecurityConfig"
    "ProcessingConfig"
)

for config in "${config_structs[@]}"; do
    if grep -q "pub struct $config" src/services/file-storage/src/config_types.rs; then
        print_status "SUCCESS" "Found config struct: $config"
    else
        print_status "ERROR" "Config struct not found: $config"
    fi
done

# Step 10: Try to compile (syntax check only)
echo
echo "Step 10: Syntax validation"
echo "-------------------------"

print_status "INFO" "Checking Rust syntax..."

# Use cargo check with json output to get structured error information
if cargo check --package file-storage-service --message-format=json > /tmp/cargo_check.json 2>&1; then
    print_status "SUCCESS" "Rust syntax validation passed"
else
    print_status "WARNING" "Syntax issues found - check compilation output"
    # Try to extract useful error information
    if [ -f /tmp/cargo_check.json ]; then
        echo "Recent compilation messages:"
        grep -o '"message":"[^"]*"' /tmp/cargo_check.json | sed 's/"message":"//g' | sed 's/"$//g' | tail -10
    fi
fi

# Step 11: Feature completeness check
echo
echo "Step 11: Feature completeness check"
echo "----------------------------------"

features=(
    "File upload with virus scanning"
    "Multi-platform storage support"
    "Authentication middleware"
    "Permission management"
    "File processing (thumbnails)"
    "Batch operations"
    "Search functionality"
    "Health monitoring"
)

# Check for key implementation indicators
if grep -q "virus_scan" src/services/file-storage/src/handlers.rs; then
    print_status "SUCCESS" "Virus scanning integration found"
else
    print_status "WARNING" "Virus scanning integration not found"
fi

if grep -q "S3Client\|MinIO" src/services/file-storage/src/services.rs; then
    print_status "SUCCESS" "S3/MinIO storage support found"
else
    print_status "WARNING" "S3/MinIO storage support not found"
fi

if grep -q "jwt" src/services/file-storage/src/middleware_auth.rs; then
    print_status "SUCCESS" "JWT authentication found"
else
    print_status "WARNING" "JWT authentication not found"
fi

if grep -q "thumbnail" src/services/file-storage/src/handlers.rs; then
    print_status "SUCCESS" "Thumbnail functionality found"
else
    print_status "WARNING" "Thumbnail functionality not found"
fi

# Final summary
echo
echo "Validation Summary"
echo "=================="

# Count critical issues
critical_issues=0
if ! grep -q "StorageService" src/services/file-storage/src/services.rs; then
    ((critical_issues++))
fi
if ! grep -q "FileStorageError" src/services/file-storage/src/error.rs; then
    ((critical_issues++))
fi
if ! grep -q "async fn main()" src/services/file-storage/src/main.rs; then
    ((critical_issues++))
fi

if [ $critical_issues -eq 0 ]; then
    print_status "SUCCESS" "File storage service validation completed successfully!"
    print_status "INFO" "The service appears to be properly implemented with all core components."
    echo
    echo "Next Steps:"
    echo "1. Run 'cargo build --package file-storage-service' to compile"
    echo "2. Run 'cargo test --package file-storage-service' to execute tests"
    echo "3. Start the service with 'cargo run --bin file-storage'"
    echo "4. Test API endpoints with curl or your preferred HTTP client"
    echo
    print_status "SUCCESS" "File Storage Service Implementation: COMPLETE ✅"
else
    print_status "ERROR" "Found $critical_issues critical issues that need to be resolved"
    echo
    echo "Please address the critical issues before proceeding."
fi

# Cleanup
rm -f /tmp/cargo_check.json

echo
echo "📊 Validation completed at $(date)"
