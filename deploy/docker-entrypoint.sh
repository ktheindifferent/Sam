#!/bin/bash
set -e

# Script optimized for CapRover deployment
echo "Starting S.A.M. (Smart Artificial Mind) v0.0.5"
echo "CapRover deployment - $(date)"

# Create necessary directories
mkdir -p "$SAM_DATA" "$SAM_LOGS"

# Handle database setup based on engine
if [ "${DATABASE_ENGINE:-sqlite}" = "sqlite" ]; then
    echo "Using SQLite database at: $SQLITE_DATABASE_PATH"
    
    # Ensure SQLite database directory exists
    mkdir -p "$(dirname "$SQLITE_DATABASE_PATH")"
    
    # Initialize database if it doesn't exist
    if [ ! -f "$SQLITE_DATABASE_PATH" ]; then
        echo "Creating new SQLite database..."
        touch "$SQLITE_DATABASE_PATH"
    fi
elif [ "${DATABASE_ENGINE}" = "postgresql" ]; then
    echo "Using PostgreSQL database"
    
    # Wait for PostgreSQL if DATABASE_URL is set
    if [ ! -z "$DATABASE_URL" ]; then
        echo "Waiting for PostgreSQL to be ready..."
        until pg_isready -d "$DATABASE_URL"; do
            echo "PostgreSQL is unavailable - sleeping for 2 seconds"
            sleep 2
        done
        echo "PostgreSQL is ready"
    fi
fi

# Start Redis if not running externally and not disabled
if [ "${REDIS_DISABLED:-false}" != "true" ] && [ -z "$REDIS_URL" ]; then
    echo "Starting local Redis server..."
    redis-server --daemonize yes --bind 127.0.0.1 --port 6379 --dir /tmp/sam
    export REDIS_URL="redis://localhost:6379"
    echo "Redis started at $REDIS_URL"
fi

# Run database migrations if requested
if [ "${RUN_MIGRATIONS:-false}" = "true" ]; then
    echo "Running database migrations..."
    /app/sam migrate || {
        echo "Warning: Database migration failed, continuing anyway"
    }
fi

# Show configuration summary
echo "Configuration:"
echo "  - SAM_HOME: $SAM_HOME"
echo "  - SAM_DATA: $SAM_DATA"
echo "  - SAM_LOGS: $SAM_LOGS"
echo "  - DATABASE_ENGINE: ${DATABASE_ENGINE:-sqlite}"
echo "  - PORT: ${PORT:-8000}"
echo "  - RUST_LOG: ${RUST_LOG:-info}"

# Health check before starting
echo "Performing initial health check..."
if command -v curl >/dev/null 2>&1; then
    echo "curl is available for health checks"
else
    echo "Warning: curl not available for health checks"
fi

# Start the application
echo "Starting S.A.M. application with arguments: $@"
exec /app/sam "$@"