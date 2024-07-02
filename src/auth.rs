use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
	username: String,
	expiry: DateTime<Utc>
}

impl AuthToken {
	pub fn new(username: String, expiry: DateTime<Utc>) -> Self {
		return Self { username, expiry };
	}
}