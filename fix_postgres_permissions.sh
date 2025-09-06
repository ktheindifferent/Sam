#!/bin/bash

echo "Fixing PostgreSQL permissions for SAM..."

# Check if PostgreSQL is running
if ! pg_isready > /dev/null 2>&1; then
    echo "PostgreSQL is not running. Please start it first."
    exit 1
fi

# Grant CREATEDB privilege to sam user
echo "Granting CREATEDB privilege to user 'sam'..."
psql -U postgres -c "ALTER USER sam CREATEDB;" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✓ Granted CREATEDB privilege to user 'sam'"
else
    echo "Creating user 'sam' with CREATEDB privilege..."
    psql -U postgres -c "CREATE USER sam WITH PASSWORD 'sam' CREATEDB;" 2>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "✓ Created user 'sam' with CREATEDB privilege"
    else
        echo "⚠ Could not create or modify user 'sam'. You may need to run:"
        echo "  sudo -u postgres psql -c \"CREATE USER sam WITH PASSWORD 'sam' CREATEDB;\""
    fi
fi

# Check if database exists
DB_EXISTS=$(psql -U postgres -tAc "SELECT 1 FROM pg_database WHERE datname='sam'" 2>/dev/null)

if [ "$DB_EXISTS" = "1" ]; then
    echo "✓ Database 'sam' already exists"
    # Make sure sam owns it
    psql -U postgres -c "ALTER DATABASE sam OWNER TO sam;" 2>/dev/null
    echo "✓ Set ownership of database 'sam' to user 'sam'"
else
    echo "Creating database 'sam'..."
    psql -U postgres -c "CREATE DATABASE sam OWNER sam;" 2>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "✓ Created database 'sam' owned by user 'sam'"
    else
        echo "⚠ Could not create database 'sam'. You may need to run:"
        echo "  sudo -u postgres psql -c \"CREATE DATABASE sam OWNER sam;\""
    fi
fi

# Grant all privileges on database to sam user
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE sam TO sam;" 2>/dev/null
if [ $? -eq 0 ]; then
    echo "✓ Granted all privileges on database 'sam' to user 'sam'"
fi

echo ""
echo "PostgreSQL setup complete! You can now run SAM with:"
echo "  cargo run"
echo ""
echo "If you still have issues, try running these commands manually:"
echo "  sudo -u postgres psql"
echo "  ALTER USER sam CREATEDB;"
echo "  GRANT ALL PRIVILEGES ON DATABASE sam TO sam;"
echo "  \\q"