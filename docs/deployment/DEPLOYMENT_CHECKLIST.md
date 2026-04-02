# S.A.M. Deployment Checklist

## Pre-Deployment

### System Requirements
- [ ] Server meets minimum specs: 2+ CPU cores, 2GB RAM, 20GB storage
- [ ] Network connectivity verified
- [ ] Port 8000 (HTTP) is accessible
- [ ] Port 5432 (PostgreSQL) is accessible (if external DB)
- [ ] Port 6379 (Redis) is accessible (if external Redis)

### Dependencies
- [ ] Rust toolchain installed (for source builds)
- [ ] Docker installed and configured (for container deployments)
- [ ] PostgreSQL 12+ available
- [ ] Redis 6.0+ available (optional but recommended)

### Credentials & Secrets
- [ ] Database credentials prepared (user, password, DB name)
- [ ] Redis credentials prepared (if required)
- [ ] JWT secret generated: `openssl rand -hex 32`
- [ ] SSH keys generated for remote operations (if needed)
- [ ] TTS/STT API keys obtained (if using external services)

---

## Environment Setup

### 1. Database Configuration

#### PostgreSQL Setup
```bash
# Create database and user
createdb sam_db
createuser sam_user
psql -c "ALTER USER sam_user WITH PASSWORD 'secure_password';"
psql -c "GRANT ALL PRIVILEGES ON DATABASE sam_db TO sam_user;"

# Set environment variables
export PG_DBNAME=sam_db
export PG_USER=sam_user
export PG_PASS=secure_password
export PG_ADDRESS=localhost:5432
```

**Verification:**
```bash
psql -U sam_user -d sam_db -c "SELECT version();"
```

**Checklist:**
- [ ] Database created
- [ ] User created with correct password
- [ ] Permissions granted
- [ ] Connection verified
- [ ] Environment variables set

#### SQLite Setup (Development Only)
```bash
# Create data directory
mkdir -p /var/lib/sam

# Set environment variables
export DATABASE_ENGINE=sqlite
export SQLITE_DATABASE_PATH=/var/lib/sam/sam.db
```

**Checklist:**
- [ ] Data directory created and writable
- [ ] Path exported to environment

### 2. Redis Configuration (Optional but Recommended)

```bash
# Start Redis server
redis-server --daemonize yes --logfile /var/log/redis/redis-server.log

# Test connection
redis-cli ping  # Should respond with "PONG"

# Set environment variable
export REDIS_URL=redis://localhost:6379
```

**Verification:**
```bash
redis-cli INFO server
```

**Checklist:**
- [ ] Redis service running
- [ ] Connection verified
- [ ] Environment variable set
- [ ] Persistence configured (if needed)

### 3. Application Configuration

Create `.env` file or set environment variables:

```bash
# Core Application
export PORT=8000
export RUST_LOG=info
export RUST_BACKTRACE=1

# Database (PostgreSQL or SQLite)
export DATABASE_ENGINE=postgresql
export DATABASE_URL=postgresql://sam_user:secure_password@localhost:5432/sam_db

# Redis (optional)
export REDIS_URL=redis://localhost:6379

# External Services (optional)
export TTS_URL=http://localhost:8002/tts
export TTS_API_KEY=your_tts_api_key
export STT_URL=http://localhost:8001/stt
export STT_MODEL=whisper-base

# Security
export JWT_SECRET=$(openssl rand -hex 32)
export SENTRY_DSN=https://your_sentry_dsn@sentry.io/project

# Deployment Mode
export SAM_ENVIRONMENT=production
export CAPROVER=false  # true if deploying to CapRover
```

**Checklist:**
- [ ] All required variables set
- [ ] .env file created (or vars exported)
- [ ] File permissions set to 600 (if .env file)

---

## Deployment Methods

### Method 1: Docker Deployment

#### Build Docker Image

```bash
# Build with default configuration
docker build -t sam:latest .

# Build with specific features
docker build -t sam:full -f Dockerfile.full .
```

**Checklist:**
- [ ] Docker image builds successfully
- [ ] Image size is reasonable (< 2GB)

#### Run Docker Container

```bash
# Create persistent volume
docker volume create sam-data
docker volume create sam-logs

# Run container with environment variables
docker run -d \
  --name sam \
  -p 8000:8000 \
  -v sam-data:/var/lib/sam \
  -v sam-logs:/var/log/sam \
  -e DATABASE_URL="postgresql://sam_user:password@host:5432/sam_db" \
  -e REDIS_URL="redis://redis-host:6379" \
  -e RUST_LOG=info \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  --restart unless-stopped \
  sam:latest
```

**Verification:**
```bash
# Check container is running
docker ps | grep sam

# View logs
docker logs -f sam

# Test health endpoint
curl http://localhost:8000/health
```

**Checklist:**
- [ ] Container starts successfully
- [ ] Logs show no errors
- [ ] Health endpoint responds
- [ ] Database connection established
- [ ] Volumes mounted correctly

#### Docker Compose Setup

See `docker-compose.yml` for complete multi-service setup:
```bash
docker-compose up -d
```

**Checklist:**
- [ ] All services start (sam, postgres, redis)
- [ ] Services can communicate
- [ ] Volumes initialized
- [ ] Health checks passing

### Method 2: CapRover Deployment

See `docs/deployment/CAPROVER_DEPLOYMENT.md` for detailed instructions.

**Quick Steps:**
```bash
# 1. Install CapRover CLI
npm install -g caprover

# 2. Login to CapRover
caprover login

# 3. Run deployment script
./scripts/deploy-caprover.sh sam sam.yourdomain.com

# 4. Configure environment variables in CapRover dashboard
# 5. Enable persistent volumes
# 6. Deploy
```

**Checklist:**
- [ ] CapRover CLI installed
- [ ] Logged into CapRover instance
- [ ] App created in CapRover
- [ ] Environment variables configured
- [ ] Persistent volumes enabled
- [ ] Deployment successful
- [ ] Health checks passing

### Method 3: Bare Metal / VPS Deployment

#### Install Build Dependencies

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  curl \
  git \
  postgresql \
  redis-server \
  libssl-dev \
  pkg-config

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Checklist:**
- [ ] Build tools installed
- [ ] Rust toolchain installed
- [ ] Database services installed
- [ ] Dependencies verified

#### Build and Install

```bash
# Clone repository
git clone https://github.com/your-org/sam.git
cd sam

# Build release binary
cargo build --release

# Install to system path
sudo cp target/release/sam /usr/local/bin/sam

# Create system user
sudo useradd -r -s /bin/false sam

# Create directories
sudo mkdir -p /var/lib/sam /var/log/sam
sudo chown sam:sam /var/lib/sam /var/log/sam
sudo chmod 750 /var/lib/sam /var/log/sam
```

**Checklist:**
- [ ] Build completes successfully
- [ ] Binary installed to PATH
- [ ] System user created
- [ ] Data directories created with correct permissions

#### Create SystemD Service

Create `/etc/systemd/system/sam.service`:

```ini
[Unit]
Description=S.A.M. - Smart Artificial Mind
After=network.target postgresql.service

[Service]
Type=simple
User=sam
WorkingDirectory=/var/lib/sam
ExecStart=/usr/local/bin/sam
Restart=always
RestartSec=10

# Environment variables
EnvironmentFile=/etc/sam/sam.env

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/lib/sam /var/log/sam

[Install]
WantedBy=multi-user.target
```

Create `/etc/sam/sam.env`:
```bash
DATABASE_URL=postgresql://sam_user:password@localhost:5432/sam_db
REDIS_URL=redis://localhost:6379
RUST_LOG=info
PORT=8000
JWT_SECRET=your_secret_here
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable sam
sudo systemctl start sam
sudo systemctl status sam
```

**Checklist:**
- [ ] Service file created
- [ ] Environment file created with correct permissions (600)
- [ ] Service enabled
- [ ] Service starts successfully
- [ ] Service auto-restarts on reboot

---

## Post-Deployment Verification

### Health Checks

```bash
# Check application health
curl http://localhost:8000/health

# Check liveness
curl http://localhost:8000/health/live

# Check readiness
curl http://localhost:8000/health/ready
```

**Expected Response:**
```json
{
  "success": true,
  "status": "healthy",
  "uptime_seconds": 3600,
  "services": {
    "database": "operational",
    "cache": "operational",
    "api": "operational"
  }
}
```

**Checklist:**
- [ ] Health endpoint responds
- [ ] All services report operational
- [ ] Uptime increases over time

### API Connectivity

```bash
# Test authentication
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}'

# Test voice endpoint
curl http://localhost:8000/api/voice/tts/voices

# Test system metrics
curl http://localhost:8000/api/system/health
```

**Checklist:**
- [ ] API endpoints accessible
- [ ] Authentication works
- [ ] No 500 errors in response
- [ ] Response times acceptable (< 1 second)

### Database Verification

```bash
# Check database connection
curl http://localhost:8000/api/system/health | jq '.health.services.database'

# Or via psql
psql -U sam_user -d sam_db -c "SELECT COUNT(*) FROM pg_tables WHERE schemaname='public';"
```

**Checklist:**
- [ ] Database connection established
- [ ] Tables exist
- [ ] Data is persisting

### Logging

```bash
# Check application logs
tail -f /var/log/sam/sam.log

# For Docker
docker logs -f sam

# For SystemD service
journalctl -u sam -f
```

**Checklist:**
- [ ] No ERROR or WARN messages on startup
- [ ] Info messages indicate normal operation
- [ ] No database connection errors

---

## Production Hardening

### Security

- [ ] HTTPS/TLS enabled (reverse proxy with certificate)
- [ ] Firewall rules configured (allow only 8000/tcp)
- [ ] Database credentials not in logs
- [ ] JWT secret stored securely
- [ ] CORS properly configured
- [ ] Rate limiting enabled

### Monitoring & Alerting

- [ ] Logs aggregated (Sentry, LogRocket, etc.)
- [ ] Metrics exposed for Prometheus
- [ ] Health checks monitored
- [ ] Alerts configured for errors
- [ ] Uptime monitoring enabled

### Backup & Disaster Recovery

- [ ] Daily database backups scheduled
- [ ] Backups tested for restoration
- [ ] Data directory backups configured
- [ ] Backup storage secure and offsite
- [ ] Recovery time objective (RTO) documented
- [ ] Recovery point objective (RPO) documented

### Performance

- [ ] Redis caching enabled
- [ ] Database indexes optimized
- [ ] Connection pooling configured
- [ ] Load testing completed
- [ ] Expected throughput documented
- [ ] Scaling plan documented

---

## Troubleshooting

### Common Issues

#### Database Connection Fails
```bash
# Check PostgreSQL is running
pg_isready -h localhost -p 5432

# Verify credentials
psql -U sam_user -d sam_db -c "SELECT 1;"

# Check environment variables
env | grep DATABASE
```

#### Redis Connection Fails
```bash
# Check Redis is running
redis-cli ping

# Verify connection string
redis-cli -u $REDIS_URL ping

# Check Redis logs
tail -f /var/log/redis/redis-server.log
```

#### Application Won't Start
```bash
# Check logs for specific error
RUST_BACKTRACE=full ./target/release/sam

# Verify all environment variables
env | grep -E "DATABASE|REDIS|PORT"

# Check port is not in use
lsof -i :8000
```

---

## Rollback Plan

If deployment fails:

1. **For Docker:**
   ```bash
   docker ps -a | grep sam
   docker stop sam
   docker rm sam
   docker run -d --name sam ... [previous_version_image]
   ```

2. **For SystemD:**
   ```bash
   sudo systemctl stop sam
   sudo cp /usr/local/bin/sam.previous /usr/local/bin/sam
   sudo systemctl start sam
   ```

3. **Database:**
   ```bash
   # Restore from backup if needed
   psql sam_db < /backup/sam_db.sql
   ```

**Checklist:**
- [ ] Rollback procedure tested
- [ ] Previous version binary backed up
- [ ] Database backups available
- [ ] Team aware of rollback process

---

Last Updated: 2026-04-02
Deployment Version: 1.0
