# S.A.M. Deployment Guide

Complete guide for deploying S.A.M. (Smart Artificial Mind) to production environments.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Docker Deployment](#docker-deployment)
3. [Traditional Server Deployment](#traditional-server-deployment)
4. [CapRover Deployment](#caprover-deployment)
5. [Database Setup](#database-setup)
6. [Environment Variables](#environment-variables)
7. [Monitoring & Maintenance](#monitoring--maintenance)
8. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Using Docker (Recommended for Production)

```bash
# 1. Clone the repository
git clone https://github.com/opensam/sam.git
cd sam

# 2. Configure environment
cp .env.caprover.example .env
# Edit .env with your database and service credentials

# 3. Build and start
docker-compose up -d

# 4. Verify
curl http://localhost:8000/api/health
```

**Required for Docker:**
- Docker 20.10+
- Docker Compose 1.29+
- 2GB+ RAM available
- 10GB+ disk space for images and data

### Using Native Binary (Advanced)

```bash
# 1. Build release binary
cargo build --release

# 2. Configure environment
export DATABASE_URL="postgresql://user:pass@host:5432/sam_db"
export REDIS_URL="redis://localhost:6379"
export PORT=8000

# 3. Run migrations
./target/release/sam migrate

# 4. Start server
./target/release/sam serve
```

---

## Docker Deployment

### Dockerfile Overview

The provided `Dockerfile` includes:
- Multi-stage build for optimal image size (~800MB)
- Pre-built Whisper models for STT/TTS
- All dependencies (PostgreSQL client, ffmpeg, etc.)
- Non-root user for security

### Building the Image

```bash
# Standard build
docker build -t sam:latest .

# Build with specific features
docker build \
  --build-arg FEATURES="voice,p2p,security" \
  -t sam:latest .

# Build with specific platform (for ARM/Apple Silicon)
docker build \
  --platform linux/arm64 \
  -t sam:latest .
```

### Docker Compose Setup

The `docker-compose.yml` includes three services:

#### 1. SAM Application Service
```yaml
services:
  sam:
    image: sam:latest
    ports:
      - "8000:8000"
    environment:
      DATABASE_URL: postgresql://sam:password@postgres:5432/sam_db
      REDIS_URL: redis://redis:6379
      RUST_LOG: info
    depends_on:
      - postgres
      - redis
```

**Key options:**
- `restart_policy: always` - Auto-restart on failure
- `volumes` - Mount for persistent data/logs
- `health_check` - Liveness probe every 30 seconds

#### 2. PostgreSQL Service
```yaml
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: sam_db
      POSTGRES_USER: sam
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
```

#### 3. Redis Service (Optional, but Recommended)
```yaml
  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis_data:/data
```

### Running Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f sam

# Stop services
docker-compose down

# Reset database (WARNING: destructive)
docker-compose down -v
docker-compose up -d
```

### Docker Networking

Services communicate via Docker network:
- `sam` connects to `postgres:5432` and `redis:6379` (internal)
- Only port 8000 exposed to host
- Use service names (not localhost) in connection strings

---

## Traditional Server Deployment

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 2 GB | 4+ GB |
| Disk | 20 GB | 50+ GB |
| OS | Ubuntu 20.04+ | Ubuntu 22.04+ |
| Network | 100 Mbps | 1 Gbps |

### Step 1: Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install system packages
sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  postgresql-15 \
  postgresql-contrib-15 \
  redis-server \
  ffmpeg \
  libavformat-dev \
  libavcodec-dev \
  libswscale-dev

# Start services
sudo systemctl enable postgresql redis-server
sudo systemctl start postgresql redis-server
```

### Step 2: Build from Source

```bash
# Clone repository
git clone https://github.com/opensam/sam.git
cd sam

# Build release binary
cargo build --release

# Binary location: ./target/release/sam
```

### Step 3: Create System User

```bash
# Create dedicated user
sudo useradd -m -s /bin/bash sam

# Create directories
sudo mkdir -p /opt/sam /var/log/sam /var/lib/sam
sudo chown -R sam:sam /opt/sam /var/log/sam /var/lib/sam

# Copy binary
sudo cp target/release/sam /opt/sam/sam
sudo chown sam:sam /opt/sam/sam
sudo chmod 755 /opt/sam/sam
```

### Step 4: Configure Environment

```bash
# Create systemd service
sudo tee /etc/systemd/system/sam.service > /dev/null <<EOF
[Unit]
Description=S.A.M. - Smart Artificial Mind
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=sam
WorkingDirectory=/opt/sam
EnvironmentFile=/opt/sam/.env
ExecStart=/opt/sam/sam serve
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=sam

[Install]
WantedBy=multi-user.target
EOF

# Create environment file
sudo tee /opt/sam/.env > /dev/null <<EOF
DATABASE_URL=postgresql://sam:$(openssl rand -base64 32)@localhost:5432/sam_db
REDIS_URL=redis://localhost:6379
PORT=8000
RUST_LOG=info
HOME=/var/lib/sam
EOF

# Secure environment file
sudo chmod 600 /opt/sam/.env
```

### Step 5: Setup PostgreSQL

```bash
# Create database and user
sudo -u postgres psql <<EOF
CREATE DATABASE sam_db;
CREATE USER sam WITH PASSWORD '$(openssl rand -base64 32)';
GRANT ALL PRIVILEGES ON DATABASE sam_db TO sam;
\connect sam_db
CREATE SCHEMA IF NOT EXISTS public;
GRANT ALL PRIVILEGES ON SCHEMA public TO sam;
EOF

# Initialize schema
sudo -u sam /opt/sam/sam migrate
```

### Step 6: Start Service

```bash
# Enable and start
sudo systemctl enable sam
sudo systemctl start sam

# Check status
sudo systemctl status sam

# View logs
sudo journalctl -u sam -f
```

### Step 7: Configure Reverse Proxy (Nginx)

```nginx
upstream sam_backend {
    server 127.0.0.1:8000;
    keepalive 32;
}

server {
    listen 80;
    server_name sam.example.com;

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name sam.example.com;

    # SSL certificates
    ssl_certificate /etc/letsencrypt/live/sam.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/sam.example.com/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Proxy configuration
    location / {
        proxy_pass http://sam_backend;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
    }

    # Health check endpoint
    location /api/health {
        access_log off;
        proxy_pass http://sam_backend;
    }
}
```

### Step 8: Setup SSL Certificates

```bash
# Install Certbot
sudo apt install -y certbot python3-certbot-nginx

# Obtain certificate
sudo certbot certonly --nginx -d sam.example.com

# Auto-renewal
sudo systemctl enable certbot.timer
sudo systemctl start certbot.timer
```

---

## CapRover Deployment

For CapRover-specific setup, see:
- [CAPROVER_DEPLOYMENT.md](./deployment/CAPROVER_DEPLOYMENT.md) - Full deployment guide
- [CAPROVER_ENV_SETUP.md](./deployment/CAPROVER_ENV_SETUP.md) - Environment configuration

### Quick CapRover Checklist

1. ✓ Copy `.env.caprover.example` to CapRover settings
2. ✓ Configure PostgreSQL and Redis in CapRover
3. ✓ Set `CAPROVER=true` environment variable
4. ✓ Deploy via `captain deploy` or GitHub webhook
5. ✓ Monitor at `https://captain.example.com`

---

## Database Setup

### PostgreSQL Production Setup

See [DATABASE_SETUP.md](./deployment/DATABASE_SETUP.md) for:
- Initial schema creation
- User and role configuration
- Backup and recovery procedures
- Performance tuning
- High availability setup

### Quick Database Test

```bash
# Test PostgreSQL connection
PGPASSWORD=password psql -h localhost -U sam -d sam_db -c "SELECT version();"

# Test Redis connection
redis-cli ping

# Test S.A.M. database connection
curl http://localhost:8000/api/health
```

---

## Environment Variables

For comprehensive variable reference, see [ENVIRONMENT_VARIABLES.md](../ENVIRONMENT_VARIABLES.md).

### Critical Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `DATABASE_URL` | PostgreSQL connection | `postgresql://sam:pass@localhost/sam_db` |
| `REDIS_URL` | Redis cache connection | `redis://localhost:6379` |
| `PORT` | Server port | `8000` |
| `RUST_LOG` | Logging level | `info` |
| `CAPROVER` | CapRover mode | `true` or `false` |

### Sensitive Variables (Keep Secret!)

- `DATABASE_URL` - Contains database password
- `REDIS_PASSWORD` - Redis authentication
- `JWT_SECRET` - Session signing key
- `TTS_API_KEY` / `STT_API_KEY` - External service keys

**Security Best Practices:**
- Use `.env` files with restricted permissions (600)
- Never commit `.env` to git
- Rotate secrets regularly
- Use environment-specific values for dev/staging/prod

---

## Monitoring & Maintenance

### Health Checks

```bash
# API health endpoint
curl http://localhost:8000/api/health

# Database connectivity
curl http://localhost:8000/api/db/health

# Redis connectivity  
curl http://localhost:8000/api/cache/health

# Metrics endpoint
curl http://localhost:8000/metrics
```

### Logging

```bash
# View application logs
docker-compose logs -f sam

# Or with systemd
sudo journalctl -u sam -f --lines=50

# Search for errors
docker-compose logs sam | grep ERROR
journalctl -u sam | grep ERROR
```

### Backup & Recovery

```bash
# Backup database
pg_dump -U sam sam_db > sam_backup_$(date +%Y%m%d).sql

# Backup Redis
redis-cli --rdb /backup/dump.rdb

# Restore database
psql -U sam sam_db < sam_backup.sql

# Restore Redis
redis-cli shutdown
cp /backup/dump.rdb /var/lib/redis/
redis-server
```

### Performance Tuning

1. **PostgreSQL:**
   - Increase `shared_buffers` to 25% of RAM
   - Set `work_mem` to RAM / (max_connections * 2)
   - Enable query logging: `log_min_duration_statement = 1000`

2. **Redis:**
   - Monitor memory usage: `redis-cli info memory`
   - Configure eviction policy: `maxmemory-policy allkeys-lru`
   - Enable persistence: `appendonly yes`

3. **S.A.M.:**
   - Adjust worker threads: `--worker-threads <count>`
   - Configure cache TTL: `CACHE_TTL_SECONDS`
   - Monitor connections: `curl http://localhost:8000/metrics`

---

## Troubleshooting

### Service Won't Start

**Problem:** `systemctl start sam` fails

**Solutions:**
```bash
# Check logs
sudo journalctl -u sam -n 50

# Verify environment file
cat /opt/sam/.env

# Check permissions
ls -la /opt/sam/

# Test binary directly
/opt/sam/sam serve

# Test database connection
psql postgresql://sam:pass@localhost/sam_db
```

### Database Connection Errors

**Problem:** "Cannot connect to database"

**Solutions:**
```bash
# Verify PostgreSQL is running
sudo systemctl status postgresql

# Check connection string format
echo $DATABASE_URL

# Test connection with psql
psql $DATABASE_URL

# Check PostgreSQL logs
sudo tail -f /var/log/postgresql/postgresql-*.log
```

### High Memory Usage

**Problem:** Service consuming excessive RAM

**Solutions:**
```bash
# Check current usage
docker stats sam
# or
ps aux | grep sam

# Reduce cache size
export REDIS_MAX_MEMORY=1gb

# Reduce worker threads
# Edit /etc/systemd/system/sam.service
# ExecStart=/opt/sam/sam serve --worker-threads 4

# Monitor memory trends
watch -n 1 free -h
```

### Slow API Responses

**Problem:** Endpoints responding slowly

**Solutions:**
```bash
# Check database performance
psql $DATABASE_URL -c "SELECT * FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 10;"

# Check Redis connectivity
redis-cli latency latest

# View application logs for errors
tail -f /var/log/sam/app.log

# Check CPU usage
top -p $(pgrep -f 'sam serve')
```

### WebSocket Connection Issues

**Problem:** WebSocket connections failing

**Solutions:**
```bash
# Verify WebSocket support in proxy
# Check nginx config includes:
#   proxy_upgrade http;
#   upgrade http_upgrade;

# Check firewall
sudo ufw status

# Enable port if needed
sudo ufw allow 8000/tcp

# Verify with wscat
npm install -g wscat
wscat -c ws://localhost:8000/api/ws
```

---

## Getting Help

- **Documentation:** https://github.com/opensam/sam/docs
- **Issues:** https://github.com/opensam/sam/issues
- **Discussions:** https://github.com/opensam/sam/discussions
- **Security:** See SECURITY_GUIDE.md for vulnerability reporting

---

## Deployment Checklist

See [DEPLOYMENT_CHECKLIST.md](./deployment/DEPLOYMENT_CHECKLIST.md) for complete pre-deployment checklist.

Quick summary:
- [ ] System meets minimum requirements
- [ ] All dependencies installed
- [ ] Environment variables configured
- [ ] Database initialized
- [ ] SSL certificates obtained
- [ ] Reverse proxy configured
- [ ] Health checks passing
- [ ] Monitoring setup
- [ ] Backup strategy in place
