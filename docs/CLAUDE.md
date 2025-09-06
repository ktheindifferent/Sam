# S.A.M. (Smart Artificial Mind) - Codebase Documentation

## Project Overview

S.A.M. is an open-source AI assistant platform built with Rust that combines home automation, media center capabilities, security features, and AI-powered services into a unified system. The project is actively developed and aims to provide a comprehensive smart home and personal assistant solution.

**Current Version:** 0.0.5 (Work in Progress)  
**License:** GPLv3  
**Primary Language:** Rust (2021 Edition)  
**Development Status:** Active Development (Not Production Ready)

## Core Architecture

### Technology Stack

- **Core Framework:** Rust with Tokio async runtime
- **Databases:** 
  - SQLite (default, embedded)
  - PostgreSQL (optional, full-featured)
  - Redis (caching and session management)
- **Web Framework:** Rouille for HTTP server
- **WebSocket:** simple-websockets for real-time communication
- **AI/ML Integration:**
  - Whisper for STT/TTS
  - Darknet for computer vision
  - LLaMA for language models
  - OpenAI API integration
- **Security:** Argon2 for password hashing, AES-256 for encryption
- **Monitoring:** OpenTelemetry, Prometheus metrics, structured logging

### Directory Structure

```
/root/repo/
├── src/                        # Rust source code
│   ├── main.rs                 # Application entry point
│   ├── sam.rs                   # Core SAM module
│   ├── lib/                    # Library modules
│   │   └── services/           # Platform-specific services
│   └── sam/                    # Core SAM modules
│       ├── cli/                # Command-line interface
│       ├── db/                 # Database abstraction layer
│       ├── http/               # HTTP server and API
│       ├── jobs/               # Job queue system
│       ├── memory/             # Data models and caching
│       ├── security/           # Security modules
│       ├── services/           # Core services (30+ services)
│       ├── resource_management/ # Resource monitoring
│       └── websocket/          # WebSocket handlers
├── docs/                       # 📚 All documentation (organized by category)
│   ├── api/                   # API documentation  
│   ├── deployment/            # Deployment guides
│   ├── development/           # Technical implementation docs
│   ├── features/              # Feature descriptions
│   ├── security/              # Security guides and fixes
│   ├── CLAUDE.md              # This file
│   ├── DIRECTORY_STRUCTURE.md # Detailed directory documentation
│   └── [other documentation]
├── deploy/                     # 🚢 Deployment configurations
│   ├── docker-compose*.yml    # Docker Compose files
│   ├── Dockerfile*            # Docker build files
│   └── docker-entrypoint.sh   # Container entrypoint
├── config/                     # ⚙️ Development configuration
│   ├── .env.example           # Environment template
│   └── [build configurations]
├── scripts/                    # 🔧 Application scripts
├── tools/                      # 🛠️ Development tools  
├── tests/                      # 🧪 All test files and utilities
│   ├── integration/           # Integration tests
│   ├── fixtures/              # Test data
│   └── [test files and utilities]
├── www/                        # 🌐 Web frontend
│   ├── assets/                # Static assets
│   └── dashboard.html         # Main dashboard
├── cfg/                        # Application configuration files
├── data/                       # Data files and models
└── packages/                   # Package definitions
```

## Key Components

### 1. Service Orchestration Layer
- **Location:** `src/sam/services/orchestrator.rs`
- **Purpose:** Manages service lifecycle, dependencies, and health monitoring
- **Features:** Auto-recovery, dependency resolution, graceful shutdown

### 2. Database Abstraction
- **Location:** `src/sam/db/`
- **Features:** 
  - Multi-database support (SQLite default, PostgreSQL optional)
  - Connection pooling (32 concurrent connections)
  - Automatic migrations
  - Health monitoring

### 3. Job Queue System
- **Location:** `src/sam/jobs/`
- **Features:**
  - Async job processing
  - Scheduled tasks
  - Dead letter queue
  - Worker pools

### 4. Security Layer
- **Location:** `src/sam/security/`
- **Features:**
  - Input validation and sanitization
  - CSRF protection
  - Session management with Redis
  - Rate limiting
  - SQL injection prevention
  - Command injection prevention

### 5. Web Crawler
- **Location:** `src/sam/services/crawler/`
- **Features:**
  - Robots.txt compliance
  - Sitemap support
  - Circuit breaker pattern
  - Link extraction and summarization
  - Port scanning capabilities

### 6. AI/ML Services
- **STT/TTS:** Whisper integration with GPU support
- **Vision:** Darknet for object detection (YOLO)
- **Language:** LLaMA and OpenAI API support
- **Speech Recognition:** Custom speaker identification

### 7. Home Automation
- **Lifx Integration:** Full smart lighting control
- **Matter Protocol:** Smart home device support
- **mDNS Discovery:** Automatic device discovery

### 8. Media Center
- **Game Support:** Native emulation capabilities
- **Streaming:** Netflix, YouTube, Spotify integration
- **Media Processing:** FFmpeg integration

## Dependencies

### Core Dependencies
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `postgres` / `tokio-postgres` - PostgreSQL support
- `rusqlite` / `diesel` - SQLite and ORM support
- `deadpool-redis` / `deadpool-postgres` - Connection pooling
- `reqwest` - HTTP client
- `rouille` - HTTP server

### AI/ML Dependencies
- `whisper-rs` - Speech-to-text
- `opencl3` - GPU acceleration

### Security Dependencies
- `argon2` - Password hashing
- `ammonia` - HTML sanitization
- `base64` - Encoding
- `native-tls` - TLS support

### Monitoring Dependencies
- `tracing` / `tracing-subscriber` - Structured logging
- `prometheus` - Metrics collection
- `opentelemetry` - Distributed tracing

## API Endpoints

### Health Check
- `GET /health` - Basic health check
- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe
- `GET /health/detailed` - Detailed service status

### Core APIs
- `/api/v1/observations` - Observation management
- `/api/v1/humans` - User management
- `/api/v1/things` - Device management
- `/api/v1/locations` - Location services
- `/api/v1/services` - Service control
- `/api/v1/jobs` - Job queue management

### Voice Services
- `POST /api/tts` - Text-to-speech
- `POST /api/stt` - Speech-to-text
- `WebSocket /ws` - Real-time communication

## Development Commands

### Building
```bash
# Development build
cargo build

# Release build
cargo build --release

# Run with specific features
cargo run -- --help
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration

# Run with coverage
cargo tarpaulin
```

### Database Setup
```bash
# SQLite (default)
export DATABASE_ENGINE=sqlite
cargo run

# PostgreSQL
export DATABASE_ENGINE=postgresql
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_DB=sam
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=password
cargo run
```

### Docker Deployment
```bash
# SQLite deployment
docker-compose -f docker-compose.sqlite.yml up

# PostgreSQL deployment
docker-compose up
```

## Security Considerations

⚠️ **This software is in active development and NOT production ready**

### Recent Security Fixes
- ✅ Command injection vulnerabilities patched
- ✅ SQL injection prevention implemented
- ✅ Network error handling improved
- 🔄 Ongoing replacement of unsafe `.unwrap()` calls

### Security Features
- Input validation on all user inputs
- CSRF token validation
- Rate limiting on API endpoints
- Encrypted password storage
- Session management with Redis
- Private IP blocking for SSRF prevention

## Testing Strategy

### Test Coverage
- **Unit Tests:** Core business logic testing
- **Integration Tests:** Service interaction testing
- **Security Tests:** Vulnerability and penetration testing
- **Performance Tests:** Benchmarking and load testing

### Test Files
- `tests/unit/` - Unit test modules
- `tests/integration/` - Integration test suites
- `tests/security/` - Security test cases
- `benches/` - Performance benchmarks

## Recent Updates (2025)

### Latest Features
- Multi-database support with SQLite as default
- Enhanced error handling with retry logic
- Comprehensive monitoring system
- Advanced backup service with encryption
- Production-ready web crawler
- Job queue system implementation
- Resource management and cleanup
- WebSocket security enhancements

### In Progress
- Mobile application development
- Windows platform stabilization
- Additional emulator support
- GUI overhaul without jQuery
- Extended AI model integration

## Contributing

This is an open-source project licensed under GPLv3. The project is actively developed and welcomes contributions. Key areas for contribution:

1. Security improvements
2. Platform compatibility
3. Test coverage expansion
4. Documentation
5. UI/UX enhancements

## Notes for AI Assistants

When working with this codebase:

1. **Security First:** Always validate inputs and handle errors properly
2. **Async by Default:** Use Tokio async patterns throughout
3. **Test Coverage:** Write tests for new features
4. **Error Handling:** Avoid `.unwrap()`, use proper error types
5. **Documentation:** Update this file when making significant changes
6. **Database Agnostic:** Ensure code works with both SQLite and PostgreSQL
7. **Performance:** Consider connection pooling and caching

## Build Requirements

- Rust 1.70+
- PostgreSQL 13+ (optional)
- Redis (optional)
- Python 3.8+ (for AI components)
- FFmpeg (for media processing)
- OpenSSL development libraries
- ALSA development libraries (Linux)
- CMake (for certain dependencies)

## Contact and Support

- Repository: (GitHub repository URL)
- License: GPLv3
- Status: Active Development - Not Production Ready