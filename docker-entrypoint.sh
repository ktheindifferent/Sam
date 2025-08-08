#!/bin/bash
set -e

# Start Redis if not running externally
if [ -z "$REDIS_URL" ]; then
    echo "Starting local Redis server..."
    redis-server --daemonize yes
    export REDIS_URL="redis://localhost:6379"
fi

# Wait for PostgreSQL if DATABASE_URL is set
if [ ! -z "$DATABASE_URL" ]; then
    echo "Waiting for PostgreSQL..."
    until pg_isready -d "$DATABASE_URL"; do
        echo "PostgreSQL is unavailable - sleeping"
        sleep 1
    done
    echo "PostgreSQL is up"
fi

# Run database migrations if needed
if [ "$RUN_MIGRATIONS" = "true" ]; then
    echo "Running database migrations..."
    /opt/sam/sam migrate
fi

# Start the application
exec /opt/sam/sam "$@"