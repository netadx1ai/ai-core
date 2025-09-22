#!/bin/bash

# AI Instructions Synchronization Tool - AI-CORE (FAANG-Enhanced) - IMPROVED VERSION
# Automatically synchronizes complete AI instructions across all platforms
#
# Usage: ./tools/ai-instructions-sync-improved.sh -Action <action> [options]
#
# Actions:
#   sync-all        - Synchronize all platform files from master AGENTS.md
#   validate        - Validate all platform files are in sync
#   github-sync     - Sync with GitHub specifically
#   status          - Show current sync status
#   init            - Initialize sync system
#
# Platform Support: macOS, Linux, WSL2
# Last Updated: 2025-01-11T16:10:00+00:00

set -euo pipefail

# Configuration
PROJECT_NAME="AI-CORE"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MASTER_FILE="$PROJECT_ROOT/AGENTS.md"
BACKUP_DIR="$PROJECT_ROOT/.ai-sync-backups"
LOG_FILE="$PROJECT_ROOT/.ai-sync.log"
SYNC_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S+00:00")

# Platform files configuration
PLATFORMS=("github" "vscode" "zed" "claude" "gemini")

# Get platform file function
get_platform_file() {
    case $1 in
        "github") echo "$PROJECT_ROOT/.github/copilot-instructions.md" ;;
        "vscode") echo "$PROJECT_ROOT/.vscode/README.md" ;;
        "zed") echo "$PROJECT_ROOT/.zed/README.md" ;;
        "claude") echo "$PROJECT_ROOT/CLAUDE.md" ;;
        "gemini") echo "$PROJECT_ROOT/GEMINI.md" ;;
        *) echo "" ;;
    esac
}

# Get platform display name
get_platform_display_name() {
    case $1 in
        "github") echo "GitHub Copilot" ;;
        "vscode") echo "VS Code" ;;
        "zed") echo "Zed Editor" ;;
        "claude") echo "Claude AI" ;;
        "gemini") echo "Google Gemini" ;;
        *) echo "Unknown Platform" ;;
    esac
}

# Logging function
log() {
    local level=$1
    shift
    echo "$(date -u +"%Y-%m-%d %H:%M:%S UTC") [$level] $*" | tee -a "$LOG_FILE"
}

# Error handling
error_exit() {
    log "ERROR" "$1"
    exit 1
}

# Create backup before sync
create_backup() {
    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local backup_path="$BACKUP_DIR/backup_$timestamp"

    log "INFO" "Creating backup at $backup_path"
    mkdir -p "$backup_path"

    # Backup master file
    if [[ -f "$MASTER_FILE" ]]; then
        cp "$MASTER_FILE" "$backup_path/"
    fi

    # Backup platform files
    for platform in "${PLATFORMS[@]}"; do
        local file=$(get_platform_file "$platform")
        if [[ -f "$file" ]]; then
            local dir=$(dirname "$file")
            local relative_dir=${dir#$PROJECT_ROOT/}
            mkdir -p "$backup_path/$relative_dir"
            cp "$file" "$backup_path/$relative_dir/"
        fi
    done

    log "INFO" "Backup completed: $backup_path"
}

# Validate master file exists
validate_master() {
    if [[ ! -f "$MASTER_FILE" ]]; then
        error_exit "Master file not found: $MASTER_FILE"
    fi

    if [[ ! -s "$MASTER_FILE" ]]; then
        error_exit "Master file is empty: $MASTER_FILE"
    fi

    log "INFO" "Master file validated: $MASTER_FILE"
}

# Generate platform header
generate_platform_header() {
    local platform=$1
    local platform_display_name=$(get_platform_display_name "$platform")

    cat << EOF
<!-- AUTO-GENERATED FROM: AGENTS.md -->
<!-- Platform: $platform | Generated: $SYNC_TIMESTAMP -->
<!-- DO NOT EDIT DIRECTLY - Changes will be overwritten -->

# AI-CORE $platform_display_name Instructions (FAANG-Enhanced)

**🔄 SYNCHRONIZED FROM MASTER FILE: AGENTS.md**

This file contains the complete AI-CORE project instructions optimized for $platform_display_name with FAANG-level development patterns and intelligent automation.

EOF
}

# Generate platform-specific optimizations
generate_platform_optimization() {
    local platform=$1

    case $platform in
        "github")
            cat << EOF
## 🎯 GitHub Copilot Optimization

As GitHub Copilot working on **AI-CORE**, your primary objective is to help build, maintain, and evolve this intelligent automation platform using proven patterns and FAANG-level engineering excellence with the **Kiro Method**.

### **Copilot-Specific Features**
- **Code Completion**: Leverage AI-powered suggestions for Rust and TypeScript
- **Comment-to-Code**: Generate implementations from detailed comments
- **Test Generation**: Create comprehensive test cases with Playwright
- **Documentation**: Auto-generate inline documentation and README sections

### **GitHub Integration**
- **Pull Request Reviews**: Automated code quality analysis
- **Issue Resolution**: Context-aware bug fix suggestions
- **CI/CD Integration**: Seamless GitHub Actions workflow support
- **Repository Management**: Intelligent project structure recommendations

EOF
            ;;

        "vscode")
            cat << EOF
## 🎯 VS Code Integration Optimization

As VS Code AI working on **AI-CORE**, integrate seamlessly with the development workflow using built-in tools, extensions, and intelligent assistance.

### **VS Code Specific Features**
- **IntelliSense**: Advanced code completion and parameter hints
- **Debugging**: Integrated debugging for Rust and TypeScript
- **Extensions**: Leverage Rust Analyzer, Tauri, and Docker extensions
- **Terminal**: Integrated terminal for all development commands

### **Essential Extensions**
- **GitHub Copilot**: AI-powered code completion
- **Rust Analyzer**: Advanced Rust language support
- **TypeScript and JavaScript**: Enhanced TypeScript development
- **Tauri**: Desktop application development support
- **Docker**: Container development and management
- **PostgreSQL**: Database query and management
- **Error Lens**: Inline error highlighting
- **GitLens**: Advanced Git integration

### **Quality and Testing Extensions**
- **Thunder Client**: API testing and development
- **Test Explorer**: Integrated testing interface
- **Coverage Gutters**: Test coverage visualization

EOF
            ;;

        "zed")
            cat << EOF
## 🎯 Zed Editor Optimization

As Zed Editor AI working on **AI-CORE**, leverage native AI capabilities, performance optimization, and seamless development workflow.

### **Zed-Specific Features**
- **Native Performance**: Optimized for Apple Silicon M1/M2 Macs
- **Real-time Collaboration**: Team development and code sharing
- **AI Integration**: Built-in Claude 3.5 Sonnet and GPT-4 support
- **Language Servers**: Native Rust and TypeScript language support

### **AI Model Configuration**
- **Claude 3.5 Sonnet (latest)**: Primary development assistance
- **Claude 3.5 Sonnet**: Advanced architectural analysis
- **GPT-4**: Code review and optimization

### **Performance Features**
- **Instant Startup**: Sub-second application launch
- **File Indexing**: Real-time project-wide search
- **Memory Efficiency**: Optimized for large codebases
- **Native Git**: Built-in version control integration

EOF
            ;;

        "claude")
            cat << EOF
## 🎯 Claude AI Enhanced Mission

As Claude AI working on **AI-CORE**, leverage your advanced reasoning capabilities, comprehensive context understanding, and superior code generation to provide exceptional assistance for building, maintaining, and evolving this intelligent automation platform.

### **Claude-Specific Capabilities**
- **Advanced Reasoning**: Complex architectural analysis and system design
- **Context Understanding**: Comprehensive project context and dependencies
- **Code Generation**: High-quality Rust and TypeScript implementation
- **Documentation**: Detailed technical documentation and specifications

### **Claude Excellence Patterns**
- **Multi-step Analysis**: Break complex problems into manageable components
- **Code Quality**: Focus on maintainable, scalable, and secure implementations
- **Performance**: Optimize for Rust performance and TypeScript efficiency
- **Testing**: Comprehensive test coverage with Playwright and Rust testing

### **Advanced Development Support**
- **Architecture Design**: System design and microservices architecture
- **Code Review**: Detailed analysis and improvement suggestions
- **Debugging**: Root cause analysis and solution recommendations
- **Optimization**: Performance analysis and enhancement strategies

EOF
            ;;

        "gemini")
            cat << EOF
## 🎯 Google Gemini Integration

As Google Gemini working on **AI-CORE**, utilize multimodal capabilities, Google's advanced AI, and integration with Google Cloud services for comprehensive development assistance.

### **Gemini-Specific Features**
- **Multimodal Analysis**: Process code, documentation, and diagrams
- **Google Integration**: Leverage Google Cloud and Workspace integration
- **Advanced AI**: Next-generation language model capabilities
- **Real-time Processing**: Fast response times for development tasks

### **Google Excellence Integration**
- **SRE Principles**: Apply Google's Site Reliability Engineering practices
- **Scalability**: Design for Google-scale performance and reliability
- **Cloud Native**: Optimize for Google Cloud Platform deployment
- **Security**: Implement Google-level security best practices

### **Multimodal Development**
- **Code + Diagrams**: Analyze both code and architectural diagrams
- **Documentation Images**: Process screenshots and technical drawings
- **UI/UX Analysis**: Visual interface analysis and recommendations
- **Data Visualization**: Charts and metrics interpretation

EOF
            ;;
    esac
}

# Generate complete platform file
generate_platform_content() {
    local platform=$1
    local output_file=$(get_platform_file "$platform")
    local temp_file="${output_file}.tmp"

    # Ensure directory exists
    mkdir -p "$(dirname "$output_file")"

    # Generate complete content
    {
        # Platform header
        generate_platform_header "$platform"

        # Platform-specific optimization
        generate_platform_optimization "$platform"

        # Include complete master content (skip the header comments)
        echo "---"
        echo ""
        echo "# Complete AI-CORE Instructions (Master Content)"
        echo ""
        echo "*The following content is the complete master AGENTS.md file for full context and instructions.*"
        echo ""

        # Skip the first few lines of comments and include the rest
        tail -n +20 "$MASTER_FILE"

        echo ""
        echo "---"
        echo ""
        echo "<!-- Sync Metadata -->"
        echo "<!-- Synced: $SYNC_TIMESTAMP | Source: AGENTS.md | Target: $platform -->"
        echo "<!-- Master File Size: $(wc -l < "$MASTER_FILE") lines | Platform: $platform -->"
        echo "<!-- Sync Version: improved-v2.0 | Complete Content: YES -->"

    } > "$temp_file"

    # Atomic move to prevent partial writes
    mv "$temp_file" "$output_file"

    log "INFO" "Generated complete content for $platform: $(wc -l < "$output_file") lines"
}

# Synchronize specific platform
sync_platform() {
    local platform=$1
    log "INFO" "Synchronizing platform: $platform"

    local output_file=$(get_platform_file "$platform")
    if [[ -z "$output_file" ]]; then
        error_exit "Unknown platform: $platform"
    fi

    generate_platform_content "$platform"

    if [[ -f "$output_file" ]]; then
        log "INFO" "Successfully synchronized: $output_file"
    else
        error_exit "Failed to create platform file: $output_file"
    fi
}

# Synchronize all platforms
sync_all() {
    log "INFO" "Starting synchronization of all platforms"

    # Validate master file
    validate_master

    # Create backup
    create_backup

    # Sync each platform
    for platform in "${PLATFORMS[@]}"; do
        sync_platform "$platform"
    done

    # Update metrics
    update_metrics

    log "INFO" "All platforms synchronized successfully"

    # Show summary
    echo ""
    echo "=== SYNCHRONIZATION COMPLETE ==="
    printf "%-10s | %-8s | %-10s\n" "Platform" "Status" "Lines"
    echo "----------------------------------------"
    for platform in "${PLATFORMS[@]}"; do
        local file=$(get_platform_file "$platform")
        local lines=$(wc -l < "$file")
        printf "%-10s | %-8s | %-10d\n" "$platform" "✅" "$lines"
    done
    echo "----------------------------------------"
    echo "Master file: $(wc -l < "$MASTER_FILE") lines"
    echo ""
    echo "All platform files now contain the complete AGENTS.md content!"
}

# Update sync metrics
update_metrics() {
    local metrics_file="$PROJECT_ROOT/.ai-sync-metrics.json"
    local platforms_json=""

    # Build platforms array for JSON
    for platform in "${PLATFORMS[@]}"; do
        if [[ -n "$platforms_json" ]]; then
            platforms_json="$platforms_json,"
        fi
        platforms_json="$platforms_json\"$platform\""
    done

    cat > "$metrics_file" << EOF
{
  "last_sync": "$SYNC_TIMESTAMP",
  "master_file_lines": $(wc -l < "$MASTER_FILE"),
  "platforms_synced": [$platforms_json],
  "sync_version": "improved-v2.0",
  "complete_content": true
}
EOF
}

# Show sync status
show_status() {
    echo "AI Instructions Sync Status - $PROJECT_NAME (Improved)"
    echo "=================================================="
    echo "Master file: $MASTER_FILE ($(wc -l < "$MASTER_FILE") lines)"

    if [[ -f "$PROJECT_ROOT/.ai-sync-metrics.json" ]]; then
        local last_sync=$(grep '"last_sync"' "$PROJECT_ROOT/.ai-sync-metrics.json" | cut -d'"' -f4)
        echo "Last sync: $last_sync"
    else
        echo "Last sync: Never"
    fi

    echo ""
    echo "Platform files:"
    printf "%-10s | %-8s | %-20s | %-10s\n" "Platform" "Status" "Timestamp" "Lines"
    echo "--------------------------------------------------------------"
    for platform in "${PLATFORMS[@]}"; do
        local file=$(get_platform_file "$platform")
        if [[ -f "$file" ]]; then
            local lines=$(wc -l < "$file")
            local timestamp=""
            if grep -q "Generated:" "$file" 2>/dev/null; then
                timestamp=$(grep "Generated:" "$file" | head -1 | grep -o '[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}T[0-9]\{2\}:[0-9]\{2\}:[0-9]\{2\}+[0-9]\{2\}:[0-9]\{2\}')
            fi
            printf "%-10s | %-8s | %-20s | %-10d\n" "$platform" "✅" "$timestamp" "$lines"
        else
            printf "%-10s | %-8s | %-20s | %-10s\n" "$platform" "❌" "Not synchronized" "N/A"
        fi
    done
}

# Validate sync
validate_sync() {
    log "INFO" "Validating synchronization"

    local all_valid=true
    local master_lines=$(wc -l < "$MASTER_FILE")

    for platform in "${PLATFORMS[@]}"; do
        local file=$(get_platform_file "$platform")
        if [[ ! -f "$file" ]]; then
            log "ERROR" "Missing platform file: $file"
            all_valid=false
        elif ! grep -q "SYNCHRONIZED FROM MASTER FILE: AGENTS.md" "$file"; then
            log "ERROR" "Invalid platform file (missing sync header): $file"
            all_valid=false
        else
            local platform_lines=$(wc -l < "$file")
            # Platform files should be significantly larger than master (due to added headers/optimizations)
            if [[ $platform_lines -lt $master_lines ]]; then
                log "WARNING" "Platform file seems incomplete: $file ($platform_lines lines vs master $master_lines lines)"
            else
                log "INFO" "Platform file validated: $file ($platform_lines lines)"
            fi
        fi
    done

    if $all_valid; then
        log "INFO" "All platforms validated successfully"
        return 0
    else
        log "ERROR" "Validation failed"
        return 1
    fi
}

# Main execution
main() {
    local action=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -Action)
                action="$2"
                shift 2
                ;;
            --help|-h)
                echo "Usage: $0 -Action <sync-all|validate|status|init>"
                echo ""
                echo "Actions:"
                echo "  sync-all     - Synchronize all platform files from master AGENTS.md"
                echo "  validate     - Validate all platform files are properly synchronized"
                echo "  status       - Show current synchronization status"
                echo "  init         - Initialize synchronization system"
                echo ""
                echo "This improved version includes the complete AGENTS.md content in all platform files."
                exit 0
                ;;
            *)
                error_exit "Unknown option: $1"
                ;;
        esac
    done

    if [[ -z "$action" ]]; then
        error_exit "Action required. Use --help for usage information."
    fi

    # Create log file if it doesn't exist
    mkdir -p "$(dirname "$LOG_FILE")"
    touch "$LOG_FILE"

    case $action in
        "sync-all")
            sync_all
            ;;
        "validate")
            validate_sync
            ;;
        "status")
            show_status
            ;;
        "init")
            log "INFO" "Initializing AI instructions sync system"
            mkdir -p "$BACKUP_DIR"
            for platform in "${PLATFORMS[@]}"; do
                local file=$(get_platform_file "$platform")
                mkdir -p "$(dirname "$file")"
            done
            log "INFO" "Sync system initialized"
            ;;
        *)
            error_exit "Unknown action: $action"
            ;;
    esac
}

# Run main function
main "$@"
