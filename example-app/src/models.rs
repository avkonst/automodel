use serde::{Deserialize, Serialize};

/// Example custom type for test_data field  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestData {
    pub message: String,
    pub value: i32,
    pub metadata: Option<serde_json::Value>,
}

/// User profile information stored as JSON in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub preferences: UserPreferences,
    pub social_links: Vec<SocialLink>,
}

/// User preferences within the profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub language: String,
    pub notifications_enabled: bool,
}

/// Social media links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLink {
    pub platform: String,
    pub url: String,
}

/// User settings stored as JSON in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub privacy_level: String,
    pub email_notifications: bool,
    pub two_factor_enabled: bool,
    pub api_access: bool,
}
