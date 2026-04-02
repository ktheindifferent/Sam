# S.A.M. API Documentation

## Overview
The S.A.M. API provides RESTful endpoints for interacting with all system services including voice recognition, P2P networking, security features, and media management.

## Base URL
```
http://localhost:8000/api
```

## Authentication

### Session-Based Authentication
Most endpoints use session-based authentication with CSRF protection.

### Required Headers
```http
Authorization: Bearer <session_token>
X-CSRF-Token: <csrf_token>
Content-Type: application/json
```

### Getting a Session
1. Use TUI/CLI to establish a local session
2. Or authenticate via HTTP POST to `/auth/login`
3. Session tokens are stored in Redis or memory
4. CSRF tokens provided in login response

## Core API Endpoints

### Session & Authentication

#### GET /api/sid
Get current session ID.

**Response:**
```json
"session_id_abc123"
```

#### GET /api/current_session
Get current session information.

**Response:**
```json
{
  "sid": "session_abc123",
  "human_oid": "human_123",
  "user_id": "user_123",
  "created_at": "2026-04-02T10:00:00Z"
}
```

#### GET /api/current_human
Get current user information.

**Response:**
```json
{
  "oid": "human_123",
  "name": "John Doe",
  "email": "john@example.com",
  "roles": ["user", "admin"],
  "created_at": "2025-01-01T00:00:00Z"
}
```

### Core Data Endpoints

#### Humans

##### GET /api/humans
List all humans.

**Query Parameters:**
- `limit`: Number of results
- `offset`: Pagination offset

**Response:**
```json
{
  "humans": [
    {
      "oid": "human_123",
      "name": "John Doe",
      "email": "john@example.com"
    }
  ],
  "total": 100
}
```

##### POST /api/humans
Create new human.

**Request:**
```json
{
  "name": "Jane Smith",
  "email": "jane@example.com"
}
```

##### GET /api/humans/{oid}
Get human by OID.

##### PUT /api/humans/{oid}
Update human information.

##### DELETE /api/humans/{oid}
Delete human.

#### Rooms

##### GET /api/rooms
List all rooms.

**Response:**
```json
{
  "rooms": [
    {
      "oid": "room_123",
      "name": "Living Room",
      "description": "Main living space",
      "location_oid": "location_456"
    }
  ]
}
```

##### POST /api/rooms
Create new room.

##### PUT /api/rooms/{oid}
Update room details.

##### DELETE /api/rooms/{oid}
Delete room.

#### Locations

##### GET /api/locations
List all locations.

**Response:**
```json
{
  "locations": [
    {
      "oid": "location_123",
      "name": "Home",
      "latitude": 37.7749,
      "longitude": -122.4194
    }
  ]
}
```

##### POST /api/locations
Create new location.

##### PUT /api/locations/{oid}
Update location.

#### Things (IoT Devices)

##### GET /api/things
List all connected IoT devices.

**Response:**
```json
{
  "things": [
    {
      "oid": "thing_123",
      "name": "Living Room Light",
      "type": "light",
      "room_oid": "room_456",
      "state": "on"
    }
  ]
}
```

##### POST /api/things
Register new IoT device.

##### PUT /api/things/{oid}
Update thing state/properties.

##### DELETE /api/things/{oid}
Unregister device.

### Input & Output (I/O)

#### POST /api/io
Send command to Sam assistant.

**Request:**
```json
{
  "type": "command",
  "text": "Turn on the living room light",
  "context": {
    "room_oid": "room_123",
    "human_oid": "human_123"
  }
}
```

**Response:**
```json
{
  "success": true,
  "response": "I've turned on the living room light.",
  "actions": [
    {
      "type": "device_control",
      "device_id": "thing_123",
      "action": "turn_on"
    }
  ]
}
```

#### POST /api/io/process
Process natural language input.

**Request:**
```json
{
  "text": "What's the temperature in the living room?",
  "mode": "query"
}
```

**Response:**
```json
{
  "intent": "query_temperature",
  "entities": {
    "location": "living room"
  },
  "confidence": 0.95
}
```

#### POST /api/io/validate
Validate input for security threats.

**Request:**
```json
{
  "text": "user input text",
  "checks": ["xss", "sql", "path_traversal"]
}
```

**Response:**
```json
{
  "is_safe": true,
  "threats": [],
  "sanitized_text": "user input text"
}
```

### Observations & Telemetry

#### POST /api/observations
Record observation/event.

**Request:**
```json
{
  "type": "temperature_reading",
  "source": "sensor_123",
  "value": 72.5,
  "unit": "fahrenheit",
  "location_oid": "room_123",
  "timestamp": "2026-04-02T10:47:00Z"
}
```

**Response:**
```json
{
  "success": true,
  "observation_id": "obs_789"
}
```

#### GET /api/observations
Query observations.

**Query Parameters:**
- `type`: Observation type filter
- `source`: Source filter
- `limit`: Results limit
- `offset`: Pagination offset

#### GET /api/telemetry
Get system telemetry data.

**Response:**
```json
{
  "timestamp": "2026-04-02T10:47:00Z",
  "uptime_seconds": 3600,
  "requests_total": 1500,
  "requests_per_minute": 25,
  "active_sessions": 3
}
```

### Services Control

#### GET /api/services/status
Get all services status.

**Response:**
```json
{
  "services": {
    "postgres": {
      "status": "operational",
      "uptime_seconds": 86400
    },
    "redis": {
      "status": "operational",
      "memory_used_mb": 256
    },
    "voice": {
      "status": "operational",
      "model": "whisper-base"
    },
    "crawler": {
      "status": "idle",
      "jobs_completed": 15
    }
  }
}
```

#### POST /api/services/redis/test
Test Redis connection.

**Response:**
```json
{
  "success": true,
  "message": "Redis connection successful",
  "ping": "PONG"
}
```

#### POST /api/services/postgres/test
Test PostgreSQL connection.

**Response:**
```json
{
  "success": true,
  "message": "PostgreSQL connection successful",
  "version": "PostgreSQL 13.0"
}
```

#### GET /api/services/voice/status
Get voice service status.

**Response:**
```json
{
  "status": "operational",
  "stt_engine": "whisper-base",
  "tts_engine": "piper",
  "models_loaded": true
}
```

#### GET /api/environment
Get environment information.

**Response:**
```json
{
  "mode": "production",
  "rust_log": "info",
  "database_engine": "postgresql",
  "redis_enabled": true,
  "version": "0.0.2",
  "uptime_seconds": 86400
}
```

---

## Legacy/Deprecated Endpoints

### Voice Services (Deprecated)

#### Speech-to-Text (STT)

##### POST /voice/stt/transcribe
Transcribe audio file to text.

**Request:**
```json
{
  "audio_data": "base64_encoded_audio",
  "format": "wav",
  "language": "en",
  "model": "base"
}
```

**Response:**
```json
{
  "success": true,
  "transcription": {
    "text": "Hello, how can I help you?",
    "confidence": 0.95,
    "language": "en",
    "duration_ms": 3500,
    "words": [
      {"word": "Hello", "start": 0.0, "end": 0.5, "confidence": 0.98}
    ]
  }
}
```

##### POST /voice/stt/stream
Start streaming transcription session.

**WebSocket Connection:**
```javascript
ws://localhost:8000/api/v1/voice/stt/stream
```

**Messages:**
```json
// Client -> Server
{
  "type": "audio_chunk",
  "data": "base64_encoded_chunk",
  "sequence": 1
}

// Server -> Client
{
  "type": "partial_transcript",
  "text": "Hello, how",
  "is_final": false
}
```

#### Text-to-Speech (TTS)

##### POST /voice/tts/synthesize
Convert text to speech audio.

**Request:**
```json
{
  "text": "Hello, this is SAM speaking.",
  "voice": "female",
  "language": "en-US",
  "format": "mp3",
  "speed": 1.0,
  "pitch": 1.0
}
```

**Response:**
```json
{
  "success": true,
  "audio": {
    "data": "base64_encoded_audio",
    "format": "mp3",
    "duration_ms": 2500,
    "size_bytes": 40000
  }
}
```

##### GET /voice/tts/voices
List available TTS voices.

**Response:**
```json
{
  "success": true,
  "voices": [
    {
      "id": "en-US-1",
      "name": "Sarah",
      "language": "en-US",
      "gender": "female",
      "age": "adult"
    }
  ]
}
```

#### Voice Assistant

##### POST /voice/assistant/query
Send query to voice assistant.

**Request:**
```json
{
  "query": "What's the weather today?",
  "context": {
    "session_id": "abc123",
    "location": "auto"
  }
}
```

**Response:**
```json
{
  "success": true,
  "response": {
    "text": "Today's weather is sunny with a high of 75°F.",
    "audio": "base64_encoded_audio",
    "actions": [
      {"type": "display_weather", "data": {}}
    ]
  }
}
```

### P2P Network

#### Peer Management

##### GET /p2p/peers
List connected peers.

**Response:**
```json
{
  "success": true,
  "peers": [
    {
      "id": "peer_123",
      "address": "192.168.1.100:8080",
      "status": "connected",
      "latency_ms": 15,
      "last_seen": "2025-01-08T10:30:00Z"
    }
  ]
}
```

##### POST /p2p/connect
Connect to a new peer.

**Request:**
```json
{
  "address": "192.168.1.101:8080",
  "peer_id": "optional_peer_id"
}
```

**Response:**
```json
{
  "success": true,
  "peer": {
    "id": "peer_456",
    "status": "connected"
  }
}
```

##### DELETE /p2p/peers/{peer_id}
Disconnect from a peer.

**Response:**
```json
{
  "success": true,
  "message": "Peer disconnected"
}
```

#### File Sharing

##### POST /p2p/files/send
Send file to peer.

**Request:**
```json
{
  "peer_id": "peer_123",
  "file_path": "/path/to/file.txt",
  "chunk_size": 1048576
}
```

**Response:**
```json
{
  "success": true,
  "transfer": {
    "id": "transfer_789",
    "status": "in_progress",
    "total_bytes": 10485760,
    "transferred_bytes": 0
  }
}
```

##### GET /p2p/files/transfers
List active file transfers.

**Response:**
```json
{
  "success": true,
  "transfers": [
    {
      "id": "transfer_789",
      "direction": "send",
      "peer_id": "peer_123",
      "file_name": "document.pdf",
      "progress": 0.45,
      "speed_bps": 1048576
    }
  ]
}
```

#### State Synchronization

##### POST /p2p/sync/state
Synchronize state with peers.

**Request:**
```json
{
  "key": "app_settings",
  "value": {"theme": "dark", "language": "en"},
  "broadcast": true
}
```

**Response:**
```json
{
  "success": true,
  "synced_peers": 3,
  "conflicts": []
}
```

### Security

#### Session Management

##### POST /auth/login
Authenticate user and create session.

**Request:**
```json
{
  "username": "user@example.com",
  "password": "secure_password",
  "remember_me": false
}
```

**Response:**
```json
{
  "success": true,
  "session": {
    "id": "session_abc123",
    "token": "jwt_token_here",
    "csrf_token": "csrf_token_here",
    "expires_at": "2025-01-09T10:00:00Z"
  }
}
```

##### POST /auth/logout
Terminate current session.

**Response:**
```json
{
  "success": true,
  "message": "Session terminated"
}
```

##### GET /auth/session
Get current session information.

**Response:**
```json
{
  "success": true,
  "session": {
    "user_id": "user_123",
    "username": "john_doe",
    "roles": ["user", "admin"],
    "expires_at": "2025-01-09T10:00:00Z"
  }
}
```

#### Input Validation

##### POST /security/validate
Validate user input for security threats.

**Request:**
```json
{
  "input": "user provided text",
  "checks": ["xss", "sql", "ssrf", "path_traversal"]
}
```

**Response:**
```json
{
  "success": true,
  "validation": {
    "is_safe": true,
    "threats_detected": [],
    "sanitized_input": "user provided text"
  }
}
```

#### Password Management

##### POST /security/password/check
Check password strength.

**Request:**
```json
{
  "password": "MyP@ssw0rd123"
}
```

**Response:**
```json
{
  "success": true,
  "strength": {
    "score": 4,
    "level": "strong",
    "entropy": 72.5,
    "suggestions": [
      "Consider adding more special characters"
    ]
  }
}
```

### Web Crawler

##### POST /crawler/scan
Start web crawling task.

**Request:**
```json
{
  "url": "https://example.com",
  "depth": 2,
  "follow_robots": true,
  "extract_content": true,
  "check_security": true
}
```

**Response:**
```json
{
  "success": true,
  "task": {
    "id": "crawl_task_123",
    "status": "queued",
    "estimated_time": 30
  }
}
```

##### GET /crawler/tasks/{task_id}
Get crawling task status.

**Response:**
```json
{
  "success": true,
  "task": {
    "id": "crawl_task_123",
    "status": "completed",
    "pages_crawled": 25,
    "links_found": 150,
    "security_issues": 2,
    "results_url": "/api/v1/crawler/results/crawl_task_123"
  }
}
```

### Media Services

##### GET /media/library
List media files in library.

**Query Parameters:**
- `type`: audio, video, image
- `limit`: number of results
- `offset`: pagination offset

**Response:**
```json
{
  "success": true,
  "media": [
    {
      "id": "media_123",
      "name": "song.mp3",
      "type": "audio",
      "size_bytes": 5242880,
      "duration_seconds": 180,
      "metadata": {
        "artist": "Artist Name",
        "album": "Album Name"
      }
    }
  ],
  "total": 100,
  "offset": 0
}
```

##### POST /media/stream/{media_id}
Start streaming media file.

**Response:**
```json
{
  "success": true,
  "stream": {
    "url": "http://localhost:8000/stream/abc123",
    "protocol": "HLS",
    "expires_at": "2025-01-08T12:00:00Z"
  }
}
```

### Smart Home

##### GET /lifx/lights
List connected Lifx lights.

**Response:**
```json
{
  "success": true,
  "lights": [
    {
      "id": "light_123",
      "label": "Living Room",
      "power": "on",
      "brightness": 0.8,
      "color": {
        "hue": 240,
        "saturation": 1.0,
        "kelvin": 3500
      }
    }
  ]
}
```

##### PUT /lifx/lights/{light_id}
Control Lifx light.

**Request:**
```json
{
  "power": "on",
  "brightness": 0.5,
  "color": {
    "hue": 120,
    "saturation": 1.0
  },
  "duration_ms": 1000
}
```

**Response:**
```json
{
  "success": true,
  "light": {
    "id": "light_123",
    "status": "updated"
  }
}
```

### System

##### GET /system/health
Get system health status.

**Response:**
```json
{
  "success": true,
  "health": {
    "status": "healthy",
    "uptime_seconds": 86400,
    "cpu_usage": 0.25,
    "memory_usage": 0.45,
    "disk_usage": 0.60,
    "services": {
      "voice": "operational",
      "p2p": "operational",
      "security": "operational",
      "media": "operational"
    }
  }
}
```

##### GET /system/metrics
Get system metrics.

**Response:**
```json
{
  "success": true,
  "metrics": {
    "requests_per_minute": 150,
    "active_sessions": 5,
    "connected_peers": 3,
    "cache_hit_rate": 0.85,
    "error_rate": 0.02
  }
}
```

## Error Responses

All endpoints follow a consistent error response format:

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input provided",
    "details": {
      "field": "email",
      "reason": "Invalid email format"
    }
  }
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|------------|-------------|
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Input validation failed |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

## Rate Limiting

API endpoints are rate limited to prevent abuse:

- **Default limit**: 100 requests per minute
- **Authenticated users**: 1000 requests per minute
- **File uploads**: 10 per hour
- **Crawling tasks**: 5 per hour

Rate limit information is included in response headers:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1704721200
```

## WebSocket Events

### Voice Assistant Events

```javascript
// Connection
ws://localhost:8000/api/v1/ws/voice

// Events
{
  "event": "wake_word_detected",
  "data": {"confidence": 0.95}
}

{
  "event": "listening_started",
  "data": {"timeout_ms": 5000}
}

{
  "event": "speech_detected",
  "data": {"energy": 0.8}
}
```

### P2P Network Events

```javascript
// Connection
ws://localhost:8000/api/v1/ws/p2p

// Events
{
  "event": "peer_connected",
  "data": {"peer_id": "peer_123", "address": "192.168.1.100"}
}

{
  "event": "file_transfer_progress",
  "data": {"transfer_id": "transfer_789", "progress": 0.75}
}

{
  "event": "state_sync",
  "data": {"key": "settings", "value": {}}
}
```

## SDK Examples

### JavaScript/TypeScript

```typescript
import { SamClient } from '@sam/client';

const client = new SamClient({
  baseUrl: 'http://localhost:8000',
  apiKey: 'your_api_key'
});

// Voice transcription
const transcription = await client.voice.transcribe({
  audio: audioBuffer,
  language: 'en'
});

// P2P file sharing
const transfer = await client.p2p.sendFile({
  peerId: 'peer_123',
  filePath: '/path/to/file.pdf'
});

// Monitor transfer progress
transfer.on('progress', (percent) => {
  console.log(`Transfer ${percent}% complete`);
});
```

### Python

```python
from sam_client import SamClient

client = SamClient(
    base_url='http://localhost:8000',
    api_key='your_api_key'
)

# Voice synthesis
audio = client.voice.synthesize(
    text="Hello from Python",
    voice="female"
)

# Security validation
result = client.security.validate(
    input_text=user_input,
    checks=['xss', 'sql']
)

if result['is_safe']:
    process_input(result['sanitized_input'])
```

### Rust

```rust
use sam_client::SamClient;

let client = SamClient::new(
    "http://localhost:8000",
    "your_api_key"
)?;

// P2P connection
let peer = client.p2p.connect("192.168.1.100:8080").await?;

// State synchronization
client.p2p.sync_state(
    "app_settings",
    json!({"theme": "dark"})
).await?;
```

## Changelog

### Version 0.0.4 (Current)
- Added enhanced voice services with Whisper STT/TTS
- Implemented P2P communication system
- Added comprehensive security validation
- Enhanced clock widget with multiple formats
- Improved API documentation

### Version 0.0.3
- Added web crawler with robots.txt compliance
- Implemented session management
- Added rate limiting
- Security improvements

### Version 0.0.2
- Initial API implementation
- Basic voice services
- Lifx integration
- Media streaming

---

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Voice Services | ✅ Implemented | STT via Whisper, TTS support |
| P2P Networking | ✅ Implemented | Peer discovery and file sharing |
| Security | ✅ Implemented | Session management, CSRF protection |
| Media Services | ✅ Implemented | Library management, streaming |
| Smart Home (LIFX) | ✅ Implemented | Light control and automation |
| Job Queue | ✅ Implemented | Background task processing |
| Web Crawler | ✅ Implemented | With robots.txt support |

---

*Last Updated: 2026-04-02*
*API Version: 0.0.2*
*Documentation Version: 2.0.0*