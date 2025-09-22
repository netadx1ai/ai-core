#!/bin/bash

# AI-CORE Project Context Analysis Tool
# Description: Reviews current project phase, priorities, system health, and resource availability
# Version: 1.0
# Created: 2025-01-17

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="$PROJECT_ROOT/dev-works/logs/project-context-analysis.log"
CACHE_DIR="$PROJECT_ROOT/.cache/context-analysis"

# Ensure required directories exist
mkdir -p "$(dirname "$LOG_FILE")"
mkdir -p "$CACHE_DIR"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        echo -e "$1"
    fi
}

# Error handling
error_exit() {
    echo -e "${RED}ERROR: $1${NC}" >&2
    log "ERROR: $1"
    exit 1
}

# Usage information
show_usage() {
    cat << EOF
${CYAN}AI-CORE Project Context Analysis Tool${NC}

${YELLOW}USAGE:${NC}
    $0 [OPTIONS]

${YELLOW}OPTIONS:${NC}
    -v, --verbose               Enable verbose output
    -o, --output FORMAT         Output format (json|yaml|text) [default: json]
    -s, --system-health         Include detailed system health check
    -p, --performance           Include performance metrics
    -r, --resources             Include resource availability analysis
    -a, --all                   Include all analysis types
    -h, --help                  Show this help message

${YELLOW}EXAMPLES:${NC}
    $0 --all --output json
    $0 --system-health --verbose
    $0 --performance --resources

${YELLOW}OUTPUT:${NC}
    Project phase, priorities, system health, resource availability, recent issues
EOF
}

# Initialize variables
VERBOSE=false
OUTPUT_FORMAT="json"
INCLUDE_SYSTEM_HEALTH=false
INCLUDE_PERFORMANCE=false
INCLUDE_RESOURCES=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -s|--system-health)
            INCLUDE_SYSTEM_HEALTH=true
            shift
            ;;
        -p|--performance)
            INCLUDE_PERFORMANCE=true
            shift
            ;;
        -r|--resources)
            INCLUDE_RESOURCES=true
            shift
            ;;
        -a|--all)
            INCLUDE_SYSTEM_HEALTH=true
            INCLUDE_PERFORMANCE=true
            INCLUDE_RESOURCES=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            error_exit "Unknown option: $1"
            ;;
        *)
            error_exit "Unexpected argument: $1"
            ;;
    esac
done

# Get current timestamp in UTC
get_timestamp() {
    date -u '+%Y-%m-%dT%H:%M:%S+00:00'
}

# Analyze project phase
analyze_project_phase() {
    local phase="unknown"
    local phase_confidence=0
    local indicators=()

    # Check for MVP indicators
    if [[ -f "$PROJECT_ROOT/AGENTS.md" ]] && grep -q "MVP Phase" "$PROJECT_ROOT/AGENTS.md" 2>/dev/null; then
        phase="mvp"
        phase_confidence=90
        indicators+=("AGENTS.md indicates MVP phase")
    fi

    # Check development activity
    if [[ -d "$PROJECT_ROOT/dev-works/sessions" ]]; then
        local recent_sessions=$(find "$PROJECT_ROOT/dev-works/sessions" -name "*.md" -mtime -7 2>/dev/null | wc -l)
        if (( recent_sessions > 0 )); then
            indicators+=("$recent_sessions active sessions in past week")
            phase_confidence=$((phase_confidence + 10))
        fi
    fi

    # Check build configuration
    if [[ -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        indicators+=("Rust project with Cargo configuration")
        phase_confidence=$((phase_confidence + 5))
    fi

    # Check frontend setup
    if [[ -f "$PROJECT_ROOT/src/ui/package.json" ]] || [[ -f "$PROJECT_ROOT/package.json" ]]; then
        indicators+=("Frontend build configuration present")
        phase_confidence=$((phase_confidence + 5))
    fi

    # Default to development if no clear indicators
    if [[ "$phase" == "unknown" ]]; then
        phase="development"
        phase_confidence=70
        indicators+=("Default development phase assumed")
    fi

    echo "$phase:$phase_confidence:$(IFS=';'; echo "${indicators[*]}")"
}

# Get project priorities
get_project_priorities() {
    local priorities=()
    local priority_scores=()

    # Check AGENTS.md for priorities
    if [[ -f "$PROJECT_ROOT/AGENTS.md" ]]; then
        if grep -q "CRITICAL RULES" "$PROJECT_ROOT/AGENTS.md" 2>/dev/null; then
            priorities+=("quality_gates")
            priority_scores+=("95")
        fi
        if grep -q "Session Tracking" "$PROJECT_ROOT/AGENTS.md" 2>/dev/null; then
            priorities+=("session_management")
            priority_scores+=("90")
        fi
        if grep -q "Hook System" "$PROJECT_ROOT/AGENTS.md" 2>/dev/null; then
            priorities+=("automation_hooks")
            priority_scores+=("85")
        fi
    fi

    # Check recent activity patterns
    if [[ -d "$PROJECT_ROOT/.kiro/specs" ]]; then
        local spec_count=$(find "$PROJECT_ROOT/.kiro/specs" -name "*.md" 2>/dev/null | wc -l)
        if (( spec_count > 0 )); then
            priorities+=("feature_development")
            priority_scores+=("80")
        fi
    fi

    # Check for testing focus
    if [[ -d "$PROJECT_ROOT/tests" ]] && [[ $(find "$PROJECT_ROOT/tests" -name "*.rs" 2>/dev/null | wc -l) -gt 0 ]]; then
        priorities+=("testing_quality")
        priority_scores+=("75")
    fi

    # Default priorities if none found
    if [[ ${#priorities[@]} -eq 0 ]]; then
        priorities=("general_development" "code_quality" "documentation")
        priority_scores=("70" "65" "60")
    fi

    # Format output
    local result=""
    for i in "${!priorities[@]}"; do
        if [[ -n "$result" ]]; then
            result+=","
        fi
        result+="${priorities[$i]}:${priority_scores[$i]}"
    done
    echo "$result"
}

# Check system health
check_system_health() {
    local health_status="healthy"
    local health_score=100
    local issues=()
    local warnings=()

    # Check if in project root
    if [[ ! -f "$PROJECT_ROOT/AGENTS.md" ]]; then
        health_status="degraded"
        health_score=$((health_score - 20))
        issues+=("Not in AI-CORE project root")
    fi

    # Check critical directories
    local critical_dirs=(".kiro" "dev-works" "tools" "src")
    for dir in "${critical_dirs[@]}"; do
        if [[ ! -d "$PROJECT_ROOT/$dir" ]]; then
            health_status="degraded"
            health_score=$((health_score - 15))
            issues+=("Missing critical directory: $dir")
        fi
    done

    # Check Rust build health
    if [[ -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        if ! command -v cargo >/dev/null 2>&1; then
            health_status="degraded"
            health_score=$((health_score - 25))
            issues+=("Cargo not available")
        else
            # Try a quick check (don't actually build)
            if ! cargo check --quiet --message-format=json >/dev/null 2>&1; then
                health_status="degraded"
                health_score=$((health_score - 20))
                warnings+=("Rust project may have compilation issues")
            fi
        fi
    fi

    # Check frontend health
    if [[ -f "$PROJECT_ROOT/src/ui/package.json" ]]; then
        if [[ ! -d "$PROJECT_ROOT/src/ui/node_modules" ]]; then
            warnings+=("Frontend dependencies may need installation")
            health_score=$((health_score - 5))
        fi
    fi

    # Check log directory health
    if [[ -d "$PROJECT_ROOT/dev-works/logs" ]]; then
        local old_logs=$(find "$PROJECT_ROOT/dev-works/logs" -name "*.log" -mtime +30 2>/dev/null | wc -l)
        if (( old_logs > 10 )); then
            warnings+=("$old_logs old log files may need cleanup")
        fi
    fi

    # Determine final health status
    if (( health_score < 50 )); then
        health_status="critical"
    elif (( health_score < 80 )); then
        health_status="degraded"
    fi

    echo "$health_status:$health_score:$(IFS=';'; echo "${issues[*]}"):$(IFS=';'; echo "${warnings[*]}")"
}

# Get performance metrics
get_performance_metrics() {
    local build_time="unknown"
    local test_time="unknown"
    local project_size="unknown"
    local complexity_score=0

    # Calculate project size
    if command -v find >/dev/null 2>&1; then
        local file_count=$(find "$PROJECT_ROOT" -type f -name "*.rs" -o -name "*.ts" -o -name "*.js" 2>/dev/null | wc -l)
        local total_lines=0

        if command -v wc >/dev/null 2>&1; then
            while IFS= read -r file; do
                if [[ -f "$file" ]]; then
                    local lines=$(wc -l < "$file" 2>/dev/null || echo "0")
                    total_lines=$((total_lines + lines))
                fi
            done < <(find "$PROJECT_ROOT" -type f -name "*.rs" -o -name "*.ts" -o -name "*.js" 2>/dev/null)
        fi

        project_size="${file_count}_files_${total_lines}_lines"

        # Simple complexity scoring
        complexity_score=$(( (file_count * 2) + (total_lines / 100) ))
    fi

    # Check recent build performance if build logs exist
    if [[ -f "$PROJECT_ROOT/dev-works/logs/build.log" ]]; then
        local recent_build=$(grep "finished in" "$PROJECT_ROOT/dev-works/logs/build.log" 2>/dev/null | tail -1)
        if [[ -n "$recent_build" ]]; then
            build_time=$(echo "$recent_build" | grep -o '[0-9.]*s' | head -1)
        fi
    fi

    # Estimate based on project size if no metrics available
    if [[ "$build_time" == "unknown" ]] && [[ "$project_size" != "unknown" ]]; then
        local file_count=$(echo "$project_size" | cut -d'_' -f1)
        if (( file_count > 100 )); then
            build_time="slow_estimated"
        elif (( file_count > 50 )); then
            build_time="medium_estimated"
        else
            build_time="fast_estimated"
        fi
    fi

    echo "$build_time:$test_time:$project_size:$complexity_score"
}

# Check resource availability
check_resource_availability() {
    local disk_usage="unknown"
    local memory_status="unknown"
    local cpu_load="unknown"
    local network_status="available"

    # Check disk usage
    if command -v df >/dev/null 2>&1; then
        local disk_info=$(df "$PROJECT_ROOT" 2>/dev/null | tail -1)
        if [[ -n "$disk_info" ]]; then
            local usage_percent=$(echo "$disk_info" | awk '{print $5}' | sed 's/%//')
            if [[ "$usage_percent" =~ ^[0-9]+$ ]]; then
                if (( usage_percent > 90 )); then
                    disk_usage="critical_${usage_percent}%"
                elif (( usage_percent > 80 )); then
                    disk_usage="high_${usage_percent}%"
                else
                    disk_usage="normal_${usage_percent}%"
                fi
            fi
        fi
    fi

    # Check memory (simplified)
    if command -v free >/dev/null 2>&1; then
        local mem_info=$(free 2>/dev/null | grep "Mem:")
        if [[ -n "$mem_info" ]]; then
            local mem_used=$(echo "$mem_info" | awk '{printf "%.0f", ($3/$2)*100}')
            if (( mem_used > 90 )); then
                memory_status="high_${mem_used}%"
            else
                memory_status="normal_${mem_used}%"
            fi
        fi
    elif command -v vm_stat >/dev/null 2>&1; then
        # macOS memory check
        memory_status="available_macos"
    fi

    # Check CPU load (simplified)
    if command -v uptime >/dev/null 2>&1; then
        local load_avg=$(uptime 2>/dev/null | grep -o 'load average[s]*: [0-9.]*' | grep -o '[0-9.]*$')
        if [[ -n "$load_avg" ]] && [[ "$load_avg" =~ ^[0-9.]+$ ]]; then
            if (( $(echo "$load_avg > 2.0" | bc -l 2>/dev/null || echo 0) )); then
                cpu_load="high_${load_avg}"
            else
                cpu_load="normal_${load_avg}"
            fi
        fi
    fi

    echo "$disk_usage:$memory_status:$cpu_load:$network_status"
}

# Get recent issues and blockers
get_recent_issues() {
    local issues=()
    local blockers=()

    # Check for recent error logs
    if [[ -d "$PROJECT_ROOT/dev-works/logs" ]]; then
        local error_logs=$(find "$PROJECT_ROOT/dev-works/logs" -name "*.log" -mtime -7 2>/dev/null)
        for log_file in $error_logs; do
            if [[ -f "$log_file" ]]; then
                local error_count=$(grep -c "ERROR\|CRITICAL\|FATAL" "$log_file" 2>/dev/null || echo "0")
                if (( error_count > 0 )); then
                    issues+=("$(basename "$log_file"): $error_count errors in past week")
                fi
            fi
        done
    fi

    # Check for failed sessions
    if [[ -d "$PROJECT_ROOT/dev-works/sessions" ]]; then
        local failed_sessions=$(find "$PROJECT_ROOT/dev-works/sessions" -name "FAILED-*.md" -mtime -7 2>/dev/null | wc -l)
        if (( failed_sessions > 0 )); then
            blockers+=("$failed_sessions failed sessions in past week")
        fi
    fi

    # Check git status for potential issues
    if [[ -d "$PROJECT_ROOT/.git" ]] && command -v git >/dev/null 2>&1; then
        cd "$PROJECT_ROOT"
        local unstaged_changes=$(git status --porcelain 2>/dev/null | wc -l)
        if (( unstaged_changes > 10 )); then
            issues+=("$unstaged_changes unstaged changes may indicate work in progress")
        fi
        cd - >/dev/null
    fi

    # Default if no issues found
    if [[ ${#issues[@]} -eq 0 ]] && [[ ${#blockers[@]} -eq 0 ]]; then
        issues+=("No recent issues detected")
    fi

    echo "$(IFS=';'; echo "${issues[*]}"):$(IFS=';'; echo "${blockers[*]}")"
}

# Generate JSON output
generate_json_output() {
    local timestamp=$(get_timestamp)
    local phase_info=$(analyze_project_phase)
    local priorities=$(get_project_priorities)
    local issues_info=$(get_recent_issues)

    local phase=$(echo "$phase_info" | cut -d':' -f1)
    local phase_confidence=$(echo "$phase_info" | cut -d':' -f2)
    local phase_indicators=$(echo "$phase_info" | cut -d':' -f3 | tr ';' '\n')

    local recent_issues=$(echo "$issues_info" | cut -d':' -f1 | tr ';' '\n')
    local blockers=$(echo "$issues_info" | cut -d':' -f2 | tr ';' '\n')

    cat << EOF
{
    "timestamp": "$timestamp",
    "project_context": {
        "current_phase": "$phase",
        "phase_confidence": $phase_confidence,
        "phase_indicators": [$(format_json_array "$phase_indicators")],
        "priorities": [$(format_priorities_json "$priorities")],
        "project_root": "$PROJECT_ROOT"
    },
EOF

    if [[ "$INCLUDE_SYSTEM_HEALTH" == "true" ]]; then
        local health_info=$(check_system_health)
        local health_status=$(echo "$health_info" | cut -d':' -f1)
        local health_score=$(echo "$health_info" | cut -d':' -f2)
        local health_issues=$(echo "$health_info" | cut -d':' -f3 | tr ';' '\n')
        local health_warnings=$(echo "$health_info" | cut -d':' -f4 | tr ';' '\n')

        cat << EOF
    "system_health": {
        "status": "$health_status",
        "score": $health_score,
        "issues": [$(format_json_array "$health_issues")],
        "warnings": [$(format_json_array "$health_warnings")]
    },
EOF
    fi

    if [[ "$INCLUDE_PERFORMANCE" == "true" ]]; then
        local perf_info=$(get_performance_metrics)
        local build_time=$(echo "$perf_info" | cut -d':' -f1)
        local test_time=$(echo "$perf_info" | cut -d':' -f2)
        local project_size=$(echo "$perf_info" | cut -d':' -f3)
        local complexity_score=$(echo "$perf_info" | cut -d':' -f4)

        cat << EOF
    "performance_metrics": {
        "build_time": "$build_time",
        "test_time": "$test_time",
        "project_size": "$project_size",
        "complexity_score": $complexity_score
    },
EOF
    fi

    if [[ "$INCLUDE_RESOURCES" == "true" ]]; then
        local resource_info=$(check_resource_availability)
        local disk_usage=$(echo "$resource_info" | cut -d':' -f1)
        local memory_status=$(echo "$resource_info" | cut -d':' -f2)
        local cpu_load=$(echo "$resource_info" | cut -d':' -f3)
        local network_status=$(echo "$resource_info" | cut -d':' -f4)

        cat << EOF
    "resource_availability": {
        "disk_usage": "$disk_usage",
        "memory_status": "$memory_status",
        "cpu_load": "$cpu_load",
        "network_status": "$network_status"
    },
EOF
    fi

    cat << EOF
    "recent_activity": {
        "issues": [$(format_json_array "$recent_issues")],
        "blockers": [$(format_json_array "$blockers")]
    },
    "recommendations": {
        "focus_areas": [$(get_focus_areas)],
        "next_actions": [$(get_next_actions)]
    }
}
EOF
}

# Helper functions for JSON formatting
format_json_array() {
    local items="$1"
    local result=""
    local first=true

    while IFS= read -r item; do
        if [[ -n "$item" ]]; then
            if [[ $first == true ]]; then
                first=false
            else
                result+=", "
            fi
            result+="\"$(echo "$item" | sed 's/"/\\"/g')\""
        fi
    done <<< "$items"

    echo "$result"
}

format_priorities_json() {
    local priorities="$1"
    local result=""
    local first=true

    IFS=',' read -ra PRIORITY_ARRAY <<< "$priorities"
    for priority_info in "${PRIORITY_ARRAY[@]}"; do
        if [[ $first == true ]]; then
            first=false
        else
            result+=", "
        fi
        local name=$(echo "$priority_info" | cut -d':' -f1)
        local score=$(echo "$priority_info" | cut -d':' -f2)
        result+="{\"name\": \"$name\", \"score\": $score}"
    done

    echo "$result"
}

get_focus_areas() {
    local focus=()

    # Always include quality as focus area
    focus+=("code_quality")

    # Add based on project context
    if [[ -f "$PROJECT_ROOT/AGENTS.md" ]] && grep -q "Hook System" "$PROJECT_ROOT/AGENTS.md" 2>/dev/null; then
        focus+=("automation_enhancement")
    fi

    if [[ -d "$PROJECT_ROOT/.kiro/specs" ]] && [[ $(find "$PROJECT_ROOT/.kiro/specs" -name "*.md" 2>/dev/null | wc -l) -gt 0 ]]; then
        focus+=("feature_completion")
    fi

    focus+=("documentation_updates")

    local result=""
    local first=true
    for area in "${focus[@]}"; do
        if [[ $first == true ]]; then
            first=false
        else
            result+=", "
        fi
        result+="\"$area\""
    done

    echo "$result"
}

get_next_actions() {
    local actions=()

    # Default next actions
    actions+=("Review current session status")
    actions+=("Check system health and resolve any issues")
    actions+=("Update documentation if needed")

    # Context-specific actions
    if [[ ! -d "$PROJECT_ROOT/dev-works/sessions" ]] || [[ $(find "$PROJECT_ROOT/dev-works/sessions" -name "ACTIVE-*.md" 2>/dev/null | wc -l) -eq 0 ]]; then
        actions+=("Start new development session")
    fi

    local result=""
    local first=true
    for action in "${actions[@]}"; do
        if [[ $first == true ]]; then
            first=false
        else
            result+=", "
        fi
        result+="\"$action\""
    done

    echo "$result"
}

# Generate text output
generate_text_output() {
    echo -e "${CYAN}🎯 AI-CORE Project Context Analysis${NC}"
    echo ""

    local phase_info=$(analyze_project_phase)
    local phase=$(echo "$phase_info" | cut -d':' -f1)
    local phase_confidence=$(echo "$phase_info" | cut -d':' -f2)

    echo -e "${GREEN}📋 Project Status:${NC}"
    echo "  Current Phase: $phase (confidence: $phase_confidence%)"
    echo "  Project Root: $PROJECT_ROOT"
    echo ""

    local priorities=$(get_project_priorities)
    echo -e "${YELLOW}🎯 Current Priorities:${NC}"
    IFS=',' read -ra PRIORITY_ARRAY <<< "$priorities"
    for priority_info in "${PRIORITY_ARRAY[@]}"; do
        local name=$(echo "$priority_info" | cut -d':' -f1)
        local score=$(echo "$priority_info" | cut -d':' -f2)
        echo "  $name (score: $score)"
    done
    echo ""

    if [[ "$INCLUDE_SYSTEM_HEALTH" == "true" ]]; then
        local health_info=$(check_system_health)
        local health_status=$(echo "$health_info" | cut -d':' -f1)
        local health_score=$(echo "$health_info" | cut -d':' -f2)

        echo -e "${BLUE}🔍 System Health:${NC}"
        echo "  Status: $health_status"
        echo "  Score: $health_score/100"
        echo ""
    fi

    local issues_info=$(get_recent_issues)
    local recent_issues=$(echo "$issues_info" | cut -d':' -f1)

    echo -e "${PURPLE}⚠️ Recent Activity:${NC}"
    if [[ -n "$recent_issues" ]]; then
        echo "$recent_issues" | tr ';' '\n' | sed 's/^/  /'
    else
        echo "  No recent issues detected"
    fi
    echo ""

    echo -e "${CYAN}💡 Recommendations:${NC}"
    echo "  - Focus on code quality and testing"
    echo "  - Keep documentation updated"
    echo "  - Monitor system health regularly"
    echo "  - Maintain session tracking discipline"
}

# Main execution
main() {
    log "Starting project context analysis"

    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${CYAN}🔍 AI-CORE Project Context Analysis${NC}"
        echo ""
    fi

    case "$OUTPUT_FORMAT" in
        "json")
            generate_json_output
            ;;
        "text"|*)
            generate_text_output
            ;;
    esac

    log "Project context analysis completed"
}

# Execute main function
main "$@"
