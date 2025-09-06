# S.A.M. CapRover Deployment Guide

Quick reference guide for deploying S.A.M. (Smart Artificial Mind) to CapRover.

## Quick Start

### Prerequisites
```bash
# Install CapRover CLI
npm install -g caprover

# Login to your CapRover instance
caprover login
```

### Automated Deployment
```bash
# Run the deployment script
./scripts/deploy-caprover.sh

# Or with custom app name and domain
./scripts/deploy-caprover.sh my-sam-app sam.mydomain.com
```

### Manual Deployment

1. **Create app in CapRover dashboard**
   - App name: `sam`
   - Enable persistent data

2. **Configure environment variables:**
   ```bash
   DATABASE_ENGINE=sqlite
   SQLITE_DATABASE_PATH=/var/lib/sam/sam.db
   PORT=8000
   SAM_HOME=/app
   SAM_DATA=/var/lib/sam
   SAM_LOGS=/var/log/sam
   RUST_LOG=info
   RUST_BACKTRACE=1
   RUN_MIGRATIONS=true
   ```

3. **Configure persistent volumes:**
   - `/var/lib/sam` → `sam-data`
   - `/var/log/sam` → `sam-logs`

4. **Deploy:**
   ```bash
   caprover deploy --appName sam
   ```

## Configuration Options

### SQLite (Default - Recommended for small deployments)
```bash
DATABASE_ENGINE=sqlite
SQLITE_DATABASE_PATH=/var/lib/sam/sam.db
```

### PostgreSQL (Recommended for production)
```bash
DATABASE_ENGINE=postgresql
DATABASE_URL=postgresql://user:pass@hostname:5432/sam_db
```

### Redis (Optional)
```bash
REDIS_URL=redis://your-redis-instance:6379
# Or to disable Redis entirely:
REDIS_DISABLED=true
```

## Resource Requirements

| Environment | CPU | RAM | Storage |
|------------|-----|-----|---------|
| Development | 0.5 CPU | 512MB | 5GB |
| Production | 2+ CPU | 2GB+ | 20GB+ |
| With AI Features | 4+ CPU | 4GB+ | 50GB+ |

## Health Checks

S.A.M. provides these health check endpoints:
- `/health` - Basic health check
- `/health/live` - Liveness probe (used by CapRover)
- `/health/ready` - Readiness probe

## Troubleshooting

### Build Issues
```bash
# View build logs
caprover logs --app sam --lines 100

# Check app configuration
caprover api --path "/api/v2/user/apps/data/sam" --method "GET"
```

### Runtime Issues
```bash
# View real-time logs
caprover logs --app sam --follow

# Execute commands in container
caprover exec --app sam --command "/bin/bash"

# Check environment variables
caprover exec --app sam --command "printenv | grep SAM"
```

### Database Issues
```bash
# For PostgreSQL
caprover exec --app sam --command "pg_isready -d $DATABASE_URL"

# For SQLite - check if database file exists
caprover exec --app sam --command "ls -la /var/lib/sam/"
```

## Scaling

### Horizontal Scaling
```bash
# Scale to 3 instances
caprover api --path "/api/v2/user/apps/data/sam" --method "POST" \
  --data '{"instanceCount": 3}'
```

### Vertical Scaling
- Increase CPU/RAM limits in CapRover app settings
- Monitor performance with: `caprover stats --app sam`

## Security

### Production Security Checklist
- [ ] Use PostgreSQL instead of SQLite
- [ ] Set `RUST_LOG=warn` (not debug/info)
- [ ] Set `RUST_BACKTRACE=0`
- [ ] Enable HTTPS in CapRover
- [ ] Use strong database passwords
- [ ] Configure firewall rules
- [ ] Regular security updates

### Environment Variables for Production
```bash
RUST_LOG=warn
RUST_BACKTRACE=0
DATABASE_ENGINE=postgresql
DATABASE_URL=postgresql://secure_user:strong_password@db:5432/sam_db
RUN_MIGRATIONS=false  # Run migrations manually for production
```

## Monitoring

### Built-in Monitoring
- CapRover dashboard shows resource usage
- Health checks run automatically
- Logs are centralized in CapRover

### External Monitoring
S.A.M. exposes metrics at `/metrics` (Prometheus format)

## Backup Strategy

### Database Backup
```bash
# For PostgreSQL
caprover exec --app sam --command "pg_dump $DATABASE_URL > /var/lib/sam/backup.sql"

# For SQLite
caprover exec --app sam --command "cp /var/lib/sam/sam.db /var/lib/sam/backup.db"
```

### File Backup
Persistent volumes are automatically backed up by CapRover if configured.

## Common Deployment Patterns

### Single Instance (Small Sites)
- 1 CPU, 1GB RAM
- SQLite database
- Local Redis (optional)

### High Availability (Production)
- 3+ instances
- External PostgreSQL
- External Redis
- Load balancer (automatic with CapRover)

### AI-Enhanced (Full Features)
- 4+ CPU, 4GB+ RAM
- GPU support (if available)
- Large storage for models
- External databases

## Support

For issues:
1. Check the main [README.md](README.md) CapRover section
2. Review CapRover logs: `caprover logs --app sam --follow`
3. Test locally: `docker-compose -f docker-compose.caprover.yml up`
4. Check [CapRover documentation](https://caprover.com/docs/)

---

**Last Updated:** September 2025
**S.A.M. Version:** 0.0.5
**CapRover Compatibility:** v1.10+