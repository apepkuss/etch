# API Gateway Scripts - Archived (Unused)

## Archive Date
2025-01-26

## Reason for Archival

The `generate-sqlx-cache.sh` script was archived because it was **never used in production** and has **outdated configuration** that doesn't match the current project setup.

## Files Archived

- `generate-sqlx-cache.sh` - SQLx query cache generation script (118 lines)

## Why This Script Was Never Used

1. **No References**: The script is not referenced in:
   - CI/CD pipelines (GitHub Actions, etc.)
   - Docker build process
   - Other project scripts
   - Documentation

2. **Outdated Configuration**: The script uses incorrect settings:
   - Database port: `localhost:5432` (actual: `localhost:10035`)
   - Container name: `postgres-echo_db` (actual: `postgres`)
   - Database credentials don't match docker-compose.yml

3. **Different Approach Adopted**: The project uses a simpler method:
   - Dockerfile: `ENV SQLX_OFFLINE=true` (line 20)
   - Builds without requiring database connection
   - No need for pre-generated query cache

## What This Script Was Supposed To Do

The script was designed to:
1. Check if Docker and PostgreSQL are running
2. Connect to the database
3. Run `cargo check` to generate SQLx query cache
4. Create `.sqlx/` directory with query metadata

This allows SQLx to compile without a live database connection (offline mode).

## Current SQLx Usage in Project

**Offline Mode (Default)**:
```dockerfile
# In Dockerfile
ENV SQLX_OFFLINE=true
RUN cargo build -p echo-api-gateway --release
```

**Local Development**:
```bash
# In .env.example
SQLX_OFFLINE=false  # Optional: set to false for online mode during development
```

**If You Need to Generate SQLx Cache**:

Instead of using this archived script, use the official SQLx CLI:

```bash
# Install SQLx CLI (if not installed)
cargo install sqlx-cli

# Generate query metadata
DATABASE_URL="postgres://echo_user:echo_password@localhost:10035/echo_db" \
cargo sqlx prepare

# Or use the shorthand
DATABASE_URL="postgres://echo_user:echo_password@localhost:10035/echo_db" \
cargo sqlx prepare --workspace
```

## Current .sqlx Directory

The `.sqlx/` directory in api-gateway contains only a minimal metadata file:

```json
{
  "version": "1",
  "db_type": "PostgreSQL"
}
```

This is sufficient for the current offline build setup.

## Why Offline Mode Works Better

**Advantages of SQLX_OFFLINE=true**:
1. ✅ **No database dependency** - Can build without running PostgreSQL
2. ✅ **Faster CI/CD** - No need to spin up database in build pipeline
3. ✅ **Simpler setup** - One environment variable, no scripts to maintain
4. ✅ **Consistent builds** - Same behavior across all environments

**When you might need online mode**:
- Adding new database queries
- Changing existing query schemas
- Validating queries against live database

## Related Documentation

- SQLx Offline Mode: https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query
- Project Database Setup: See `database/init/01-init-database.sql`
- Schema Version: 2025.01 (tracked in `schema_versions` table)
