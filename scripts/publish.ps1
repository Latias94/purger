# Purger Publishing Script (PowerShell)
# This script helps with local publishing and version management

param(
    [Parameter(Position=0)]
    [string]$Command = "help",
    
    [switch]$SkipTests,
    [switch]$SkipChecks,
    [string]$BumpType
)

# Function to print colored output
function Write-Info {
    param([string]$Message)
    Write-Host "ℹ️  $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠️  $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "❌ $Message" -ForegroundColor Red
}

# Function to show usage
function Show-Usage {
    Write-Host "Usage: .\scripts\publish.ps1 [COMMAND] [OPTIONS]"
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  check       - Run pre-publish checks (format, clippy, tests)"
    Write-Host "  dry-run     - Test publishing without actually publishing"
    Write-Host "  publish     - Publish all packages to crates.io"
    Write-Host "  version     - Show current version information"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -SkipTests    - Skip running tests"
    Write-Host "  -SkipChecks   - Skip format and clippy checks"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\scripts\publish.ps1 check                    # Run all checks"
    Write-Host "  .\scripts\publish.ps1 dry-run                  # Test publishing"
    Write-Host "  .\scripts\publish.ps1 publish                  # Publish to crates.io"
    Write-Host "  .\scripts\publish.ps1 version                  # Show version info"
}

# Function to check if cargo is available
function Test-Cargo {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo is not installed or not in PATH"
        exit 1
    }
}

# Function to get current version
function Get-CurrentVersion {
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    $purgerPackage = $metadata.packages | Where-Object { $_.name -eq "purger" }
    return $purgerPackage.version
}

# Function to run pre-publish checks
function Invoke-Checks {
    param(
        [bool]$SkipTests = $false,
        [bool]$SkipChecks = $false
    )
    
    Write-Info "Running pre-publish checks..."
    
    if (-not $SkipChecks) {
        Write-Info "Checking code formatting..."
        $formatResult = cargo fmt --all -- --check
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Code formatting is correct"
        } else {
            Write-Error "Code formatting issues found. Run 'cargo fmt' to fix."
            return $false
        }
        
        Write-Info "Running clippy..."
        $clippyResult = cargo clippy --workspace --all-targets --all-features -- -D warnings
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Clippy checks passed"
        } else {
            Write-Error "Clippy found issues"
            return $false
        }
    }
    
    if (-not $SkipTests) {
        Write-Info "Running tests..."
        $testResult = cargo test --workspace --all-features
        if ($LASTEXITCODE -eq 0) {
            Write-Success "All tests passed"
        } else {
            Write-Error "Some tests failed"
            return $false
        }
    }
    
    Write-Success "All checks passed!"
    return $true
}

# Function to dry run publishing
function Invoke-DryRunPublish {
    Write-Info "Running dry-run publishing..."
    
    Write-Info "Dry-run: purger-core"
    cargo publish --dry-run -p purger-core --allow-dirty
    if ($LASTEXITCODE -ne 0) { return $false }

    Write-Info "Dry-run: purger-cli"
    cargo publish --dry-run -p purger-cli --allow-dirty
    if ($LASTEXITCODE -ne 0) { return $false }

    Write-Info "Dry-run: purger-gui"
    cargo publish --dry-run -p purger-gui --allow-dirty
    if ($LASTEXITCODE -ne 0) { return $false }

    Write-Info "Dry-run: purger"
    cargo publish --dry-run -p purger --allow-dirty
    if ($LASTEXITCODE -ne 0) { return $false }
    
    Write-Success "Dry-run completed successfully!"
    return $true
}

# Function to publish packages
function Invoke-PublishPackages {
    Write-Info "Publishing packages to crates.io..."
    
    # Check if CARGO_REGISTRY_TOKEN is set
    if (-not $env:CARGO_REGISTRY_TOKEN) {
        Write-Warning "CARGO_REGISTRY_TOKEN not set. Make sure you're logged in with 'cargo login'"
    }
    
    Write-Info "Publishing purger-core..."
    cargo publish -p purger-core
    if ($LASTEXITCODE -ne 0) { 
        Write-Error "Failed to publish purger-core"
        return $false 
    }
    Write-Success "purger-core published!"
    
    Write-Info "Waiting 60 seconds for crates.io to propagate..."
    Start-Sleep -Seconds 60
    
    Write-Info "Publishing purger-cli..."
    cargo publish -p purger-cli
    if ($LASTEXITCODE -ne 0) { 
        Write-Error "Failed to publish purger-cli"
        return $false 
    }
    Write-Success "purger-cli published!"
    
    Write-Info "Publishing purger-gui..."
    cargo publish -p purger-gui
    if ($LASTEXITCODE -ne 0) { 
        Write-Error "Failed to publish purger-gui"
        return $false 
    }
    Write-Success "purger-gui published!"
    
    Write-Info "Waiting 30 seconds for dependencies to propagate..."
    Start-Sleep -Seconds 30
    
    Write-Info "Publishing purger..."
    cargo publish -p purger
    if ($LASTEXITCODE -ne 0) { 
        Write-Error "Failed to publish purger"
        return $false 
    }
    Write-Success "purger published!"
    
    Write-Success "All packages published successfully!"
    Write-Host ""
    Write-Info "Links:"
    Write-Host "  - https://crates.io/crates/purger-core"
    Write-Host "  - https://crates.io/crates/purger-cli"
    Write-Host "  - https://crates.io/crates/purger-gui"
    Write-Host "  - https://crates.io/crates/purger"
    
    return $true
}

# Function to show version information
function Show-Version {
    Write-Info "Current version information:"
    Write-Host ""
    
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    foreach ($package in $metadata.packages) {
        Write-Host "  $($package.name): $($package.version)"
    }
}

# Main script logic
function Main {
    Test-Cargo
    
    switch ($Command.ToLower()) {
        "check" {
            $result = Invoke-Checks -SkipTests $SkipTests -SkipChecks $SkipChecks
            if (-not $result) { exit 1 }
        }
        "dry-run" {
            $checksResult = Invoke-Checks -SkipTests $SkipTests -SkipChecks $SkipChecks
            if (-not $checksResult) { exit 1 }
            
            $dryRunResult = Invoke-DryRunPublish
            if (-not $dryRunResult) { exit 1 }
        }
        "publish" {
            $checksResult = Invoke-Checks -SkipTests $SkipTests -SkipChecks $SkipChecks
            if (-not $checksResult) { exit 1 }
            
            $publishResult = Invoke-PublishPackages
            if (-not $publishResult) { exit 1 }
        }
        "version" {
            Show-Version
        }
        { $_ -in @("help", "--help", "-h") } {
            Show-Usage
        }
        default {
            Write-Error "Unknown command: $Command"
            Write-Host ""
            Show-Usage
            exit 1
        }
    }
}

# Run main function
Main
