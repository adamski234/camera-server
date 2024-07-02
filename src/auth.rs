use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
	pub username: String,
	/// Expiration time
	pub exp: i64
}

impl AuthToken {
	pub fn new(username: String, expiry: DateTime<Utc>) -> Self {
		return Self { username, exp: expiry.timestamp() };
	}
}