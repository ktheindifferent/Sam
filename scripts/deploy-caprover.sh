#!/bin/bash
#
# CapRover Deployment Script for S.A.M.
# Usage: ./scripts/deploy-caprover.sh [app-name] [domain]
#
# Prerequisites:
# - CapRover CLI installed: npm install -g caprover
# - Already logged into CapRover: caprover login
#

set -e

# Configuration
APP_NAME="${1:-sam}"
DOMAIN="${2:-}"
CAPROVER_URL="${CAPROVER_URL:-}"

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
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if caprover CLI is installed
    if ! command -v caprover &> /dev/null; then
        log_error "CapRover CLI is not installed. Install it with: npm install -g caprover"
        exit 1
    fi
    
    # Check if we're in the S.A.M. project directory
    if [[ ! -f "captain-definition" || ! -f "Cargo.toml" ]]; then
        log_error "This script must be run from the S.A.M. project root directory"
        exit 1
    fi
    
    # Check if user is logged into CapRover
    if ! caprover list &> /dev/null; then
        log_error "Not logged into CapRover. Run: caprover login"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Display deployment information
show_deployment_info() {
    log_info "Deployment Configuration:"
    echo "  App Name: ${APP_NAME}"
    echo "  Domain: ${DOMAIN:-'Not configured'}"
    echo "  CapRover URL: ${CAPROVER_URL:-'Using logged-in instance'}"
    echo ""
}

# Create app if it doesn't exist
create_app_if_needed() {
    log_info "Checking if app '${APP_NAME}' exists..."
    
    # Try to get app info - if it fails, app doesn't exist
    if ! caprover api --path "/api/v2/user/apps/data/${APP_NAME}" --method "GET" &> /dev/null; then
        log_info "App '${APP_NAME}' doesn't exist. Creating it..."
        
        # Create the app
        caprover api --path "/api/v2/user/apps/appData/${APP_NAME}" --method "POST" \
            --data '{"hasPersistentData": true}' || {
            log_error "Failed to create app"
            exit 1
        }
        
        log_success "App '${APP_NAME}' created successfully"
        
        # Wait a moment for app to be ready
        sleep 3
    else
        log_info "App '${APP_NAME}' already exists"
    fi
}

# Configure environment variables
configure_environment() {
    log_info "Configuring environment variables..."
    
    # Default environment variables for S.A.M.
    local env_vars='{
        "envVars": [
            {"key": "DATABASE_ENGINE", "value": "sqlite"},
            {"key": "SQLITE_DATABASE_PATH", "value": "/var/lib/sam/sam.db"},
            {"key": "PORT", "value": "8000"},
            {"key": "SAM_HOME", "value": "/app"},
            {"key": "SAM_DATA", "value": "/var/lib/sam"},
            {"key": "SAM_LOGS", "value": "/var/log/sam"},
            {"key": "RUST_LOG", "value": "info"},
            {"key": "RUST_BACKTRACE", "value": "1"},
            {"key": "RUN_MIGRATIONS", "value": "true"}
        ]
    }'
    
    caprover api --path "/api/v2/user/apps/appData/${APP_NAME}" --method "POST" \
        --data "$env_vars" || {
        log_warning "Failed to set environment variables - you may need to configure them manually"
    }
    
    log_success "Environment variables configured"
}

# Configure persistent volumes
configure_volumes() {
    log_info "Configuring persistent volumes..."
    
    # Configure volume mappings
    local volume_config='{
        "volumes": [
            {"hostPath": "/var/lib/sam", "containerPath": "/var/lib/sam"},
            {"hostPath": "/var/log/sam", "containerPath": "/var/log/sam"}
        ]
    }'
    
    caprover api --path "/api/v2/user/apps/appData/${APP_NAME}" --method "POST" \
        --data "$volume_config" || {
        log_warning "Failed to configure volumes - you may need to configure them manually"
    }
    
    log_success "Persistent volumes configured"
}

# Deploy the application
deploy_application() {
    log_info "Deploying S.A.M. to CapRover..."
    
    # Deploy using the current directory
    caprover deploy --appName "${APP_NAME}" || {
        log_error "Deployment failed"
        log_info "Check the build logs with: caprover logs --app ${APP_NAME} --lines 100"
        exit 1
    }
    
    log_success "Application deployed successfully"
}

# Configure domain if provided
configure_domain() {
    if [[ -n "$DOMAIN" ]]; then
        log_info "Configuring domain: ${DOMAIN}"
        
        # Enable HTTPS and set domain
        local domain_config='{
            "customDomain": "'$DOMAIN'",
            "hasDefaultSubDomainSsl": true,
            "forceSsl": true,
            "redirectDomain": "",
            "customNginxConfig": ""
        }'
        
        caprover api --path "/api/v2/user/apps/appData/${APP_NAME}" --method "POST" \
            --data "$domain_config" || {
            log_warning "Failed to configure domain - you may need to configure it manually"
        }
        
        log_success "Domain configured"
    fi
}

# Wait for deployment and perform health check
wait_and_health_check() {
    log_info "Waiting for deployment to stabilize..."
    sleep 30
    
    # Get app URL
    local app_url="https://${DOMAIN:-${APP_NAME}.your-caprover-domain.com}"
    if [[ -z "$DOMAIN" ]]; then
        log_warning "No domain configured. Update the app_url variable with your actual domain"
        app_url="http://localhost"  # Fallback
    fi
    
    # Perform health check
    log_info "Performing health check..."
    local health_endpoint="${app_url}/health"
    
    if command -v curl &> /dev/null; then
        if curl -f -s "$health_endpoint" > /dev/null; then
            log_success "Health check passed"
        else
            log_warning "Health check failed - application may still be starting"
            log_info "Check logs with: caprover logs --app ${APP_NAME} --follow"
        fi
    else
        log_warning "curl not available - skipping health check"
    fi
}

# Show post-deployment information
show_post_deployment_info() {
    echo ""
    log_success "=== S.A.M. Deployment Complete ==="
    echo ""
    echo "App Name: ${APP_NAME}"
    echo "CapRover URL: Check your CapRover dashboard"
    if [[ -n "$DOMAIN" ]]; then
        echo "Application URL: https://${DOMAIN}"
    else
        echo "Application URL: Configure domain in CapRover dashboard"
    fi
    echo ""
    echo "Useful commands:"
    echo "  View logs: caprover logs --app ${APP_NAME} --follow"
    echo "  App status: caprover api --path \"/api/v2/user/apps/data/${APP_NAME}\" --method GET"
    echo "  Redeploy: caprover deploy --appName ${APP_NAME}"
    echo ""
    log_info "For troubleshooting, check the README.md CapRover section"
}

# Main deployment flow
main() {
    echo "🚢 S.A.M. CapRover Deployment Script"
    echo "===================================="
    echo ""
    
    check_prerequisites
    show_deployment_info
    create_app_if_needed
    configure_environment
    configure_volumes
    deploy_application
    configure_domain
    wait_and_health_check
    show_post_deployment_info
}

# Show help
show_help() {
    cat << EOF
S.A.M. CapRover Deployment Script

Usage: $0 [APP_NAME] [DOMAIN]

Arguments:
  APP_NAME    Name of the CapRover app (default: sam)
  DOMAIN      Custom domain for the app (optional)

Environment Variables:
  CAPROVER_URL    CapRover instance URL (optional, uses logged-in instance)

Examples:
  $0                                    # Deploy as 'sam'
  $0 my-sam-app                        # Deploy as 'my-sam-app'
  $0 sam sam.mydomain.com              # Deploy with custom domain
  
Prerequisites:
  1. Install CapRover CLI: npm install -g caprover
  2. Login to CapRover: caprover login
  3. Run from S.A.M. project root directory

For more information, see the CapRover section in README.md
EOF
}

# Handle command line arguments
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    show_help
    exit 0
fi

# Run main deployment
main "$@"