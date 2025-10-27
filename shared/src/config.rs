use crate::types::{AppConfig, ServerConfig, DatabaseConfig, RedisConfig, MqttConfig, JwtConfig};
use anyhow::Result;
use config::{Config, Environment, File};
use dotenvy::dotenv;
use std::env;

pub fn load_config() -> Result<AppConfig> {
    // 加载 .env 文件
    dotenv().ok();

    let settings = Config::builder()
        // 添加默认配置文件
        .add_source(File::with_name("config/default").required(false))
        // 添加环境特定配置文件
        .add_source(
            File::with_name(&format!("config/{}", env::var("ENV").unwrap_or_else(|_| "development".to_string())))
                .required(false)
        )
        // 添加环境变量，使用 APP_ 前缀
        .add_source(Environment::with_prefix("APP").separator("_"))
        .build()?;

    // 构建配置
    let config: AppConfig = settings.try_deserialize()?;

    // 验证必要配置
    validate_config(&config)?;

    Ok(config)
}

fn validate_config(config: &AppConfig) -> Result<()> {
    if config.jwt.secret.is_empty() {
        return Err(anyhow::anyhow!("JWT secret cannot be empty"));
    }

    if config.database.url.is_empty() {
        return Err(anyhow::anyhow!("Database URL cannot be empty"));
    }

    if config.redis.url.is_empty() {
        return Err(anyhow::anyhow!("Redis URL cannot be empty"));
    }

    Ok(())
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: num_cpus::get(),
            },
            database: DatabaseConfig {
                url: "postgresql://echo_user:echo_pass@localhost:5432/echo_db".to_string(),
                max_connections: 20,
                min_connections: 5,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
            },
            mqtt: MqttConfig {
                broker: "localhost".to_string(),
                port: 1883,
                username: None,
                password: None,
            },
            jwt: JwtConfig {
                secret: "your-super-secret-jwt-key".to_string(),
                expiration_hours: 24,
            },
        }
    }
}