// Bridge服务简单存储测试 - 验证数据库连接
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🔧 开始测试Bridge服务数据库连接...");

    // 初始化数据库连接
    println!("📊 初始化数据库连接...");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://echo_user:echo_password@localhost:5432/echo_db".to_string());

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;

    println!("✅ 数据库连接成功");

    // 测试基本查询
    println!("🔍 测试基本数据库查询...");

    // 测试用户表
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("用户表查询失败: {}", e))?;

    println!("  👥 用户表记录数: {}", user_count);

    // 测试设备表
    let device_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices")
        .fetch_one(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("设备表查询失败: {}", e))?;

    println!("  🔊 设备表记录数: {}", device_count);

    // 测试会话表
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("会话表查询失败: {}", e))?;

    println!("  🎭 会话表记录数: {}", session_count);

    // 创建测试设备
    println!("📱 创建测试设备...");
    let device_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO devices (id, name, device_type, owner_id)
        VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(device_id)
    .bind("Bridge Test Device")
    .bind("speaker")
    .bind(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap())
    .execute(&db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("测试设备创建失败: {}", e))?;

    println!("  ✅ 测试设备创建成功: {}", device_id);

    // 创建测试会话
    println!("🎭 创建测试会话...");
    let session_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO sessions (id, device_id, status, wake_reason)
        VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(session_id)
    .bind(device_id)
    .bind("active")
    .bind(Some("Bridge Test Wake".to_string()))
    .execute(&db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("测试会话创建失败: {}", e))?;

    println!("  ✅ 测试会话创建成功: {}", session_id);

    // 查询测试数据
    println!("🔍 验证测试数据...");
    let test_session: (String, String, String, Option<String>) = sqlx::query_as(
        r#"
        SELECT s.id, d.name, s.status, s.wake_reason
        FROM sessions s
        JOIN devices d ON s.device_id = d.id
        WHERE s.id = $1
        "#
    )
    .bind(session_id)
    .fetch_one(&db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("测试数据查询失败: {}", e))?;

    println!("  📋 会话信息:");
    println!("    - 会话ID: {}", test_session.0);
    println!("    - 设备名称: {}", test_session.1);
    println!("    - 会话状态: {}", test_session.2);
    println!("    - 唤醒原因: {:?}", test_session.3);

    // 清理测试数据
    println!("🧹 清理测试数据...");

    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("测试会话清理失败: {}", e))?;

    sqlx::query("DELETE FROM devices WHERE id = $1")
        .bind(device_id)
        .execute(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("测试设备清理失败: {}", e))?;

    println!("  ✅ 测试数据清理完成");

    // 关闭连接池
    db_pool.close().await;

    println!("\n🎉 Bridge服务数据库连接测试完成！");
    println!("✅ 数据库连接正常，可以进行完整的CRUD操作");

    Ok(())
}