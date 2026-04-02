# S.A.M. Environment Variables Reference

Complete reference of all environment variables supported by S.A.M.

## Quick Reference

### Required Variables (must be set)
- `DATABASE_URL` or `PG_*` variables - Database connection

### Recommended Variables
- `REDIS_URL` - Redis cache connection
- `RUST_LOG` - Logging level
- `PORT` - Server port

### Optional Variables
- External service URLs (TTS, STT, etc.)
- Monitoring and security settings

---

## Database Configuration

### PostgreSQL (Recommended for Production)

#### Connection String Method
**Variable:** `DATABASE_URL`  
**Format:** `postgresql://[user]:[password]@[host]:[port]/[database]`  
**Example:** `postgresql://sam:secret@localhost:5432/sam_db`  
**Required:** Yes (if not using individual PG_* vars)

```bash
export DATABASE_URL="postgresql://sam:mypassword@db.example.com:5432/sam_db"
```

#### Individual Variables
When `DATABASE_URL` is not set, these variables are used:

| Variable | Required | Default | Example |
|----------|----------|---------|---------|
| `PG_DBNAME` | Yes | `sam` | `sam_db` |
| `PG_USER` | Yes | `sam` | `sam_user` |
| `PG_PASS` | Yes | `sam` | `secure_password_123` |
| `PG_ADDRESS` | Yes | `localhost:5432` | `db.example.com:5432` |

```bash
export PG_DBNAME=sam_db
export PG_USER=sam_user
export PG_PASS=secure_password_123
export PG_ADDRESS=localhost:5432
```

**Precedence:** `DATABASE_URL` takes precedence over individual variables.

### SQLite (Development Only)

| Variable | Required | Default | Example |
|----------|----------|---------|---------|
| `DATABASE_ENGINE` | No | `postgresql` | `sqlite` |
| `SQLITE_DATABASE_PATH` | No | `./sam.db` | `/var/lib/sam/sam.db` |

```bash
export DATABASE_ENGINE=sqlite
export SQLITE_DATABASE_PATH=/var/lib/sam/sam.db
```

**Note:** SQLite is NOT recommended for production. Use PostgreSQL for production deployments.

---

## Redis Cache Configuration

| Variable | Required | Default | Example |
|----------|----------|---------|---------|
| `REDIS_URL` | No | (disabled) | `redis://localhost:6379` |
| `REDIS_TIMEOUT` | No | `5000` (ms) | `10000` |
| `REDIS_MAX_POOL_SIZE` | No | `10` | `20` |

```bash
# Basic setup
export REDIS_URL=redis://localhost:6379

# With authentication
export REDIS_URL=redis://:password@localhost:6379

# With specific database
export REDIS_URL=redis://localhost:6379/1

# Connection options
export REDIS_TIMEOUT=10000
export REDIS_MAX_POOL_SIZE=20
```

**Features enabled when Redis is configured:**
- Session caching
- Query result caching
- Rate limiting
- Distributed locks (for multi-instance deployments)

**Fallback:** When Redis is not available, in-memory caching is used (limited to single instance).

---

## Application Settings

### Core Configuration

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `PORT` | No | `8000` | `8080` | HTTP server listening port |
| `RUST_LOG` | No | `info` | `debug` | Logging level: trace, debug, info, warn, error |
| `RUST_BACKTRACE` | No | `0` | `1` or `full` | Enable Rust panic backtraces |

```bash
# Production
export PORT=8000
export RUST_LOG=info
export RUST_BACKTRACE=0

# Development
export PORT=3000
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Detailed debugging
export RUST_LOG=trace
export RUST_BACKTRACE=full
```

### Environment Mode

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `SAM_ENVIRONMENT` | No | `development` | `production` | Deployment environment |
| `CAPROVER` | No | `false` | `true` | Running on CapRover platform |
| `DOCKER_CONTAINER` | No | `false` | `true` | Running inside Docker |

```bash
# Production on CapRover
export SAM_ENVIRONMENT=production
export CAPROVER=true

# Development locally
export SAM_ENVIRONMENT=development
export CAPROVER=false
export DOCKER_CONTAINER=false
```

### Paths and Directories

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `SAM_HOME` | No | `~/.sam` | `/opt/sam` | Application home directory |
| `SAM_DATA` | No | `/var/lib/sam` | `/data/sam` | Data storage directory |
| `SAM_LOGS` | No | `/var/log/sam` | `/logs/sam` | Log directory |
| `SAM_TEMP` | No | `/tmp` | `/var/tmp/sam` | Temporary files directory |

```bash
export SAM_HOME=/opt/sam
export SAM_DATA=/var/lib/sam
export SAM_LOGS=/var/log/sam
```

---

## Voice Services Configuration

### Speech-to-Text (STT)

#### Local Whisper Model
| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `STT_ENGINE` | No | `whisper-local` | `whisper-remote` | STT engine type |
| `STT_MODEL` | No | `whisper-base` | `whisper-large` | Model size (tiny, base, small, medium, large) |
| `STT_LANGUAGE` | No | `en` | `fr`, `es`, `de` | Default language |

```bash
# Local Whisper (requires GPU recommended)
export STT_ENGINE=whisper-local
export STT_MODEL=whisper-base
export STT_LANGUAGE=en
```

#### External STT Service
| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `STT_URL` | No | - | `http://stt-service:8001` | External STT endpoint |
| `STT_API_KEY` | No | - | `sk_test_123...` | API key for external service |
| `STT_TIMEOUT` | No | `30000` (ms) | `60000` | Request timeout |

```bash
# Using external service (e.g., Assembly.ai, Google Cloud)
export STT_ENGINE=whisper-remote
export STT_URL=http://localhost:8001/transcribe
export STT_API_KEY=your_api_key
export STT_TIMEOUT=30000
```

### Text-to-Speech (TTS)

#### Local TTS
| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `TTS_ENGINE` | No | `piper` | `espeak` | TTS engine (piper, espeak) |
| `TTS_DEFAULT_VOICE` | No | `default` | `en_US-libritts-high` | Default voice ID |
| `TTS_DEFAULT_LANGUAGE` | No | `en-US` | `fr-FR` | Default language |
| `TTS_SPEED` | No | `1.0` | `0.8` | Speech speed (0.5 - 2.0) |

```bash
# Using Piper TTS (lightweight, good quality)
export TTS_ENGINE=piper
export TTS_DEFAULT_VOICE=en_US-libritts-high
export TTS_SPEED=1.0
```

#### External TTS Service
| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `TTS_URL` | No | - | `http://tts-service:8002` | External TTS endpoint |
| `TTS_API_KEY` | No | - | `sk_live_123...` | API key for external service |
| `TTS_TIMEOUT` | No | `30000` (ms) | `60000` | Request timeout |

```bash
# Using external service (e.g., ElevenLabs, Google Cloud)
export TTS_URL=http://localhost:8002/synthesize
export TTS_API_KEY=your_api_key
export TTS_TIMEOUT=30000
```

---

## Security Configuration

### Authentication & Tokens

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `JWT_SECRET` | Yes | - | `47e3d8c2a5f9...` | JWT signing secret |
| `SESSION_TIMEOUT` | No | `1440` (min) | `480` | Session expiration time |
| `CSRF_PROTECTION` | No | `true` | `false` | Enable CSRF protection |

```bash
# Generate a new JWT secret
export JWT_SECRET=$(openssl rand -hex 32)

# Custom session timeout (8 hours)
export SESSION_TIMEOUT=480

# Disable CSRF for development (NOT recommended for production)
export CSRF_PROTECTION=false
```

### SSL/TLS

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `HTTPS_ENABLED` | No | `false` | `true` | Enable HTTPS |
| `CERT_FILE` | No | - | `/etc/sam/cert.pem` | SSL certificate file |
| `KEY_FILE` | No | - | `/etc/sam/key.pem` | SSL private key file |

```bash
# Note: Use reverse proxy (Nginx, Caddy) for HTTPS in production
# These variables are for direct HTTPS support
export HTTPS_ENABLED=false
```

### Credentials & Secrets

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `SSH_KEY_PATH` | No | `~/.ssh/id_rsa` | `/etc/sam/ssh_key` | SSH private key for remote ops |
| `SSH_KNOWN_HOSTS` | No | `~/.ssh/known_hosts` | `/etc/sam/known_hosts` | SSH known hosts file |

```bash
export SSH_KEY_PATH=/etc/sam/ssh_key
export SSH_KNOWN_HOSTS=/etc/sam/known_hosts
```

---

## Monitoring & Observability

### Logging

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `LOG_FORMAT` | No | `json` | `text` | Log format (json, text) |
| `LOG_FILE` | No | `-` (stdout) | `/var/log/sam/app.log` | Log file path |
| `LOG_MAX_SIZE` | No | `100MB` | `500MB` | Max log file size |
| `LOG_RETENTION_DAYS` | No | `30` | `90` | Retention period |

```bash
export LOG_FORMAT=json
export LOG_FILE=/var/log/sam/app.log
export LOG_MAX_SIZE=100MB
export LOG_RETENTION_DAYS=30
```

### Error Tracking (Sentry)

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `SENTRY_DSN` | No | - | `https://key@sentry.io/123456` | Sentry error tracking DSN |
| `SENTRY_ENVIRONMENT` | No | `development` | `production` | Sentry environment |
| `SENTRY_TRACE_SAMPLE_RATE` | No | `0.1` | `1.0` | Trace sample rate (0.0-1.0) |

```bash
export SENTRY_DSN=https://key@sentry.io/123456
export SENTRY_ENVIRONMENT=production
export SENTRY_TRACE_SAMPLE_RATE=0.1
```

### Prometheus Metrics

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `METRICS_ENABLED` | No | `true` | `false` | Enable Prometheus metrics |
| `METRICS_PORT` | No | `9090` | `9091` | Metrics endpoint port |

```bash
export METRICS_ENABLED=true
export METRICS_PORT=9090
```

---

## Feature Flags

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `FEATURE_VOICE_ENABLED` | No | `true` | `false` | Enable voice services |
| `FEATURE_P2P_ENABLED` | No | `true` | `false` | Enable P2P networking |
| `FEATURE_CRAWLER_ENABLED` | No | `true` | `false` | Enable web crawler |
| `FEATURE_SECURITY_ENABLED` | No | `true` | `false` | Enable security features |

```bash
# Disable specific features in development
export FEATURE_P2P_ENABLED=false
export FEATURE_CRAWLER_ENABLED=false
```

---

## Performance Tuning

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `MAX_CONNECTIONS` | No | `100` | `200` | Max database connections |
| `THREAD_POOL_SIZE` | No | `(num_cpus)` | `16` | Worker thread pool size |
| `REQUEST_TIMEOUT` | No | `30000` (ms) | `60000` | HTTP request timeout |
| `CACHE_TTL` | No | `3600` (sec) | `7200` | Default cache TTL |

```bash
# For high-traffic production deployment
export MAX_CONNECTIONS=200
export THREAD_POOL_SIZE=16
export REQUEST_TIMEOUT=60000
export CACHE_TTL=7200
```

---

## Development & Testing

| Variable | Required | Default | Example | Description |
|----------|----------|---------|---------|-------------|
| `MOCK_SERVICES` | No | `false` | `true` | Use mock services instead of real |
| `SKIP_DB_MIGRATIONS` | No | `false` | `true` | Skip database migrations |
| `DEBUG_MODE` | No | `false` | `true` | Enable debug logging |
| `DISABLE_CACHE` | No | `false` | `true` | Disable all caching |

```bash
# Development setup
export MOCK_SERVICES=true
export DEBUG_MODE=true
export DISABLE_CACHE=true
export RUST_LOG=debug
```

---

## Configuration Examples

### Minimal Production Setup
```bash
# Required
DATABASE_URL=postgresql://sam:password@db.example.com:5432/sam_db

# Recommended
REDIS_URL=redis://localhost:6379
PORT=8000
RUST_LOG=info
JWT_SECRET=$(openssl rand -hex 32)
SAM_ENVIRONMENT=production
```

### Full Production Setup
```bash
# Database
DATABASE_URL=postgresql://sam:password@db.example.com:5432/sam_db

# Redis Cache
REDIS_URL=redis://:password@cache.example.com:6379/0
REDIS_MAX_POOL_SIZE=20

# Application
PORT=8000
RUST_LOG=info
SAM_ENVIRONMENT=production
SAM_HOME=/opt/sam
SAM_DATA=/var/lib/sam
SAM_LOGS=/var/log/sam

# Security
JWT_SECRET=your_jwt_secret
SESSION_TIMEOUT=1440
CSRF_PROTECTION=true
SSH_KEY_PATH=/etc/sam/ssh_key

# Voice Services
STT_ENGINE=whisper-remote
STT_URL=http://stt-service:8001
STT_API_KEY=your_stt_key
TTS_ENGINE=piper
TTS_DEFAULT_VOICE=en_US-libritts-high

# Monitoring
SENTRY_DSN=https://key@sentry.io/123456
SENTRY_ENVIRONMENT=production
METRICS_ENABLED=true
LOG_FORMAT=json
LOG_FILE=/var/log/sam/app.log
LOG_RETENTION_DAYS=30
```

### Development Setup
```bash
# Database (SQLite for simplicity)
DATABASE_ENGINE=sqlite
SQLITE_DATABASE_PATH=/tmp/sam-dev.db

# Application
PORT=3000
RUST_LOG=debug
SAM_ENVIRONMENT=development
RUST_BACKTRACE=1

# Security
JWT_SECRET=dev_secret_not_secure
CSRF_PROTECTION=false

# Development
MOCK_SERVICES=true
DEBUG_MODE=true
DISABLE_CACHE=true
```

---

## Loading Configuration

### 1. Environment Variables (Highest Priority)
```bash
export DATABASE_URL=...
./target/release/sam
```

### 2. .env File
Create `.env` file in current directory:
```bash
DATABASE_URL=postgresql://...
REDIS_URL=redis://...
PORT=8000
```

Then run:
```bash
source .env
./target/release/sam
```

### 3. System Environment
```bash
# Linux/macOS: /etc/environment
# Or in systemd service: EnvironmentFile=/etc/sam/sam.env
```

**Note:** Command-line environment variables override `.env` file values.

---

## Validation

Check your configuration before starting:

```bash
# Print all S.A.M. environment variables
env | grep -E "^(SAM_|DATABASE_|REDIS_|JWT_|RUST_)" | sort

# Validate database connection
psql $DATABASE_URL -c "SELECT version();"

# Test Redis connection (if configured)
redis-cli -u "$REDIS_URL" ping
```

---

## Troubleshooting

### "Missing required variable DATABASE_URL"
```bash
# Set one of:
export DATABASE_URL=postgresql://...
# OR
export PG_DBNAME=...
export PG_USER=...
export PG_PASS=...
export PG_ADDRESS=...
```

### "Connection refused" for database
```bash
# Test database availability
psql -h localhost -U sam_user -d sam_db -c "SELECT 1"

# Check credentials
echo "PG_USER=$PG_USER PG_PASS=$PG_PASS PG_ADDRESS=$PG_ADDRESS"
```

### "Redis connection timeout"
```bash
# Test Redis
redis-cli -u "$REDIS_URL" ping

# Disable Redis if not needed
unset REDIS_URL
# (In-memory cache will be used instead)
```

---

Last Updated: 2026-04-02
Reference Version: 1.0
