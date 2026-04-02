# S.A.M. Configuration Guide

This document describes all environment variables, configuration files, and settings available for S.A.M.

## Environment Variables

### Database Configuration (Required)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `PG_DBNAME` | Yes | `sam` | PostgreSQL database name |
| `PG_USER` | Yes | `sam` | PostgreSQL username |
| `PG_PASS` | Yes | `sam` | PostgreSQL password |
| `PG_ADDRESS` | Yes | `localhost` | PostgreSQL host and port (e.g., `localhost:5432`) |
| `DATABASE_URL` | No | - | Full PostgreSQL connection string (overrides individual PG_* vars) |

**Example:**
```bash
export PG_DBNAME=sam_db
export PG_USER=sam_user
export PG_PASS=secure_password
export PG_ADDRESS=localhost:5432
```

### Redis Configuration (Optional)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `REDIS_URL` | No | - | Redis connection URL for caching and session management |

**Example:**
```bash
export REDIS_URL=redis://localhost:6379
```

When Redis is not configured, in-memory caching is used as fallback.

### Application Settings

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `PORT` | No | `8000` | HTTP server port |
| `RUST_LOG` | No | `info` | Logging level (trace, debug, info, warn, error) |
| `HOME` | No | - | Home directory path (auto-detected if not set) |
| `CAPROVER` | No | `false` | Enable CapRover deployment mode |
| `DOCKER_CONTAINER` | No | `false` | Indicate running in Docker container |

**Example:**
```bash
export PORT=8080
export RUST_LOG=debug
export CAPROVER=true
```

### External Services (Optional)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TTS_URL` | No | - | External Text-to-Speech service URL |
| `TTS_API_KEY` | No | - | API key for external TTS service |
| `TTS_DEFAULT_VOICE` | No | `default` | Default voice for TTS |
| `TTS_DEFAULT_LANGUAGE` | No | `en-US` | Default language for TTS |
| `STT_URL` | No | - | External Speech-to-Text service URL |
| `STT_API_KEY` | No | - | API key for external STT service |
| `STT_MODEL` | No | `whisper-base` | STT model to use |

**Example:**
```bash
export TTS_URL=http://localhost:8002/tts
export TTS_API_KEY=your_api_key
export STT_URL=http://localhost:8001/stt
export STT_MODEL=whisper-large
```

### Security Settings

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SENTRY_DSN` | No | - | Sentry error tracking DSN |
| `SSH_KEY_PATH` | No | - | Path to SSH private key for remote connections |

## Configuration Files

### Main Configuration

**Location:** `/opt/sam/config.json` or `./config.json`

JSON format configuration file with application settings:

```json
{
  "database_url": "postgresql://sam:password@localhost/sam_db",
  "redis_url": "redis://localhost:6379",
  "http_port": 8000,
  "enable_tts": true,
  "enable_stt": true,
  "enable_crawler": true,
  "log_level": "info",
  "session_timeout_minutes": 1440,
  "jwt_secret": "your_jwt_secret_key",
  "csrf_protection_enabled": true,
  "cors_allowed_origins": [
    "http://localhost:3000",
    "https://example.com"
  ]
}
```

### Deployment Configuration

**CapRover Deployment:** See `.env.caprover.example`

Create `.env` file for CapRover deployments:

```bash
CAPROVER=true
DATABASE_URL=postgres://sam_user:password@captain-postgres:5432/sam_db
REDIS_URL=redis://captain-redis:6379
PORT=8000
SAM_LOG_LEVEL=info
SAM_ENVIRONMENT=production
```

### Example Environment Files

#### `.env.development`
```bash
PG_DBNAME=sam_dev
PG_USER=sam
PG_PASS=sam
PG_ADDRESS=localhost:5432
REDIS_URL=redis://localhost:6379
PORT=8000
RUST_LOG=debug
```

#### `.env.production`
```bash
PG_DBNAME=sam_prod
PG_USER=sam_prod_user
PG_PASS=<secure_password>
PG_ADDRESS=db.example.com:5432
REDIS_URL=redis://cache.example.com:6379
PORT=8000
RUST_LOG=warn
SENTRY_DSN=<your_sentry_dsn>
```

## Configuration Priority

Environment variables are loaded in this order (highest priority first):

1. **Explicit environment variables** (exported in shell or .env file)
2. **Config file** (`/opt/sam/config.json`)
3. **Default values** (hardcoded in application)

Example:
```bash
# This will use the exported PORT, ignoring config.json
export PORT=9000
./sam serve
```

## Required vs Optional

### Minimum Required Configuration
- PostgreSQL database connection (`PG_DBNAME`, `PG_USER`, `PG_PASS`, `PG_ADDRESS`)

### Recommended for Production
- PostgreSQL: Dedicated user, strong password, secure host
- Redis: For caching and session management
- RUST_LOG: Set to `warn` to reduce noise
- SENTRY_DSN: For error tracking and monitoring

### Optional but Useful
- External TTS/STT services: For better voice quality
- JWT_SECRET: For secure token generation
- CORS settings: For cross-origin API access

## Quick Start

### Development Setup
```bash
# Create .env.development
cat > .env.development << EOF
PG_DBNAME=sam_dev
PG_USER=sam
PG_PASS=sam
PG_ADDRESS=localhost:5432
REDIS_URL=redis://localhost:6379
PORT=8000
RUST_LOG=debug
EOF

# Load and run
source .env.development
./sam serve
```

### Docker Setup
```bash
docker-compose up -d postgres redis
docker build -t sam .
docker run -e DATABASE_URL="postgresql://sam:password@postgres/sam_db" \
           -e REDIS_URL="redis://redis:6379" \
           -p 8000:8000 \
           sam
```

### CapRover Setup
```bash
cp .env.caprover.example .env
# Edit .env with your CapRover settings
caprover deploy
```

## Troubleshooting

### Database Connection Issues
```bash
# Test PostgreSQL connection
psql -h $PG_ADDRESS -U $PG_USER -d $PG_DBNAME

# Check environment variables are set
env | grep PG_
```

### Redis Connection Issues
```bash
# Test Redis connection
redis-cli -h localhost ping

# If Redis is not available, Sam will use in-memory cache
```

### Configuration Not Loading
1. Check file permissions: `chmod 644 config.json`
2. Verify JSON syntax: `jq . config.json`
3. Check environment variable exports: `env | grep SAM_`
4. Review logs: `RUST_LOG=debug ./sam serve`

## Security Best Practices

1. **Never commit credentials** to version control
2. **Use strong passwords** for production databases
3. **Enable CSRF protection** in production
4. **Set CORS_ALLOWED_ORIGINS** explicitly
5. **Use HTTPS** for external service communication
6. **Rotate JWT secrets** regularly
7. **Monitor Sentry errors** for security issues

---

For more information, see:
- [README.md](../README.md) - General overview
- [API.md](./API.md) - API endpoint documentation
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System design
