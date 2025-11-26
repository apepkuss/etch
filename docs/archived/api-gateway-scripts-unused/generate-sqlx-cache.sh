#!/bin/bash

# SQLx Query Cache Generation Script
# This script helps generate SQLx query cache files when a database is available

set -e

echo "🔧 SQLx Query Cache Generation Script"
echo "===================================="

# Check if Docker is running and PostgreSQL container is available
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Database configuration
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="echo_db"
DB_USER="echo_user"
DB_PASSWORD="echo_password"
DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

echo "📋 Database Configuration:"
echo "   Host: ${DB_HOST}"
echo "   Port: ${DB_PORT}"
echo "   Database: ${DB_NAME}"
echo "   User: ${DB_USER}"
echo ""

# Function to wait for database to be ready
wait_for_db() {
    echo "⏳ Waiting for database to be ready..."
    until docker exec -i postgres-${DB_NAME} pg_isready -U ${DB_USER} -d ${DB_NAME} > /dev/null 2>&1; do
        echo "   Database not ready, waiting 2 seconds..."
        sleep 2
    done
    echo "✅ Database is ready!"
}

# Check if we can connect to the database
if docker exec -i postgres-${DB_NAME} pg_isready -U ${DB_USER} -d ${DB_NAME} > /dev/null 2>&1; then
    echo "✅ Database connection available"
    wait_for_db
else
    echo "❌ Cannot connect to database. Please ensure PostgreSQL is running:"
    echo "   docker run -d --name postgres-${DB_NAME} \\"
    echo "     -e POSTGRES_DB=${DB_NAME} \\"
    echo "     -e POSTGRES_USER=${DB_USER} \\"
    echo "     -e POSTGRES_PASSWORD=${DB_PASSWORD} \\"
    echo "     -p ${DB_PORT}:5432 \\"
    echo "     postgres:15"
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Export database URL for SQLx
export DATABASE_URL="${DATABASE_URL}"
export SQLX_OFFLINE=false

echo ""
echo "🔍 Checking current SQLx cache..."
if [ -d ".sqlx" ]; then
    echo "   Current .sqlx cache size: $(du -sh .sqlx 2>/dev/null | cut -f1 || echo 'unknown')"
    find .sqlx -name "*.json" -type f | wc -l | xargs -I {} echo "   Query files: {}"
else
    echo "   No .sqlx cache directory found"
fi

echo ""
echo "🚀 Generating SQLx query cache..."
echo "   This may take a few minutes..."

# Clean any existing cache to ensure fresh generation
if [ -d ".sqlx" ]; then
    echo "   Cleaning existing cache..."
    rm -rf .sqlx
fi

# Run cargo check to generate query cache
echo "   Running cargo check with database connection..."
if cargo check --all-targets; then
    echo "✅ SQLx query cache generated successfully!"
else
    echo "❌ Failed to generate SQLx query cache"
    echo "   This might be due to missing database schema or connection issues"
    exit 1
fi

echo ""
echo "📊 Cache generation results:"
if [ -d ".sqlx" ]; then
    echo "   New .sqlx cache size: $(du -sh .sqlx 2>/dev/null | cut -f1 || echo 'unknown')"
    find .sqlx -name "*.json" -type f | wc -l | xargs -I {} echo "   Query files: {}"
    echo ""
    echo "   Cache files:"
    find .sqlx -name "*.json" -type f -exec basename {} \; | sort | head -10
    if [ $(find .sqlx -name "*.json" -type f | wc -l) -gt 10 ]; then
        echo "   ... and more"
    fi
fi

echo ""
echo "🔄 Restoring offline mode..."
export SQLX_OFFLINE=true

echo ""
echo "✅ SQLx query cache generation complete!"
echo ""
echo "📝 Next steps:"
echo "   1. Keep SQLX_OFFLINE=true in .env for development without database"
echo "   2. The generated cache will be used for offline compilation"
echo "   3. Run this script again after schema changes to regenerate cache"
echo ""
echo "💡 To compile offline:"
echo "   cargo check --bin echo-api-gateway"