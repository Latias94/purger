#!/bin/bash

# Purger Publishing Script
# This script helps with local publishing and version management

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  check       - Run pre-publish checks (format, clippy, tests)"
    echo "  dry-run     - Test publishing without actually publishing"
    echo "  publish     - Publish all packages to crates.io"
    echo "  version     - Show current version information"
    echo "  bump        - Bump version (patch|minor|major)"
    echo ""
    echo "Options:"
    echo "  --skip-tests    - Skip running tests"
    echo "  --skip-checks   - Skip format and clippy checks"
    echo ""
    echo "Examples:"
    echo "  $0 check                    # Run all checks"
    echo "  $0 dry-run                  # Test publishing"
    echo "  $0 publish                  # Publish to crates.io"
    echo "  $0 bump patch               # Bump patch version"
    echo "  $0 version                  # Show version info"
}

# Function to check if cargo is available
check_cargo() {
    if ! command -v cargo &> /dev/null; then
        print_error "cargo is not installed or not in PATH"
        exit 1
    fi
}

# Function to get current version
get_current_version() {
    cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "purger") | .version'
}

# Function to run pre-publish checks
run_checks() {
    local skip_tests=${1:-false}
    local skip_checks=${2:-false}
    
    print_info "Running pre-publish checks..."
    
    if [[ "$skip_checks" != "true" ]]; then
        print_info "Checking code formatting..."
        if cargo fmt --all -- --check; then
            print_success "Code formatting is correct"
        else
            print_error "Code formatting issues found. Run 'cargo fmt' to fix."
            return 1
        fi
        
        print_info "Running clippy..."
        if cargo clippy --workspace --all-targets --all-features -- -D warnings; then
            print_success "Clippy checks passed"
        else
            print_error "Clippy found issues"
            return 1
        fi
    fi
    
    if [[ "$skip_tests" != "true" ]]; then
        print_info "Running tests..."
        if cargo test --workspace --all-features; then
            print_success "All tests passed"
        else
            print_error "Some tests failed"
            return 1
        fi
    fi
    
    print_success "All checks passed!"
}

# Function to dry run publishing
dry_run_publish() {
    print_info "Running dry-run publishing..."
    
    print_info "Dry-run: purger-core"
    cargo publish --dry-run -p purger-core
    
    print_info "Dry-run: purger-cli"
    cargo publish --dry-run -p purger-cli
    
    print_info "Dry-run: purger-gui"
    cargo publish --dry-run -p purger-gui
    
    print_info "Dry-run: purger"
    cargo publish --dry-run -p purger
    
    print_success "Dry-run completed successfully!"
}

# Function to publish packages
publish_packages() {
    print_info "Publishing packages to crates.io..."
    
    # Check if CARGO_REGISTRY_TOKEN is set
    if [[ -z "$CARGO_REGISTRY_TOKEN" ]]; then
        print_warning "CARGO_REGISTRY_TOKEN not set. Make sure you're logged in with 'cargo login'"
    fi
    
    print_info "Publishing purger-core..."
    cargo publish -p purger-core
    print_success "purger-core published!"
    
    print_info "Waiting 60 seconds for crates.io to propagate..."
    sleep 60
    
    print_info "Publishing purger-cli..."
    cargo publish -p purger-cli
    print_success "purger-cli published!"
    
    print_info "Publishing purger-gui..."
    cargo publish -p purger-gui
    print_success "purger-gui published!"
    
    print_info "Waiting 30 seconds for dependencies to propagate..."
    sleep 30
    
    print_info "Publishing purger..."
    cargo publish -p purger
    print_success "purger published!"
    
    print_success "All packages published successfully!"
    echo ""
    print_info "Links:"
    echo "  - https://crates.io/crates/purger-core"
    echo "  - https://crates.io/crates/purger-cli"
    echo "  - https://crates.io/crates/purger-gui"
    echo "  - https://crates.io/crates/purger"
}

# Function to show version information
show_version() {
    print_info "Current version information:"
    echo ""
    cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | "  \(.name): \(.version)"'
}

# Function to bump version
bump_version() {
    local bump_type=$1
    
    if [[ -z "$bump_type" ]]; then
        print_error "Version bump type required (patch|minor|major)"
        return 1
    fi
    
    if [[ ! "$bump_type" =~ ^(patch|minor|major)$ ]]; then
        print_error "Invalid bump type. Use: patch, minor, or major"
        return 1
    fi
    
    print_info "Bumping $bump_type version..."
    
    # This would require cargo-edit to be installed
    if ! command -v cargo-set-version &> /dev/null; then
        print_warning "cargo-edit not installed. Install with: cargo install cargo-edit"
        print_info "Manual version bump required in Cargo.toml"
        return 1
    fi
    
    # Get current version and calculate new version
    local current_version=$(get_current_version)
    print_info "Current version: $current_version"
    
    # This is a simplified version bump - in practice you'd want more robust version parsing
    print_warning "Automatic version bumping not implemented yet."
    print_info "Please manually update the version in Cargo.toml [workspace.package] section"
    print_info "Or use the GitHub Actions workflow for automated version management"
}

# Main script logic
main() {
    check_cargo
    
    local command=${1:-help}
    local skip_tests=false
    local skip_checks=false
    
    # Parse options
    shift || true
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-tests)
                skip_tests=true
                shift
                ;;
            --skip-checks)
                skip_checks=true
                shift
                ;;
            *)
                break
                ;;
        esac
    done
    
    case $command in
        check)
            run_checks $skip_tests $skip_checks
            ;;
        dry-run)
            run_checks $skip_tests $skip_checks
            dry_run_publish
            ;;
        publish)
            run_checks $skip_tests $skip_checks
            publish_packages
            ;;
        version)
            show_version
            ;;
        bump)
            bump_version $1
            ;;
        help|--help|-h)
            show_usage
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
