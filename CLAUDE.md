# SAM (Smart Artificial Mind) - AI Assistant Guide

## Project Overview
SAM is a comprehensive home automation and AI assistant platform written in Rust. It features a web dashboard, WebSocket real-time communication, service orchestration, and a terminal user interface (TUI). The system is designed for modularity, scalability, and real-time performance with support for distributed deployment and various AI integrations.

## Quick Start Commands

### Development
```bash
# Build the project
cargo build

# Run in interactive TUI mode
cargo run

# Run in server mode (no TUI, for production)
cargo run serve

# Run tests
cargo test

# Check for compilation errors
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Database Setup
```bash
# Fix PostgreSQL permissions (if needed)
./fix_postgres_permissions.sh

# Or manually:
psql -U $USER -c "ALTER USER sam CREATEDB;"
psql -U $USER -c "CREATE DATABASE sam OWNER sam;"
```

## Project Structure

```
/Sam/
├── src/
│   ├── main.rs                    # Entry point with Tokio runtime configuration
│   ├── sam.rs                     # Module declarations
│   ├── test_runner.rs            # Test utilities
│   ├── bin/                      # Binary executables
│   │   └── installer/            # Installation utilities
│   ├── lib/                      # Library code (libsam)
│   │   └── mod.rs               # Core library functionality
│   └── sam/
│       ├── cli/                  # Command-line interface
│       │   ├── tui.rs           # Terminal UI (ratatui-based)
│       │   ├── commands.rs      # CLI command handlers
│       │   └── helpers.rs       # CLI utilities
│       ├── http/                 # HTTP server components
│       │   ├── mod.rs           # Main HTTP server
│       │   └── api/             # REST API endpoints
│       │       ├── service_control.rs
│       │       └── io.rs        # I/O operations
│       ├── websocket/           # WebSocket implementation
│       │   ├── mod.rs           # WebSocket server & message handling
│       │   ├── security.rs      # Rate limiting, auth, audit logging
│       │   ├── error.rs         # Error types and handling
│       │   └── tests.rs         # WebSocket unit tests
│       ├── services/            # Service modules (60+ services)
│       │   ├── mod.rs           # Service registry
│       │   ├── redis.rs         # Redis with circuit breaker
│       │   ├── docker.rs        # Docker container orchestration
│       │   ├── crawler.rs       # Web scraping service
│       │   ├── rivescript.rs   # AI conversation engine
│       │   ├── openai.rs        # OpenAI integration
│       │   ├── llama.rs         # Local LLM support
│       │   ├── github.rs        # GitHub API integration
│       │   ├── spotify.rs       # Music streaming control
│       │   ├── lifx.rs          # Smart lighting control
│       │   ├── matter.rs        # Matter protocol support
│       │   ├── mdns.rs          # Service discovery
│       │   ├── ssh.rs           # SSH client operations
│       │   ├── backup.rs        # Backup management
│       │   ├── monitoring.rs    # System monitoring
│       │   ├── cache.rs         # Hybrid caching layer
│       │   ├── database.rs      # Database abstractions
│       │   ├── orchestrator.rs  # Service orchestration
│       │   ├── environment.rs   # Environment configuration
│       │   ├── error_handling.rs # Circuit breakers & retry logic
│       │   └── thread_manager.rs # Thread pool management
│       ├── memory/              # Data persistence
│       │   ├── config/          # Database configurations
│       │   └── mod.rs           # Memory/database operations
│       ├── db/                  # Database implementations
│       ├── jobs/                # Background job processing
│       ├── logging/             # Logging infrastructure
│       ├── models/              # Data models
│       ├── resource_management/ # Resource monitoring
│       ├── security/            # Security modules
│       ├── network_monitor.rs   # Network monitoring
│       ├── network_config.rs    # Network configuration
│       └── tools.rs             # Utility functions
├── www/                         # Web dashboard
│   ├── index.html              # Main dashboard UI
│   └── assets/
│       ├── js/
│       │   └── dashboard-service-control.js  # WebSocket client
│       └── vendor/             # Third-party assets
├── deploy/                      # Deployment configurations
│   ├── docker-compose.yml      # Production Docker setup
│   ├── docker-compose.caprover.yml  # CapRover deployment
│   ├── docker-compose.sqlite.yml    # SQLite variant
│   ├── Dockerfile              # Container build
│   └── nginx.conf              # Nginx configuration
├── config/                      # Configuration files
│   ├── TaskTemplate.yml        # Task templates
│   └── tarpaulin.toml         # Code coverage config
├── benches/                     # Performance benchmarks
├── scripts/                     # Utility scripts
├── nginx-websocket.conf        # WebSocket proxy config
└── fix_postgres_permissions.sh # Database setup helper
```

## Key Components

### 1. HTTP Server (Port 8000)
- Built with Rouille framework for high performance
- Serves web dashboard and static files from `/www`
- RESTful API endpoints for service control
- Session-based authentication
- Health check endpoints: `/health`, `/health/detailed`
- CORS support for cross-origin requests
- File upload/download capabilities
- API routes under `/api/` prefix

### 2. WebSocket Server (Port 8080)
- Tokio-based async implementation using tokio-tungstenite
- Real-time bidirectional communication
- Message types: Subscribe, Command, Heartbeat, ServiceStatus, SystemStats
- Built-in security features:
  - Rate limiting (100 msgs/min per client)
  - Connection limits (20 per IP)
  - Message size validation (max 1MB)
  - Audit logging to `websocket_audit.log`
- Circuit breaker pattern for resilient connections
- Automatic reconnection with exponential backoff
- Channel-based pub/sub system

### 3. Services Architecture

#### Core Infrastructure Services
- **Redis**: High-performance cache with circuit breaker, connection pooling via deadpool-redis
- **PostgreSQL**: Primary database with migrations, connection pooling via deadpool-postgres
- **Docker**: Container orchestration using Bollard library
- **Thread Manager**: Custom thread pool implementation with work stealing

#### AI & Automation Services
- **RiveScript**: Rule-based conversation engine (Python integration)
- **OpenAI**: GPT integration for advanced AI capabilities
- **Llama**: Local LLM support for offline AI
- **Whisper**: Speech-to-text using whisper-rs
- **Voice/TTS**: Text-to-speech synthesis

#### Home Automation
- **LIFX**: Smart lighting control via lifx-rs
- **Matter**: Matter protocol support for IoT devices
- **mDNS**: Service discovery on local network

#### Developer Tools
- **GitHub**: Repository management and CI/CD
- **Git**: Version control operations
- **SSH**: Remote server management
- **Copilot**: AI code assistance integration

#### Media Services
- **Spotify**: Music streaming control
- **YouTube**: Video service integration (via rustube)
- **Dropbox**: Cloud storage integration

#### Security & Monitoring
- **ClamAV**: Antivirus scanning
- **Vulnerability Scanner**: Security auditing
- **Network Monitor**: Real-time network statistics
- **System Monitor**: CPU, memory, disk monitoring via sysinfo
- **Backup Services**: Automated backup with encryption

### 4. Terminal UI (TUI)
- Built with ratatui and crossterm
- Navigation modes (F1-F7):
  - F1: Command mode (default)
  - F2: Services management
  - F3: Log viewer with filtering
  - F4: System information
  - F5: Database management
  - F6: File browser
  - F7: Help screen
- Real-time updates using channels
- Service status indicators with color coding
- Resource usage graphs
- Interactive command prompt

## Environment Variables

### Required for Production
```bash
# Database
PG_USER=sam
PG_PASS=sam
PG_ADDRESS=localhost
PG_DBNAME=sam

# CapRover deployment
CAPROVER=true           # Set to true when deployed via CapRover
PORT=8000               # HTTP server port

# External services (when CAPROVER=true)
REDIS_URL=redis://srv-captain--sam-redis:6379
POSTGRES_URL=postgresql://sam:sam@srv-captain--sam-db:5432/sam
```

### Optional
```bash
DATABASE_ENGINE=postgres  # or 'sqlite' for lightweight deployment
JWT_SECRET=your-secret   # For WebSocket authentication
```

## WebSocket Protocol

### Client → Server Messages
```javascript
// Subscribe to channels
{ type: 'subscribe', channels: ['services', 'stats', 'alerts'] }

// Service control
{ type: 'command', id: 'unique-id', command: 'start_service', args: { service: 'redis' } }
{ type: 'command', id: 'unique-id', command: 'stop_service', args: { service: 'redis' } }
{ type: 'command', id: 'unique-id', command: 'restart_service', args: { service: 'redis' } }

// Get status
{ type: 'command', id: 'unique-id', command: 'get_services', args: {} }

// Heartbeat
{ type: 'heartbeat', timestamp: 1234567890 }
```

### Server → Client Messages
```javascript
// Service status update
{ type: 'service_status', service: 'redis', status: { state: 'healthy', message: 'Running' } }

// Command response
{ type: 'command_response', id: 'unique-id', success: true, data: { message: 'Service started' } }

// System statistics
{ type: 'system_stats', stats: { cpu: 45.2, memory_percent: 60.5, ... } }

// Activity log
{ type: 'activity', activity: { message: 'Connected to server', ... } }
```

## Common Development Tasks

### Adding a New Service
1. Create service module in `src/sam/services/`
2. Implement the service trait with required methods:
   ```rust
   pub async fn start() -> Result<()>
   pub async fn stop() -> Result<()>
   pub async fn is_running() -> bool
   pub async fn health_check() -> ServiceStatus
   ```
3. Add error types in the service module using `thiserror`
4. Register in `src/sam/services/mod.rs`
5. Add WebSocket command handler in `src/sam/websocket/mod.rs`:
   - Add to `handle_command()` match statement
   - Create response message type if needed
6. Add REST API endpoints in `src/sam/http/api/service_control.rs`
7. Update dashboard UI in `www/index.html`:
   - Add service card
   - Wire up control buttons
   - Add to WebSocket message handling
8. Add TUI support in `src/sam/cli/tui.rs` if applicable
9. Write unit tests in service module
10. Update documentation

### Debugging WebSocket Issues
1. Check browser console for connection logs
2. Look for server logs with `[INFO] WebSocket` prefix
3. Verify port 8080 is accessible
4. Check connection limit (max 20 per IP in dev)
5. Ensure heartbeat is being sent (every 30 seconds)

### Database Migrations
- Tables are created automatically on startup
- Schema defined in `src/sam/memory/config/mod.rs`
- Manual migrations: Connect to PostgreSQL and run SQL

### Production Deployment (CapRover)
1. Set `CAPROVER=true` environment variable
2. Configure external Redis/PostgreSQL URLs
3. Set up Nginx reverse proxy (see `nginx-websocket.conf`)
4. Ensure WebSocket endpoint `/ws` is proxied to port 8080

## Known Issues & Solutions

### Issue: TUI shows no logs
**Solution**: Ensure `env_logger::init()` is not called in TUI mode (only in serve mode)

### Issue: WebSocket commands not working
**Solution**: Check permissions in `src/sam/websocket/security.rs` - service commands are allowed without auth

### Issue: Service status shows "unknown"
**Solution**: Ensure services are properly initialized and status check functions are implemented

### Issue: PostgreSQL permission denied
**Solution**: Run `./fix_postgres_permissions.sh` or manually grant CREATEDB permission

### Issue: Too many WebSocket connections
**Solution**: Connection limit is 20 per IP. Restart server or increase limit in `security.rs`

## Testing Checklist

Before committing changes:
- [ ] Run `cargo build` - No compilation errors
- [ ] Run `cargo clippy` - Address all warnings
- [ ] Run `cargo fmt` - Format code consistently
- [ ] Run `cargo test` - All tests pass
- [ ] Test TUI mode - Services show correct status
- [ ] Test dashboard - WebSocket connects and controls work
- [ ] Check browser console - No JavaScript errors
- [ ] Verify API endpoints - Return JSON, not HTML for errors
- [ ] Check memory usage - No leaks in long-running operations
- [ ] Verify error handling - Graceful failures
- [ ] Test with both PostgreSQL and SQLite backends
- [ ] Verify Docker deployment works

## Code Style Guidelines

1. **Error Handling**: Use `Result<T, Error>` types, avoid panics
2. **Async Code**: Use `tokio` for async operations
3. **Logging**: Use `log` macros (`info!`, `error!`, `warn!`, `debug!`)
4. **WebSocket**: All commands should return responses
5. **Security**: Validate all inputs, use rate limiting
6. **Documentation**: Add comments for complex logic

## Contact & Resources

- **Project**: SAM (Smart Artificial Mind)
- **Version**: 0.0.2
- **License**: GPLv3
- **Copyright**: 2021-2026 The Open Sam Foundation (OSF)
- **Developer**: Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
- **Repository**: [GitHub URL if available]
- **Issues**: Report bugs and feature requests in GitHub Issues

## Useful Commands for Debugging

```bash
# Watch service logs in real-time
tail -f /var/log/sam_panic.log

# Check PostgreSQL connection
psql -U sam -d sam -c "SELECT 1;"

# Test WebSocket connection
websocat ws://localhost:8080/ws

# Check service status via API
curl http://localhost:8000/api/services/redis/status

# Monitor system resources
htop

# Check Docker containers
docker ps

# View Redis data
redis-cli
```

## Architecture Decisions

1. **Rust**: Chosen for performance, memory safety, and system-level control
2. **Tokio**: Multi-threaded async runtime for handling concurrent operations
   - Custom runtime configuration with CPU cores + 2 worker threads
   - 4MB thread stack size for complex operations
3. **WebSocket + HTTP**: Dual protocol architecture
   - HTTP for RESTful APIs and static content
   - WebSocket for real-time bidirectional communication
4. **TUI (Ratatui)**: Terminal interface for server administration
   - Non-blocking event loop
   - Channel-based communication with services
5. **Docker & Bollard**: Container orchestration
   - Programmatic Docker control
   - Service isolation and deployment
6. **Database Strategy**:
   - PostgreSQL: Primary database with JSONB support
   - SQLite: Lightweight alternative for edge deployments
   - Redis: High-performance cache and session storage
7. **Error Handling**:
   - Circuit breaker pattern for external services
   - Exponential backoff retry logic
   - Comprehensive error types with thiserror
8. **Security**:
   - Rate limiting and connection limits
   - JWT-based authentication
   - Audit logging for compliance
9. **Monitoring**:
   - Prometheus metrics export
   - OpenTelemetry tracing
   - Custom health check endpoints

## Dependencies Overview

### Core Runtime
- `tokio`: Async runtime with full features
- `futures`: Async primitives and utilities
- `async-trait`: Async trait support

### Web & Networking
- `rouille`: HTTP server framework
- `tokio-tungstenite`: WebSocket support
- `reqwest`: HTTP client with rustls
- `trust-dns-resolver`: DNS resolution

### Database
- `tokio-postgres`: Async PostgreSQL driver
- `deadpool-postgres`: Connection pooling
- `rusqlite`: SQLite with bundled engine
- `diesel`: ORM with migrations
- `deadpool-redis`: Redis connection pooling

### Security
- `argon2`: Password hashing
- `jsonwebtoken`: JWT authentication
- `ring`: Cryptographic operations
- `rustls`: TLS implementation
- `aes-gcm`: Authenticated encryption

### AI & ML
- `whisper-rs`: Speech recognition
- Integration points for OpenAI, Llama

### Monitoring
- `prometheus`: Metrics collection
- `opentelemetry`: Distributed tracing
- `tracing`: Structured logging
- `sysinfo`: System monitoring

### UI
- `ratatui`: Terminal UI framework
- `crossterm`: Terminal manipulation
- `tui-logger`: TUI log widget

## Performance Optimizations

1. **Connection Pooling**: All database connections use pooling
2. **Circuit Breakers**: Prevent cascading failures
3. **Caching Strategy**: Multi-tier cache (memory -> Redis -> DB)
4. **Async Everything**: Non-blocking I/O throughout
5. **Work Stealing**: Thread pool with work stealing queue
6. **Lazy Loading**: Services initialized on-demand
7. **Message Batching**: WebSocket messages batched for efficiency

## Future Improvements

### In Progress
- [ ] Complete voice/TTS service implementation
- [ ] Extend RiveScript with bootcamp prompts
- [ ] Whisper.cpp integration
- [ ] GUI and API overhaul

### Planned Features
- [ ] Mobile app (iOS/Android)
- [ ] Enhanced user authentication with RBAC
- [ ] Service auto-recovery with health checks
- [ ] Distributed deployment with consensus
- [ ] Plugin system for third-party extensions
- [ ] GraphQL API alongside REST
- [ ] Kubernetes operator for cloud deployment
- [ ] Enhanced ML capabilities with local models
- [ ] Data goblin apps (recipe, shopping, calendar)
- [ ] Extended IoT device support
- [ ] Real-time video processing
- [ ] Blockchain integration for audit trails

---

## Troubleshooting Guide

### Build Issues

#### OpenSSL/LibSSL Errors
```bash
# macOS
brew install openssl@3
export OPENSSL_DIR=$(brew --prefix openssl@3)

# Linux
sudo apt-get install libssl-dev pkg-config
```

#### Whisper/GGML Build Failures
```bash
# Install build dependencies
cargo clean
cargo build --release
```

### Runtime Issues

#### Port Already in Use
```bash
# Find process using port
lsof -i :8000  # HTTP
lsof -i :8080  # WebSocket

# Kill process
kill -9 <PID>
```

#### Database Connection Failed
```bash
# Check PostgreSQL status
pg_isready -h localhost -p 5432

# Fix permissions
./fix_postgres_permissions.sh

# Verify connection
psql -U sam -d sam -c "SELECT version();"
```

#### WebSocket Connection Drops
- Check firewall rules for port 8080
- Verify Nginx proxy configuration
- Check client-side heartbeat implementation
- Review rate limiting settings

#### High Memory Usage
- Check for connection leaks
- Review cache size limits
- Monitor thread pool size
- Enable memory profiling

## Security Best Practices

1. **Never commit secrets**: Use environment variables
2. **Validate all inputs**: Prevent injection attacks
3. **Use HTTPS in production**: Configure TLS certificates
4. **Implement rate limiting**: Prevent DoS attacks
5. **Audit log sensitive operations**: Track access
6. **Regular dependency updates**: `cargo update`
7. **Use least privilege**: Limit service permissions
8. **Encrypt sensitive data**: Use AES-GCM for storage
9. **Implement CORS properly**: Restrict origins
10. **Monitor for vulnerabilities**: Use cargo-audit

## Performance Tuning

### Database
```sql
-- PostgreSQL tuning
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET work_mem = '4MB';
ALTER SYSTEM SET max_connections = 200;
```

### Redis
```bash
# redis.conf
maxmemory 512mb
maxmemory-policy allkeys-lru
save 900 1
save 300 10
```

### System
```bash
# Increase file descriptors
ulimit -n 65536

# TCP tuning
sysctl -w net.core.somaxconn=1024
sysctl -w net.ipv4.tcp_tw_reuse=1
```

*Last Updated: 2025-09-06*
*This document should be updated when significant architectural changes are made.*
*Version: 0.0.2*
*Maintainer: Caleb Mitchell Smith (ktheindifferent)*