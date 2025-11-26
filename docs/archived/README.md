# 归档文件说明

## migrations-unused/

**归档日期**: 2025-11-25

**归档原因**:

这个目录包含的数据库 migration 文件从未被实际执行过，原因如下：

1. **API Gateway 的 `run_migrations()` 函数为空实现**
   - 位置: `api-gateway/src/database.rs:34-43`
   - 状态: 函数体为空，只打印日志

2. **Bridge 服务没有 migration 代码**
   - 搜索结果: 无任何 migration 相关代码

3. **实际使用的初始化方式**
   - 方式: PostgreSQL Docker 镜像的 `/docker-entrypoint-initdb.d/` 自动初始化
   - 脚本: `database/init/01-init-database.sql`
   - 执行时机: 容器首次启动时

4. **Migration 文件存在的问题**
   - 编号混乱（多个文件使用 001、002 编号）
   - 执行顺序不明确
   - 与实际数据库结构不一致

**当前数据库初始化方式**:

```yaml
# docker-compose.yml
postgres:
  volumes:
    - ./database/init:/docker-entrypoint-initdb.d
```

PostgreSQL 容器首次启动时自动执行 `database/init/01-init-database.sql` 完成初始化。

**参考文档**:

- [DATABASE_DEPLOYMENT_ANALYSIS.md](../DATABASE_DEPLOYMENT_ANALYSIS.md) - 完整的数据库部署分析
- [DATABASE_SCHEMA_ALIGNMENT_COMPLETE.md](../DATABASE_SCHEMA_ALIGNMENT_COMPLETE.md) - Schema 对齐完成报告

**如果将来需要 Migration 系统**:

可以使用以下工具重新实现：
- `sqlx-cli` (Rust)
- `diesel_cli` (Rust)
- `flyway` (通用)
- `liquibase` (通用)

---

**归档状态**: 这些文件仅作为历史参考保留，不会被使用。
