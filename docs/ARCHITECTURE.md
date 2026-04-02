# S.A.M. Architecture

This document describes the high-level system design, module relationships, and data flow of S.A.M.

## System Overview

S.A.M. (Smart Artificial Mind) is a distributed, voice-enabled AI assistant system built in Rust. It provides:

- **Voice Services**: Speech-to-Text (STT) and Text-to-Speech (TTS)
- **P2P Networking**: Peer-to-peer communication and file sharing
- **Smart Home Control**: LIFX lights and IoT device integration
- **Security**: Session management, input validation, and security scanning
- **Media Management**: Crawling, streaming, and library management
- **Task Queue**: Background job processing and scheduling

## Core Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP REST API                          │
│                    (Rouille Framework)                      │
└────────────────────────────┬────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────▼───┐          ┌────▼────┐        ┌────▼────┐
    │ Routes │          │ Handlers │       │ Middleware
    └────┬───┘          └────┬────┘       └────┬─────┘
         │                   │                  │
    ┌────▼───────────────────▼──────────────────▼────┐
    │           Core Service Layer                   │
    ├──────────────────────────────────────────────────┤
    │ ┌─────────┐ ┌────────┐ ┌──────────┐ ┌────────┐ │
    │ │ Voice   │ │  P2P   │ │ Security │ │ Media  │ │
    │ │ Service │ │Service │ │ Service  │ │Service │ │
    │ └────┬────┘ └───┬────┘ └─────┬────┘ └───┬────┘ │
    └──────┼──────────┼────────────┼──────────┼──────┘
           │          │            │          │
    ┌──────▼──────────▼────────────▼──────────▼──────┐
    │           Data & State Layer                   │
    ├──────────────────────────────────────────────────┤
    │ ┌────────────┐ ┌──────────┐ ┌────────────────┐  │
    │ │ PostgreSQL │ │  Redis   │ │ File Storage   │  │
    │ │ (Primary)  │ │ (Cache)  │ │ (Media/Crawl)  │  │
    │ └────────────┘ └──────────┘ └────────────────┘  │
    └──────────────────────────────────────────────────┘
```

## Module Organization

### `/src/lib/` - Core Library

**libsam** is the shared library containing all reusable components:

#### Voice Services (`lib/voice/`)
- **STT (Speech-to-Text)**: Audio transcription using Whisper
- **TTS (Text-to-Speech)**: Speech synthesis with multiple voices
- **Voice Assistant**: Query processing with NLU

Key files:
- `stt.rs` - Speech recognition
- `tts.rs` - Speech synthesis
- `assistant.rs` - NLU and query handling

#### P2P Networking (`lib/p2p/`)
- **Peer Management**: Connection handling, discovery
- **File Sharing**: Chunked transfer with progress tracking
- **State Synchronization**: Distributed state across peers

Key files:
- `peer.rs` - Peer connection management
- `file_transfer.rs` - File sharing protocol
- `sync.rs` - State synchronization

#### Security (`lib/security/`)
- **Authentication**: Session tokens, JWT, CSRF protection
- **Input Validation**: XSS, SQL injection, SSRF prevention
- **Password Management**: Hashing with Argon2

Key files:
- `session.rs` - Session management
- `input_validation.rs` - Input sanitization
- `auth.rs` - Authentication handlers

#### Media Services (`lib/media/`)
- **Crawler**: Web scraping with robots.txt compliance
- **Streaming**: HLS/DASH streaming support
- **Library Management**: Media metadata and indexing

Key files:
- `crawler.rs` - Web crawling engine
- `streaming.rs` - Stream management
- `library.rs` - Media library indexing

#### Job Queue (`lib/jobs/`)
- **Scheduler**: Cron-like scheduling
- **Worker**: Background task processing
- **Dead Letter Queue**: Failed job handling

Key files:
- `scheduler.rs` - Task scheduling
- `worker.rs` - Job execution
- `queue.rs` - Job queue management

#### Smart Home (`lib/smart_home/`)
- **LIFX Integration**: Light control and color management
- **Device Management**: IoT device discovery and control
- **Automation**: Rule-based device control

Key files:
- `lifx.rs` - LIFX API integration
- `device.rs` - Device management
- `automation.rs` - Automation rules

### `/src/http/` - HTTP Layer

**HTTP API Layer** built with Rouille framework:

```
http/
├── api/
│   ├── voice.rs       # Voice endpoints
│   ├── p2p.rs         # P2P endpoints
│   ├── security.rs    # Auth/security endpoints
│   ├── media.rs       # Media endpoints
│   ├── lifx.rs        # Smart home endpoints
│   ├── jobs.rs        # Job queue endpoints
│   ├── system.rs      # System status endpoints
│   └── mod.rs         # API router
├── csrf.rs            # CSRF token management
├── middleware.rs      # Request/response middleware
└── mod.rs             # HTTP server setup
```

### `/src/cli/` - Command Line Interface

**TUI and CLI** for local interaction:

```
cli/
├── tui/
│   ├── terminal.rs    # Terminal UI rendering
│   ├── components/    # UI components
│   └── mod.rs         # TUI setup
├── commands/
│   ├── voice.rs       # Voice commands
│   ├── system.rs      # System commands
│   └── mod.rs         # Command router
└── mod.rs             # CLI setup
```

## Data Flow

### Voice Transcription Flow

```
Audio Input
    │
    ├─► [HTTP POST /voice/stt/transcribe]
    │
    ├─► Voice Service: Process Audio
    │
    ├─► Whisper Model: Run STT
    │
    ├─► Result Formatting
    │
    └─► [HTTP Response with Transcription]
```

### P2P File Transfer Flow

```
Send File Request
    │
    ├─► [HTTP POST /p2p/files/send]
    │
    ├─► P2P Service: Locate Peer
    │
    ├─► Establish Connection
    │
    ├─► Split File into Chunks
    │
    ├─► Transfer Loop
    │   ├─► Send Chunk
    │   ├─► Await Acknowledgment
    │   └─► Update Progress
    │
    └─► Verify Checksum
```

### Query Processing Flow

```
User Query
    │
    ├─► [HTTP POST /voice/assistant/query]
    │
    ├─► Input Validation
    │
    ├─► NLU Processing
    │
    ├─► Intent Classification
    │
    ├─► Service Routing
    │
    ├─► Action Execution
    │   ├─► Get Information
    │   ├─► Control Device
    │   └─► Execute Command
    │
    └─► Response Generation
        ├─► Format Text Response
        ├─► Generate TTS Audio
        └─► Return to Client
```

### Background Job Flow

```
Job Submission
    │
    ├─► [HTTP POST /jobs/queue]
    │
    ├─► Store in Queue (Redis or DB)
    │
    ├─► Scheduler: Check for Ready Jobs
    │
    ├─► Worker Pool: Acquire Job
    │
    ├─► Execute Handler
    │
    ├─► Success?
    │   ├─► YES: Mark Complete, Store Result
    │   └─► NO: Retry Count < Max?
    │       ├─► YES: Re-queue with Backoff
    │       └─► NO: Move to Dead Letter Queue
    │
    └─► [HTTP GET /jobs/{job_id}] ─► Status Update
```

## Database Schema Overview

### Key Tables

```
sessions
├── id (UUID)
├── user_id (UUID)
├── token (TEXT)
├── expires_at (TIMESTAMP)
└── created_at (TIMESTAMP)

peers
├── id (UUID)
├── peer_id (TEXT)
├── address (TEXT)
├── status (TEXT)
├── last_seen (TIMESTAMP)
└── latency_ms (INTEGER)

media
├── id (UUID)
├── name (TEXT)
├── type (TEXT)
├── size_bytes (INTEGER)
├── metadata (JSONB)
└── created_at (TIMESTAMP)

jobs
├── id (UUID)
├── type (TEXT)
├── status (TEXT)
├── payload (JSONB)
├── result (JSONB)
├── retry_count (INTEGER)
└── scheduled_at (TIMESTAMP)

crawl_tasks
├── id (UUID)
├── url (TEXT)
├── status (TEXT)
├── pages_crawled (INTEGER)
├── security_issues (INTEGER)
└── created_at (TIMESTAMP)
```

## Concurrency Model

### Tokio Runtime
- Multi-threaded async runtime
- 4+ worker threads (based on CPU cores)
- 8MB thread stack size

### Request Handling
```
Incoming Request
    │
    ├─► Tokio Worker Thread
    │
    ├─► Route Matching
    │
    ├─► Service Call (Async)
    │   ├─► Database Query (blocking pool)
    │   ├─► External API Call (non-blocking)
    │   └─► Computation
    │
    └─► Response Formatting
```

### Background Jobs
```
Job Scheduler (spawned task)
    │
    └─► ThreadPool Worker Pool
        ├─► Worker 1: Process Job
        ├─► Worker 2: Process Job
        └─► Worker N: Process Job
```

## External Integrations

### Voice Services
- **Whisper.cpp**: Local STT (offline)
- **External TTS**: Optional speech synthesis service
- **Ollama**: Local LLM integration

### Smart Home
- **LIFX**: Smart light control
- **Generic IoT**: MQTT, HTTP-based devices

### Storage
- **PostgreSQL**: Primary data store
- **Redis**: Caching, sessions, job queue
- **File System**: Media files, downloads

### Third-party APIs
- **Dropbox SDK**: File sync integration
- **YouTube (Rustube)**: Video content
- **Wikipedia**: Information retrieval

## Deployment Scenarios

### Local Development
```
SAM CLI (TUI)
    │
    └─► HTTP Server (localhost:8000)
        │
        └─► PostgreSQL (localhost:5432)
```

### Docker Container
```
Docker Network
    │
    ├─► Sam Container
    │   ├─► HTTP API (8000)
    │   └─► CLI
    │
    ├─► PostgreSQL Container
    │
    └─► Redis Container
```

### CapRover
```
CapRover Registry
    │
    ├─► Sam Service
    │   ├─► HTTP API (8000)
    │   └─► Scaling Replicas
    │
    ├─► PostgreSQL Service
    │
    └─► Redis Service
```

## Performance Considerations

### Caching Strategy
- **Redis**: Session tokens, API responses, media metadata
- **In-memory LRU**: Small datasets, frequently accessed data
- **Database indices**: User queries, job lookups

### Optimization Points
1. **Async I/O**: All database and network calls use async/await
2. **Connection Pooling**: Deadpool for PostgreSQL and Redis
3. **Compression**: Gzip for API responses
4. **Chunking**: Large file transfers split into chunks

### Monitoring
- **Prometheus**: System metrics export
- **Sentry**: Error tracking
- **Tracing**: Distributed tracing with OpenTelemetry

## Security Layers

```
Request
    │
    ├─► TLS/HTTPS (transport security)
    │
    ├─► CSRF Token Validation
    │
    ├─► Authentication (Bearer token)
    │
    ├─► Authorization (role-based)
    │
    ├─► Input Validation & Sanitization
    │
    ├─► Rate Limiting
    │
    └─► Application Logic
```

## Scaling Strategy

### Horizontal Scaling
- Stateless HTTP API (multiple instances)
- Shared PostgreSQL backend
- Shared Redis cache

### Vertical Scaling
- Increased worker threads
- Larger thread stack (currently 8MB)
- Connection pool expansion

## Future Architecture Plans

- **WASM Plugins**: Runtime extensibility
- **gRPC Support**: High-performance inter-service communication
- **Message Queue**: Kafka/RabbitMQ for event streaming
- **Graph Database**: Knowledge graph for semantic queries
- **Distributed Cache**: Cross-deployment cache sync

---

For more information, see:
- [README.md](../README.md) - Project overview
- [API.md](./API.md) - API endpoints
- [CONFIGURATION.md](./CONFIGURATION.md) - Configuration guide
- [design.md](./design.md) - Detailed design decisions
