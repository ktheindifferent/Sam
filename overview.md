# S.A.M. Project Architecture Overview

## Executive Summary
S.A.M. (Smart Artificial Mind) is a comprehensive AI assistant platform that combines home automation, media management, security monitoring, and distributed computing into a unified Rust-based system. The project leverages modern technologies to create a secure, scalable, and feature-rich personal assistant.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Web UI Layer                         │
│  (Dashboard, Media Center, Settings, Voice Interface)        │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                      HTTP API Layer                          │
│  (REST Endpoints, WebSocket, Voice API, File Upload)         │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                     Core Service Layer                       │
├───────────────┬──────────────┬──────────────┬───────────────┤
│  Voice Module │  P2P Network │ Security Mod │  Media Center │
│  (STT/TTS)    │  (Discovery) │ (Validation) │  (Streaming)  │
├───────────────┼──────────────┼──────────────┼───────────────┤
│  Web Crawler  │  Smart Home  │ Password Mgr │  Emulators    │
│  (Research)   │  (Lifx)      │ (Encryption) │  (Gaming)     │
└───────────────┴──────────────┴──────────────┴───────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                      Data Layer                              │
├─────────────────────┬───────────────┬───────────────────────┤
│    PostgreSQL       │    Redis      │   File System         │
│  (Primary Store)    │  (Cache)      │  (Media/Backups)      │
└─────────────────────┴───────────────┴───────────────────────┘
```

## Core Components

### 1. Voice Services (`src/sam/services/voice.rs`)
- **Whisper STT Integration**: GPU-accelerated speech-to-text
- **Multi-Model Support**: tiny, base, large models
- **TTS Engine**: Cross-platform text-to-speech (Windows SAPI, macOS Say, Linux espeak)
- **Conversation History**: Context-aware responses
- **REST API**: HTTP endpoints for voice interactions

### 2. P2P Communication (`src/sam/services/p2p/`)
- **Peer Discovery**: Automatic network discovery via UDP broadcast
- **Cryptographic Auth**: Ed25519 signatures for peer verification
- **File Sharing**: Chunked transfers with resume support
- **State Sync**: Distributed state with conflict resolution
- **Message Routing**: Efficient message broadcasting
- **Capacity**: Supports up to 50 concurrent peers

### 3. Security Module (`src/sam/security/`)
- **Input Validation**: SSRF, XSS, SQL injection prevention
- **Session Management**: Redis-backed sessions with CSRF tokens
- **Rate Limiting**: DDoS protection via middleware
- **Password Manager**: AES-256 encrypted vault
- **Vulnerability Scanner**: Port scanning and CVE detection

### 4. Web Crawler (`src/sam/services/crawler/`)
- **Compliance**: robots.txt and sitemap support
- **Circuit Breaker**: Fault tolerance for failed requests
- **Link Analysis**: Content summarization and indexing
- **Security Headers**: HTTP header analysis
- **Performance**: Async/concurrent crawling

### 5. Media Center (`src/sam/media/`)
- **Streaming**: Audio/video streaming support
- **Library Management**: Media organization and metadata
- **Gaming Platform**: Emulator integration (planned)
- **Transcoding**: FFmpeg-based media processing

### 6. Smart Home (`src/sam/services/lifx/`)
- **Lifx Integration**: Smart lighting control
- **Offline Support**: Local control without internet
- **Scene Management**: Automated lighting scenes
- **Schedule Support**: Time-based automation

## Technology Stack

### Backend
- **Language**: Rust (performance, safety, concurrency)
- **Web Framework**: Actix-web (async HTTP server)
- **Async Runtime**: Tokio (concurrent operations)
- **Database**: PostgreSQL (primary), Redis (cache/sessions)

### Security & Cryptography
- **Encryption**: Ring (cryptographic operations)
- **Hashing**: SHA2, Argon2 (password hashing)
- **Signatures**: Ed25519 (peer authentication)
- **TLS**: Native TLS support

### AI/ML Integration
- **STT**: Whisper (speech recognition)
- **TTS**: Platform-native engines
- **LLM**: Llama integration (planned)

### Frontend
- **Framework**: Vanilla JS (migrating to modern framework)
- **Widgets**: Clock, calendar, weather (customizable)
- **Themes**: Multiple theme support
- **Storage**: localStorage for preferences

### Infrastructure
- **Containerization**: Docker (multi-stage builds)
- **CI/CD**: GitHub Actions (automated testing/deployment)
- **Testing**: Comprehensive test suite (75% coverage)
- **Monitoring**: Metrics and health endpoints

## Data Flow

### Voice Interaction Flow
1. User speaks → Microphone capture
2. Audio → Whisper STT processing
3. Text → AI processing (context-aware)
4. Response → TTS engine
5. Audio output → Speaker

### P2P Communication Flow
1. Peer discovery via UDP broadcast
2. TCP connection establishment
3. Cryptographic handshake (Ed25519)
4. Message exchange (encrypted)
5. State synchronization
6. File transfers (chunked)

### Security Validation Flow
1. Input received → Validation module
2. SSRF/XSS/SQL injection checks
3. Rate limiting verification
4. Session validation
5. CSRF token verification
6. Request processing

## Deployment Architecture

### Docker Deployment
```yaml
services:
  sam:
    - Main application container
    - Rust backend services
    - Web server
  
  postgres:
    - Primary database
    - Persistent storage
  
  redis:
    - Session cache
    - Rate limiting store
    - Temporary data
```

### System Requirements
- **OS**: Linux, macOS, Windows (partial)
- **CPU**: 2+ cores recommended
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 10GB for base installation
- **GPU**: Optional (for Whisper acceleration)

## Security Model

### Authentication & Authorization
- Session-based authentication
- CSRF protection on all state-changing operations
- Rate limiting per IP/session
- Secure password storage (Argon2)

### Network Security
- TLS for all external communications
- Ed25519 signatures for P2P authentication
- Private IP blocking for SSRF prevention
- Port scanning detection

### Data Protection
- AES-256 encryption for sensitive data
- Secure key derivation (PBKDF2)
- Memory-safe Rust implementation
- Input sanitization at all entry points

## Performance Characteristics

### Scalability
- Async/concurrent request handling
- Connection pooling for databases
- Efficient caching strategy
- Lazy loading for resources

### Optimization
- Zero-copy operations where possible
- Chunked file transfers
- Compressed P2P messages
- Optimized database queries

### Monitoring
- Health check endpoints
- Metrics collection
- Performance profiling hooks
- Error tracking and logging

## Future Roadmap

### Short-term (Q1 2025)
- Windows build stabilization
- 90% test coverage
- Complete API documentation
- Mobile interface development

### Medium-term (Q2-Q3 2025)
- Modern web UI (React/Vue)
- Gaming emulator integration
- Advanced AI features
- Cloud sync capabilities

### Long-term (Q4 2025+)
- Distributed SAM network
- Plugin ecosystem
- Voice assistant marketplace
- Enterprise features

## Development Guidelines

### Code Organization
```
src/sam/
├── services/      # Core service modules
├── http/          # HTTP handlers
├── security/      # Security components
├── models/        # Data models
├── config/        # Configuration
└── utils/         # Utilities

tests/
├── unit/          # Unit tests
├── integration/   # Integration tests
├── security/      # Security tests
└── performance/   # Performance tests
└── e2e/           # End-to-end tests
```

### Best Practices
- Error handling over panics
- Comprehensive input validation
- Security-first design
- Test-driven development
- Documentation for all public APIs

## Conclusion
S.A.M. represents a comprehensive approach to personal AI assistance, combining cutting-edge technologies with robust security and distributed computing capabilities. The modular architecture ensures extensibility while maintaining performance and reliability.

---

*Last Updated: 2025-08-08*
*Version: 0.0.4*
*Maintained by: Terry (Terragon Labs)*