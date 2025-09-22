#!/usr/bin/env pwsh
<#
.SYNOPSIS
AI Work Tracker Automation Script - Simplified Single-File Session Management

.DESCRIPTION
This PowerShell script manages AI work sessions using a simplified single-file approach.
Each session is stored as a single markdown file with the naming convention:
{STATUS}-{timestamp}-{session-name}.md

Features:
- Simple file-based session management
- Automatic session naming with timestamps
- Status tracking (ACTIVE, COMPLETED, PAUSED, CANCELLED)
- Token usage tracking and metrics
- Cross-platform compatibility (Windows, macOS, Linux, WSL2)
- Session templates for consistent documentation

.PARAMETER Action
The session action to perform:
- start-session: Create a new AI work session
- update-session: Update an existing session with progress
- complete-session: Mark session as completed
- pause-session: Pause an active session
- cancel-session: Cancel a session
- list-sessions: List all sessions with filters
- show-session: Display a specific session
- rename-session: Rename a session
- archive-old: Archive completed sessions older than specified days

.PARAMETER AgentName
The AI agent name for the session (e.g., backend-agent, frontend-agent)

.PARAMETER Objective
The objective or description of the work session

.PARAMETER SessionFile
The session file name (without path) for operations on existing sessions

.PARAMETER Status
Filter sessions by status (ACTIVE, COMPLETED, PAUSED, CANCELLED)

.PARAMETER TokensUsed
Number of tokens used in this session update

.PARAMETER Activity
Description of current activity or progress

.PARAMETER Priority
Session priority (Low, Medium, High, Critical)

.PARAMETER EstimatedDuration
Estimated duration for the session (e.g., "2 hours", "30 minutes")

.PARAMETER ArchiveDays
Number of days after which to archive completed sessions (default: 30)

.EXAMPLE
.\ai-work-tracker.ps1 -Action start-session -AgentName "backend-agent" -Objective "Fix compilation errors in auth service"

.EXAMPLE
.\ai-work-tracker.ps1 -Action update-session -SessionFile "ACTIVE-20250113-142530-fixing-auth-service.md" -TokensUsed 250 -Activity "Fixed import statements"

.EXAMPLE
.\ai-work-tracker.ps1 -Action complete-session -SessionFile "ACTIVE-20250113-142530-fixing-auth-service.md"

.EXAMPLE
.\ai-work-tracker.ps1 -Action list-sessions -Status ACTIVE

.NOTES
Author: AI Instructions Template
Version: 2.0.0
Created: 2025-01-13
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("start-session", "update-session", "complete-session", "pause-session", "cancel-session", "list-sessions", "show-session", "rename-session", "archive-old")]
    [string]$Action,

    [Parameter(Mandatory=$false)]
    [string]$AgentName = "",

    [Parameter(Mandatory=$false)]
    [string]$Objective = "",

    [Parameter(Mandatory=$false)]
    [string]$SessionFile = "",

    [Parameter(Mandatory=$false)]
    [ValidateSet("ACTIVE", "COMPLETED", "PAUSED", "CANCELLED", "")]
    [string]$Status = "",

    [Parameter(Mandatory=$false)]
    [int]$TokensUsed = 0,

    [Parameter(Mandatory=$false)]
    [string]$Activity = "",

    [Parameter(Mandatory=$false)]
    [ValidateSet("Low", "Medium", "High", "Critical")]
    [string]$Priority = "Medium",

    [Parameter(Mandatory=$false)]
    [string]$EstimatedDuration = "",

    [Parameter(Mandatory=$false)]
    [int]$ArchiveDays = 30
)

# Initialize paths and configuration
$ScriptRoot = Split-Path $PSScriptRoot -Parent
# $ProjectRoot variable removed as it's unused
$SessionsPath = Join-Path $PSScriptRoot "sessions"
$MetricsPath = Join-Path $PSScriptRoot "metrics"
$LogsPath = Join-Path $PSScriptRoot "logs"

# Ensure directories exist
@($SessionsPath, $MetricsPath, $LogsPath) | ForEach-Object {
    if (-not (Test-Path $_)) {
        New-Item -ItemType Directory -Path $_ -Force | Out-Null
    }
}

# Session status colors
$StatusColors = @{
    "ACTIVE" = "Green"
    "COMPLETED" = "Blue"
    "PAUSED" = "Yellow"
    "CANCELLED" = "Red"
}

# Logging function
function Write-Log {
    param(
        [string]$Message,
        [string]$Level = "INFO"
    )

    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [$Level] $Message"

    $levelColors = @{
        "INFO" = "White"
        "SUCCESS" = "Green"
        "WARNING" = "Yellow"
        "ERROR" = "Red"
    }

    $color = $levelColors[$Level]
    if ($color) {
        Write-Host $logMessage -ForegroundColor $color
    } else {
        Write-Host $logMessage
    }

    # Write to log file
    $logFile = Join-Path $LogsPath "ai-work-tracker-$(Get-Date -Format 'yyyy-MM-dd').log"
    $logMessage | Out-File -FilePath $logFile -Append -Encoding UTF8
}

# Generate session filename
function New-SessionFileName {
    param(
        [string]$Status,
        [string]$Objective
    )

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $cleanObjective = $Objective -replace '[^\w\s-]', '' -replace '\s+', '-' -replace '-+', '-'
    $cleanObjective = $cleanObjective.Trim('-').ToLower()

    # Limit length to keep filename manageable
    if ($cleanObjective.Length -gt 50) {
        $cleanObjective = $cleanObjective.Substring(0, 50).TrimEnd('-')
    }

    return "$Status-$timestamp-$cleanObjective.md"
}

# Parse session filename
function Get-SessionInfo {
    param([string]$FileName)

    if ($FileName -match '^(ACTIVE|COMPLETED|PAUSED|CANCELLED)-(\d{8}-\d{6})-(.+)\.md$') {
        return @{
            Status = $Matches[1]
            Timestamp = $Matches[2]
            Name = $Matches[3] -replace '-', ' '
            FullPath = Join-Path $SessionsPath $FileName
        }
    }

    return $null
}

# Create session template
function New-SessionTemplate {
    param(
        [string]$Status,
        [string]$AgentName,
        [string]$Objective,
        [string]$Priority,
        [string]$EstimatedDuration
    )

    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $sessionTitle = $Objective

    $template = @"
# AI Work Session - $sessionTitle

**Session Status**: $Status
**Started**: $timestamp
**Agent**: $AgentName
**Objective**: $Objective
**Priority**: $Priority$(if ($EstimatedDuration) { "
**Estimated Duration**: $EstimatedDuration" })

## Session Overview

Brief description of the work being performed and its context.

## Current Progress

### ✅ Completed Tasks
- [$(Get-Date -Format 'HH:mm')] Session initialized

### 🔄 In Progress
- Working on: $Objective

### 📋 Next Steps
- Define specific tasks and milestones
- Track progress and update status

## Technical Details

### Key Information
- Technology stack: [Specify technologies used]
- Files being modified: [List key files]
- Dependencies: [Note any dependencies or blockers]

### Code Changes Made
```
[Add code snippets or descriptions of changes]
```

## Metrics Tracking

- **Session Start Time**: $(Get-Date -Format 'HH:mm:ss')
- **Time Spent**: [Update as session progresses]
- **Tokens Used**: 0
- **Progress**: 0% complete

## Token Usage

- **Tokens Used This Session**: 0
- **Primary Activities**:
  - Session setup: 0 tokens

## Knowledge Gained

### Patterns That Worked
- [Document successful approaches]

### Lessons Learned
- [Capture insights and learnings]

## Context for AI Handoff

### Current State
- Working directory: [Specify current working directory]
- Files being modified: [List active files]
- Environment setup: [Note any special setup requirements]

### Next Priority Actions
1. [List immediate next steps]
2. [Include any important context]

## Session Notes

- [Add any additional notes or observations]
- [Include links to relevant resources]

---

**Last Updated**: $timestamp
**Status**: $Status
**Confidence Level**: [Low/Medium/High] - [Brief assessment]
"@

    return $template
}

# Start a new session
function Start-Session {
    param(
        [string]$AgentName,
        [string]$Objective,
        [string]$Priority,
        [string]$EstimatedDuration
    )

    if (-not $AgentName) {
        Write-Log "AgentName is required for start-session action" "ERROR"
        return $false
    }

    if (-not $Objective) {
        Write-Log "Objective is required for start-session action" "ERROR"
        return $false
    }

    # Check for existing active sessions
    $activeSessions = Get-ChildItem -Path $SessionsPath -Filter "ACTIVE-*.md" -ErrorAction SilentlyContinue
    if ($activeSessions.Count -gt 0) {
        Write-Log "Warning: $(activeSessions.Count) active session(s) found:" "WARNING"
        foreach ($session in $activeSessions) {
            # Using session name directly since sessionInfo was unused
            Write-Host "  - $($session.Name)" -ForegroundColor Yellow
        }
        Write-Host ""
    }

    $fileName = New-SessionFileName -Status "ACTIVE" -Objective $Objective
    $filePath = Join-Path $SessionsPath $fileName

    $template = New-SessionTemplate -Status "ACTIVE" -AgentName $AgentName -Objective $Objective -Priority $Priority -EstimatedDuration $EstimatedDuration

    $template | Out-File -FilePath $filePath -Encoding UTF8

    Write-Log "Created new session: $fileName" "SUCCESS"
    Write-Host "Session file: $filePath" -ForegroundColor Gray

    # Update metrics
    Update-SessionMetrics -Action "session_started" -AgentName $AgentName -SessionFile $fileName

    return $true
}

# Update an existing session
function Update-Session {
    param(
        [string]$SessionFile,
        [int]$TokensUsed,
        [string]$Activity
    )

    if (-not $SessionFile) {
        Write-Log "SessionFile is required for update-session action" "ERROR"
        return $false
    }

    $sessionPath = Join-Path $SessionsPath $SessionFile
    if (-not (Test-Path $sessionPath)) {
        Write-Log "Session file not found: $SessionFile" "ERROR"
        return $false
    }

    $sessionInfo = Get-SessionInfo $SessionFile
    if (-not $sessionInfo) {
        Write-Log "Invalid session file format: $SessionFile" "ERROR"
        return $false
    }

    if ($sessionInfo.Status -ne "ACTIVE") {
        Write-Log "Cannot update non-active session (status: $($sessionInfo.Status))" "ERROR"
        return $false
    }

    # Read current content
    $content = Get-Content $sessionPath -Raw
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $timeOnly = Get-Date -Format "HH:mm"

    # Update token usage if provided
    if ($TokensUsed -gt 0) {
        if ($content -match 'Tokens Used This Session\*\*: (\d+)') {
            $currentTokens = [int]$Matches[1]
            $newTokens = $currentTokens + $TokensUsed
            $content = $content -replace 'Tokens Used This Session\*\*: \d+', "Tokens Used This Session**: $newTokens"
        }
    }

    # Add activity if provided
    if ($Activity) {
        # Find the "In Progress" section and add the activity
        $activityEntry = "- [$timeOnly] $Activity"

        if ($content -match '### 🔄 In Progress\r?\n(.*?)(?=\r?\n### |\r?\n## |\Z)') {
            $inProgressSection = $Matches[1]
            $updatedSection = $inProgressSection.TrimEnd() + "`n$activityEntry"
            $content = $content -replace [regex]::Escape($Matches[0]), "### 🔄 In Progress`n$updatedSection"
        }
    }

    # Update timestamp
    $content = $content -replace '\*\*Last Updated\*\*: [^\r\n]+', "**Last Updated**: $timestamp"

    # Write updated content
    $content | Out-File -FilePath $sessionPath -Encoding UTF8

    Write-Log "Updated session: $SessionFile" "SUCCESS"
    if ($TokensUsed -gt 0) {
        Write-Host "Added $TokensUsed tokens" -ForegroundColor Gray
    }
    if ($Activity) {
        Write-Host "Added activity: $Activity" -ForegroundColor Gray
    }

    return $true
}

# Change session status
function Set-SessionStatus {
    param(
        [string]$SessionFile,
        [string]$NewStatus,
        [string]$Reason = ""
    )

    $sessionPath = Join-Path $SessionsPath $SessionFile
    if (-not (Test-Path $sessionPath)) {
        Write-Log "Session file not found: $SessionFile" "ERROR"
        return $false
    }

    $sessionInfo = Get-SessionInfo $SessionFile
    if (-not $sessionInfo) {
        Write-Log "Invalid session file format: $SessionFile" "ERROR"
        return $false
    }

    # Generate new filename with updated status
    $newFileName = $SessionFile -replace "^(ACTIVE|COMPLETED|PAUSED|CANCELLED)-", "$NewStatus-"
    $newPath = Join-Path $SessionsPath $newFileName

    # Update content
    $content = Get-Content $sessionPath -Raw
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"

    # Update status in content
    $content = $content -replace '\*\*Session Status\*\*: (ACTIVE|COMPLETED|PAUSED|CANCELLED)', "**Session Status**: $NewStatus"

    # Add completion/pause/cancellation timestamp
    switch ($NewStatus) {
        "COMPLETED" {
            $content = $content -replace '(\*\*Started\*\*: [^\r\n]+)', "`$1  `n**Completed**: $timestamp"
            if ($Reason) {
                $content = $content -replace '(\*\*Completed\*\*: [^\r\n]+)', "`$1  `n**Final Result**: ✅ $Reason"
            }
        }
        "PAUSED" {
            $content = $content -replace '(\*\*Started\*\*: [^\r\n]+)', "`$1  `n**Paused**: $timestamp"
            if ($Reason) {
                $content = $content -replace '(\*\*Paused\*\*: [^\r\n]+)', "`$1  `n**Pause Reason**: $Reason"
            }
        }
        "CANCELLED" {
            $content = $content -replace '(\*\*Started\*\*: [^\r\n]+)', "`$1  `n**Cancelled**: $timestamp"
            if ($Reason) {
                $content = $content -replace '(\*\*Cancelled\*\*: [^\r\n]+)', "`$1  `n**Cancellation Reason**: $Reason"
            }
        }
    }

    # Update last updated timestamp
    $content = $content -replace '\*\*Last Updated\*\*: [^\r\n]+', "**Last Updated**: $timestamp"

    # Update status in final summary
    $content = $content -replace '\*\*Status\*\*: (ACTIVE|COMPLETED|PAUSED|CANCELLED)', "**Status**: $NewStatus"

    # Write to new file and remove old
    $content | Out-File -FilePath $newPath -Encoding UTF8
    Remove-Item $sessionPath -Force

    Write-Log "Changed session status from $($sessionInfo.Status) to $NewStatus" "SUCCESS"
    Write-Host "Renamed: $SessionFile -> $newFileName" -ForegroundColor Gray

    # Update metrics
    Update-SessionMetrics -Action "status_changed" -SessionFile $newFileName -NewStatus $NewStatus

    return $true
}

# List sessions with optional filtering
function Get-Sessions {
    param([string]$StatusFilter = "")

    $sessions = Get-ChildItem -Path $SessionsPath -Filter "*.md" -ErrorAction SilentlyContinue

    if (-not $sessions) {
        Write-Log "No sessions found" "INFO"
        return
    }

    $filteredSessions = $sessions | ForEach-Object {
        $sessionInfo = Get-SessionInfo $_.Name
        if ($sessionInfo -and ($StatusFilter -eq "" -or $sessionInfo.Status -eq $StatusFilter)) {
            [PSCustomObject]@{
                FileName = $_.Name
                Status = $sessionInfo.Status
                Timestamp = $sessionInfo.Timestamp
                Name = $sessionInfo.Name
                LastModified = $_.LastWriteTime
                Size = [math]::Round($_.Length / 1KB, 1)
            }
        }
    } | Where-Object { $_ -ne $null } | Sort-Object LastModified -Descending

    if (-not $filteredSessions) {
        Write-Log "No sessions found matching criteria" "INFO"
        return
    }

    Write-Host "`nAI Work Sessions:" -ForegroundColor Cyan
    Write-Host "=================" -ForegroundColor Cyan

    $groupedSessions = $filteredSessions | Group-Object Status

    foreach ($group in $groupedSessions) {
        $color = $StatusColors[$group.Name]
        Write-Host "`n$($group.Name) Sessions ($($group.Count)):" -ForegroundColor $color

        foreach ($session in $group.Group) {
            $ageInHours = [math]::Round(((Get-Date) - $session.LastModified).TotalHours, 1)
            Write-Host "  📄 $($session.Name)" -ForegroundColor White
            Write-Host "     File: $($session.FileName)" -ForegroundColor Gray
            Write-Host "     Modified: $ageInHours hours ago | Size: $($session.Size) KB" -ForegroundColor DarkGray
        }
    }

    Write-Host "`nTotal: $($filteredSessions.Count) sessions" -ForegroundColor Cyan
}

# Show session content
function Show-Session {
    param([string]$SessionFile)

    $sessionPath = Join-Path $SessionsPath $SessionFile
    if (-not (Test-Path $sessionPath)) {
        Write-Log "Session file not found: $SessionFile" "ERROR"
        return $false
    }

    $sessionInfo = Get-SessionInfo $SessionFile
    if ($sessionInfo) {
        $color = $StatusColors[$sessionInfo.Status]
        Write-Host "`nSession: $($sessionInfo.Name)" -ForegroundColor $color
        Write-Host "Status: $($sessionInfo.Status)" -ForegroundColor $color
        Write-Host "File: $SessionFile" -ForegroundColor Gray
        Write-Host "=" * 60 -ForegroundColor Gray
    }

    Get-Content $sessionPath | Write-Host
    return $true
}

# Update session metrics
function Update-SessionMetrics {
    param(
        [string]$Action,
        [string]$AgentName = "",
        [string]$SessionFile = "",
        [string]$NewStatus = ""
    )

    $metricsFile = Join-Path $MetricsPath "session-metrics-$(Get-Date -Format 'yyyy-MM').json"

    $metrics = @{
        timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        action = $Action
        agentName = $AgentName
        sessionFile = $SessionFile
        newStatus = $NewStatus
    }

    # Append to metrics file
    $metrics | ConvertTo-Json -Compress | Out-File -FilePath $metricsFile -Append -Encoding UTF8
}

# Archive old completed sessions (renamed to use approved verb)
function Move-OldSessionsToArchive {
    param([int]$Days)

    $cutoffDate = (Get-Date).AddDays(-$Days)
    $archivePath = Join-Path $PSScriptRoot "archive"

    if (-not (Test-Path $archivePath)) {
        New-Item -ItemType Directory -Path $archivePath -Force | Out-Null
    }

    $sessionsToArchive = Get-ChildItem -Path $SessionsPath -Filter "COMPLETED-*.md" |
        Where-Object { $_.LastWriteTime -lt $cutoffDate }

    if (-not $sessionsToArchive) {
        Write-Log "No completed sessions older than $Days days found" "INFO"
        return
    }

    Write-Log "Archiving $($sessionsToArchive.Count) completed sessions older than $Days days" "INFO"

    foreach ($session in $sessionsToArchive) {
        $archiveFile = Join-Path $archivePath $session.Name
        Move-Item $session.FullName $archiveFile -Force
        Write-Host "Archived: $($session.Name)" -ForegroundColor Gray
    }

    Write-Log "Archive completed: $($sessionsToArchive.Count) sessions moved to archive" "SUCCESS"
}

# Main execution logic
switch ($Action) {
    "start-session" {
        $success = Start-Session -AgentName $AgentName -Objective $Objective -Priority $Priority -EstimatedDuration $EstimatedDuration
        if (-not $success) { exit 1 }
    }

    "update-session" {
        $success = Update-Session -SessionFile $SessionFile -TokensUsed $TokensUsed -Activity $Activity
        if (-not $success) { exit 1 }
    }

    "complete-session" {
        $success = Set-SessionStatus -SessionFile $SessionFile -NewStatus "COMPLETED" -Reason $Activity
        if (-not $success) { exit 1 }
    }

    "pause-session" {
        $success = Set-SessionStatus -SessionFile $SessionFile -NewStatus "PAUSED" -Reason $Activity
        if (-not $success) { exit 1 }
    }

    "cancel-session" {
        $success = Set-SessionStatus -SessionFile $SessionFile -NewStatus "CANCELLED" -Reason $Activity
        if (-not $success) { exit 1 }
    }

    "list-sessions" {
        Get-Sessions -StatusFilter $Status
    }

    "show-session" {
        $success = Show-Session -SessionFile $SessionFile
        if (-not $success) { exit 1 }
    }

    "rename-session" {
        Write-Log "Session renaming will be implemented in future version" "INFO"
    }

    "archive-old" {
        Archive-OldSessions -Days $ArchiveDays
    }
}

Write-Log "AI work tracker operation completed" "SUCCESS"
