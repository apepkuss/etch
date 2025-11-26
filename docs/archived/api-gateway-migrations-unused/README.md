# API Gateway Migrations - Archived (Unused)

## Archive Date
2025-01-26

## Reason for Archival

These migration files were archived because they were **never executed** and are **outdated** compared to the current production database schema.

## Why These Were Never Used

1. **Empty Implementation**: The `run_migrations()` function in `api-gateway/src/database.rs` was never implemented:
   ```rust
   pub async fn run_migrations(&self) -> Result<()> {
       info!("Running database migrations...");
       // 这里可以使用 sqlx migrate 或者手动执行 SQL 文件
       // 由于我们使用 SQLX_OFFLINE=true，暂时跳过自动迁移
       info!("Database migrations completed");
       Ok(())
   }
   ```

2. **Different Initialization Method**: The project uses PostgreSQL's `docker-entrypoint-initdb.d` mechanism:
   - Database initialization: `database/init/01-init-database.sql`
   - This script runs automatically when the PostgreSQL container is first created

## Schema Differences

These migration files define an **outdated schema** that doesn't match the current production database:

### Missing Fields in devices table:
- `echokit_server_url` (VARCHAR(500) NOT NULL)
- `pairing_code` (VARCHAR(6))
- `serial_number` (VARCHAR(100))
- `registration_token` (VARCHAR(100))
- `mac_address` (VARCHAR(17))
- And devices.id changed from VARCHAR(36) to VARCHAR(255)

### Missing Fields in sessions table:
- `transcription` (TEXT)
- `response` (TEXT)
- `duration` (INTEGER)
- `start_time`, `end_time` (TIMESTAMP)
- And other fields restructured

### Missing Tables:
- `device_registration_tokens`
- `echokit_servers`
- `user_devices`
- `schema_versions`

## Current Database Management

**Schema Version**: 2025.01

**Initialization Method**:
- File: `database/init/01-init-database.sql`
- Mechanism: PostgreSQL `docker-entrypoint-initdb.d`
- Version Tracking: `schema_versions` table

**For schema changes**:
1. Update `database/init/01-init-database.sql`
2. Increment version in `schema_versions` table
3. Rebuild database container for changes to take effect

## Files Archived

- `001_create_users_table.sql` - Users table creation (still valid structure)
- `002_create_devices_table.sql` - Devices table creation (outdated schema)
- `003_create_sessions_table.sql` - Sessions table creation (outdated schema)

## Related Documentation

- See `docs/archived/migrations-unused/README.md` for the root-level migrations that were also archived
- See `docs/DATABASE_SCHEMA_ALIGNMENT_COMPLETE.md` for current schema details
