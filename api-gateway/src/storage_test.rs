// 存储层集成测试程序
use std::sync::Arc;
use anyhow::Result;
use echo_shared::{CreateDeviceRequest, DeviceType, CreateUserRequest, UserRole, DeviceStatus};
use storage::{Storage, StorageConfig};
use device_service::DeviceService;
use user_service::UserService;

mod storage;
mod device_service;
mod user_service;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🔧 开始测试Echo系统存储层集成...");

    // 初始化存储层
    println!("📊 初始化存储层...");
    let storage_config = StorageConfig::default();
    let storage = Arc::new(Storage::new(storage_config).await?);
    println!("✅ 存储层初始化成功");

    // 创建服务层
    println!("🏗️  创建服务层...");
    let device_service = Arc::new(DeviceService::new(storage.db.clone(), storage.cache.clone()));
    let user_service = Arc::new(UserService::new(storage.db.clone(), storage.cache.clone()));
    println!("✅ 服务层创建成功");

    // 健康检查
    println!("🏥 执行健康检查...");
    let health = storage.health_check().await?;
    println!("📊 数据库状态: {}", if health.database { "✅ 正常" } else { "❌ 异常" });
    println!("💾 缓存状态: {}", if health.cache { "✅ 正常" } else { "❌ 异常" });

    // 测试用户管理
    println!("\n👤 测试用户管理...");
    test_user_management(&user_service).await?;

    // 测试设备管理
    println!("\n🔊 测试设备管理...");
    test_device_management(&device_service, &user_service).await?;

    println!("\n🎉 存储层集成测试完成！所有功能正常工作");

    Ok(())
}

async fn test_user_management(user_service: &UserService) -> Result<()> {
    // 创建测试用户
    println!("  📝 创建测试用户...");
    let user_request = CreateUserRequest {
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
        password: "test_password123".to_string(),
        role: UserRole::User,
    };

    let user = user_service.create_user(user_request).await?;
    println!("  ✅ 用户创建成功: {} ({})", user.username, user.id);

    // 验证用户密码
    println!("  🔐 验证用户密码...");
    let verified_user = user_service.verify_password("test_user", "test_password123").await?;
    assert!(verified_user.is_some(), "密码验证失败");
    println!("  ✅ 密码验证成功");

    // 根据ID获取用户
    println!("  🔍 根据ID获取用户...");
    let found_user = user_service.get_user_by_id(&user.id).await?;
    assert!(found_user.is_some(), "用户未找到");
    println!("  ✅ 用户获取成功: {}", found_user.unwrap().username);

    Ok(())
}

async fn test_device_management(device_service: &DeviceService, user_service: &UserService) -> Result<()> {
    // 获取测试用户
    let user = user_service.get_user_by_username("test_user").await?
        .ok_or_else(|| anyhow::anyhow!("测试用户未找到"))?;

    // 创建测试设备
    println!("  📱 创建测试设备...");
    let device_request = CreateDeviceRequest {
        name: "Test Speaker".to_string(),
        device_type: DeviceType::Speaker,
        location: Some("Living Room".to_string()),
        firmware_version: Some("1.0.0".to_string()),
    };

    let device = device_service.create_device(device_request, &user.id).await?;
    println!("  ✅ 设备创建成功: {} ({})", device.name, device.id);

    // 根据ID获取设备
    println!("  🔍 根据ID获取设备...");
    let found_device = device_service.get_device_by_id(&device.id).await?;
    assert!(found_device.is_some(), "设备未找到");
    println!("  ✅ 设备获取成功: {}", found_device.unwrap().name);

    // 获取用户设备列表
    println!("  📋 获取用户设备列表...");
    let devices = device_service.get_user_devices(&user.id, None).await?;
    assert!(!devices.is_empty(), "设备列表为空");
    println!("  ✅ 设备列表获取成功，共 {} 个设备", devices.len());

    // 更新设备状态
    println!("  🔄 更新设备状态...");
    let updated = device_service.update_device_status(
        &device.id,
        echo_shared::DeviceStatus::Online,
        Some(85),
        Some(60),
        Some(chrono::Utc::now()),
        Some(true),
    ).await?;
    assert!(updated, "设备状态更新失败");
    println!("  ✅ 设备状态更新成功");

    // 检查设备权限
    println!("  🔐 检查设备权限...");
    let has_permission = device_service.check_device_permission(&user.id, &device.id).await?;
    assert!(has_permission, "设备权限检查失败");
    println!("  ✅ 设备权限检查成功");

    Ok(())
}