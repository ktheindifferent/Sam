# Database Configuration Guide

SAM now supports multiple database engines, with SQLite as the default for easy setup and portability.

## Supported Database Engines

- **SQLite** (Default) - Embedded, zero-configuration database
- **PostgreSQL** - Full-featured relational database
- **MySQL** (Coming Soon) - Popular open-source database
- **MariaDB** (Coming Soon) - MySQL-compatible database

## Quick Start with SQLite (Default)

SQLite is now the default database engine, requiring no additional setup:

```bash
# Run with SQLite (default)
cargo run

# Or with Docker
docker-compose -f docker-compose.sqlite.yml up
```

The SQLite database will be automatically created at `./data/sam.db`.

## Configuration

### Environment Variables

Set the database engine using the `DATABASE_ENGINE` environment variable:

```bash
# Use SQLite (default)
export DATABASE_ENGINE=sqlite

# Use PostgreSQL
export DATABASE_ENGINE=postgresql

# Use MySQL (coming soon)
export DATABASE_ENGINE=mysql

# Use MariaDB (coming soon)
export DATABASE_ENGINE=mariadb
```

### SQLite Configuration

```bash
# Set custom database path (default: ./data/sam.db)
export SQLITE_PATH=/path/to/your/database.db
```

### PostgreSQL Configuration

```bash
export DATABASE_ENGINE=postgresql
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_DB=sam
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=your_password
```

## Docker Deployment

### Using SQLite (Recommended for Development)

```bash
# Start with SQLite only
docker-compose -f docker-compose.sqlite.yml up

# Start with SQLite and Redis caching
docker-compose -f docker-compose.sqlite.yml --profile with-redis up

# Start with SQLite and monitoring
docker-compose -f docker-compose.sqlite.yml --profile monitoring up
```

### Using PostgreSQL

```bash
# Start with PostgreSQL
docker-compose -f docker-compose.sqlite.yml --profile with-postgres up

# Then set environment variable
export DATABASE_ENGINE=postgresql
```

Or use the original docker-compose.yml:

```bash
docker-compose up
```

## Migration Between Databases

To migrate from one database engine to another:

1. **Export data from current database**
   ```bash
   # Example: Export from SQLite
   sqlite3 data/sam.db .dump > backup.sql
   ```

2. **Switch database engine**
   ```bash
   export DATABASE_ENGINE=postgresql
   ```

3. **Import data to new database**
   ```bash
   # Example: Import to PostgreSQL
   psql -U postgres -d sam < backup.sql
   ```

## Performance Considerations

### SQLite
- **Pros**: Zero configuration, portable, fast for read-heavy workloads
- **Cons**: Limited concurrent writes, not suitable for distributed systems
- **Best for**: Development, single-server deployments, embedded systems

### PostgreSQL
- **Pros**: Full ACID compliance, advanced features, excellent concurrency
- **Cons**: Requires separate server, more resource intensive
- **Best for**: Production deployments, multi-user systems, complex queries

## Platform-Specific Installation

### Linux
```bash
# SQLite (usually pre-installed)
sudo apt-get install sqlite3 libsqlite3-dev

# PostgreSQL
sudo apt-get install postgresql postgresql-client
```

### macOS
```bash
# SQLite (pre-installed)
# PostgreSQL
brew install postgresql
```

### Windows
```bash
# SQLite - Download from https://www.sqlite.org/download.html
# PostgreSQL - Download installer from https://www.postgresql.org/download/windows/
```

## Troubleshooting

### SQLite Issues

1. **Database locked error**
   - Ensure only one writer at a time
   - Check file permissions: `chmod 664 data/sam.db`

2. **Performance issues**
   - Enable WAL mode (automatically done)
   - Consider PostgreSQL for high-concurrency scenarios

### PostgreSQL Issues

1. **Connection refused**
   - Check PostgreSQL is running: `systemctl status postgresql`
   - Verify connection settings in environment variables

2. **Authentication failed**
   - Check pg_hba.conf configuration
   - Verify username and password

## Development Tips

1. **Use SQLite for development** - Fast iteration, no setup required
2. **Use PostgreSQL for production** - Better performance, scalability
3. **Test with both engines** - Ensure compatibility across databases
4. **Regular backups** - Implement automated backup strategy

## Future Database Support

We plan to add support for:
- MySQL
- MariaDB
- CockroachDB
- TimescaleDB

## Contributing

To add support for a new database engine:

1. Implement the `DatabaseConnection` trait in `src/sam/db/database_engine.rs`
2. Add configuration in `DatabaseEngine::from_env()`
3. Update schema initialization
4. Add tests
5. Update documentation

For questions or issues, please open an issue on GitHub.