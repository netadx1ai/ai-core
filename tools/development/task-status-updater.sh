#!/bin/bash

# Task Status Updater - AI-CORE
# Automatically updates task status in tasks.md based on completed work
#
# Usage: ./tools/task-status-updater.sh [options]
#
# Options:
#   -task <task-id>          - Mark specific task as completed
#   -subtask <description>   - Mark subtask as completed
#   -session <session-id>    - Update tasks from session file
#   -auto                    - Auto-detect completed tasks from recent work
#   -list                    - List all available tasks
#   -status                  - Show current completion status
#
# Examples:
#   ./tools/task-status-updater.sh -task "MVP-001"
#   ./tools/task-status-updater.sh -subtask "Docker development environment"
#   ./tools/task-status-updater.sh -session "20250912034623-mvp-demo-runner-session"
#   ./tools/task-status-updater.sh -auto
#
# Last Updated: 2025-09-11T21:03:25+00:00

set -euo pipefail

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Dynamically find tasks.md in any spec folder
TASKS_FILE=$(find "$PROJECT_ROOT/.kiro/specs" -name "tasks.md" -type f | head -n 1)
if [[ -z "$TASKS_FILE" ]]; then
    TASKS_FILE="$PROJECT_ROOT/.kiro/specs/EARLY-LAUNCH/tasks.md"  # Fallback
fi
SESSIONS_DIR="$PROJECT_ROOT/dev-works/sessions"
LOG_FILE="$PROJECT_ROOT/dev-works/logs/task-status-updater.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Ensure log directory exists
mkdir -p "$(dirname "$LOG_FILE")"

# Logging function
log() {
    local level=$1
    shift
    echo "$(date -u +"%Y-%m-%d %H:%M:%S UTC") [$level] $*" | tee -a "$LOG_FILE"
}

# Output functions
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

# Error handling
error_exit() {
    error "$1"
    exit 1
}

# Check if tasks.md exists and find it dynamically
check_tasks_file() {
    # Re-scan for tasks.md in case it moved
    local found_tasks=$(find "$PROJECT_ROOT/.kiro/specs" -name "tasks.md" -type f | head -n 1)
    if [[ -n "$found_tasks" ]]; then
        TASKS_FILE="$found_tasks"
        info "Using tasks file: $TASKS_FILE"
    elif [[ ! -f "$TASKS_FILE" ]]; then
        error_exit "Tasks file not found in any spec folder: $PROJECT_ROOT/.kiro/specs/*/tasks.md"
    fi
}

# Backup tasks.md before modification
backup_tasks_file() {
    local backup_file="${TASKS_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
    cp "$TASKS_FILE" "$backup_file"
    info "Created backup: $(basename "$backup_file")"
}

# Mark task as completed by task ID
mark_task_completed() {
    local task_id="$1"

    info "Marking task $task_id as completed..."

    # Find and update the task
    if grep -q "#### Task $task_id:" "$TASKS_FILE"; then
        # Create temporary file for updates
        local temp_file=$(mktemp)

        # Process the file to mark subtasks as completed
        awk -v task_id="$task_id" '
        BEGIN { in_target_task = 0 }

        # Detect start of target task
        /^#### Task/ {
            if ($3 == task_id ":") {
                in_target_task = 1
                print
                next
            } else {
                in_target_task = 0
            }
        }

        # Detect end of target task (next task or major section)
        /^#### Task/ && in_target_task && $3 != task_id ":" {
            in_target_task = 0
        }
        /^###/ && in_target_task {
            in_target_task = 0
        }

        # Update checkboxes in target task
        in_target_task && /^- \[ \]/ {
            gsub(/^- \[ \]/, "- [x]")
        }

        # Print all lines
        { print }
        ' "$TASKS_FILE" > "$temp_file"

        # Replace original file
        mv "$temp_file" "$TASKS_FILE"
        success "Task $task_id marked as completed"

        # Show updated status
        show_task_status "$task_id"
    else
        error "Task $task_id not found in tasks.md"
        return 1
    fi
}

# Mark subtask as completed by description
mark_subtask_completed() {
    local description="$1"

    info "Marking subtask '$description' as completed..."

    # Create temporary file for updates
    local temp_file=$(mktemp)

    # Update specific subtask
    awk -v desc="$description" '
    {
        # Match checkbox with description (case insensitive)
        if (/^- \[ \]/ && tolower($0) ~ tolower(desc)) {
            gsub(/^- \[ \]/, "- [x]")
        }
        print
    }
    ' "$TASKS_FILE" > "$temp_file"

    # Check if any changes were made
    if ! cmp -s "$TASKS_FILE" "$temp_file"; then
        mv "$temp_file" "$TASKS_FILE"
        success "Subtask '$description' marked as completed"
    else
        rm "$temp_file"
        warning "Subtask '$description' not found or already completed"
        return 1
    fi
}

# Update tasks from session file
update_from_session() {
    local session_id="$1"
    local session_file=""

    # Find session file
    if [[ -f "$SESSIONS_DIR/ACTIVE-${session_id}.md" ]]; then
        session_file="$SESSIONS_DIR/ACTIVE-${session_id}.md"
    elif [[ -f "$SESSIONS_DIR/COMPLETED-${session_id}.md" ]]; then
        session_file="$SESSIONS_DIR/COMPLETED-${session_id}.md"
    elif [[ -f "$SESSIONS_DIR/${session_id}.md" ]]; then
        session_file="$SESSIONS_DIR/${session_id}.md"
    else
        error_exit "Session file not found: $session_id"
    fi

    info "Analyzing session: $(basename "$session_file")"

    # Extract completed tasks and subtasks from session
    local completed_tasks=()
    local completed_subtasks=()

    # Look for task completion patterns in session file
    while IFS= read -r line; do
        # Match patterns like "Completed: MVP-001" or "✅ MVP-002"
        if [[ "$line" =~ (Completed|✅|DONE).*(MVP|PA|AL|BE|PR)-[0-9]{3} ]]; then
            local task_id=$(echo "$line" | grep -o -E '(MVP|PA|AL|BE|PR)-[0-9]{3}')
            if [[ -n "$task_id" ]]; then
                completed_tasks+=("$task_id")
            fi
        fi

        # Match subtask completions - look for common patterns
        if [[ "$line" =~ ✅.*(Docker|API|demo|documentation|test|build|deploy) ]] || [[ "$line" =~ (completed|finished|done).*(setup|config|integration) ]]; then
            local subtask=$(echo "$line" | sed 's/.*✅[[:space:]]*\([^:]*\).*/\1/' | sed 's/.*completed[[:space:]]*\([^:]*\).*/\1/' | xargs)
            if [[ -n "$subtask" && ${#subtask} -gt 3 ]]; then
                completed_subtasks+=("$subtask")
            fi
        fi
    done < "$session_file"

    # Update tasks
    local updates=0
    if [[ ${#completed_tasks[@]} -gt 0 ]]; then
        for task_id in "${completed_tasks[@]}"; do
            if mark_task_completed "$task_id"; then
                ((updates++))
            fi
        done
    fi

    # Update subtasks
    if [[ ${#completed_subtasks[@]} -gt 0 ]]; then
        for subtask in "${completed_subtasks[@]}"; do
            if mark_subtask_completed "$subtask"; then
                ((updates++))
            fi
        done
    fi

    if [[ $updates -gt 0 ]]; then
        success "Updated $updates task(s) from session $session_id"
    else
        info "No task updates found in session $session_id"
    fi
}

# Auto-detect completed tasks from recent work
auto_detect_completed() {
    info "Auto-detecting completed tasks from recent work..."

    local updates=0

    # Check recent session files (last 24 hours)
    local recent_sessions=$(find "$SESSIONS_DIR" -name "*.md" -mtime -1 2>/dev/null || true)

    if [[ -z "$recent_sessions" ]]; then
        info "No recent session files found"
        return 0
    fi

    # Process each recent session
    while IFS= read -r session_file; do
        local session_id=$(basename "$session_file" .md | sed 's/^ACTIVE-\|^COMPLETED-//')
        info "Checking session: $session_id"

        if update_from_session "$session_id"; then
            ((updates++))
        fi
    done <<< "$recent_sessions"

    # Check for completed files in dev-works
    local demo_files=$(find "$PROJECT_ROOT/dev-works/demos" -name "*.md" -mtime -1 2>/dev/null || true)
    local summary_files=$(find "$PROJECT_ROOT/dev-works/summaries" -name "*.md" -mtime -1 2>/dev/null || true)

    # If demo files exist, mark demo-related tasks as completed
    if [[ -n "$demo_files" ]]; then
        info "Found recent demo files, marking demo tasks as completed"
        mark_subtask_completed "demo" && ((updates++))
    fi

    # If summary files exist, mark documentation tasks as completed
    if [[ -n "$summary_files" ]]; then
        info "Found recent summary files, marking documentation tasks as completed"
        mark_subtask_completed "documentation" && ((updates++))
    fi

    success "Auto-detection completed. Updated $updates task(s)"
}

# List all available tasks
list_tasks() {
    info "Available tasks in tasks.md:"
    echo

    grep -n "#### Task" "$TASKS_FILE" | while IFS=: read -r line_num line_content; do
        local task_id=$(echo "$line_content" | grep -o 'MVP-[0-9]\{3\}\|PA-[0-9]\{3\}\|AL-[0-9]\{3\}\|BE-[0-9]\{3\}\|PR-[0-9]\{3\}')
        local task_title=$(echo "$line_content" | sed 's/#### Task [^:]*: //')
        echo -e "${BLUE}$task_id${NC}: $task_title"
    done

    echo
    info "Use -task <task-id> to mark a task as completed"
}

# Show task completion status
show_task_status() {
    local task_id="${1:-}"

    if [[ -n "$task_id" ]]; then
        info "Status for task $task_id:"
        echo

        # Show specific task status
        awk -v task_id="$task_id" '
        BEGIN { in_target_task = 0; total = 0; completed = 0 }

        /^#### Task/ {
            if ($3 == task_id ":") {
                in_target_task = 1
                print "\033[1;34m" $0 "\033[0m"
                next
            } else {
                in_target_task = 0
            }
        }

        /^#### Task/ && in_target_task && $3 != task_id ":" {
            in_target_task = 0
        }
        /^###/ && in_target_task {
            in_target_task = 0
        }

        in_target_task && /^- \[/ {
            total++
            if (/^- \[x\]/ || /^- ✅/) {
                completed++
                print "\033[0;32m" $0 "\033[0m"
            } else {
                print "\033[0;31m" $0 "\033[0m"
            }
        }

        END {
            if (total > 0) {
                percentage = int((completed * 100) / total)
                printf "\n\033[1;33mCompletion: %d/%d (%d%%)\033[0m\n", completed, total, percentage
            }
        }
        ' "$TASKS_FILE"
    else
        info "Overall task completion status:"
        echo

        # Show overall status
        awk '
        BEGIN {
            current_task = ""
            task_total = 0
            task_completed = 0
            overall_total = 0
            overall_completed = 0
        }

        /^#### Task/ {
            # Print previous task summary
            if (current_task != "" && task_total > 0) {
                percentage = int((task_completed * 100) / task_total)
                printf "%-8s: %2d/%2d (%3d%%) ", current_task, task_completed, task_total, percentage
                if (percentage == 100) print "\033[0;32m✅ COMPLETE\033[0m"
                else if (percentage >= 75) print "\033[0;33m🚧 NEARLY DONE\033[0m"
                else if (percentage >= 25) print "\033[0;34m⚡ IN PROGRESS\033[0m"
                else print "\033[0;31m🔴 NOT STARTED\033[0m"
            }

            # Reset for new task
            current_task = ""
            if (match($0, /MVP-[0-9]{3}|PA-[0-9]{3}|AL-[0-9]{3}|BE-[0-9]{3}|PR-[0-9]{3}/)) {
                current_task = substr($0, RSTART, RLENGTH)
            }
            task_total = 0
            task_completed = 0
        }

        current_task != "" && (/^- \[ \]/ || /^- \[x\]/ || /^- ✅/) {
            task_total++
            overall_total++
            if (/^- \[x\]/ || /^- ✅/) {
                task_completed++
                overall_completed++
            }
        }

        END {
            # Print last task
            if (current_task != "" && task_total > 0) {
                percentage = int((task_completed * 100) / task_total)
                printf "%-8s: %2d/%2d (%3d%%) ", current_task, task_completed, task_total, percentage
                if (percentage == 100) print "\033[0;32m✅ COMPLETE\033[0m"
                else if (percentage >= 75) print "\033[0;33m🚧 NEARLY DONE\033[0m"
                else if (percentage >= 25) print "\033[0;34m⚡ IN PROGRESS\033[0m"
                else print "\033[0;31m🔴 NOT STARTED\033[0m"
            }

            # Print overall summary
            if (overall_total > 0) {
                overall_percentage = int((overall_completed * 100) / overall_total)
                printf "\n\033[1;36mOVERALL: %d/%d (%d%%) Complete\033[0m\n", overall_completed, overall_total, overall_percentage
            }
        }
        ' "$TASKS_FILE"
    fi
}

# Main function
main() {
    # Check prerequisites
    check_tasks_file

    # Parse arguments
    case "${1:-}" in
        -task)
            [[ -z "${2:-}" ]] && error_exit "Task ID required. Usage: -task <task-id>"
            backup_tasks_file
            mark_task_completed "$2"
            ;;
        -subtask)
            [[ -z "${2:-}" ]] && error_exit "Subtask description required. Usage: -subtask <description>"
            backup_tasks_file
            mark_subtask_completed "$2"
            ;;
        -session)
            [[ -z "${2:-}" ]] && error_exit "Session ID required. Usage: -session <session-id>"
            backup_tasks_file
            update_from_session "$2"
            ;;
        -auto)
            backup_tasks_file
            auto_detect_completed
            ;;
        -list)
            list_tasks
            ;;
        -status)
            show_task_status "${2:-}"
            ;;
        -h|--help|help)
            echo "Task Status Updater - AI-CORE"
            echo
            echo "Usage: $0 [options]"
            echo
            echo "Options:"
            echo "  -task <task-id>          Mark specific task as completed"
            echo "  -subtask <description>   Mark subtask as completed"
            echo "  -session <session-id>    Update tasks from session file"
            echo "  -auto                    Auto-detect completed tasks"
            echo "  -list                    List all available tasks"
            echo "  -status [task-id]        Show completion status"
            echo "  -h, --help               Show this help message"
            echo
            echo "Examples:"
            echo "  $0 -task MVP-001"
            echo "  $0 -subtask 'Docker development environment'"
            echo "  $0 -session 20250912034623-mvp-demo-runner-session"
            echo "  $0 -auto"
            echo "  $0 -status MVP-001"
            ;;
        "")
            info "Task Status Updater - AI-CORE"
            echo
            show_task_status
            echo
            info "Use -h for help or -auto to automatically update from recent work"
            ;;
        *)
            error_exit "Unknown option: $1. Use -h for help."
            ;;
    esac
}

# Run main function
main "$@"
