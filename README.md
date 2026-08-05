# S.A.M.
Smart Artificial Mind
WIP. Dont use this software yet.
Licensed under GPL version 3.

TODO: 
1. Migrate all core files to libsam so that they can be shared with benchmarks, tests and other binaries beyond just "sam".

TTS API:
https://tts.opensam.foundation/api/tts?text=hello%20world&speaker_id=&style_wav=
https://tts.alpha.opensam.foundation/api/tts?text=hello%20world&speaker_id=&style_wav=


## Implemented Work

This list reflects the code currently present in the repository. SAM is still in active development, and several areas have partial implementations or integration gaps, but the following pieces are implemented or substantially scaffolded.

### Core Runtime and Application Shell
- Rust application split between the `sam` binary and reusable `libsam` library.
- Tokio multi-threaded runtime setup with interactive TUI mode and server/CapRover mode.
- First-run config initialization under the user SAM directory.
- Dual console/file logging setup plus panic reporting and cleanup hooks.
- Doctor/diagnostic command entry point.
- Installer binaries and migration helpers.
- Docker and CapRover deployment files, including SQLite and PostgreSQL variants.

### Web, API, and Dashboard
- Rouille-based HTTP server and static web dashboard under `www/`.
- REST API routing for sessions, humans, locations, observations, rooms, services, settings, things, jobs, IO commands, Ollama, telemetry, and service control.
- Health, liveness, readiness, detailed health, metrics, resource middleware, CSRF, and HTTP rate-limit modules.
- Web dashboard assets for service control, media, LIFX/touch controls, humans, rooms, locations, settings, setup, sanitization, and mobile-responsive UI.
- Console web app under `www/apps/console`.

### Service Architecture
- Large modular service layer with shared config, validation, retry, HTTP client, traits, and common error types.
- Service orchestrator with service registry, dependency ordering, start/stop flow, restart manager integration, and status tracking.
- Restart manager with restart strategies, circuit-breaker state, metrics, and notifier hooks.
- Thread manager and resource-management modules for limits, monitoring, pools, and cleanup.

### Infrastructure Services
- PostgreSQL service with connection pooling and schema initialization paths.
- SQLite support through `rusqlite` and database-engine abstractions.
- Redis service with connection pooling, health checks, circuit-breaker style protection, and tests.
- Docker service using Bollard for container orchestration and helper startup for PostgreSQL/Redis.
- Hybrid cache modules for web sessions, crawled pages, Wikipedia summaries, and crawler extension results.

### WebSocket and Real-Time Messaging
- WebSocket message model for subscribe/unsubscribe, commands, authentication, heartbeat, service status, system stats, network stats, activity, alerts, and command responses.
- Connection security modules for rate limits, message validation, session info, permissions, and audit logging.
- System stats collection and alert generation logic.
- WebSocket tests and audit log support.

### Security
- Security modules for auth, audit logging, sessions, input validation, HTTP middleware, and validation middleware.
- Documented fixes for command injection, SQL injection, WebSocket security, and general hardening.
- JWT-related security tests.
- Input validation coverage for URLs, SQL/XSS-style payloads, path traversal, command injection, email, and usernames.

### Web Crawler
- Crawler service with URL crawling, runner, jobs, pages, rejected URL tracking, and database integration.
- Robots.txt compliance, sitemap parsing, RSS/Atom feed parsing, DNS cache, circuit breaker, adaptive rate limiting, retry handling, and user-agent rotation.
- Persistent Redis-backed job queue with distributed locks, priority scheduling, orphan recovery, retries, and stats.
- Memory-optimized crawl state with Bloom filter, LRU cache, bounded queue, and Redis spillover.
- Content storage with compression, deduplication, metadata extraction, full-text search support, and language/content-type filtering.
- Configurable crawl jobs with max depth, whitelist/blacklist, regex filters, cron-style schedules, priorities, tags, custom headers, and presets.
- REST crawler management API with CRUD, start/stop/restart, pause/resume, queue management, stats, metrics, and preset endpoints.
- Prometheus crawler metrics, webhook notifications, error categorization, data export, benchmarks, and crawler test suites.

### Storage and Files
- File storage trait layer with list/upload/download/delete/folder/move/copy/exists/metadata/search/share/version/batch/stream/sync operations.
- Local filesystem storage provider.
- Dropbox integration with OAuth, folder creation, listing, download, delete, and auth helpers.
- Nextcloud and SeaweedFS provider modules.
- Memory/storage modules for database-backed file records.

### Backup and Recovery
- Legacy backup service plus enhanced backup implementation.
- Full and incremental backups, restore flow, verification, compression, checksums, metadata, retention policies, restore points, metrics, and tests.
- Backup scheduler entry point.

### AI, Voice, and Language
- Whisper/STT modules, enhanced Whisper service configuration, installation helpers, and model download/build workflow.
- TTS modules with enhanced, external, and legacy paths.
- Voice assistant service with conversation history, command processing, and TTS/STT integration points.
- RiveScript service and bundled brain/scripts.
- LLM services for OpenAI, Llama, and Ollama.
- Ollama HTTP API endpoints and coding-agent Ollama auto-configuration.

### Coding Agent
- Coding agent service, executor, config, and TUI rendering support.
- Workspace context analysis, project detection, dependency scanning, git context, and session context.
- Code completion, review, explanation, refactoring, automated debugging, security analysis, migration, pair programming, collaboration, templates, code-flow visualization, bug prediction, and documentation generation modules.
- Provider abstraction with Ollama, OpenAI, and local provider modules.

### Home Automation and Devices
- LIFX service with configuration, protocol handling, discovery, bulbs, handlers, traits, and API servers.
- Touch-friendly LIFX frontend controls and media/LIFX integration assets.
- Matter and mDNS service modules.
- Memory models for humans, face encodings, notifications, locations, rooms, things, observations, and observation objects.

### Media and Entertainment
- Media service modules for games, images, YouTube, Snapcast, and Snapcast API.
- Image-processing modules including OCNN, SRGAN, and neural style transfer.
- Spotify, sound, speaker recognition, RTSP download/manager/recording, and camera/observation support modules.
- Bundled media packages and app icons for Netflix, Spotify, YouTube, console, games, FFmpeg, Snapcast, Whisper, and object-recognition assets.

### Network, Remote Access, and Security Tools
- SSH client/server modules with remote command server support.
- Remote access helper script and documented telnet/netcat/SSH tunnel workflows.
- Vulnerability scanner with network scanning, port scanning, service detection, OS hints, vulnerability classification, and report generation.
- ClamAV service module and resource-management virus-scan integration point.
- P2P modules for enhanced nodes, secure messaging, file sharing, sync, network segmentation, rate limits, peer stats, and tests.

### Notifications and Communications
- Notification service modules for channels, rules, and HTTP handling.
- SMS service modules for Plivo, Twilio, and Vonage.
- Event service module.

### Package Management and Platform Support
- Package manager modules for Cargo, pip, apt, dnf, yum, pacman, zypper, Homebrew, MacPorts, Chocolatey, and winget.
- vcpkg support.
- Linux/macOS/Windows-specific installer and package-management scaffolding.

### Plugins and Extensibility
- Plugin trait and registry for compiled-in plugins.
- Feature-gated WASM plugin runtime and hot-reload loader modules.
- Plugin manifest support.

### Testing, Benchmarks, and Documentation
- Unit, integration, functional, binary-interface, JWT security, DB pool safety, Redis safety, WebSocket build, telemetry, crawler, RTSP, LIFX exhaustion, and command tests.
- Benchmarks for services and crawler.
- Organized docs for API, deployment, development, features, security, design, directory structure, CapRover, database setup, Snapcast, LIFX, NST, Sentry, SSH, thread manager, resource management, cache, migrations, Redis refactor, crawler enhancements, and test reports.

## 🚨 Security Notice

**THIS SOFTWARE IS IN ACTIVE DEVELOPMENT AND NOT PRODUCTION READY**

Before using S.A.M., please review the [SECURITY.md](docs/security/SECURITY.md) file for important security considerations. Recent security fixes include:

- ✅ Fixed critical command injection vulnerabilities
- ✅ Fixed SQL injection prevention
- ✅ Fixed application crash from network errors
- 🔄 Ongoing work to replace unsafe `.unwrap()` calls

## 🛠️ Installation & Setup

### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **PostgreSQL 13+** - Required for data storage
- **Redis** (optional) - For caching and session management
- **Docker** (optional) - For containerized services
- **Python 3.8+** - For AI/ML components
- **FFmpeg** - For media processing

### Quick Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-org/sam.git
   cd sam
   ```

2. **Install system dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt update
   sudo apt install postgresql-client libpq-dev redis-server ffmpeg python3-pip

   # macOS
   brew install postgresql redis ffmpeg python3
   
   # Arch Linux
   sudo pacman -S postgresql-libs redis ffmpeg python
   ```

3. **Setup database**
   ```bash
   # Create PostgreSQL database
   createdb sam_db
   
   # Set environment variables
   export DATABASE_URL="postgresql://username:password@localhost/sam_db"
   export REDIS_URL="redis://localhost:6379"
   ```

4. **Build and run**
   ```bash
   cargo build --release
   ./target/release/sam --help
   ```

### Configuration

Create `/opt/sam/config.json` with your settings:
```json
{
  "database_url": "postgresql://localhost/sam_db",
  "redis_url": "redis://localhost:6379",
  "http_port": 8000,
  "enable_tts": true,
  "enable_stt": true,
  "log_level": "info"
}
```

### Development Setup

1. **Run installer (development mode)**
   ```bash
   cargo run --bin installer -- --dev-setup
   ```

2. **Start development server**
   ```bash
   cargo run -- serve --dev
   ```

3. **Run tests**
   ```bash
   cargo test
   ```

## 🚢 CapRover Deployment

CapRover is a Platform-as-a-Service (PaaS) solution that makes deploying applications easy. S.A.M. supports CapRover as a rapid local development and testing target, especially when iterating on the web UI, API surface, and service wiring.

S.A.M.'s primary deployment target is a direct Linux host install. In direct host mode, S.A.M. can use the full feature set: orchestrating local containers, discovering and communicating with LAN devices and services, controlling LIFX through offline LAN APIs, integrating with Snapcast, and managing local runtime dependencies directly. CapRover mode is intentionally narrower. When `CAPROVER=true`, local Docker orchestration should be hidden or disabled in the UI, and infrastructure such as Redis and PostgreSQL should be reached through CapRover-managed apps, external service URLs, or the CapRover API rather than being treated as local Docker resources.

### Prerequisites for CapRover Deployment

1. **CapRover Server** - Set up your CapRover instance following the [official documentation](https://caprover.com/docs/get-started.html)
2. **CapRover CLI** - Install the CapRover command line tool:
   ```bash
   npm install -g caprover
   ```
3. **Git Repository** - Your S.A.M. code should be in a Git repository

### Deployment Methods

#### Method 1: Direct Git Deployment (Recommended)

1. **Login to your CapRover instance**
   ```bash
   caprover login
   ```

2. **Create a new app in CapRover**
   - Log into your CapRover web interface
   - Go to "Apps" → "Create New App"
   - Enter app name: `sam` (or your preferred name)
   - Click "Create New App"

3. **Configure environment variables**
   In your CapRover app settings, add these environment variables:
   ```bash
   # Database Configuration
   DATABASE_ENGINE=sqlite
   SQLITE_DATABASE_PATH=/var/lib/sam/sam.db
   
   # Optional: Use PostgreSQL instead
   # DATABASE_ENGINE=postgresql
   # DATABASE_URL=postgresql://user:pass@hostname:5432/sam_db
   
   # Redis Configuration (optional)
   REDIS_URL=redis://srv-captain--sam-redis:6379
   # REDIS_DISABLED=false

   # Voice companion service
   TTS_URL=http://srv-captain--sam-voice:8002/tts
   STT_URL=http://srv-captain--sam-voice:8002/stt

   # SeaweedFS companion services
   SEAWEEDFS_MASTER_URL=http://srv-captain--sam-seaweed-master:9333
   SEAWEEDFS_VOLUME_URL=http://srv-captain--sam-seaweed-volume:8080
   SEAWEEDFS_FILER_URL=http://srv-captain--sam-seaweed-filer:8888
   
   # Application Configuration
   PORT=8000
   SAM_HOME=/app
   SAM_DATA=/var/lib/sam
   SAM_LOGS=/var/log/sam
   RUST_LOG=info
   RUST_BACKTRACE=1
   
   # Migration Settings
   RUN_MIGRATIONS=true
   ```

4. **Deploy using Git**
   ```bash
   # In your S.A.M. project directory
   caprover deploy --appName sam
   ```

5. **Enable HTTPS and configure domain**
   - In CapRover web interface, go to your app
   - Under "HTTP Settings": 
     - Enable HTTPS
     - Configure your domain (e.g., `sam.yourdomain.com`)
     - Force HTTPS redirect

#### Method 2: Dockerfile Deployment

1. **Create your app in CapRover**
2. **Use the "Deploy via Image Name" option**
   - Build image locally: `docker build -t sam:latest .`
   - Push to registry or use CapRover's built-in deployment

#### Method 3: Upload tar file

1. **Create deployment package**
   ```bash
   # Create a tar file excluding unnecessary files
   tar --exclude='target' --exclude='.git' --exclude='node_modules' \
       -czf sam-deployment.tar.gz .
   ```

2. **Upload via CapRover web interface**
   - Go to your app in CapRover
   - Use "Deploy via Tarball" option
   - Upload `sam-deployment.tar.gz`

### Post-Deployment Configuration

#### Persistent Volumes

Configure persistent volumes for data storage:

1. In CapRover app settings, go to "App Configs"
2. Add persistent directories:
   ```
   /var/lib/sam → sam-data
   /var/log/sam → sam-logs
   ```

#### Database Setup

**For SQLite (Default):**
- No additional setup needed
- Database will be created automatically on first run

**For PostgreSQL:**
1. Create a PostgreSQL app in CapRover or use external service
2. Update the `DATABASE_URL` environment variable
3. Enable `RUN_MIGRATIONS=true`

#### Health Checks

CapRover will automatically use the Docker HEALTHCHECK. S.A.M. provides these endpoints:
- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe  
- `GET /health` - Basic health check

### Production Recommendations

#### Security
```bash
# Environment variables for production
RUST_LOG=warn
RUST_BACKTRACE=0
DATABASE_ENGINE=postgresql  # Recommended for production
```

#### Performance
- **CPU**: Minimum 1 CPU, recommended 2+ CPUs
- **RAM**: Minimum 1GB, recommended 2GB+ for AI features
- **Storage**: At least 10GB for logs, database, and assets

#### Monitoring
Enable CapRover's built-in monitoring or integrate with external services:
- **Metrics**: Prometheus endpoints available at `/metrics`
- **Logs**: Structured logging with JSON format
- **Health**: Built-in health check endpoints

### Scaling

#### Horizontal Scaling
```bash
# Scale to multiple instances
caprover api --path "/api/v2/user/apps/data/sam" --method "POST" \
  --data '{"instanceCount": 3}'
```

#### Load Balancing
CapRover automatically handles load balancing between instances.

### Troubleshooting

#### Common Issues

1. **Build Failures**
   ```bash
   # Check build logs
   caprover logs --app sam --lines 100
   
   # Common solutions:
   # - Increase build timeout in CapRover settings
   # - Check Dockerfile syntax
   # - Verify all required files are included
   ```

2. **Database Connection Issues**
   ```bash
   # Check environment variables
   caprover api --path "/api/v2/user/apps/data/sam" --method "GET"
   
   # Test database connectivity
   caprover exec --app sam --command "pg_isready -d $DATABASE_URL"
   ```

3. **Memory Issues**
   ```bash
   # Increase app memory limit in CapRover settings
   # Monitor memory usage
   caprover stats --app sam
   ```

#### Logs and Debugging
```bash
# View real-time logs
caprover logs --app sam --follow

# View recent logs
caprover logs --app sam --lines 1000

# Execute commands in container
caprover exec --app sam --command "/bin/bash"
```

### Example Production Deployment Script

```bash
#!/bin/bash
set -e

APP_NAME="sam"
DOMAIN="sam.yourdomain.com"

echo "Deploying S.A.M. to CapRover..."

# Login to CapRover
caprover login

# Deploy application
caprover deploy --appName "$APP_NAME"

# Wait for deployment
echo "Waiting for deployment to complete..."
sleep 30

# Check health
curl -f "https://$DOMAIN/health" || {
    echo "Health check failed!"
    caprover logs --app "$APP_NAME" --lines 50
    exit 1
}

echo "✅ S.A.M. deployed successfully!"
echo "🌐 Access your application at: https://$DOMAIN"
```

### Environment-Specific Configurations

#### Development
```bash
RUST_LOG=debug
RUST_BACKTRACE=full
DATABASE_ENGINE=sqlite
RUN_MIGRATIONS=true
```

#### Staging
```bash
RUST_LOG=info
RUST_BACKTRACE=1
DATABASE_ENGINE=postgresql
RUN_MIGRATIONS=true
```

#### Production
```bash
RUST_LOG=warn
RUST_BACKTRACE=0
DATABASE_ENGINE=postgresql
RUN_MIGRATIONS=false  # Run migrations manually
```

### Local CapRover Testing

To test your CapRover deployment locally before deploying:

```bash
# Test the CapRover-optimized configuration locally
docker-compose -f deploy/docker-compose.caprover.yml up

# Include management tools (Redis Commander, pgAdmin)
docker-compose -f deploy/docker-compose.caprover.yml --profile tools up

# Test just the S.A.M. application
docker-compose -f deploy/docker-compose.caprover.yml up sam
```

This will spin up S.A.M. with the same configuration and environment as CapRover deployment.

For detailed CapRover documentation, visit: https://caprover.com/docs/

## 📁 Project Structure

This project follows an organized directory structure for better maintainability:

```
├── docs/                    # 📚 All documentation (organized by category)
│   ├── api/                # API documentation  
│   ├── deployment/         # Deployment guides
│   ├── development/        # Technical implementation docs
│   ├── features/           # Feature descriptions
│   └── security/           # Security guides and fixes
├── deploy/                 # 🚢 Deployment configurations
│   ├── docker-compose*.yml # Docker Compose files
│   ├── Dockerfile*         # Docker build files
│   └── docker-entrypoint.sh
├── config/                 # ⚙️ Development configuration
├── scripts/                # 🔧 Application scripts
├── tools/                  # 🛠️ Development tools
├── tests/                  # 🧪 All test files and utilities
├── src/                    # 💻 Rust source code
└── www/                    # 🌐 Web frontend
```

For detailed information about the directory structure, see [docs/DIRECTORY_STRUCTURE.md](docs/DIRECTORY_STRUCTURE.md).

## Partial Work and Roadmap

These are active or incomplete areas discovered from the README, docs, TODO comments, and code placeholders.

### Security & Stability (High Priority)
- 🔄 Replace remaining `.unwrap()` calls with proper error handling
- 🔄 Audit and fix remaining command injection vectors
- ⏳ Add comprehensive input validation
- ⏳ Implement proper session management
- ⏳ Add rate limiting and DOS protection
- Finish unsafe-block and memory-safety audit.
- Finish SQL-security audit and parameterization pass.
- Fix resource leaks in backup, SSH, and P2P/file-sharing paths.

### Core Features
- Harden WebSocket lifecycle management, including graceful shutdown for background tasks and deployment configuration.
- Complete service orchestrator health monitoring, restart execution, and service-specific handlers for all registered service variants.
- Integrate Whisper as a primary STT/TTS engine https://github.com/ggerganov/whisper.cpp
- Use Whisper for realtime STT/TTS using wasm2js.
- Keep existing TTS/STT methods as fallback paths.
- Implement STT HTTP API handling, DeepSpeech path, language detection, and configurable local/external STT servers.
- Finish local/Coqui TTS integration.
- Add metadata to file storage API.
- SSH command pipeline support for cli
- Finish notification history and notification settings UI.
- Finish SMS support and additional notification providers such as email/push.
- Complete cache databases as Redis/PostgreSQL hybrid.
- Add support for additional database backends such as MySQL where abstractions exist but implementation is missing.
- Add support for additional storage backends such as S3 and Google Cloud Storage.
- Create an OID for SAM on server startup with root-only access.
- Bootcamp service for collected prompts/data and model training.
- Revise default RiveScript with bootcamp prompts.
- Extend thing/device support to more device types and platforms.
- Complete P2P job tasking, hive communication, state sync, load reporting, and key-agreement work.
- Finish backup encryption.

### Platform Support
- Stablize windows build
- Mobile App
- Docker containerization

### UI/UX Improvements  
- Overhaul web interface for no jquery, gulp asset pipelines, etc
- Overhaul help command
- Finish TUI file browser and database management modes.
- GUI and API overhaul.

### Gaming & Emulation
- Complete PS1 emulation and native file support (.ps1) https://github.com/js-emulators/WASMpsx
- NES emulation and native file support (.nes) https://github.com/takahirox/nes-rust
- Gameboy emulation https://github.com/andrewimm/wasm-gb
- Chip-8 Emulation (.ch8)

### Crawler Future Work
- Implement focused/topical crawling with ML classification.
- Add link graph analysis and visualization.
- Support crawling API endpoints, not just HTML.
- Add archive.org integration for historical snapshots.
- Implement distributed crawling across multiple nodes.
- Add machine-learning crawl prioritization.
- Create browser extension for manual URL submission.
- Add Tor/proxy crawling support.
- Implement change detection for recrawling.
- Add screenshot capture and crawl replay.
- Finish full PDF/document extraction, image metadata extraction, and non-Chrome JS renderers.

### Coding Agent Future Work
- Replace placeholder OpenAI/local provider responses with real inference calls where still stubbed.
- Finish PDF generation in documentation generator.
- Finish unimplemented visualization types and GPU providers.
- Complete IO module implementation.

0.0.4(WIP):
- database restructured
- crawler for deep web research
- docker, redis, postgres installer workflow for automated setup


0.0.3:
- Fix file browser (dropbox, deleting files, moving files, etc.)
- Add support for visual RSTP observations
- amd64 support for ffmpeg and whisper packages
- Copy fonts file to sam directory during setup (DONE)
- Finish package installer (search, install, uninstall)
- Fix settings to actually do something
- Redesign humans page with avatar support
- Fix tracker for heard_count
- Add ability to correct observations in the observation deck
- Review build sprec code
- Redesign notifications to be instant when initiadted from the client side
- Link web session microphone to new sound pipeline s1,s2,s3
- Associate observations with things and/or web sessions
- Redesign locations UIX
- Build calendar/clock widget


AI Features to Expolore:
- Image Super Resoloution
- Minst handwriting
- Speech recognition (DONE)
- Speaker recognition (DONE)
- Deep Vision
- GAN art generation
- Ability to generate reports on any topic
- Ability to sumerize news stories and prioritize feed based on user preferences


Socket API:
- Notifications
- Ability to launch web apps/files on individual devices (web->socket sesssions)

Weather API:
- Uses in house server package "Jupiter" to generate weather reports for your zip code

Console:
- whoami: returns current human users name
- whereami: returns current human users location

News API:
- Copy RSS feed comsuption code from BOT
- Use rust-bert to generate summaries for articles

Stonks API:
- Ability to track blockchain prices
- Ability to track stock prices
- Ability to track multiple wallet realtime values using public keys
- Ability to set high/low notifications

Notifications API:
- Store in SQL
- User Specific
- Can be location specific, but not necessarily
- Ability to send notifications via email or SMS after X minutes of being unread

Storage API:
- Ability to store files in SQL, Cloud Storage, etc. (WIP)

Media Center:
- Controller Support (DONE)
- Music (Depends on new storage pipeline)
- Movies (Depends on new storage pipeline)
- Make games packages that can be installed into the storage pipeline

Settings:
- Ability to set audio recognition noise threshold (WIP)
- Ability to set custom STT server (WIP)

Speech pipeline redesign:
- Recode web recorder to store wav files in the stream pipeline instead of running stt in the browser
- Stitch continuous speech wav file streams and trim whitenoise (DONEish)
- Perform STT using API from settings
- Use sprec to identify the speaker
- Store speech as an observation 
- Split pipeline stream by thing_oid:xxx AND web_session:xxx

Remote Device Management API:
- Ability to execute remote commands over an ssh tunnel connection
- Installs remote utility which gathers system information
- Very useful for keeping multiple touch screen panels, servers updated

Network Security API:
- Scans local network for security vulnerabilities and exposed ports
- Allows sam to make recomendations for incresing network security

#### To Override DNS:
{
  "TaskTemplate": {
      "DNSConfig": {
        "Nameservers": [
          "172.16.0.15"
        ]
      }
    }
}
