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
CAPROVER_NAME="${CAPROVER_NAME:-captain-01}"
REDIS_APP_NAME="${SAM_REDIS_APP_NAME:-${APP_NAME}-redis}"
VOICE_APP_NAME="${SAM_VOICE_APP_NAME:-${APP_NAME}-voice}"
SEAWEED_MASTER_APP_NAME="${SAM_SEAWEED_MASTER_APP_NAME:-${APP_NAME}-seaweed-master}"
SEAWEED_VOLUME_APP_NAME="${SAM_SEAWEED_VOLUME_APP_NAME:-${APP_NAME}-seaweed-volume}"
SEAWEED_FILER_APP_NAME="${SAM_SEAWEED_FILER_APP_NAME:-${APP_NAME}-seaweed-filer}"
POSTGRES_APP_NAME="${SAM_POSTGRES_APP_NAME:-${APP_NAME}-db}"
DATABASE_ENGINE="${DATABASE_ENGINE:-postgres}"
POSTGRES_URL="${POSTGRES_URL:-postgresql://sam:sam@srv-captain--${POSTGRES_APP_NAME}:5432/sam}"
REDIS_URL="${REDIS_URL:-redis://srv-captain--${REDIS_APP_NAME}:6379}"
TTS_URL="${TTS_URL:-http://srv-captain--${VOICE_APP_NAME}:8002/tts}"
STT_URL="${STT_URL:-http://srv-captain--${VOICE_APP_NAME}:8002/stt}"
SEAWEEDFS_MASTER_URL="${SEAWEEDFS_MASTER_URL:-http://srv-captain--${SEAWEED_MASTER_APP_NAME}:9333}"
SEAWEEDFS_VOLUME_URL="${SEAWEEDFS_VOLUME_URL:-http://srv-captain--${SEAWEED_VOLUME_APP_NAME}:8080}"
SEAWEEDFS_FILER_URL="${SEAWEEDFS_FILER_URL:-http://srv-captain--${SEAWEED_FILER_APP_NAME}:8888}"

export APP_NAME DATABASE_ENGINE POSTGRES_URL REDIS_URL TTS_URL STT_URL
export SEAWEEDFS_MASTER_URL SEAWEEDFS_VOLUME_URL SEAWEEDFS_FILER_URL

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

caprover_args() {
    if [[ -n "$CAPROVER_NAME" ]]; then
        echo "--caproverName ${CAPROVER_NAME}"
    fi
}

caprover_api() {
    # shellcheck disable=SC2046
    caprover api $(caprover_args) "$@"
}

caprover_deploy() {
    # shellcheck disable=SC2046
    caprover deploy $(caprover_args) "$@"
}

app_exists() {
    local app_name="$1"
    local raw_file

    raw_file="$(mktemp /tmp/sam-caprover-apps.XXXXXX.json)"
    if ! caprover_api --path "/user/apps/appDefinitions/" --method "GET" --data '{}' > "$raw_file"; then
        rm -f "$raw_file"
        return 1
    fi

    SAM_LOOKUP_APP_NAME="$app_name" node - "$raw_file" <<'NODE'
const fs = require('fs');
const raw = fs.readFileSync(process.argv[2], 'utf8');
const jsonStart = raw.indexOf('{');
if (jsonStart < 0) process.exit(1);
const response = JSON.parse(raw.slice(jsonStart));
const appName = process.env.SAM_LOOKUP_APP_NAME;
process.exit(response.appDefinitions.some((definition) => definition.appName === appName) ? 0 : 1);
NODE
    local result=$?
    rm -f "$raw_file"
    return $result
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if caprover CLI is installed
    if ! command -v caprover &> /dev/null; then
        log_error "CapRover CLI is not installed. Install it with: npm install -g caprover"
        exit 1
    fi

    if ! command -v node &> /dev/null; then
        log_error "Node.js is required to update CapRover app definitions safely"
        exit 1
    fi
    
    # Check if we're in the S.A.M. project directory
    if [[ ! -f "captain-definition" || ! -f "Cargo.toml" ]]; then
        log_error "This script must be run from the S.A.M. project root directory"
        exit 1
    fi

    if ! grep -Eq '^[[:space:]]*name[[:space:]]*=[[:space:]]*"sam"' Cargo.toml; then
        log_error "Cargo.toml does not declare package name \"sam\""
        exit 1
    fi

    if ! grep -q '"dockerfilePath"[[:space:]]*:[[:space:]]*"./deploy/Dockerfile"' captain-definition; then
        log_error "captain-definition must deploy ./deploy/Dockerfile for the sam app"
        exit 1
    fi

    if [[ ! -f "deploy/Dockerfile" ]] || ! grep -q 'cargo build --release --bin sam' deploy/Dockerfile || ! grep -q 'COPY --from=builder /usr/src/sam/target/release/sam /app/sam' deploy/Dockerfile; then
        log_error "deploy/Dockerfile does not build and copy the sam binary"
        exit 1
    fi

    if grep -Rqs 'samllm-cli' captain-definition deploy/Dockerfile; then
        log_error "Refusing to deploy SAMLLM artifacts to the S.A.M. CapRover app"
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
    echo "  Redis App: ${REDIS_APP_NAME}"
    echo "  Voice App: ${VOICE_APP_NAME}"
    echo "  SeaweedFS Master App: ${SEAWEED_MASTER_APP_NAME}"
    echo "  SeaweedFS Volume App: ${SEAWEED_VOLUME_APP_NAME}"
    echo "  SeaweedFS Filer App: ${SEAWEED_FILER_APP_NAME}"
    echo "  Domain: ${DOMAIN:-'Not configured'}"
    echo "  CapRover Name: ${CAPROVER_NAME:-'Using CLI default'}"
    echo "  CapRover URL: ${CAPROVER_URL:-'Using logged-in instance'}"
    echo ""
}

# Create app if it doesn't exist
create_named_app_if_needed() {
    local app_name="$1"
    local has_persistent_data="${2:-true}"
    log_info "Checking if app '${app_name}' exists..."
    
    if ! app_exists "$app_name"; then
        log_info "App '${app_name}' doesn't exist. Creating it..."
        
        # Create the app
        caprover_api --path "/user/apps/appDefinitions/register" --method "POST" \
            --data "{\"appName\":\"${app_name}\",\"hasPersistentData\":${has_persistent_data}}" || {
            log_error "Failed to create app '${app_name}'"
            exit 1
        }
        
        log_success "App '${app_name}' created successfully"
        
        # Wait a moment for app to be ready
        sleep 3
    else
        log_info "App '${app_name}' already exists"
    fi
}

create_app_if_needed() {
    create_named_app_if_needed "${APP_NAME}" true
}

deploy_companion_services() {
    log_info "Deploying CapRover companion services..."

    create_named_app_if_needed "${REDIS_APP_NAME}" true
    caprover_deploy --appName "${REDIS_APP_NAME}" --imageName redis:7-alpine || {
        log_error "Failed to deploy Redis companion app '${REDIS_APP_NAME}'"
        exit 1
    }

    create_named_app_if_needed "${VOICE_APP_NAME}" false
    local voice_tar
    voice_tar="$(mktemp /tmp/sam-voice-caprover.XXXXXX.tar)"
    tar -C deploy/caprover-sam-voice -cf "$voice_tar" .
    caprover_deploy --appName "${VOICE_APP_NAME}" --tarFile "$voice_tar" || {
        rm -f "$voice_tar"
        log_error "Failed to deploy voice companion app '${VOICE_APP_NAME}'"
        exit 1
    }
    rm -f "$voice_tar"

    create_named_app_if_needed "${SEAWEED_MASTER_APP_NAME}" true
    local seaweed_master_tar
    seaweed_master_tar="$(mktemp /tmp/sam-seaweed-master-caprover.XXXXXX.tar)"
    tar -C deploy/caprover-sam-seaweed-master -cf "$seaweed_master_tar" .
    caprover_deploy --appName "${SEAWEED_MASTER_APP_NAME}" --tarFile "$seaweed_master_tar" || {
        rm -f "$seaweed_master_tar"
        log_error "Failed to deploy SeaweedFS master app '${SEAWEED_MASTER_APP_NAME}'"
        exit 1
    }
    rm -f "$seaweed_master_tar"

    create_named_app_if_needed "${SEAWEED_VOLUME_APP_NAME}" true
    local seaweed_volume_tar
    seaweed_volume_tar="$(mktemp /tmp/sam-seaweed-volume-caprover.XXXXXX.tar)"
    tar -C deploy/caprover-sam-seaweed-volume -cf "$seaweed_volume_tar" .
    caprover_deploy --appName "${SEAWEED_VOLUME_APP_NAME}" --tarFile "$seaweed_volume_tar" || {
        rm -f "$seaweed_volume_tar"
        log_error "Failed to deploy SeaweedFS volume app '${SEAWEED_VOLUME_APP_NAME}'"
        exit 1
    }
    rm -f "$seaweed_volume_tar"

    create_named_app_if_needed "${SEAWEED_FILER_APP_NAME}" true
    local seaweed_filer_tar
    seaweed_filer_tar="$(mktemp /tmp/sam-seaweed-filer-caprover.XXXXXX.tar)"
    tar -C deploy/caprover-sam-seaweed-filer -cf "$seaweed_filer_tar" .
    caprover_deploy --appName "${SEAWEED_FILER_APP_NAME}" --tarFile "$seaweed_filer_tar" || {
        rm -f "$seaweed_filer_tar"
        log_error "Failed to deploy SeaweedFS filer app '${SEAWEED_FILER_APP_NAME}'"
        exit 1
    }
    rm -f "$seaweed_filer_tar"

    log_success "Companion services deployed"
}

update_app_definition() {
    local app_name="$1"
    local env_json="$2"
    local volumes_json="$3"
    local domain="${4:-}"
    local custom_nginx_config="${5:-}"
    local raw_file
    local payload_file

    raw_file="$(mktemp /tmp/sam-caprover-apps.XXXXXX.json)"
    payload_file="$(mktemp /tmp/sam-caprover-update.XXXXXX.json)"

    caprover_api --path "/user/apps/appDefinitions/" --method "GET" --data '{}' > "$raw_file"

    SAM_UPDATE_APP_NAME="$app_name" \
    SAM_UPDATE_ENV_VARS="$env_json" \
    SAM_UPDATE_VOLUMES="$volumes_json" \
    SAM_UPDATE_DOMAIN="$domain" \
    SAM_UPDATE_CUSTOM_NGINX_CONFIG="$custom_nginx_config" \
    node - "$raw_file" "$payload_file" <<'NODE'
const fs = require('fs');

const raw = fs.readFileSync(process.argv[2], 'utf8');
const jsonStart = raw.indexOf('{');
if (jsonStart < 0) {
  throw new Error('CapRover app definitions response did not include JSON');
}

const response = JSON.parse(raw.slice(jsonStart));
const appName = process.env.SAM_UPDATE_APP_NAME;
const app = response.appDefinitions.find((definition) => definition.appName === appName);
if (!app) {
  throw new Error(`CapRover app '${appName}' was not found`);
}

const desiredEnv = JSON.parse(process.env.SAM_UPDATE_ENV_VARS || '[]');
const envByKey = new Map((app.envVars || []).map((entry) => [entry.key, String(entry.value)]));
for (const entry of desiredEnv) {
  envByKey.set(entry.key, String(entry.value));
}

const desiredVolumes = JSON.parse(process.env.SAM_UPDATE_VOLUMES || '[]');
const volumeByContainerPath = new Map(
  (app.volumes || []).map((entry) => [entry.containerPath, entry.hostPath])
);
for (const entry of desiredVolumes) {
  volumeByContainerPath.set(entry.containerPath, entry.hostPath);
}

const domain = process.env.SAM_UPDATE_DOMAIN;
const customNginxConfig = process.env.SAM_UPDATE_CUSTOM_NGINX_CONFIG;
const customDomain = Array.isArray(app.customDomain) ? [...app.customDomain] : [];
if (domain && !customDomain.includes(domain)) {
  customDomain.push(domain);
}

const payload = {
  appName: app.appName,
  projectId: app.projectId || '',
  description: app.description || '',
  instanceCount: String(app.instanceCount || 1),
  captainDefinitionRelativeFilePath: app.captainDefinitionRelativeFilePath || './captain-definition',
  notExposeAsWebApp: !!app.notExposeAsWebApp,
  tags: app.tags || [],
  customNginxConfig: customNginxConfig || app.customNginxConfig || '',
  forceSsl: !!app.forceSsl || !!domain,
  websocketSupport: !!app.websocketSupport || !!customNginxConfig,
  appPushWebhook: app.appPushWebhook || {},
  repoInfo: app.repoInfo || {},
  envVars: Array.from(envByKey, ([key, value]) => ({ key, value })),
  volumes: Array.from(volumeByContainerPath, ([containerPath, hostPath]) => ({ containerPath, hostPath })),
  ports: app.ports || [],
  nodeId: app.nodeId || '',
  redirectDomain: app.redirectDomain || '',
  customDomain,
  preDeployFunction: app.preDeployFunction || '',
  serviceUpdateOverride: app.serviceUpdateOverride || '',
  containerHttpPort: Number(app.containerHttpPort || 8000),
  httpAuth: app.httpAuth || {},
  appDeployTokenConfig: app.appDeployTokenConfig || { enabled: false },
};

fs.writeFileSync(process.argv[3], JSON.stringify(payload));
NODE

    caprover_api --path "/user/apps/appDefinitions/update/" --method "POST" \
        --data "$(cat "$payload_file")"

    rm -f "$raw_file" "$payload_file"
}

# Configure environment variables
configure_environment() {
    log_info "Configuring environment variables..."

    local env_vars
    env_vars="$(node -e '
const env = process.env;
const vars = [
  ["CAPROVER", "true"],
  ["DATABASE_ENGINE", env.DATABASE_ENGINE],
  ["POSTGRES_URL", env.POSTGRES_URL],
  ["REDIS_URL", env.REDIS_URL],
  ["REDIS_DISABLED", "false"],
  ["TTS_URL", env.TTS_URL],
  ["STT_URL", env.STT_URL],
  ["SEAWEEDFS_MASTER_URL", env.SEAWEEDFS_MASTER_URL],
  ["SEAWEEDFS_VOLUME_URL", env.SEAWEEDFS_VOLUME_URL],
  ["SEAWEEDFS_FILER_URL", env.SEAWEEDFS_FILER_URL],
  ["SQLITE_DATABASE_PATH", "/var/lib/sam/sam.db"],
  ["PORT", "8000"],
  ["SAM_HOME", "/app"],
  ["SAM_DATA", "/var/lib/sam"],
  ["SAM_LOGS", "/var/log/sam"],
  ["RUST_LOG", "info"],
  ["RUST_BACKTRACE", "1"],
  ["RUN_MIGRATIONS", "true"],
  ["CRAWLER_DISABLED", env.CRAWLER_DISABLED || "false"],
  ["CRAWLER_THREADS", env.CRAWLER_THREADS || "1"],
  ["CRAWLER_DNS_THREADS", env.CRAWLER_DNS_THREADS || "2"],
  ["TELEMETRY_ENABLED", env.TELEMETRY_ENABLED || "false"],
];
console.log(JSON.stringify(vars.map(([key, value]) => ({ key, value }))));
')"

    update_app_definition "$APP_NAME" "$env_vars" '[]' || {
        log_warning "Failed to set environment variables - you may need to configure them manually"
    }
    
    log_success "Environment variables configured"
}

# Configure persistent volumes
configure_volumes() {
    log_info "Configuring persistent volumes..."

    local volume_config
    volume_config="$(node -e '
const app = process.env.APP_NAME || "sam";
console.log(JSON.stringify([
  { hostPath: `/home/kal/caprover-data/${app}/data`, containerPath: "/var/lib/sam" },
  { hostPath: `/home/kal/caprover-data/${app}/logs`, containerPath: "/var/log/sam" },
]));
')"

    update_app_definition "$APP_NAME" '[]' "$volume_config" || {
        log_warning "Failed to configure volumes - you may need to configure them manually"
    }
    
    log_success "Persistent volumes configured"
}

configure_websocket_proxy() {
    log_info "Configuring CapRover WebSocket proxy..."

    if [[ ! -f "deploy/caprover-sam-nginx.conf" ]]; then
        log_warning "deploy/caprover-sam-nginx.conf not found - WebSocket proxy was not configured"
        return
    fi

    local nginx_config
    nginx_config="$(cat deploy/caprover-sam-nginx.conf)"

    update_app_definition "$APP_NAME" '[]' '[]' "" "$nginx_config" || {
        log_warning "Failed to configure WebSocket nginx proxy - you may need to configure it manually"
    }

    log_success "WebSocket proxy configured"
}

# Deploy the application
deploy_application() {
    log_info "Deploying S.A.M. to CapRover..."
    
    # Deploy using the current directory
    caprover_deploy --appName "${APP_NAME}" || {
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

        update_app_definition "$APP_NAME" '[]' '[]' "$DOMAIN" || {
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
    deploy_companion_services
    configure_environment
    configure_volumes
    configure_websocket_proxy
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
