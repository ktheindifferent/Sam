#!/bin/bash
#
# Directory Structure Validation Script
# Validates that the S.A.M. project follows the organized directory structure
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# Track validation results
ERRORS=0
WARNINGS=0

# Check if we're in the project root
check_project_root() {
    if [[ ! -f "Cargo.toml" || ! -f "README.md" ]]; then
        log_error "Not in S.A.M. project root directory"
        exit 1
    fi
    log_success "In S.A.M. project root"
}

# Validate directory structure
validate_directories() {
    log_info "Validating directory structure..."
    
    # Required directories
    required_dirs=(
        "src"
        "docs" 
        "deploy"
        "config"
        "scripts"
        "tools"
        "tests"
        "www"
        "cfg"
        "data"
    )
    
    for dir in "${required_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            log_success "Directory exists: $dir/"
        else
            log_error "Missing required directory: $dir/"
            ((ERRORS+=1))
        fi
    done
    
    # Documentation subdirectories
    docs_subdirs=("api" "deployment" "development" "features" "security")
    for subdir in "${docs_subdirs[@]}"; do
        if [[ -d "docs/$subdir" ]]; then
            log_success "Documentation category exists: docs/$subdir/"
        else
            log_warning "Documentation category missing: docs/$subdir/"
            ((WARNINGS+=1))
        fi
    done
}

# Validate key files are in correct locations
validate_file_locations() {
    log_info "Validating file locations..."
    
    # Root files that should exist
    root_files=(
        "Cargo.toml"
        "Cargo.lock"
        "README.md"
        "LICENSE.md"
        "captain-definition"
        ".dockerignore"
        "build.rs"
    )
    
    for file in "${root_files[@]}"; do
        if [[ -f "$file" ]]; then
            log_success "Root file exists: $file"
        else
            log_error "Missing root file: $file"
            ((ERRORS+=1))
        fi
    done
    
    # Files that should be in deploy/
    deploy_files=(
        "deploy/Dockerfile"
        "deploy/docker-compose.yml"
        "deploy/docker-entrypoint.sh"
    )
    
    for file in "${deploy_files[@]}"; do
        if [[ -f "$file" ]]; then
            log_success "Deploy file exists: $file"
        else
            log_error "Missing deploy file: $file"
            ((ERRORS+=1))
        fi
    done
    
    # Key documentation files
    doc_files=(
        "docs/DIRECTORY_STRUCTURE.md"
        "docs/CLAUDE.md"
        "docs/security/SECURITY.md"
        "docs/deployment/CAPROVER_DEPLOYMENT.md"
    )
    
    for file in "${doc_files[@]}"; do
        if [[ -f "$file" ]]; then
            log_success "Documentation exists: $file"
        else
            log_warning "Documentation missing: $file"
            ((WARNINGS+=1))
        fi
    done
}

# Check for files that shouldn't be in root
validate_clean_root() {
    log_info "Validating clean root directory..."
    
    # Patterns that shouldn't be in root
    unwanted_patterns=(
        "*.md"
        "docker-compose*.yml" 
        "Dockerfile*"
        "test_*.rs"
        "test_*.sh"
        "*.deb"
        ".env.example"
    )
    
    root_clean=true
    
    for pattern in "${unwanted_patterns[@]}"; do
        # Use find to check for unwanted files
        found_files=$(find . -maxdepth 1 -name "$pattern" \
            -not -name "README.md" \
            -not -name "LICENSE.md" \
            -not -name "AGENTS.md" \
            -not -name "CLAUDE.md" \
            2>/dev/null || true)
        if [[ -n "$found_files" ]]; then
            log_warning "Files matching '$pattern' found in root: $found_files"
            root_clean=false
            ((WARNINGS+=1))
        fi
    done
    
    if $root_clean; then
        log_success "Root directory is clean"
    fi
}

# Count files in different categories
count_organization() {
    log_info "Analyzing project organization..."
    
    # Count files by category
    total_files=$(find . -type f -name "*.md" | wc -l)
    doc_files=$(find docs/ -type f -name "*.md" 2>/dev/null | wc -l || echo 0)
    test_files=$(find tests/ -type f \( -name "*.rs" -o -name "*.sh" \) 2>/dev/null | wc -l || echo 0)
    deploy_files=$(find deploy/ -type f 2>/dev/null | wc -l || echo 0)
    script_files=$(find scripts/ -type f -name "*.sh" 2>/dev/null | wc -l || echo 0)
    tool_files=$(find tools/ -type f 2>/dev/null | wc -l || echo 0)
    
    echo ""
    echo "📊 Organization Summary:"
    echo "  Total .md files: $total_files"
    echo "  Documentation files: $doc_files"
    echo "  Test files: $test_files"
    echo "  Deployment files: $deploy_files"
    echo "  Script files: $script_files"
    echo "  Tool files: $tool_files"
    echo ""
    
    if [[ $doc_files -gt 0 ]]; then
        log_success "Documentation is organized in docs/"
    fi
    
    if [[ $test_files -gt 0 ]]; then
        log_success "Tests are organized in tests/"
    fi
    
    if [[ $deploy_files -gt 0 ]]; then
        log_success "Deployment files are organized in deploy/"
    fi
}

# Validate Docker build context
validate_docker_context() {
    log_info "Validating Docker configurations..."
    
    # Check captain-definition references correct Dockerfile
    if [[ -f "captain-definition" ]]; then
        if grep -q "deploy/Dockerfile" captain-definition; then
            log_success "captain-definition references correct Dockerfile path"
        else
            log_error "captain-definition should reference ./deploy/Dockerfile"
            ((ERRORS+=1))
        fi
    fi
    
    # Check docker-compose files use correct context
    for compose_file in deploy/docker-compose*.yml; do
        if [[ -f "$compose_file" ]]; then
            if grep -q "context: \.\." "$compose_file" && grep -q "dockerfile: deploy/Dockerfile" "$compose_file"; then
                log_success "Docker Compose file has correct build context: $(basename $compose_file)"
            else
                log_warning "Docker Compose file may have incorrect build context: $(basename $compose_file)"
                ((WARNINGS+=1))
            fi
        fi
    done
}

# Main validation flow
main() {
    echo "🔍 S.A.M. Directory Structure Validation"
    echo "========================================"
    echo ""
    
    check_project_root
    validate_directories
    validate_file_locations
    validate_clean_root
    count_organization
    validate_docker_context
    
    echo ""
    echo "📋 Validation Results:"
    echo "======================"
    
    if [[ $ERRORS -eq 0 && $WARNINGS -eq 0 ]]; then
        log_success "✅ Perfect! Directory structure is fully compliant"
        echo ""
        echo "🎉 The S.A.M. project follows excellent organization practices:"
        echo "   • Clean root directory with only essential files"
        echo "   • Documentation organized by category in docs/"
        echo "   • Deployment files separated in deploy/"
        echo "   • Tests consolidated in tests/"
        echo "   • Tools and scripts properly categorized"
        echo ""
        exit 0
    elif [[ $ERRORS -eq 0 ]]; then
        log_success "✅ Structure is valid with minor suggestions"
        echo ""
        echo "Summary: $WARNINGS warnings (non-critical)"
        echo "The project structure is well-organized with room for minor improvements."
        echo ""
        exit 0
    else
        log_error "❌ Structure validation failed"
        echo ""
        echo "Summary: $ERRORS errors, $WARNINGS warnings"
        echo "Please address the errors above to maintain project organization."
        echo ""
        echo "💡 Tips:"
        echo "   • Move misplaced files to appropriate directories"
        echo "   • Create missing required directories"  
        echo "   • Update file references after moving files"
        echo ""
        exit 1
    fi
}

# Show help
show_help() {
    cat << EOF
S.A.M. Directory Structure Validation Script

Usage: $0 [--help|-h]

This script validates that the S.A.M. project follows the organized
directory structure as documented in docs/DIRECTORY_STRUCTURE.md

Validation checks:
  ✓ Required directories exist
  ✓ Key files are in correct locations  
  ✓ Root directory is clean and organized
  ✓ Docker configurations reference correct paths
  ✓ Documentation is properly categorized

Exit codes:
  0 - Structure is valid
  1 - Critical errors found

Run from the project root directory.
EOF
}

# Handle command line arguments
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    show_help
    exit 0
fi

# Run validation
main "$@"
