use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use crate::types::{Claims, UserRole, EchoError};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

// JWT 工具函数
pub fn generate_jwt(user_id: &str, username: &str, role: UserRole, secret: &str, expiration_hours: u64) -> Result<String, EchoError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expiration_hours as i64))
        .expect("valid timestamp")
        .timestamp() as usize;

    let issued_at = Utc::now().timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role,
        exp: expiration,
        iat: issued_at,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref()))?;
    Ok(token)
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, EchoError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

// 密码哈希工具函数
pub fn hash_password(password: &str) -> Result<String, EchoError> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, EchoError> {
    let is_valid = verify(password, hash)?;
    Ok(is_valid)
}

// UUID 生成工具函数
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

// 时间工具函数
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub fn format_timestamp(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

// 验证工具函数
pub fn validate_email(email: &str) -> bool {
    use regex::Regex;
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    email_regex.is_match(email)
}

pub fn validate_device_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 50
}

pub fn validate_username(username: &str) -> bool {
    use regex::Regex;
    let username_regex = Regex::new(r"^[a-zA-Z0-9_-]{3,20}$").unwrap();
    username_regex.is_match(username)
}

// 分页计算工具函数
pub fn calculate_offset(page: u32, page_size: u32) -> u32 {
    (page - 1) * page_size
}

pub fn calculate_total_pages(total: u64, page_size: u32) -> u32 {
    ((total as f64) / (page_size as f64)).ceil() as u32
}

// 字符串工具函数
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// 错误处理工具函数
pub fn map_anyhow_error(err: anyhow::Error) -> EchoError {
    EchoError::Internal(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserRole;

    #[test]
    fn test_jwt_generation_and_verification() {
        let secret = "test-secret";
        let user_id = "user123";
        let username = "testuser";
        let role = UserRole::User;

        let token = generate_jwt(user_id, username, role.clone(), secret, 24).unwrap();
        let claims = verify_jwt(&token, secret).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "testpassword123";

        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }

    #[test]
    fn test_uuid_generation() {
        let uuid1 = generate_uuid();
        let uuid2 = generate_uuid();

        assert_ne!(uuid1, uuid2);
        assert_eq!(uuid1.len(), 36); // 标准UUID长度
    }

    #[test]
    fn test_email_validation() {
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name+tag@domain.co.uk"));
        assert!(!validate_email("invalid-email"));
        assert!(!validate_email("@domain.com"));
        assert!(!validate_email("user@"));
    }

    #[test]
    fn test_username_validation() {
        assert!(validate_username("testuser"));
        assert!(validate_username("test_user123"));
        assert!(validate_username("user-123"));
        assert!(!validate_username("us")); // 太短
        assert!(!validate_username("user@123")); // 包含非法字符
        assert!(!validate_username("very_long_username_that_exceeds_limit")); // 太长
    }

    #[test]
    fn test_pagination_calculations() {
        assert_eq!(calculate_offset(1, 20), 0);
        assert_eq!(calculate_offset(2, 20), 20);
        assert_eq!(calculate_offset(3, 10), 20);

        assert_eq!(calculate_total_pages(100, 20), 5);
        assert_eq!(calculate_total_pages(95, 20), 5);
        assert_eq!(calculate_total_pages(0, 20), 0);
    }

    #[test]
    fn test_string_truncation() {
        let long_string = "This is a very long string that needs to be truncated";
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string(long_string, 20), "This is a very lo...");
    }
}