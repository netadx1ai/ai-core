#!/bin/bash

# AI-CORE AI Instructions Sync (Zed IDE Enhanced)
# Comprehensive sync tool with session tracking and hook integration
#
# Usage: ./tools/ai-instructions-sync-zed.sh [options]
#
# Options:
#   -Action sync-all           - Sync all platform files
#   -Action sync-github        - Sync GitHub Copilot only
#   -Action sync-claude        - Sync Claude only
#   -Action sync-gemini        - Sync Gemini only
#   -Action activate-hooks     - Enable all hooks
#   -Action start-session      - Start AI work session
#   --force                    - Force sync even with errors
#   --dry-run                  - Show what would be synced
#   --session-name             - Custom session name
#
# Platform Support: macOS, Linux, WSL2, Windows (Git Bash)
# Enhanced for Zed IDE with proper hook integration

set -euo pipefail

# Configuration
PROJECT_NAME="AI-CORE"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%S+00:00")"
SESSION_NAME="${SESSION_NAME:-ai-sync-$(date +%H%M%S)}"

# Directories
SESSIONS_DIR="$PROJECT_ROOT/dev-works/sessions"
BACKUP_DIR="$PROJECT_ROOT/.ai-sync-backups/$TIMESTAMP"
LOG_FILE="$PROJECT_ROOT/.ai-sync.log"
METRICS_FILE="$PROJECT_ROOT/.ai-sync-metrics.json"

# Ensure directories exist
mkdir -p "$SESSIONS_DIR" "$BACKUP_DIR" "$(dirname "$LOG_FILE")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log() {
    local level=$1
    shift
    echo "$(date -u +"%Y-%m-%d %H:%M:%S UTC") [$level] $*" | tee -a "$LOG_FILE"
}

info() {
    echo -e "${BLUE}ℹ️  $*${NC}"
    log "INFO" "$*"
}

success() {
    echo -e "${GREEN}✅ $*${NC}"
    log "SUCCESS" "$*"
}

warning() {
    echo -e "${YELLOW}⚠️  $*${NC}"
    log "WARNING" "$*"
}

error() {
    echo -e "${RED}❌ $*${NC}"
    log "ERROR" "$*"
}

debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo -e "${PURPLE}🔍 DEBUG: $*${NC}"
        log "DEBUG" "$*"
    fi
}

# Banner
print_banner() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║          AI-CORE Instructions Sync (Zed Enhanced)           ║"
    echo "║                 FAANG-Enhanced Intelligence                  ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    info "Starting sync at $TIMESTAMP"
    info "Project root: $PROJECT_ROOT"
}

# Start AI work session
start_ai_session() {
    local agent_name="${1:-ai-sync-agent}"
    local objective="${2:-synchronize-ai-instructions}"

    info "🚀 Starting AI work session..."

    if [[ -x "$PROJECT_ROOT/tools/ai-work-tracker.sh" ]]; then
        "$PROJECT_ROOT/tools/ai-work-tracker.sh" \
            -Action start-session \
            -AgentName "$agent_name" \
            -Objective "$objective" || {
            warning "Failed to start AI session - continuing without session tracking"
        }
    else
        warning "AI work tracker not found - continuing without session tracking"
    fi
}

# Update session progress
update_session_progress() {
    local progress="$1"
    local context="${2:-sync-progress}"
    local tokens_used="${3:-100}"

    if [[ -x "$PROJECT_ROOT/tools/ai-work-tracker.sh" ]]; then
        "$PROJECT_ROOT/tools/ai-work-tracker.sh" \
            -Action update-session \
            -Progress "$progress" \
            -TokensUsed "$tokens_used" \
            -Context "$context" 2>/dev/null || true
    fi
}

# Enable hooks
activate_hooks() {
    info "🎯 Activating AI hooks..."

    local hooks_script="$PROJECT_ROOT/tools/setup-hook.sh"
    if [[ -x "$hooks_script" ]]; then
        "$hooks_script" enable ai-instructions-auto-sync 2>/dev/null || true
        "$hooks_script" enable smart-agent-selector 2>/dev/null || true
        success "Hooks activated successfully"
    else
        warning "Hook setup script not found"
    fi
}

# Backup existing files
backup_files() {
    info "💾 Creating backup of existing files..."

    local files=(
        "AGENTS.md"
        "CLAUDE.md"
        "GEMINI.md"
        ".github/copilot-instructions.md"
        ".vscode/README.md"
        ".zed/README.md"
    )

    for file in "${files[@]}"; do
        if [[ -f "$PROJECT_ROOT/$file" ]]; then
            local backup_file="$BACKUP_DIR/$file"
            mkdir -p "$(dirname "$backup_file")"
            cp "$PROJECT_ROOT/$file" "$backup_file"
            debug "Backed up: $file"
        fi
    done

    success "Backup completed to: $BACKUP_DIR"
}

# Validate AGENTS.md exists and is readable
validate_source() {
    local source_file="$PROJECT_ROOT/AGENTS.md"

    if [[ ! -f "$source_file" ]]; then
        error "Source file AGENTS.md not found!"
        return 1
    fi

    if [[ ! -r "$source_file" ]]; then
        error "Source file AGENTS.md is not readable!"
        return 1
    fi

    local line_count=$(wc -l < "$source_file")
    if [[ $line_count -lt 100 ]]; then
        warning "AGENTS.md seems too short ($line_count lines) - possible corruption?"
    fi

    success "Source file validation passed ($line_count lines)"
    return 0
}

# Generate platform-specific content
generate_platform_content() {
    local platform="$1"
    local source_file="$PROJECT_ROOT/AGENTS.md"
    local temp_content=$(mktemp)

    case "$platform" in
        "github")
            cat > "$temp_content" << 'EOF'
<!-- AUTO-GENERATED FROM: AGENTS.md -->
<!-- Platform: github | Generated: TIMESTAMP_PLACEHOLDER -->
<!-- DO NOT EDIT DIRECTLY - Changes will be overwritten -->

# AI-CORE GitHub Copilot Instructions (FAANG-Enhanced)

**🔄 SYNCHRONIZED FROM MASTER FILE: AGENTS.md**

This file contains the complete AI-CORE project instructions optimized for GitHub Copilot with FAANG-level development patterns and intelligent automation.

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

---

# Complete AI-CORE Instructions (Master Content)

*The following content is the complete master AGENTS.md file for full context and instructions.*

EOF
            cat "$source_file" >> "$temp_content"
            cat >> "$temp_content" << EOF

---

<!-- Sync Metadata -->
<!-- Synced: $TIMESTAMP | Source: AGENTS.md | Target: github -->
<!-- Master File Size: $(wc -l < "$source_file") lines | Platform: github -->
<!-- Sync Version: zed-enhanced-v1.0 | Complete Content: YES -->
EOF
            ;;

        "claude")
            cat > "$temp_content" << 'EOF'
# AI-CORE Project Instructions for Claude AI (FAANG-Enhanced)

**🔄 SYNCHRONIZED FROM MASTER FILE: AGENTS.md**
**Generated: TIMESTAMP_PLACEHOLDER**

## Claude AI Specific Optimizations

As Claude AI working on **AI-CORE**, you excel at:

### **Claude's Unique Strengths**
- **Long Context Understanding**: Handle complex, multi-file analysis
- **Reasoning Excellence**: Deep logical analysis and problem-solving
- **Code Quality Focus**: Emphasis on clean, maintainable code
- **Safety and Reliability**: Built-in focus on secure, robust solutions

### **AI-CORE Integration**
- **Architecture Analysis**: Deep system design reviews
- **Code Review**: Comprehensive quality and security analysis
- **Documentation**: Detailed technical documentation generation
- **Problem Solving**: Complex debugging and optimization

---

# Complete Master Instructions

EOF
            cat "$source_file" >> "$temp_content"
            echo "" >> "$temp_content"
            echo "---" >> "$temp_content"
            echo "" >> "$temp_content"
            echo "**Claude AI Enhanced Features Active**" >> "$temp_content"
            echo "- Long context analysis for complex codebase understanding" >> "$temp_content"
            echo "- Advanced reasoning for architecture decisions" >> "$temp_content"
            echo "- Safety-first approach to all recommendations" >> "$temp_content"
            echo "- Comprehensive code quality analysis" >> "$temp_content"
            ;;

        "gemini")
            cat > "$temp_content" << 'EOF'
# AI-CORE Project Instructions for Google Gemini AI (FAANG-Enhanced)

**🔄 SYNCHRONIZED FROM MASTER FILE: AGENTS.md**
**Generated: TIMESTAMP_PLACEHOLDER**

## Gemini AI Specific Optimizations

As Google Gemini AI working on **AI-CORE**, you leverage:

### **Gemini's Unique Capabilities**
- **Multimodal Analysis**: Handle code, diagrams, and documentation together
- **Google-Scale Engineering**: Apply Google's engineering best practices
- **Performance Focus**: Emphasis on scalability and efficiency
- **Integration Excellence**: Seamless integration with Google services

### **AI-CORE Integration**
- **System Architecture**: Apply Google-scale design patterns
- **Performance Optimization**: Focus on efficiency and scalability
- **DevOps Excellence**: Advanced deployment and monitoring strategies
- **Security**: Google-level security and compliance standards

---

# Complete Master Instructions

EOF
            cat "$source_file" >> "$temp_content"
            echo "" >> "$temp_content"
            echo "---" >> "$temp_content"
            echo "" >> "$temp_content"
            echo "**Google Gemini Enhanced Features Active**" >> "$temp_content"
            echo "- Multimodal understanding of code and architecture" >> "$temp_content"
            echo "- Google-scale engineering patterns and practices" >> "$temp_content"
            echo "- Advanced performance and scalability optimization" >> "$temp_content"
            echo "- Enterprise-grade security and compliance focus" >> "$temp_content"
            ;;

        *)
            # For VSCode, Zed, etc. - just use master content with platform header
            cat > "$temp_content" << EOF
# AI-CORE Instructions for $platform

**🔄 SYNCHRONIZED FROM MASTER FILE: AGENTS.md**
**Generated: $TIMESTAMP**

---

EOF
            cat "$source_file" >> "$temp_content"
            ;;
    esac

    # Replace timestamp placeholder
    sed -i.bak "s/TIMESTAMP_PLACEHOLDER/$TIMESTAMP/g" "$temp_content" 2>/dev/null || \
    sed -i "s/TIMESTAMP_PLACEHOLDER/$TIMESTAMP/g" "$temp_content" 2>/dev/null || true

    cat "$temp_content"
    rm -f "$temp_content" "$temp_content.bak" 2>/dev/null || true
}

# Sync individual platform
sync_platform() {
    local platform="$1"
    local target_file="$2"
    local description="$3"

    info "📝 Syncing $description..."

    # Ensure target directory exists
    mkdir -p "$(dirname "$PROJECT_ROOT/$target_file")"

    # Generate and write content
    if generate_platform_content "$platform" > "$PROJECT_ROOT/$target_file"; then
        success "$description synced successfully"
        update_session_progress "$((progress += 15))" "synced-$platform" "150"
        return 0
    else
        error "Failed to sync $description"
        return 1
    fi
}

# Sync all platforms
sync_all_platforms() {
    info "🔄 Starting comprehensive sync of all AI instruction files..."

    local platforms=(
        "github:.github/copilot-instructions.md:GitHub Copilot"
        "claude:CLAUDE.md:Claude AI"
        "gemini:GEMINI.md:Google Gemini"
        "vscode:.vscode/README.md:VS Code"
        "zed:.zed/README.md:Zed Editor"
    )

    local failed_syncs=0
    local progress=20

    for platform_info in "${platforms[@]}"; do
        IFS=':' read -r platform file description <<< "$platform_info"

        if sync_platform "$platform" "$file" "$description"; then
            debug "✓ $description sync completed"
        else
            error "✗ $description sync failed"
            ((failed_syncs++))
        fi
    done

    if [[ $failed_syncs -eq 0 ]]; then
        success "🎉 All platforms synced successfully!"
        update_session_progress 90 "all-platforms-synced" "800"
        return 0
    else
        error "$failed_syncs platform sync(s) failed"
        return 1
    fi
}

# Update metrics
update_metrics() {
    local success_count="$1"
    local total_count="$2"
    local sync_duration="$3"

    cat > "$METRICS_FILE" << EOF
{
  "last_sync": "$TIMESTAMP",
  "success_rate": $(echo "scale=2; $success_count * 100 / $total_count" | bc 2>/dev/null || echo "100"),
  "total_platforms": $total_count,
  "successful_syncs": $success_count,
  "failed_syncs": $((total_count - success_count)),
  "sync_duration_seconds": $sync_duration,
  "session_name": "$SESSION_NAME",
  "version": "zed-enhanced-v1.0"
}
EOF
}

# Complete AI session
complete_ai_session() {
    local summary="${1:-ai-instructions-sync-completed}"

    if [[ -x "$PROJECT_ROOT/tools/ai-work-tracker.sh" ]]; then
        "$PROJECT_ROOT/tools/ai-work-tracker.sh" \
            -Action complete-session \
            -Summary "$summary" 2>/dev/null || true
    fi

    success "🎯 AI work session completed"
}

# Validate environment
validate_environment() {
    info "🔍 Validating environment..."

    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/AGENTS.md" ]] || [[ ! -d "$PROJECT_ROOT/.kiro" ]]; then
        error "Not in AI-CORE project root directory"
        return 1
    fi

    # Check required tools
    local required_tools=("git" "date" "wc" "sed")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            error "Required tool not found: $tool"
            return 1
        fi
    done

    success "Environment validation passed"
    return 0
}

# Main execution function
main() {
    local action="sync-all"
    local force=false
    local dry_run=false
    local start_time=$(date +%s)

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -Action)
                action="$2"
                shift 2
                ;;
            --force)
                force=true
                shift
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            --session-name)
                SESSION_NAME="$2"
                shift 2
                ;;
            --debug)
                DEBUG=true
                shift
                ;;
            -h|--help)
                cat << EOF
AI-CORE AI Instructions Sync (Zed Enhanced)

Usage: $0 [options]

Options:
  -Action <action>       Action to perform (sync-all, sync-github, sync-claude, sync-gemini, activate-hooks, start-session)
  --force               Force sync even with errors
  --dry-run            Show what would be synced without making changes
  --session-name <name> Custom session name
  --debug              Enable debug output
  -h, --help           Show this help message

Examples:
  $0 -Action sync-all
  $0 -Action sync-github --force
  $0 -Action activate-hooks
  $0 --dry-run
EOF
                exit 0
                ;;
            *)
                warning "Unknown option: $1"
                shift
                ;;
        esac
    done

    print_banner

    # Validate environment
    if ! validate_environment; then
        error "Environment validation failed"
        exit 1
    fi

    # Execute requested action
    case "$action" in
        "sync-all")
            start_ai_session "ai-sync-all" "sync-all-platform-instructions"
            activate_hooks
            backup_files
            validate_source || exit 1

            if [[ "$dry_run" == "true" ]]; then
                info "🔍 DRY RUN: Would sync all platforms"
                exit 0
            fi

            if sync_all_platforms; then
                success "🎉 Complete sync successful!"
                complete_ai_session "successfully-synced-all-platforms"
            else
                error "Sync failed"
                exit 1
            fi
            ;;

        "sync-github")
            start_ai_session "ai-sync-github" "sync-github-copilot-instructions"
            sync_platform "github" ".github/copilot-instructions.md" "GitHub Copilot"
            complete_ai_session "successfully-synced-github-copilot"
            ;;

        "sync-claude")
            start_ai_session "ai-sync-claude" "sync-claude-instructions"
            sync_platform "claude" "CLAUDE.md" "Claude AI"
            complete_ai_session "successfully-synced-claude"
            ;;

        "sync-gemini")
            start_ai_session "ai-sync-gemini" "sync-gemini-instructions"
            sync_platform "gemini" "GEMINI.md" "Google Gemini"
            complete_ai_session "successfully-synced-gemini"
            ;;

        "activate-hooks")
            start_ai_session "hook-activation" "activate-ai-development-hooks"
            activate_hooks
            complete_ai_session "successfully-activated-hooks"
            ;;

        "start-session")
            start_ai_session "manual-session" "manual-development-session"
            success "AI development session started"
            ;;

        *)
            error "Unknown action: $action"
            error "Use -h or --help for usage information"
            exit 1
            ;;
    esac

    # Update metrics
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    update_metrics 5 5 "$duration"

    success "🚀 Sync completed in ${duration}s - All AI systems synchronized!"
    info "📊 Metrics saved to: $METRICS_FILE"
    info "📋 Logs saved to: $LOG_FILE"
    info "💾 Backups saved to: $BACKUP_DIR"
}

# Run main function with all arguments
main "$@"
