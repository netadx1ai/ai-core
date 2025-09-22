#!/bin/bash

# AI-CORE Hook Setup & Management (Zed IDE Enhanced)
# Activate and manage intelligent automation hooks
#
# Usage: ./tools/setup-hook.sh <command> [hook-name]
#
# Commands:
#   enable <hook>     - Enable specific hook
#   disable <hook>    - Disable specific hook
#   enable-all        - Enable all hooks
#   disable-all       - Disable all hooks
#   list             - List all hooks and their status
#   status           - Show hook system status
#   validate         - Validate hook configurations
#
# Platform Support: macOS, Linux, WSL2, Windows (Git Bash)

set -euo pipefail

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.kiro/hooks"
LOG_FILE="$PROJECT_ROOT/dev-works/logs/hook-setup.log"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%S+00:00")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Logging functions
log() {
    echo "$(date -u +"%Y-%m-%d %H:%M:%S UTC") [$1] ${*:2}" | tee -a "$LOG_FILE"
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

banner() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║              AI-CORE Hook Management System                  ║"
    echo "║                  FAANG-Enhanced Automation                   ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Validate environment
validate_environment() {
    if [[ ! -d "$HOOKS_DIR" ]]; then
        error "Hooks directory not found: $HOOKS_DIR"
        return 1
    fi

    if [[ ! -f "$PROJECT_ROOT/AGENTS.md" ]]; then
        error "Not in AI-CORE project root"
        return 1
    fi

    return 0
}

# Get all available hooks
get_available_hooks() {
    find "$HOOKS_DIR" -name "*.kiro.hook" -type f | while read -r hook_file; do
        basename "$hook_file" .kiro.hook
    done | sort
}

# Check if hook is enabled
is_hook_enabled() {
    local hook_name="$1"
    local hook_file="$HOOKS_DIR/$hook_name.kiro.hook"

    if [[ ! -f "$hook_file" ]]; then
        return 1
    fi

    # Check if enabled field is true in JSON
    if command -v jq >/dev/null 2>&1; then
        jq -r '.enabled' "$hook_file" 2>/dev/null | grep -q "true"
    else
        # Fallback without jq
        grep -q '"enabled"[[:space:]]*:[[:space:]]*true' "$hook_file"
    fi
}

# Enable hook
enable_hook() {
    local hook_name="$1"
    local hook_file="$HOOKS_DIR/$hook_name.kiro.hook"

    if [[ ! -f "$hook_file" ]]; then
        error "Hook not found: $hook_name"
        return 1
    fi

    info "Enabling hook: $hook_name"

    if command -v jq >/dev/null 2>&1; then
        # Use jq for proper JSON modification
        local temp_file=$(mktemp)
        jq '.enabled = true' "$hook_file" > "$temp_file" && mv "$temp_file" "$hook_file"
    else
        # Fallback without jq
        sed -i.bak 's/"enabled"[[:space:]]*:[[:space:]]*false/"enabled": true/g' "$hook_file" 2>/dev/null || \
        sed -i 's/"enabled"[[:space:]]*:[[:space:]]*false/"enabled": true/g' "$hook_file" 2>/dev/null || true
        rm -f "$hook_file.bak" 2>/dev/null || true
    fi

    if is_hook_enabled "$hook_name"; then
        success "Hook enabled: $hook_name"

        # Start AI session for hook activation
        if [[ -x "$PROJECT_ROOT/tools/ai-work-tracker.sh" ]]; then
            "$PROJECT_ROOT/tools/ai-work-tracker.sh" \
                -Action start-session \
                -AgentName "hook-manager" \
                -Objective "activated-$hook_name-hook" 2>/dev/null || true
        fi

        return 0
    else
        error "Failed to enable hook: $hook_name"
        return 1
    fi
}

# Disable hook
disable_hook() {
    local hook_name="$1"
    local hook_file="$HOOKS_DIR/$hook_name.kiro.hook"

    if [[ ! -f "$hook_file" ]]; then
        error "Hook not found: $hook_name"
        return 1
    fi

    info "Disabling hook: $hook_name"

    if command -v jq >/dev/null 2>&1; then
        local temp_file=$(mktemp)
        jq '.enabled = false' "$hook_file" > "$temp_file" && mv "$temp_file" "$hook_file"
    else
        sed -i.bak 's/"enabled"[[:space:]]*:[[:space:]]*true/"enabled": false/g' "$hook_file" 2>/dev/null || \
        sed -i 's/"enabled"[[:space:]]*:[[:space:]]*true/"enabled": false/g' "$hook_file" 2>/dev/null || true
        rm -f "$hook_file.bak" 2>/dev/null || true
    fi

    success "Hook disabled: $hook_name"
}

# Enable all hooks
enable_all_hooks() {
    info "🚀 Enabling all hooks..."

    local enabled_count=0
    local failed_count=0

    while IFS= read -r hook_name; do
        if enable_hook "$hook_name"; then
            ((enabled_count++))
        else
            ((failed_count++))
        fi
    done < <(get_available_hooks)

    if [[ $failed_count -eq 0 ]]; then
        success "🎉 All $enabled_count hooks enabled successfully!"

        # Update AI session
        if [[ -x "$PROJECT_ROOT/tools/ai-work-tracker.sh" ]]; then
            "$PROJECT_ROOT/tools/ai-work-tracker.sh" \
                -Action update-session \
                -Progress 100 \
                -TokensUsed 200 \
                -Context "all-hooks-activated" 2>/dev/null || true
        fi

        return 0
    else
        error "$failed_count hook(s) failed to enable"
        return 1
    fi
}

# Disable all hooks
disable_all_hooks() {
    info "Disabling all hooks..."

    while IFS= read -r hook_name; do
        disable_hook "$hook_name"
    done < <(get_available_hooks)

    success "All hooks disabled"
}

# List all hooks
list_hooks() {
    info "📋 Available hooks:"
    echo ""

    local total=0
    local enabled=0

    while IFS= read -r hook_name; do
        ((total++))
        if is_hook_enabled "$hook_name"; then
            echo -e "  ${GREEN}✓${NC} $hook_name ${GREEN}(enabled)${NC}"
            ((enabled++))
        else
            echo -e "  ${RED}✗${NC} $hook_name ${RED}(disabled)${NC}"
        fi
    done < <(get_available_hooks)

    echo ""
    info "Status: $enabled/$total hooks enabled"
}

# Show system status
show_status() {
    info "🔍 Hook System Status:"
    echo ""

    local total=0
    local enabled=0
    local broken=0

    while IFS= read -r hook_name; do
        ((total++))
        local hook_file="$HOOKS_DIR/$hook_name.kiro.hook"

        if [[ ! -f "$hook_file" ]]; then
            ((broken++))
            continue
        fi

        if is_hook_enabled "$hook_name"; then
            ((enabled++))
        fi
    done < <(get_available_hooks)

    echo "  📊 Total hooks: $total"
    echo "  ✅ Enabled: $enabled"
    echo "  ❌ Disabled: $((total - enabled))"
    echo "  🔧 Broken: $broken"
    echo ""

    if [[ $enabled -gt 0 ]]; then
        success "Hook system is active with $enabled enabled hook(s)"
    else
        warning "No hooks are currently enabled"
    fi
}

# Validate hook configurations
validate_hooks() {
    info "🔍 Validating hook configurations..."

    local valid=0
    local invalid=0

    while IFS= read -r hook_name; do
        local hook_file="$HOOKS_DIR/$hook_name.kiro.hook"

        if [[ ! -f "$hook_file" ]]; then
            error "Hook file missing: $hook_name"
            ((invalid++))
            continue
        fi

        # Basic JSON validation
        if command -v jq >/dev/null 2>&1; then
            if jq empty "$hook_file" 2>/dev/null; then
                success "✓ $hook_name - Valid JSON"
                ((valid++))
            else
                error "✗ $hook_name - Invalid JSON"
                ((invalid++))
            fi
        else
            # Simple validation without jq
            if grep -q '"enabled"' "$hook_file" && grep -q '"name"' "$hook_file"; then
                success "✓ $hook_name - Basic structure valid"
                ((valid++))
            else
                error "✗ $hook_name - Missing required fields"
                ((invalid++))
            fi
        fi
    done < <(get_available_hooks)

    echo ""
    if [[ $invalid -eq 0 ]]; then
        success "🎉 All $valid hook(s) are valid"
        return 0
    else
        error "$invalid hook(s) have validation errors"
        return 1
    fi
}

# Main function
main() {
    banner

    if ! validate_environment; then
        exit 1
    fi

    local command="${1:-help}"
    local hook_name="${2:-}"

    case "$command" in
        "enable")
            if [[ -z "$hook_name" ]]; then
                error "Hook name required for enable command"
                exit 1
            fi
            enable_hook "$hook_name"
            ;;

        "disable")
            if [[ -z "$hook_name" ]]; then
                error "Hook name required for disable command"
                exit 1
            fi
            disable_hook "$hook_name"
            ;;

        "enable-all")
            enable_all_hooks
            ;;

        "disable-all")
            disable_all_hooks
            ;;

        "list")
            list_hooks
            ;;

        "status")
            show_status
            ;;

        "validate")
            validate_hooks
            ;;

        "help"|"-h"|"--help")
            echo "AI-CORE Hook Setup & Management"
            echo ""
            echo "Usage: $0 <command> [hook-name]"
            echo ""
            echo "Commands:"
            echo "  enable <hook>     Enable specific hook"
            echo "  disable <hook>    Disable specific hook"
            echo "  enable-all        Enable all hooks"
            echo "  disable-all       Disable all hooks"
            echo "  list             List all hooks and status"
            echo "  status           Show hook system status"
            echo "  validate         Validate hook configurations"
            echo "  help             Show this help"
            echo ""
            echo "Examples:"
            echo "  $0 enable-all"
            echo "  $0 enable ai-instructions-auto-sync"
            echo "  $0 list"
            echo "  $0 status"
            ;;

        *)
            error "Unknown command: $command"
            echo "Use '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Execute main function
main "$@"
