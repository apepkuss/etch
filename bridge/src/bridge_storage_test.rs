// Bridge服务存储层集成测试程序
use std::sync::Arc;
use anyhow::Result;
mod session_service_simple;
use session_service_simple::{SessionService, SessionStats};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🔧 开始测试Bridge服务存储层集成...");

    // 初始化数据库连接
    println!("📊 初始化数据库连接...");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://echo_user:echo_password@localhost:5432/echo_db".to_string());

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {}", e))?;

    println!("✅ 数据库连接成功");

    // 创建会话服务
    println!("🏗️  创建会话服务...");
    let session_service = Arc::new(SessionService::new(db_pool.into()));
    println!("✅ 会话服务创建成功");

    // 测试会话管理
    println!("\n🎭 测试会话管理...");
    test_session_management(&session_service).await?;

    println!("\n🎉 Bridge服务存储层集成测试完成！");

    Ok(())
}

async fn test_session_management(session_service: &SessionService) -> Result<()> {
    // 创建测试会话
    println!("  📝 创建测试会话...");
    let session = session_service.create_session(
        "test-device-001",
        Some("test-user-001"),
        Some("VoiceWake".to_string()),
    ).await?;
    println!("  ✅ 会话创建成功: {} (设备: {})", session.id, session.device_id);

    // 获取会话详情
    println!("  🔍 获取会话详情...");
    let found_session = session_service.get_session(&session.id.to_string()).await?;
    assert!(found_session.is_some(), "会话未找到");
    println!("  ✅ 会话获取成功: 状态 = {}", found_session.unwrap().status);

    // 更新会话
    println!("  🔄 更新会话状态...");
    let updated_session = session_service.update_session(
        &session.id.to_string(),
        "completed".to_string(),
        Some("你好，世界！".to_string()),
        Some("你好！我是Echo助手。".to_string()),
        None,
    ).await?;
    assert!(updated_session.is_some(), "会话更新失败");
    println!("  ✅ 会话更新成功: 状态 = {}", updated_session.unwrap().status);

    // 获取设备会话列表
    println!("  📋 获取设备会话列表...");
    let device_sessions = session_service.get_device_sessions(
        &session.device_id.to_string(),
        Some(10),
        Some(0),
    ).await?;
    assert!(!device_sessions.is_empty(), "设备会话列表为空");
    println!("  ✅ 设备会话列表获取成功，共 {} 个会话", device_sessions.len());

    // 获取会话统计
    println!("  📊 获取会话统计...");
    let stats = session_service.get_session_stats(
        Some(&session.device_id.to_string()),
        Some(24), // 最近24小时
    ).await?;
    println!("  ✅ 会话统计获取成功:");
    println!("    - 总会话数: {}", stats.total_sessions);
    println!("    - 活跃会话: {}", stats.active_sessions);
    println!("    - 完成会话: {}", stats.completed_sessions);
    println!("    - 失败会话: {}", stats.failed_sessions);
    println!("    - 超时会话: {}", stats.timeout_sessions);
    if let Some(avg) = stats.avg_duration_minutes {
        println!("    - 平均时长: {} 分钟", avg);
    }

    // 获取活跃会话列表
    println!("  🟢 获取活跃会话列表...");
    let active_sessions = session_service.get_active_sessions().await?;
    println!("  ✅ 活跃会话列表获取成功，共 {} 个会话", active_sessions.len());

    // 结束会话
    println!("  🔚 结束测试会话...");
    let ended_session = session_service.update_session(
        &session.id.to_string(),
        "completed".to_string(),
        Some("测试结束".to_string()),
        Some("测试响应".to_string()),
        None,
    ).await?;
    assert!(ended_session.is_some(), "会话结束失败");
    println!("  ✅ 会话结束成功");

    Ok(())
}