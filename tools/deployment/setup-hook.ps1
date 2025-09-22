#!/usr/bin/env pwsh
<#
.SYNOPSIS
Setup script for AI Instructions Auto-Sync Kiro Hook Template

.DESCRIPTION
This script helps configure the AI instructions auto-sync hook for your project by:
- Replacing template placeholders with project-specific values
- Validating sync tool availability
- Testing hook configuration
- Providing setup guidance

.PARAMETER ProjectName
The name of your project (e.g., "MyApp Core", "ProjectX")

.PARAMETER ProjectTimezone
Your project's timezone (e.g., "UTC", "America/New_York", "Asia/Tokyo")

.PARAMETER ProjectRootPath
The root path of your project (e.g., "C:\\Projects\\MyApp", "/home/user/projects/myapp")

.PARAMETER DryRun
Show what would be configured without making changes

.PARAMETER Validate
Only validate current hook configuration

.EXAMPLE
.\tools\setup-hook.ps1 -ProjectName "MyApp Core" -ProjectTimezone "America/New_York" -ProjectRootPath "C:\Projects\MyApp"

.EXAMPLE
.\tools\setup-hook.ps1 -Validate

.EXAMPLE
.\tools\setup-hook.ps1 -DryRun -ProjectName "TestProject" -ProjectTimezone "UTC" -ProjectRootPath "/home/user/test"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$ProjectName,

    [Parameter(Mandatory=$false)]
    [string]$ProjectTimezone,

    [Parameter(Mandatory=$false)]
    [string]$ProjectRootPath,

    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$Validate
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Colors for output
$Colors = @{
    Success = "Green"
    Warning = "Yellow"
    Error = "Red"
    Info = "Cyan"
    Header = "Magenta"
}

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    Write-Host $Message -ForegroundColor $Colors[$Color]
}

function Test-HookConfiguration {
    param([string]$HookPath)

    if (-not (Test-Path $HookPath)) {
        Write-ColorOutput "❌ Hook file not found: $HookPath" "Error"
        return $false
    }

    try {
        $hookContent = Get-Content $HookPath -Raw | ConvertFrom-Json

        # Check for remaining placeholders
        $placeholders = @("{PROJECT_NAME}", "{PROJECT_TIMEZONE}", "{PROJECT_ROOT_PATH}")
        $foundPlaceholders = @()

        $hookJson = $hookContent | ConvertTo-Json -Depth 10
        foreach ($placeholder in $placeholders) {
            if ($hookJson -match [regex]::Escape($placeholder)) {
                $foundPlaceholders += $placeholder
            }
        }

        if ($foundPlaceholders.Count -gt 0) {
            Write-ColorOutput "⚠️ Found unconfigured placeholders: $($foundPlaceholders -join ', ')" "Warning"
            return $false
        }

        # Check required structure
        $requiredProperties = @("enabled", "name", "when", "then")
        foreach ($prop in $requiredProperties) {
            if (-not $hookContent.PSObject.Properties[$prop]) {
                Write-ColorOutput "❌ Missing required property: $prop" "Error"
                return $false
            }
        }

        Write-ColorOutput "✅ Hook configuration is valid" "Success"
        return $true

    } catch {
        Write-ColorOutput "❌ Hook file is not valid JSON: $($_.Exception.Message)" "Error"
        return $false
    }
}

function Test-SyncTools {
    $tools = @(
        "tools\ai-instructions-sync.ps1",
        "tools\ai-instructions-sync.sh"
    )

    $foundTools = @()
    foreach ($tool in $tools) {
        if (Test-Path $tool) {
            $foundTools += $tool
            Write-ColorOutput "✅ Found sync tool: $tool" "Success"
        } else {
            Write-ColorOutput "⚠️ Sync tool not found: $tool" "Warning"
        }
    }

    if ($foundTools.Count -eq 0) {
        Write-ColorOutput "❌ No sync tools found! Please ensure ai-instructions-sync scripts exist." "Error"
        return $false
    }

    return $true
}

function Update-HookConfiguration {
    param(
        [string]$HookPath,
        [string]$ProjectName,
        [string]$ProjectTimezone,
        [string]$ProjectRootPath,
        [switch]$DryRun
    )

    if (-not (Test-Path $HookPath)) {
        Write-ColorOutput "❌ Hook file not found: $HookPath" "Error"
        return $false
    }

    try {
        $content = Get-Content $HookPath -Raw

        # Replace placeholders
        $replacements = @{
            "{PROJECT_NAME}" = $ProjectName
            "{PROJECT_TIMEZONE}" = $ProjectTimezone
            "{PROJECT_ROOT_PATH}" = $ProjectRootPath.Replace('\', '\\')  # Escape backslashes for JSON
        }

        $updatedContent = $content
        foreach ($placeholder in $replacements.Keys) {
            $value = $replacements[$placeholder]
            if ($updatedContent -match [regex]::Escape($placeholder)) {
                Write-ColorOutput "🔄 Replacing $placeholder with '$value'" "Info"
                $updatedContent = $updatedContent -replace [regex]::Escape($placeholder), $value
            }
        }

        # Validate JSON after replacement
        try {
            $updatedContent | ConvertFrom-Json | Out-Null
            Write-ColorOutput "✅ Updated configuration is valid JSON" "Success"
        } catch {
            Write-ColorOutput "❌ Updated configuration is not valid JSON: $($_.Exception.Message)" "Error"
            return $false
        }

        if ($DryRun) {
            Write-ColorOutput "🔍 DRY RUN - Would update hook configuration:" "Info"
            Write-ColorOutput $updatedContent "Info"
        } else {
            Set-Content $HookPath -Value $updatedContent -Encoding UTF8
            Write-ColorOutput "✅ Hook configuration updated successfully" "Success"
        }

        return $true

    } catch {
        Write-ColorOutput "❌ Failed to update hook configuration: $($_.Exception.Message)" "Error"
        return $false
    }
}

function Show-SetupStatus {
    Write-ColorOutput "🔍 AI Instructions Hook Setup Status" "Header"
    Write-ColorOutput "=================================" "Header"

    # Check hook file
    $hookPath = ".kiro\hooks\ai-instructions-auto-sync.kiro.hook"
    $hookValid = Test-HookConfiguration $hookPath

    # Check sync tools
    $toolsValid = Test-SyncTools

    # Check directories
    $dirs = @(".kiro", ".kiro\hooks", "tools", "dev-works/dev-agents")
    foreach ($dir in $dirs) {
        if (Test-Path $dir) {
            Write-ColorOutput "✅ Directory exists: $dir" "Success"
        } else {
            Write-ColorOutput "❌ Directory missing: $dir" "Error"
        }
    }

    # Overall status
    Write-ColorOutput "" "Info"
    if ($hookValid -and $toolsValid) {
        Write-ColorOutput "🎉 Hook setup is complete and ready!" "Success"
    } else {
        Write-ColorOutput "⚠️ Hook setup needs attention - see issues above" "Warning"
    }
}

function Show-Usage {
    Write-ColorOutput "🚀 AI Instructions Hook Setup Tool" "Header"
    Write-ColorOutput "==================================" "Header"
    Write-ColorOutput ""
    Write-ColorOutput "This tool helps configure the AI instructions auto-sync hook for your project." "Info"
    Write-ColorOutput ""
    Write-ColorOutput "REQUIRED for setup:" "Info"
    Write-ColorOutput "  -ProjectName        Your project name (e.g., 'MyApp Core')" "Info"
    Write-ColorOutput "  -ProjectTimezone    Your timezone (e.g., 'UTC', 'America/New_York')" "Info"
    Write-ColorOutput "  -ProjectRootPath    Your project root path" "Info"
    Write-ColorOutput ""
    Write-ColorOutput "OPTIONS:" "Info"
    Write-ColorOutput "  -DryRun            Show what would be changed without modifying files" "Info"
    Write-ColorOutput "  -Validate          Only validate current configuration" "Info"
    Write-ColorOutput ""
    Write-ColorOutput "EXAMPLES:" "Info"
    Write-ColorOutput "  .\tools\setup-hook.ps1 -Validate" "Info"
    Write-ColorOutput "  .\tools\setup-hook.ps1 -ProjectName 'MyApp' -ProjectTimezone 'UTC' -ProjectRootPath 'C:\Projects\MyApp'" "Info"
    Write-ColorOutput ""
}

# Main execution
try {
    if ($Validate) {
        Show-SetupStatus
        return
    }

    if (-not $ProjectName -or -not $ProjectTimezone -or -not $ProjectRootPath) {
        Show-Usage
        Show-SetupStatus
        return
    }

    Write-ColorOutput "🚀 Setting up AI Instructions Auto-Sync Hook" "Header"
    Write-ColorOutput "=============================================" "Header"
    Write-ColorOutput ""

    # Display configuration
    Write-ColorOutput "📋 Configuration:" "Info"
    Write-ColorOutput "  Project Name: $ProjectName" "Info"
    Write-ColorOutput "  Timezone: $ProjectTimezone" "Info"
    Write-ColorOutput "  Root Path: $ProjectRootPath" "Info"
    Write-ColorOutput "  Dry Run: $DryRun" "Info"
    Write-ColorOutput ""

    # Validate prerequisites
    Write-ColorOutput "🔍 Validating prerequisites..." "Info"
    if (-not (Test-SyncTools)) {
        Write-ColorOutput "❌ Prerequisite validation failed" "Error"
        return
    }

    # Update hook configuration
    $hookPath = ".kiro\hooks\ai-instructions-auto-sync.kiro.hook"
    Write-ColorOutput "🔄 Updating hook configuration..." "Info"
    if (-not (Update-HookConfiguration -HookPath $hookPath -ProjectName $ProjectName -ProjectTimezone $ProjectTimezone -ProjectRootPath $ProjectRootPath -DryRun:$DryRun)) {
        Write-ColorOutput "❌ Failed to update hook configuration" "Error"
        return
    }

    # Final validation
    if (-not $DryRun) {
        Write-ColorOutput "✅ Validating updated configuration..." "Info"
        if (Test-HookConfiguration $hookPath) {
            Write-ColorOutput "" "Info"
            Write-ColorOutput "🎉 AI Instructions Auto-Sync Hook setup completed successfully!" "Success"
            Write-ColorOutput "" "Info"
            Write-ColorOutput "NEXT STEPS:" "Info"
            Write-ColorOutput "1. Make a small change to AGENTS.md to test the hook" "Info"
            Write-ColorOutput "2. Verify automatic synchronization occurs" "Info"
            Write-ColorOutput "3. Check that all AI instruction files are updated" "Info"
            Write-ColorOutput "" "Info"
        } else {
            Write-ColorOutput "❌ Configuration validation failed after update" "Error"
        }
    } else {
        Write-ColorOutput "" "Info"
        Write-ColorOutput "🔍 DRY RUN completed - no files were modified" "Info"
        Write-ColorOutput "Run without -DryRun to apply changes" "Info"
    }

} catch {
    Write-ColorOutput "❌ Setup failed: $($_.Exception.Message)" "Error"
    Write-ColorOutput "Stack trace: $($_.ScriptStackTrace)" "Error"
}
