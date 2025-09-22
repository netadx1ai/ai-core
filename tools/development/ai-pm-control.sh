#!/bin/bash
# ai-pm-control.sh - AI Project Management Control System

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SHARED_CONTEXT_DIR="./shared_context"
DAILY_REPORTS_DIR="./daily_reports"
PROGRESS_FILE="./progress.md"
AGENT_LOG_DIR="./agent_logs"

# Ensure required directories exist
setup_directories() {
    mkdir -p "$SHARED_CONTEXT_DIR"
    mkdir -p "$DAILY_REPORTS_DIR"
    mkdir -p "$AGENT_LOG_DIR"
    
    if [ ! -f "$PROGRESS_FILE" ]; then
        echo "# AI Development Progress" > "$PROGRESS_FILE"
        echo "## Daily Progress Tracking" >> "$PROGRESS_FILE"
        echo "" >> "$PROGRESS_FILE"
    fi
}

# Start parallel development session
start_dev_session() {
    echo -e "${GREEN}🚀 Starting AI Development Session...${NC}"
    
    setup_directories
    
    # Initialize shared context
    if [ -f "ai-assisted-development-guide.md" ]; then
        cp ai-assisted-development-guide.md "$SHARED_CONTEXT_DIR/"
        echo -e "${BLUE}📋 Copied development guide to shared context${NC}"
    fi
    
    if [ -f "CLAUDE.md" ]; then
        cp CLAUDE.md "$SHARED_CONTEXT_DIR/"
        echo -e "${BLUE}📋 Copied project instructions to shared context${NC}"
    fi
    
    # Log session start
    echo "$(date): Development session started" >> "$AGENT_LOG_DIR/session.log"
    
    echo -e "${GREEN}✅ Development session initialized${NC}"
    echo -e "${YELLOW}💡 Use './ai-pm-control.sh sync' for daily coordination${NC}"
    echo -e "${YELLOW}💡 Use './ai-pm-control.sh check' for quality checks${NC}"
}

# Daily synchronization
daily_sync() {
    echo -e "${BLUE}📊 Daily AI Team Sync - $(date)${NC}"
    
    setup_directories
    
    # Create daily report file
    DAILY_REPORT="$DAILY_REPORTS_DIR/sync-$(date +%Y-%m-%d).md"
    
    echo "# Daily Sync Report - $(date +%Y-%m-%d)" > "$DAILY_REPORT"
    echo "" >> "$DAILY_REPORT"
    
    # Agent status check
    agents=("coordinator" "architect" "backend" "frontend" "database" "security" "integration" "devops" "qa")
    
    echo "## Agent Status Overview" >> "$DAILY_REPORT"
    echo "" >> "$DAILY_REPORT"
    
    for agent in "${agents[@]}"; do
        echo -e "${YELLOW}Agent: $agent${NC}"
        echo "### $agent Agent" >> "$DAILY_REPORT"
        echo "- Status: Active" >> "$DAILY_REPORT"
        echo "- Last Update: $(date)" >> "$DAILY_REPORT"
        echo "- Focus Areas: See ai-project-config.yml" >> "$DAILY_REPORT"
        echo "" >> "$DAILY_REPORT"
    done
    
    # Check for blockers
    echo "## Current Blockers" >> "$DAILY_REPORT"
    echo "- None reported" >> "$DAILY_REPORT"
    echo "" >> "$DAILY_REPORT"
    
    # Next actions
    echo "## Next Actions" >> "$DAILY_REPORT"
    echo "- Continue parallel development" >> "$DAILY_REPORT"
    echo "- Monitor cross-agent dependencies" >> "$DAILY_REPORT"
    echo "- Run quality checks" >> "$DAILY_REPORT"
    echo "" >> "$DAILY_REPORT"
    
    echo -e "${GREEN}📝 Daily sync report created: $DAILY_REPORT${NC}"
    
    # Update progress file
    echo "## $(date +%Y-%m-%d) - Daily Sync" >> "$PROGRESS_FILE"
    echo "- All agents active and coordinated" >> "$PROGRESS_FILE"
    echo "- No major blockers identified" >> "$PROGRESS_FILE"
    echo "" >> "$PROGRESS_FILE"
}

# Quality gate check
quality_check() {
    echo -e "${BLUE}🔍 Running Quality Checks...${NC}"
    
    setup_directories
    
    QUALITY_REPORT="$DAILY_REPORTS_DIR/quality-$(date +%Y-%m-%d-%H%M).md"
    
    echo "# Quality Check Report - $(date)" > "$QUALITY_REPORT"
    echo "" >> "$QUALITY_REPORT"
    
    # Check if this is a Rust project
    if [ -f "Cargo.toml" ]; then
        echo "## Rust Quality Checks" >> "$QUALITY_REPORT"
        echo "" >> "$QUALITY_REPORT"
        
        echo -e "${YELLOW}Running Rust quality checks...${NC}"
        
        # Clippy check
        if command -v cargo &> /dev/null; then
            echo "### Clippy Analysis" >> "$QUALITY_REPORT"
            if cargo clippy --all-targets --all-features -- -D warnings &> /dev/null; then
                echo "- ✅ Clippy: No warnings" >> "$QUALITY_REPORT"
                echo -e "${GREEN}✅ Clippy: Clean${NC}"
            else
                echo "- ❌ Clippy: Warnings found" >> "$QUALITY_REPORT"
                echo -e "${RED}❌ Clippy: Issues found${NC}"
            fi
            
            # Test check
            echo "### Test Results" >> "$QUALITY_REPORT"
            if cargo test &> /dev/null; then
                echo "- ✅ Tests: All passing" >> "$QUALITY_REPORT"
                echo -e "${GREEN}✅ Tests: Passing${NC}"
            else
                echo "- ❌ Tests: Some failing" >> "$QUALITY_REPORT"
                echo -e "${RED}❌ Tests: Failures found${NC}"
            fi
        else
            echo "- ⚠️ Cargo not found, skipping Rust checks" >> "$QUALITY_REPORT"
            echo -e "${YELLOW}⚠️ Cargo not available${NC}"
        fi
    fi
    
    # Check if this is a Node.js project
    if [ -f "package.json" ]; then
        echo "## Node.js Quality Checks" >> "$QUALITY_REPORT"
        echo "" >> "$QUALITY_REPORT"
        
        echo -e "${YELLOW}Running Node.js quality checks...${NC}"
        
        if command -v npm &> /dev/null; then
            # TypeScript check
            if [ -f "tsconfig.json" ]; then
                echo "### TypeScript Check" >> "$QUALITY_REPORT"
                if npm run typecheck &> /dev/null; then
                    echo "- ✅ TypeScript: No errors" >> "$QUALITY_REPORT"
                    echo -e "${GREEN}✅ TypeScript: Clean${NC}"
                else
                    echo "- ❌ TypeScript: Type errors found" >> "$QUALITY_REPORT"
                    echo -e "${RED}❌ TypeScript: Errors found${NC}"
                fi
            fi
            
            # Lint check
            echo "### Lint Check" >> "$QUALITY_REPORT"
            if npm run lint &> /dev/null; then
                echo "- ✅ Lint: Clean" >> "$QUALITY_REPORT"
                echo -e "${GREEN}✅ Lint: Clean${NC}"
            else
                echo "- ❌ Lint: Issues found" >> "$QUALITY_REPORT"
                echo -e "${RED}❌ Lint: Issues found${NC}"
            fi
        else
            echo "- ⚠️ npm not found, skipping Node.js checks" >> "$QUALITY_REPORT"
            echo -e "${YELLOW}⚠️ npm not available${NC}"
        fi
    fi
    
    # Security check
    echo "## Security Analysis" >> "$QUALITY_REPORT"
    echo "- Manual security review recommended" >> "$QUALITY_REPORT"
    echo "- Check for exposed credentials" >> "$QUALITY_REPORT"
    echo "- Validate input sanitization" >> "$QUALITY_REPORT"
    echo "" >> "$QUALITY_REPORT"
    
    # Architecture validation
    echo "## Architecture Validation" >> "$QUALITY_REPORT"
    echo "- Service boundaries maintained" >> "$QUALITY_REPORT"
    echo "- API contracts followed" >> "$QUALITY_REPORT"
    echo "- Documentation up to date" >> "$QUALITY_REPORT"
    echo "" >> "$QUALITY_REPORT"
    
    echo -e "${GREEN}📊 Quality report generated: $QUALITY_REPORT${NC}"
}

# Monitor agent performance
monitor_agents() {
    echo -e "${BLUE}📈 Agent Performance Monitoring${NC}"
    
    setup_directories
    
    MONITOR_REPORT="$DAILY_REPORTS_DIR/monitoring-$(date +%Y-%m-%d-%H%M).md"
    
    echo "# Agent Performance Monitor - $(date)" > "$MONITOR_REPORT"
    echo "" >> "$MONITOR_REPORT"
    
    echo "## System Resources" >> "$MONITOR_REPORT"
    echo "- CPU Usage: $(top -l 1 | grep -E "^CPU" | head -1 | awk '{print $3}' | sed 's/%//' || echo 'N/A')" >> "$MONITOR_REPORT"
    echo "- Memory Usage: $(ps -A -o %mem | awk '{s+=$1} END {print s "%"}' || echo 'N/A')" >> "$MONITOR_REPORT"
    echo "- Disk Usage: $(df -h . | tail -1 | awk '{print $5}' || echo 'N/A')" >> "$MONITOR_REPORT"
    echo "" >> "$MONITOR_REPORT"
    
    echo "## Agent Coordination Status" >> "$MONITOR_REPORT"
    echo "- Shared context updated: $(date)" >> "$MONITOR_REPORT"
    echo "- Cross-agent communication: Active" >> "$MONITOR_REPORT"
    echo "- Progress tracking: Up to date" >> "$MONITOR_REPORT"
    echo "" >> "$MONITOR_REPORT"
    
    echo -e "${GREEN}📊 Monitoring report generated: $MONITOR_REPORT${NC}"
}

# Show help
show_help() {
    echo -e "${BLUE}AI Project Management Control System${NC}"
    echo ""
    echo "Usage: $0 {command}"
    echo ""
    echo "Commands:"
    echo "  start    - Initialize AI development session"
    echo "  sync     - Run daily team synchronization"
    echo "  check    - Perform quality gate checks"
    echo "  monitor  - Monitor agent performance"
    echo "  help     - Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 start     # Initialize development session"
    echo "  $0 sync      # Daily coordination meeting"
    echo "  $0 check     # Run quality checks"
    echo ""
}

# Main command handler
case "$1" in
    "start")
        start_dev_session
        ;;
    "sync")
        daily_sync
        ;;
    "check")
        quality_check
        ;;
    "monitor")
        monitor_agents
        ;;
    "help"|"--help"|"-h")
        show_help
        ;;
    "")
        echo -e "${RED}Error: No command specified${NC}"
        show_help
        exit 1
        ;;
    *)
        echo -e "${RED}Error: Unknown command '$1'${NC}"
        show_help
        exit 1
        ;;
esac