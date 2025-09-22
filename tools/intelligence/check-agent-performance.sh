#!/bin/bash

# AI-CORE Agent Performance Analysis Tool
# Description: Analyzes agent performance metrics for intelligent selection decisions
# Version: 1.0
# Created: 2025-01-10

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
LOG_FILE="$PROJECT_ROOT/dev-works/logs/agent-performance.log"
METRICS_DIR="$PROJECT_ROOT/dev-works/metrics"
SESSIONS_DIR="$PROJECT_ROOT/dev-works/sessions"
AGENTS_DIR="$PROJECT_ROOT/dev-works/dev-agents"

# Ensure required directories exist
mkdir -p "$(dirname "$LOG_FILE")"
mkdir -p "$METRICS_DIR"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
    echo -e "$1"
}

# Error handling
error_exit() {
    log "${RED}ERROR: $1${NC}"
    exit 1
}

# Usage information
show_usage() {
    cat << EOF
${CYAN}AI-CORE Agent Performance Analysis Tool${NC}

${YELLOW}USAGE:${NC}
    $0 [OPTIONS]

${YELLOW}OPTIONS:${NC}
    -a, --agent AGENT_NAME      Analyze specific agent performance
    -t, --timeframe DAYS        Analysis timeframe in days [default: 7]
    -m, --metric METRIC         Focus on specific metric (success_rate|efficiency|quality)
    -f, --format FORMAT         Output format (json|yaml|text) [default: text]
    -v, --verbose               Enable verbose output
    -r, --refresh               Force refresh of cached metrics
    -h, --help                  Show this help message

${YELLOW}EXAMPLES:${NC}
    $0                          # Analyze all agents (last 7 days)
    $0 -a backend-agent         # Analyze specific agent
    $0 -t 30 -f json           # 30-day analysis in JSON format
    $0 -m success_rate -v       # Focus on success rates with verbose output

${YELLOW}AVAILABLE AGENTS:${NC}
    architect-agent, backend-agent, frontend-agent, database-agent,
    devops-agent, qa-agent, security-agent, integration-agent,
    pm-agent, coordinator-agent, hooks-agent, spec-agent, steering-agent
EOF
}

# Initialize default values
TARGET_AGENT=""
TIMEFRAME_DAYS=7
FOCUS_METRIC=""
OUTPUT_FORMAT="text"
VERBOSE=false
REFRESH_CACHE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -a|--agent)
            TARGET_AGENT="$2"
            shift 2
            ;;
        -t|--timeframe)
            TIMEFRAME_DAYS="$2"
            shift 2
            ;;
        -m|--metric)
            FOCUS_METRIC="$2"
            shift 2
            ;;
        -f|--format)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -r|--refresh)
            REFRESH_CACHE=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            error_exit "Unknown option: $1"
            ;;
    esac
done

# Available agents list
AVAILABLE_AGENTS=(
    "architect-agent"
    "backend-agent"
    "frontend-agent"
    "database-agent"
    "devops-agent"
    "qa-agent"
    "security-agent"
    "integration-agent"
    "pm-agent"
    "coordinator-agent"
    "hooks-agent"
    "spec-agent"
    "steering-agent"
)

# Validate agent name if provided
if [[ -n "$TARGET_AGENT" ]]; then
    if [[ ! " ${AVAILABLE_AGENTS[@]} " =~ " ${TARGET_AGENT} " ]]; then
        error_exit "Invalid agent name: $TARGET_AGENT. Available agents: ${AVAILABLE_AGENTS[*]}"
    fi
fi

# Performance metrics calculation
calculate_success_rate() {
    local agent="$1"
    local days="$2"

    local total_sessions=0
    local successful_sessions=0
    local cutoff_date
    cutoff_date=$(date -d "$days days ago" +%Y%m%d)

    if [[ -d "$SESSIONS_DIR" ]]; then
        while IFS= read -r -d '' session_file; do
            local session_date
            session_date=$(basename "$session_file" | grep -o '[0-9]\{8\}' | head -1 || echo "00000000")

            if [[ "$session_date" -ge "$cutoff_date" ]]; then
                if grep -q "$agent" "$session_file" 2>/dev/null; then
                    ((total_sessions++))
                    if [[ "$session_file" == *"COMPLETED"* ]] || grep -q "✅" "$session_file" 2>/dev/null; then
                        ((successful_sessions++))
                    fi
                fi
            fi
        done < <(find "$SESSIONS_DIR" -name "*.md" -type f -print0 2>/dev/null || true)
    fi

    if [[ $total_sessions -gt 0 ]]; then
        echo "scale=2; $successful_sessions * 100 / $total_sessions" | bc -l 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

calculate_efficiency_score() {
    local agent="$1"
    local days="$2"

    local total_time=0
    local completed_tasks=0
    local cutoff_date
    cutoff_date=$(date -d "$days days ago" +%Y%m%d)

    if [[ -d "$SESSIONS_DIR" ]]; then
        while IFS= read -r -d '' session_file; do
            local session_date
            session_date=$(basename "$session_file" | grep -o '[0-9]\{8\}' | head -1 || echo "00000000")

            if [[ "$session_date" -ge "$cutoff_date" ]] && grep -q "$agent" "$session_file" 2>/dev/null; then
                if [[ "$session_file" == *"COMPLETED"* ]]; then
                    ((completed_tasks++))
                    # Extract time information from session if available
                    local duration
                    duration=$(grep -o "Duration: [0-9]\+ minutes" "$session_file" | grep -o "[0-9]\+" || echo "60")
                    total_time=$((total_time + duration))
                fi
            fi
        done < <(find "$SESSIONS_DIR" -name "*.md" -type f -print0 2>/dev/null || true)
    fi

    if [[ $completed_tasks -gt 0 && $total_time -gt 0 ]]; then
        # Calculate tasks per hour
        echo "scale=2; $completed_tasks * 60 / $total_time" | bc -l 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

calculate_quality_score() {
    local agent="$1"
    local days="$2"

    local total_quality_points=0
    local quality_entries=0
    local cutoff_date
    cutoff_date=$(date -d "$days days ago" +%Y%m%d)

    if [[ -d "$SESSIONS_DIR" ]]; then
        while IFS= read -r -d '' session_file; do
            local session_date
            session_date=$(basename "$session_file" | grep -o '[0-9]\{8\}' | head -1 || echo "00000000")

            if [[ "$session_date" -ge "$cutoff_date" ]] && grep -q "$agent" "$session_file" 2>/dev/null; then
                # Look for quality indicators
                local quality_score=50 # Base score

                # Positive indicators
                if grep -q "✅" "$session_file"; then
                    quality_score=$((quality_score + 20))
                fi
                if grep -q "optimization\|improvement\|enhancement" "$session_file"; then
                    quality_score=$((quality_score + 10))
                fi
                if grep -q "test.*pass\|all.*test.*pass" "$session_file"; then
                    quality_score=$((quality_score + 15))
                fi

                # Negative indicators
                if grep -q "❌\|error\|failed\|bug" "$session_file"; then
                    quality_score=$((quality_score - 15))
                fi
                if grep -q "timeout\|stuck\|blocked" "$session_file"; then
                    quality_score=$((quality_score - 10))
                fi

                # Ensure score stays within bounds
                if [[ $quality_score -gt 100 ]]; then quality_score=100; fi
                if [[ $quality_score -lt 0 ]]; then quality_score=0; fi

                total_quality_points=$((total_quality_points + quality_score))
                ((quality_entries++))
            fi
        done < <(find "$SESSIONS_DIR" -name "*.md" -type f -print0 2>/dev/null || true)
    fi

    if [[ $quality_entries -gt 0 ]]; then
        echo "scale=2; $total_quality_points / $quality_entries" | bc -l 2>/dev/null || echo "50"
    else
        echo "50"
    fi
}

get_recent_activity() {
    local agent="$1"
    local days="$2"

    local activity_count=0
    local cutoff_date
    cutoff_date=$(date -d "$days days ago" +%Y%m%d)

    if [[ -d "$SESSIONS_DIR" ]]; then
        while IFS= read -r -d '' session_file; do
            local session_date
            session_date=$(basename "$session_file" | grep -o '[0-9]\{8\}' | head -1 || echo "00000000")

            if [[ "$session_date" -ge "$cutoff_date" ]] && grep -q "$agent" "$session_file" 2>/dev/null; then
                ((activity_count++))
            fi
        done < <(find "$SESSIONS_DIR" -name "*.md" -type f -print0 2>/dev/null || true)
    fi

    echo $activity_count
}

get_workload_status() {
    local agent="$1"

    local active_sessions=0
    if [[ -d "$SESSIONS_DIR" ]]; then
        active_sessions=$(find "$SESSIONS_DIR" -name "ACTIVE-*.md" -type f | xargs grep -l "$agent" 2>/dev/null | wc -l)
    fi

    if [[ $active_sessions -eq 0 ]]; then
        echo "AVAILABLE"
    elif [[ $active_sessions -eq 1 ]]; then
        echo "LIGHT"
    elif [[ $active_sessions -le 3 ]]; then
        echo "MODERATE"
    else
        echo "HEAVY"
    fi
}

analyze_agent_performance() {
    local agent="$1"
    local days="$2"

    local success_rate
    success_rate=$(calculate_success_rate "$agent" "$days")

    local efficiency_score
    efficiency_score=$(calculate_efficiency_score "$agent" "$days")

    local quality_score
    quality_score=$(calculate_quality_score "$agent" "$days")

    local recent_activity
    recent_activity=$(get_recent_activity "$agent" "$days")

    local workload_status
    workload_status=$(get_workload_status "$agent")

    # Calculate overall performance score
    local overall_score
    overall_score=$(echo "scale=2; ($success_rate * 0.4) + ($quality_score * 0.4) + ($efficiency_score * 20 * 0.2)" | bc -l 2>/dev/null || echo "0")

    echo "$agent:$success_rate:$efficiency_score:$quality_score:$recent_activity:$workload_status:$overall_score"
}

analyze_all_agents() {
    local days="$1"
    local results=()

    log "${BLUE}Analyzing performance for all agents (last $days days)...${NC}"

    for agent in "${AVAILABLE_AGENTS[@]}"; do
        if [[ "$VERBOSE" == true ]]; then
            log "${YELLOW}Analyzing $agent...${NC}"
        fi

        local result
        result=$(analyze_agent_performance "$agent" "$days")
        results+=("$result")
    done

    echo "${results[@]}"
}

format_performance_output() {
    local format="$1"
    shift
    local results=("$@")

    case "$format" in
        "json")
            format_json_output "${results[@]}"
            ;;
        "yaml")
            format_yaml_output "${results[@]}"
            ;;
        *)
            format_text_output "${results[@]}"
            ;;
    esac
}

format_text_output() {
    local results=("$@")

    cat << EOF

${CYAN}🎯 AGENT PERFORMANCE ANALYSIS RESULTS${NC}
${BLUE}═══════════════════════════════════════${NC}

${YELLOW}📊 ANALYSIS PARAMETERS:${NC}
  • Timeframe: Last $TIMEFRAME_DAYS days
  • Analysis Date: $(date)
  • Total Agents Analyzed: ${#results[@]}

${YELLOW}📈 PERFORMANCE RANKINGS:${NC}

EOF

    # Sort by overall score (descending)
    local sorted_results=()
    while IFS= read -r line; do
        sorted_results+=("$line")
    done < <(printf '%s\n' "${results[@]}" | sort -t: -k7 -nr)

    local rank=1
    for result in "${sorted_results[@]}"; do
        IFS=':' read -r agent success_rate efficiency quality activity workload overall <<< "$result"

        local performance_badge=""
        if (( $(echo "$overall >= 80" | bc -l) )); then
            performance_badge="${GREEN}🏆 EXCELLENT${NC}"
        elif (( $(echo "$overall >= 60" | bc -l) )); then
            performance_badge="${YELLOW}⭐ GOOD${NC}"
        elif (( $(echo "$overall >= 40" | bc -l) )); then
            performance_badge="${BLUE}📈 AVERAGE${NC}"
        else
            performance_badge="${RED}⚠️  NEEDS ATTENTION${NC}"
        fi

        local workload_indicator=""
        case "$workload" in
            "AVAILABLE") workload_indicator="${GREEN}🟢 Available${NC}" ;;
            "LIGHT") workload_indicator="${YELLOW}🟡 Light Load${NC}" ;;
            "MODERATE") workload_indicator="${BLUE}🔵 Moderate Load${NC}" ;;
            "HEAVY") workload_indicator="${RED}🔴 Heavy Load${NC}" ;;
        esac

        cat << EOF
${PURPLE}#$rank${NC} ${CYAN}$agent${NC} - $performance_badge
┌─ Overall Score: ${overall}%
├─ Success Rate: ${success_rate}%
├─ Efficiency: ${efficiency} tasks/hour
├─ Quality Score: ${quality}%
├─ Recent Activity: $activity sessions
└─ Current Workload: $workload_indicator

EOF
        ((rank++))
    done

    # Performance insights
    cat << EOF
${YELLOW}💡 KEY INSIGHTS:${NC}

EOF

    # Top performer
    local top_performer
    top_performer=$(echo "${sorted_results[0]}" | cut -d: -f1)
    cat << EOF
  🥇 Top Performer: $top_performer

EOF

    # Available agents
    local available_agents=()
    for result in "${results[@]}"; do
        IFS=':' read -r agent _ _ _ _ workload _ <<< "$result"
        if [[ "$workload" == "AVAILABLE" ]]; then
            available_agents+=("$agent")
        fi
    done

    if [[ ${#available_agents[@]} -gt 0 ]]; then
        cat << EOF
  ✅ Available Agents: ${available_agents[*]}

EOF
    fi

    # Performance alerts
    local low_performers=()
    for result in "${results[@]}"; do
        IFS=':' read -r agent _ _ _ _ _ overall <<< "$result"
        if (( $(echo "$overall < 40" | bc -l) )); then
            low_performers+=("$agent")
        fi
    done

    if [[ ${#low_performers[@]} -gt 0 ]]; then
        cat << EOF
  ⚠️ Agents Needing Attention: ${low_performers[*]}

EOF
    fi
}

format_json_output() {
    local results=("$@")

    echo "{"
    echo "  \"analysis_timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"timeframe_days\": $TIMEFRAME_DAYS,"
    echo "  \"agents\": ["

    local first=true
    for result in "${results[@]}"; do
        IFS=':' read -r agent success_rate efficiency quality activity workload overall <<< "$result"

        if [[ "$first" == true ]]; then
            first=false
        else
            echo ","
        fi

        cat << EOF
    {
      "name": "$agent",
      "performance": {
        "overall_score": $overall,
        "success_rate": $success_rate,
        "efficiency_score": $efficiency,
        "quality_score": $quality
      },
      "activity": {
        "recent_sessions": $activity,
        "workload_status": "$workload"
      }
    }
EOF
    done

    echo ""
    echo "  ]"
    echo "}"
}

format_yaml_output() {
    local results=("$@")

    cat << EOF
analysis_timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)
timeframe_days: $TIMEFRAME_DAYS
agents:
EOF

    for result in "${results[@]}"; do
        IFS=':' read -r agent success_rate efficiency quality activity workload overall <<< "$result"

        cat << EOF
  - name: $agent
    performance:
      overall_score: $overall
      success_rate: $success_rate
      efficiency_score: $efficiency
      quality_score: $quality
    activity:
      recent_sessions: $activity
      workload_status: $workload
EOF
    done
}

# Cache performance data
cache_performance_data() {
    local results=("$@")
    local cache_file="$METRICS_DIR/agent-performance-$(date +%Y%m%d).cache"

    {
        echo "# Agent Performance Cache - $(date)"
        echo "# Format: agent:success_rate:efficiency:quality:activity:workload:overall"
        for result in "${results[@]}"; do
            echo "$result"
        done
    } > "$cache_file"

    if [[ "$VERBOSE" == true ]]; then
        log "${BLUE}Performance data cached to: $cache_file${NC}"
    fi
}

# Main execution
main() {
    if [[ "$VERBOSE" == true ]]; then
        log "${BLUE}Agent Performance Analysis Tool Starting...${NC}"
        log "${BLUE}Target Agent: ${TARGET_AGENT:-"All Agents"}${NC}"
        log "${BLUE}Timeframe: $TIMEFRAME_DAYS days${NC}"
        log "${BLUE}Output Format: $OUTPUT_FORMAT${NC}"
    fi

    local results=()

    if [[ -n "$TARGET_AGENT" ]]; then
        log "${BLUE}Analyzing performance for $TARGET_AGENT...${NC}"
        local result
        result=$(analyze_agent_performance "$TARGET_AGENT" "$TIMEFRAME_DAYS")
        results=("$result")
    else
        mapfile -t results < <(analyze_all_agents "$TIMEFRAME_DAYS")
    fi

    format_performance_output "$OUTPUT_FORMAT" "${results[@]}"

    # Cache results for future reference
    cache_performance_data "${results[@]}"

    if [[ "$VERBOSE" == true ]]; then
        log "${GREEN}Analysis completed successfully${NC}"
    fi
}

# Execute main function
main "$@"
