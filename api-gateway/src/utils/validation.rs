// Validation utilities placeholder
use serde::{Deserialize, Serialize};

// Placeholder validation functions
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.contains('@') {
        Ok(())
    } else {
        Err("Invalid email format".to_string())
    }
}

pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() >= 8 {
        Ok(())
    } else {
        Err("Password must be at least 8 characters".to_string())
    }
}