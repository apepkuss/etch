// 用户管理服务 - 数据访问层
use std::sync::Arc;
use sqlx::{PgPool, Row};
use echo_shared::{
    User, UserRole, UserRecord, CreateUserRequest, DatabaseError,
    RedisCache, CacheStrategy, CacheOperations, ttl, UserSessionCache,
};
use anyhow::Result;
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};

// 用户服务
#[derive(Clone)]
pub struct UserService {
    db: Arc<PgPool>,
    cache: Arc<RedisCache>,
}

impl UserService {
    pub fn new(db: Arc<PgPool>, cache: Arc<RedisCache>) -> Self {
        Self { db, cache }
    }

    /// 根据ID获取用户
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>> {
        let cache_key = format!("user:{}", user_id);

        // 尝试从缓存获取
        if let Some(user) = self.cache.get::<User>(&cache_key).await? {
            return Ok(Some(user));
        }

        // 从数据库查询
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT id, username, email, password_hash, role as "role: UserRole",
                   created_at, updated_at, is_active
            FROM users
            WHERE id = $1 AND is_active = TRUE
            "#,
            Uuid::parse_str(user_id)
                .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            let user = self.record_to_user(record);

            // 缓存结果
            self.cache.set(&cache_key, &user, ttl::USER_SESSION).await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?;

            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// 根据用户名获取用户
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let cache_key = format!("user:username:{}", username);

        // 尝试从缓存获取
        if let Some(user) = self.cache.get::<User>(&cache_key).await? {
            return Ok(Some(user));
        }

        // 从数据库查询
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT id, username, email, password_hash, role as "role: UserRole",
                   created_at, updated_at, is_active
            FROM users
            WHERE username = $1 AND is_active = TRUE
            "#,
            username
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            let user = self.record_to_user(record);

            // 缓存结果
            self.cache.set(&cache_key, &user, ttl::USER_SESSION).await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?;

            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// 根据邮箱获取用户
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let cache_key = format!("user:email:{}", email);

        // 尝试从缓存获取
        if let Some(user) = self.cache.get::<User>(&cache_key).await? {
            return Ok(Some(user));
        }

        // 从数据库查询
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT id, username, email, password_hash, role as "role: UserRole",
                   created_at, updated_at, is_active
            FROM users
            WHERE email = $1 AND is_active = TRUE
            "#,
            email
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            let user = self.record_to_user(record);

            // 缓存结果
            self.cache.set(&cache_key, &user, ttl::USER_SESSION).await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?;

            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// 创建新用户
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        // 检查用户名是否已存在
        if let Some(_) = self.get_user_by_username(&request.username).await? {
            return Err(DatabaseError::DuplicateRecord("Username already exists".to_string()).into());
        }

        // 检查邮箱是否已存在
        if let Some(_) = self.get_user_by_email(&request.email).await? {
            return Err(DatabaseError::DuplicateRecord("Email already exists".to_string()).into());
        }

        // 密码哈希
        let password_hash = hash(&request.password, DEFAULT_COST)
            .map_err(|e| DatabaseError::Connection(format!("Password hashing failed: {}", e)))?;

        // 创建用户
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            INSERT INTO users (username, email, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, email, password_hash, role as "role: UserRole",
                      created_at, updated_at, is_active
            "#,
            request.username,
            request.email,
            password_hash,
            request.role as UserRole
        )
        .fetch_one(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let user = self.record_to_user(record);

        // 缓存新用户信息
        let user_id = user.id.clone();
        let username = user.username.clone();
        let email = user.email.clone();

        let cache_keys = vec![
            format!("user:{}", user_id),
            format!("user:username:{}", username),
            format!("user:email:{}", email),
        ];

        for key in cache_keys {
            let _ = self.cache.set(&key, &user, ttl::USER_SESSION).await;
        }

        Ok(user)
    }

    /// 验证用户密码
    pub async fn verify_password(&self, username: &str, password: &str) -> Result<Option<User>> {
        // 获取用户记录（包含密码哈希）
        let record = sqlx::query!(
            r#"
            SELECT id, username, email, password_hash, role as "role: UserRole",
                   created_at, updated_at, is_active
            FROM users
            WHERE username = $1 AND is_active = TRUE
            "#,
            username
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            // 验证密码
            if verify(password, &record.password_hash).unwrap_or(false) {
                let user = self.record_to_user(record);

                // 缓存用户信息
                let cache_key = format!("user:username:{}", username);
                let _ = self.cache.set(&cache_key, &user, ttl::USER_SESSION).await;

                Ok(Some(user))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// 更新用户信息
    pub async fn update_user(
        &self,
        user_id: &str,
        username: Option<String>,
        email: Option<String>,
        role: Option<UserRole>,
    ) -> Result<Option<User>> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?;

        let record = sqlx::query_as!(
            UserRecord,
            r#"
            UPDATE users
            SET username = COALESCE($1, username),
                email = COALESCE($2, email),
                role = COALESCE($3, role),
                updated_at = NOW()
            WHERE id = $4 AND is_active = TRUE
            RETURNING id, username, email, password_hash, role as "role: UserRole",
                      created_at, updated_at, is_active
            "#,
            username,
            email,
            role as UserRole,
            user_uuid
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = record {
            let user = self.record_to_user(record);

            // 更新缓存
            let cache_keys = vec![
                format!("user:{}", user_id),
                format!("user:username:{}", user.username),
                format!("user:email:{}", user.email),
            ];

            for key in cache_keys {
                let _ = self.cache.set(&key, &user, ttl::USER_SESSION).await;
            }

            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// 更新用户密码
    pub async fn update_password(&self, user_id: &str, old_password: &str, new_password: &str) -> Result<bool> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?;

        // 获取当前密码哈希
        let current_record = sqlx::query!(
            "SELECT password_hash FROM users WHERE id = $1 AND is_active = TRUE",
            user_uuid
        )
        .fetch_optional(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        if let Some(record) = current_record {
            // 验证旧密码
            if !verify(old_password, &record.password_hash).unwrap_or(false) {
                return Err(DatabaseError::PermissionDenied("Invalid current password".to_string()).into());
            }

            // 生成新密码哈希
            let new_password_hash = hash(new_password, DEFAULT_COST)
                .map_err(|e| DatabaseError::Connection(format!("Password hashing failed: {}", e)))?;

            // 更新密码
            let result = sqlx::query!(
                "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2",
                new_password_hash,
                user_uuid
            )
            .execute(self.db.as_ref())
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

            let updated = result.rows_affected() > 0;

            if updated {
                // 清除用户相关缓存
                let _ = CacheStrategy::clear_user_cache(self.cache.as_ref(), user_id).await;
            }

            Ok(updated)
        } else {
            Ok(false)
        }
    }

    /// 停用用户（软删除）
    pub async fn deactivate_user(&self, user_id: &str) -> Result<bool> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| DatabaseError::InvalidInput("Invalid user ID".to_string()))?;

        let result = sqlx::query!(
            "UPDATE users SET is_active = FALSE, updated_at = NOW() WHERE id = $1",
            user_uuid
        )
        .execute(self.db.as_ref())
        .await
        .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let deactivated = result.rows_affected() > 0;

        if deactivated {
            // 清除用户相关缓存
            let _ = CacheStrategy::clear_user_cache(self.cache.as_ref(), user_id).await;
        }

        Ok(deactivated)
    }

    /// 创建用户会话缓存
    pub async fn create_user_session(
        &self,
        user_id: &str,
        session_token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let user = self.get_user_by_id(user_id).await?
            .ok_or_else(|| DatabaseError::UserNotFound(user_id.to_string()))?;

        let session_cache = UserSessionCache {
            user_id: user_id.to_string(),
            username: user.username.clone(),
            role: format!("{:?}", user.role),
            permissions: self.get_user_permissions(&user.role),
            created_at: chrono::Utc::now(),
            expires_at,
        };

        let cache_key = format!("user:session:{}", session_token);
        self.cache.set(&cache_key, &session_cache, ttl::USER_SESSION).await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        Ok(())
    }

    /// 获取用户会话
    pub async fn get_user_session(&self, session_token: &str) -> Result<Option<UserSessionCache>> {
        let cache_key = format!("user:session:{}", session_token);
        self.cache.get::<UserSessionCache>(&cache_key).await
            .map_err(|e| DatabaseError::Connection(e.to_string()))
    }

    /// 删除用户会话
    pub async fn delete_user_session(&self, session_token: &str) -> Result<bool> {
        let cache_key = format!("user:session:{}", session_token);
        self.cache.delete(&cache_key).await
            .map_err(|e| DatabaseError::Connection(e.to_string()))
    }

    /// 获取用户权限列表
    fn get_user_permissions(&self, role: &UserRole) -> Vec<String> {
        match role {
            UserRole::Admin => vec![
                "user:read".to_string(),
                "user:write".to_string(),
                "user:delete".to_string(),
                "device:read".to_string(),
                "device:write".to_string(),
                "device:delete".to_string(),
                "system:admin".to_string(),
            ],
            UserRole::User => vec![
                "device:read".to_string(),
                "device:write".to_string(),
                "profile:read".to_string(),
                "profile:write".to_string(),
            ],
            UserRole::Viewer => vec![
                "device:read".to_string(),
                "profile:read".to_string(),
            ],
        }
    }

    // 辅助方法：将数据库记录转换为User结构
    fn record_to_user(&self, record: UserRecord) -> User {
        User {
            id: record.id.to_string(),
            username: record.username,
            email: record.email,
            password_hash: record.password_hash,
            role: record.role,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_crud() {
        // 这里需要模拟数据库连接，实际测试需要test database
        // 暂时跳过实际测试
    }
}