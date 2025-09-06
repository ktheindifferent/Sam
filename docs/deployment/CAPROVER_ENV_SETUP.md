# CapRover Environment Setup Guide

## Overview

When deploying SAM to CapRover, the application runs in a containerized environment where it cannot manage Docker containers directly. Instead, external services like Redis, PostgreSQL, and TTS/STT services must be provided as separate CapRover apps or external services.

## Environment Detection

SAM automatically detects CapRover mode when the `CAPROVER` environment variable is set to `true`. In this mode:

- Docker container management is disabled
- External service URLs are used instead of local containers
- Service health checks adapt to external endpoints

## Required Environment Variables

### Core Configuration

```bash
# Enable CapRover mode
CAPROVER=true

# Database Configuration (choose one)
DATABASE_ENGINE=postgres  # or sqlite
DATABASE_URL=postgres://user:password@host:5432/database
# OR individual PostgreSQL variables:
PG_DBNAME=sam_db
PG_USER=sam_user
PG_PASS=your_password
PG_ADDRESS=postgres-host:5432
```

### Optional Services

#### Redis Cache

```bash
# Redis is optional - caching features will be disabled if not provided
REDIS_URL=redis://redis-host:6379
```

#### TTS (Text-to-Speech) Service

```bash
# External TTS service endpoint
TTS_URL=http://tts-service:8002/tts
TTS_API_KEY=your_api_key  # If authentication is required
TTS_DEFAULT_VOICE=default
TTS_DEFAULT_LANGUAGE=en-US
```

#### STT (Speech-to-Text) Service

```bash
# External STT service endpoint
STT_URL=http://stt-service:8001/stt
STT_API_KEY=your_api_key  # If authentication is required
STT_MODEL=whisper-base
```

## Service Deployment Architecture

### Recommended Setup

1. **Main SAM Application** - The core application
2. **PostgreSQL Database** - Deployed as a persistent CapRover app
3. **Redis Cache** (optional) - Deployed as a CapRover app
4. **TTS Service** (optional) - Separate microservice or external API
5. **STT Service** (optional) - Separate microservice or external API

### Example CapRover App Configuration

#### PostgreSQL App

```json
{
  "schemaVersion": 2,
  "dockerfileLines": [
    "FROM postgres:14-alpine",
    "ENV POSTGRES_DB=sam_db",
    "ENV POSTGRES_USER=sam_user",
    "ENV POSTGRES_PASSWORD=your_secure_password"
  ]
}
```

#### Redis App

```json
{
  "schemaVersion": 2,
  "dockerfileLines": [
    "FROM redis:7-alpine"
  ]
}
```

## Service Behavior in CapRover Mode

### Redis Service

- **Local Mode**: Manages Redis Docker container automatically
- **CapRover Mode**: Connects to external Redis via `REDIS_URL`
- **Fallback**: If `REDIS_URL` is not set, Redis features are disabled gracefully

### PostgreSQL Service

- **Local Mode**: Can start local PostgreSQL container
- **CapRover Mode**: Requires external PostgreSQL connection
- **Required**: PostgreSQL is required unless using SQLite

### TTS/STT Services

- **Local Mode**: Uses local Whisper/TTS engines
- **CapRover Mode**: Connects to external services via HTTP
- **Fallback**: Services are disabled if URLs not provided

## Health Checks

In CapRover mode, health checks are adapted:

- Docker container checks are skipped
- External service endpoints are checked via HTTP/TCP
- Services report as "unavailable" rather than trying to start containers

## Migration from Local to CapRover

1. **Export Data**: Backup your local PostgreSQL database
2. **Setup Services**: Deploy PostgreSQL and Redis on CapRover
3. **Configure Environment**: Set all required environment variables
4. **Import Data**: Restore database to CapRover PostgreSQL
5. **Deploy SAM**: Push SAM application with `CAPROVER=true`

## Troubleshooting

### Service Not Available

If you see "Service unavailable in CapRover mode":
- Check the corresponding `*_URL` environment variable
- Verify the external service is running and accessible
- Check network connectivity between CapRover apps

### Database Connection Failed

- Verify `DATABASE_URL` or `PG_*` variables are correct
- Ensure PostgreSQL app is running
- Check if database and user exist
- Verify network policy allows connection

### Redis Features Disabled

- This is normal if `REDIS_URL` is not set
- SAM will function without Redis but with reduced caching
- To enable, deploy Redis and set `REDIS_URL`

## Example .env File

See `.env.caprover.example` in the repository for a complete example configuration.

## Best Practices

1. **Use CapRover's Secret Management** for sensitive values
2. **Deploy services in order**: Database → Redis → SAM
3. **Use persistent volumes** for PostgreSQL data
4. **Monitor logs** via CapRover dashboard
5. **Set up health checks** for all services
6. **Use internal CapRover network** for service communication

## Security Considerations

- Never expose database ports publicly
- Use strong passwords for all services
- Enable SSL/TLS for external connections
- Rotate API keys regularly
- Use CapRover's built-in HTTPS support