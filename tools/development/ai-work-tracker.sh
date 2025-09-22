#!/bin/bash

# AI Work Tracker - AI-CORE (FAANG-Enhanced)
# Session management and work tracking for AI agents
#
# Usage: ./tools/ai-work-tracker.sh -Action <action> [options]
#
# Actions:
#   start-session     - Start a new work session
#   update-session    - Update current session progress
#   complete-session  - Complete current session
#   pause-session     - Pause current session
#   resume-session    - Resume paused session
#   generate-report   - Generate progress report
#
# Platform Support: macOS, Linux, WSL2
# Last Updated: 2025-09-09T21:38:49+00:00

set -euo pipefail

# Configuration
PROJECT_NAME="AI-CORE"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SESSIONS_DIR="$PROJECT_ROOT/dev-works/sessions"
METRICS_DIR="$PROJECT_ROOT/dev-works/metrics"
REPORTS_DIR="$PROJECT_ROOT/dev-works/reports"
LOG_FILE="$PROJECT_ROOT/dev-works/logs/ai-work-tracker.log"

# Ensure directories exist
mkdir -p "$SESSIONS_DIR" "$METRICS_DIR" "$REPORTS_DIR"

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

# Generate session ID
generate_session_id() {
    local agent_name=$1
    local timestamp=$(date +"%Y%m%d%H%M%S")
    echo "${timestamp}-${agent_name}-session"
}

# Get current active session
get_active_session() {
    local active_session=$(find "$SESSIONS_DIR" -name "ACTIVE-*.md" | head -n 1)
    if [[ -n "$active_session" ]]; then
        basename "$active_session"
    fi
}

# Start new session
start_session() {
    local agent_name=""
    local objective=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -AgentName)
                agent_name="$2"
                shift 2
                ;;
            -Objective)
                objective="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    if [[ -z "$agent_name" || -z "$objective" ]]; then
        error_exit "Agent name and objective are required for starting session"
    fi

    # Check for existing active session
    local existing_session=$(get_active_session)
    if [[ -n "$existing_session" ]]; then
        log "WARNING" "Active session found: $existing_session"
        log "INFO" "Please complete or pause existing session before starting new one"
        return 1
    fi

    local session_id=$(generate_session_id "$agent_name")
    local session_file="$SESSIONS_DIR/ACTIVE-${session_id}.md"
    local start_time=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

    # Create session file
    cat > "$session_file" << EOF
# AI Work Session - $agent_name

**Session ID**: $session_id
**Agent**: $agent_name
**Objective**: $objective
**Status**: ACTIVE
**Started**: $start_time
**Last Updated**: $start_time

## Session Progress

- **Progress**: 0%
- **Tokens Used**: 0
- **Current Context**: Session initialized

## Work Log

### $start_time - Session Started
- Agent: $agent_name
- Objective: $objective
- Status: ACTIVE

## Tasks Completed

(None yet)

## Challenges Encountered

(None yet)

## Knowledge Gained

(None yet)

## Next Steps

- Begin work on: $objective

## Session Metrics

- **Start Time**: $start_time
- **Duration**: 0 minutes
- **Tokens Used**: 0
- **Files Modified**: 0
- **Commands Executed**: 0

---
*Session managed by AI Work Tracker v1.0*
EOF

    log "INFO" "Started new session: $session_id"
    log "INFO" "Agent: $agent_name"
    log "INFO" "Objective: $objective"
    log "INFO" "Session file: $session_file"

    # Update metrics
    update_session_metrics "session_started" "$agent_name" "$session_id"

    return 0
}

# Update session progress
update_session() {
    local progress=""
    local tokens_used=""
    local context=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -Progress)
                progress="$2"
                shift 2
                ;;
            -TokensUsed)
                tokens_used="$2"
                shift 2
                ;;
            -Context)
                context="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    # Find active session
    local active_session=$(get_active_session)
    if [[ -z "$active_session" ]]; then
        error_exit "No active session found. Start a session first."
    fi

    local session_file="$SESSIONS_DIR/$active_session"
    local update_time=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

    # Update session file
    if [[ -n "$progress" ]]; then
        sed -i.bak "s/- \*\*Progress\*\*: [0-9]*%/- **Progress**: ${progress}%/" "$session_file"
    fi

    if [[ -n "$tokens_used" ]]; then
        sed -i.bak "s/- \*\*Tokens Used\*\*: [0-9]*/- **Tokens Used**: ${tokens_used}/" "$session_file"
    fi

    if [[ -n "$context" ]]; then
        sed -i.bak "s/- \*\*Current Context\*\*: .*/- **Current Context**: ${context}/" "$session_file"
    fi

    # Update last updated timestamp
    sed -i.bak "s/\*\*Last Updated\*\*: .*/\*\*Last Updated\*\*: ${update_time}/" "$session_file"

    # Add work log entry
    local log_entry="### $update_time - Progress Update"
    if [[ -n "$progress" ]]; then
        log_entry="${log_entry}\n- Progress: ${progress}%"
    fi
    if [[ -n "$tokens_used" ]]; then
        log_entry="${log_entry}\n- Tokens Used: ${tokens_used}"
    fi
    if [[ -n "$context" ]]; then
        log_entry="${log_entry}\n- Context: ${context}"
    fi

    # Insert log entry after "## Work Log" line
    sed -i.bak "/## Work Log/a\\
\\
$log_entry\\
" "$session_file"

    # Clean up backup file
    rm -f "${session_file}.bak"

    log "INFO" "Updated active session: $(basename "$active_session")"
    if [[ -n "$progress" ]]; then
        log "INFO" "Progress: ${progress}%"
    fi
    if [[ -n "$tokens_used" ]]; then
        log "INFO" "Tokens Used: ${tokens_used}"
    fi
    if [[ -n "$context" ]]; then
        log "INFO" "Context: ${context}"
    fi

    # Update metrics
    update_session_metrics "session_updated" "${active_session}" "$progress" "$tokens_used"

    return 0
}

# Complete session
complete_session() {
    local summary=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -Summary)
                summary="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    # Find active session
    local active_session=$(get_active_session)
    if [[ -z "$active_session" ]]; then
        error_exit "No active session found."
    fi

    local session_file="$SESSIONS_DIR/$active_session"
    local completion_time=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

    # Calculate session duration
    local start_time=$(grep "**Started**:" "$session_file" | sed 's/.*Started\*\*: //')
    local start_epoch=$(date -j -f "%Y-%m-%d %H:%M:%S %Z" "$start_time" +%s 2>/dev/null || date -d "$start_time" +%s 2>/dev/null || echo 0)
    local end_epoch=$(date +%s)
    local duration_minutes=$(( (end_epoch - start_epoch) / 60 ))

    # Update session file to completed status
    sed -i.bak "s/\*\*Status\*\*: ACTIVE/\*\*Status\*\*: COMPLETED/" "$session_file"
    sed -i.bak "s/\*\*Last Updated\*\*: .*/\*\*Completed\*\*: ${completion_time}/" "$session_file"

    # Add completion summary
    if [[ -n "$summary" ]]; then
        cat >> "$session_file" << EOF

## Session Summary

**Completed**: $completion_time
**Duration**: ${duration_minutes} minutes
**Summary**: $summary

### Final Status
- Session completed successfully
- All objectives addressed
- Knowledge captured and documented

EOF
    fi

    # Add completion log entry
    sed -i.bak "/## Work Log/a\\
\\
### $completion_time - Session Completed\\
- Status: COMPLETED\\
- Duration: ${duration_minutes} minutes\\
$(if [[ -n "$summary" ]]; then echo "- Summary: $summary"; fi)\\
" "$session_file"

    # Clean up backup file
    rm -f "${session_file}.bak"

    # Rename file to COMPLETED status
    local completed_file="${session_file/ACTIVE-/COMPLETED-}"
    mv "$session_file" "$completed_file"

    log "INFO" "Completed session: $(basename "$completed_file")"
    log "INFO" "Duration: ${duration_minutes} minutes"
    if [[ -n "$summary" ]]; then
        log "INFO" "Summary: $summary"
    fi

    # Update metrics
    update_session_metrics "session_completed" "$(basename "$completed_file")" "$duration_minutes" "$summary"

    # Auto-update task status based on completed session
    local session_id=$(basename "$completed_file" .md | sed 's/^COMPLETED-//')
    if [[ -f "$PROJECT_ROOT/tools/task-status-updater.sh" ]]; then
        log "INFO" "Auto-updating task status from session: $session_id"
        if "$PROJECT_ROOT/tools/task-status-updater.sh" -session "$session_id" 2>/dev/null; then
            log "INFO" "Task status updated successfully"
        else
            log "WARNING" "Task status update failed or no tasks found to update"
        fi
    else
        log "WARNING" "Task status updater not found: $PROJECT_ROOT/tools/task-status-updater.sh"
    fi

    return 0
}

# Pause session
pause_session() {
    local reason=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -Reason)
                reason="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    # Find active session
    local active_session=$(get_active_session)
    if [[ -z "$active_session" ]]; then
        error_exit "No active session found."
    fi

    local session_file="$SESSIONS_DIR/$active_session"
    local pause_time=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

    # Update session file to paused status
    sed -i.bak "s/\*\*Status\*\*: ACTIVE/\*\*Status\*\*: PAUSED/" "$session_file"
    sed -i.bak "s/\*\*Last Updated\*\*: .*/\*\*Paused\*\*: ${pause_time}/" "$session_file"

    # Add pause log entry
    local log_entry="### $pause_time - Session Paused"
    if [[ -n "$reason" ]]; then
        log_entry="${log_entry}\n- Reason: ${reason}"
    fi

    sed -i.bak "/## Work Log/a\\
\\
$log_entry\\
" "$session_file"

    # Clean up backup file
    rm -f "${session_file}.bak"

    # Rename file to PAUSED status
    local paused_file="${session_file/ACTIVE-/PAUSED-}"
    mv "$session_file" "$paused_file"

    log "INFO" "Paused session: $(basename "$paused_file")"
    if [[ -n "$reason" ]]; then
        log "INFO" "Reason: $reason"
    fi

    # Update metrics
    update_session_metrics "session_paused" "$(basename "$paused_file")" "$reason"

    return 0
}

# Resume session
resume_session() {
    local session_id=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -SessionId)
                session_id="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    # Find paused session
    local paused_session=""
    if [[ -n "$session_id" ]]; then
        paused_session=$(find "$SESSIONS_DIR" -name "PAUSED-*${session_id}*.md" | head -n 1)
    else
        paused_session=$(find "$SESSIONS_DIR" -name "PAUSED-*.md" | head -n 1)
    fi

    if [[ -z "$paused_session" ]]; then
        error_exit "No paused session found."
    fi

    local resume_time=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

    # Update session file to active status
    sed -i.bak "s/\*\*Status\*\*: PAUSED/\*\*Status\*\*: ACTIVE/" "$paused_session"
    sed -i.bak "s/\*\*Paused\*\*: .*/\*\*Resumed\*\*: ${resume_time}/" "$paused_session"

    # Add resume log entry
    sed -i.bak "/## Work Log/a\\
\\
### $resume_time - Session Resumed\\
- Status: ACTIVE\\
- Ready to continue work\\
" "$paused_session"

    # Clean up backup file
    rm -f "${paused_session}.bak"

    # Rename file to ACTIVE status
    local active_file="${paused_session/PAUSED-/ACTIVE-}"
    mv "$paused_session" "$active_file"

    log "INFO" "Resumed session: $(basename "$active_file")"

    # Update metrics
    update_session_metrics "session_resumed" "$(basename "$active_file")"

    return 0
}

# Generate report
generate_report() {
    local report_type="weekly"
    local period="weekly"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -Type)
                report_type="$2"
                shift 2
                ;;
            -Period)
                period="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    local report_time=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
    local report_file="$REPORTS_DIR/${report_type}-report-$(date +%Y%m%d).md"

    # Count sessions by status
    local total_sessions=$(find "$SESSIONS_DIR" -name "*.md" | wc -l | tr -d ' ')
    local completed_sessions=$(find "$SESSIONS_DIR" -name "COMPLETED-*.md" | wc -l | tr -d ' ')
    local active_sessions=$(find "$SESSIONS_DIR" -name "ACTIVE-*.md" | wc -l | tr -d ' ')
    local paused_sessions=$(find "$SESSIONS_DIR" -name "PAUSED-*.md" | wc -l | tr -d ' ')

    # Generate report
    cat > "$report_file" << EOF
# AI Work Tracking Report - $PROJECT_NAME

**Report Type**: $report_type
**Period**: $period
**Generated**: $report_time

## Session Summary

- **Total Sessions**: $total_sessions
- **Completed**: $completed_sessions
- **Active**: $active_sessions
- **Paused**: $paused_sessions

## Completion Rate

- **Success Rate**: $(( completed_sessions * 100 / (total_sessions == 0 ? 1 : total_sessions) ))%

## Recent Sessions

EOF

    # Add recent sessions to report
    find "$SESSIONS_DIR" -name "*.md" -type f -exec basename {} \; | sort -r | head -10 | while read -r session_file; do
        local status=$(echo "$session_file" | cut -d'-' -f1)
        local session_id=$(echo "$session_file" | sed 's/^[A-Z]*-//' | sed 's/.md$//')
        echo "- **$status**: $session_id" >> "$report_file"
    done

    cat >> "$report_file" << EOF

## Productivity Metrics

- **Average Session Duration**: Calculating...
- **Total Tokens Used**: Calculating...
- **Most Active Agent**: Calculating...

## Recommendations

- Continue current productivity patterns
- Monitor token usage for optimization
- Ensure regular session updates for better tracking

---
*Report generated by AI Work Tracker v1.0*
EOF

    log "INFO" "Generated report: $report_file"
    log "INFO" "Total sessions: $total_sessions"
    log "INFO" "Completion rate: $(( completed_sessions * 100 / (total_sessions == 0 ? 1 : total_sessions) ))%"

    return 0
}

# Update session metrics
update_session_metrics() {
    local action="$1"
    local session_info="$2"
    local additional_data="${3:-}"
    local extra_data="${4:-}"

    local metrics_file="$METRICS_DIR/session-metrics-$(date +%Y%m).json"
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%S.%3NZ")

    # Create metrics file if it doesn't exist
    if [[ ! -f "$metrics_file" ]]; then
        cat > "$metrics_file" << EOF
{
  "version": "1.0",
  "month": "$(date +%Y-%m)",
  "metrics": []
}
EOF
    fi

    # Add new metric entry (simplified JSON append)
    local temp_file=$(mktemp)
    head -n -2 "$metrics_file" > "$temp_file"

    # Add comma if not the first entry
    if grep -q '"metrics": \[' "$metrics_file" && ! grep -q '"metrics": \[\]' "$metrics_file"; then
        echo "," >> "$temp_file"
    fi

    # Add new metric
    cat >> "$temp_file" << EOF
    {
      "timestamp": "$timestamp",
      "action": "$action",
      "session": "$session_info",
      "data": "$additional_data",
      "extra": "$extra_data"
    }
  ]
}
EOF

    mv "$temp_file" "$metrics_file"

    log "DEBUG" "Updated metrics: $action for $session_info"
}

# Main execution
main() {
    local action="${1:-}"

    case "$action" in
        "-Action")
            shift
            local command="${1:-}"
            shift
            case "$command" in
                "start-session")
                    start_session "$@"
                    ;;
                "update-session")
                    update_session "$@"
                    ;;
                "complete-session")
                    complete_session "$@"
                    ;;
                "pause-session")
                    pause_session "$@"
                    ;;
                "resume-session")
                    resume_session "$@"
                    ;;
                "generate-report")
                    generate_report "$@"
                    ;;
                *)
                    echo "Usage: $0 -Action <action> [options]"
                    echo ""
                    echo "Actions:"
                    echo "  start-session     -AgentName <name> -Objective <objective>"
                    echo "  update-session    -Progress <0-100> -TokensUsed <number> -Context <description>"
                    echo "  complete-session  -Summary <summary>"
                    echo "  pause-session     -Reason <reason>"
                    echo "  resume-session    [-SessionId <id>]"
                    echo "  generate-report   [-Type <type>] [-Period <period>]"
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Usage: $0 -Action <action> [options]"
            echo ""
            echo "Actions:"
            echo "  start-session     -AgentName <name> -Objective <objective>"
            echo "  update-session    -Progress <0-100> -TokensUsed <number> -Context <description>"
            echo "  complete-session  -Summary <summary>"
            echo "  pause-session     -Reason <reason>"
            echo "  resume-session    [-SessionId <id>]"
            echo "  generate-report   [-Type <type>] [-Period <period>]"
            exit 1
            ;;
    esac
}

# Execute main function with all arguments
main "$@"
