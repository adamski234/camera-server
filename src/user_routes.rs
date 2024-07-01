use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use diesel::{insert_into, result::DatabaseErrorKind, ExpressionMethods, QueryDsl, RunQueryDsl};
use rand::Rng;
use rocket::{http::Status, post, response::status, routes, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

use crate::{model::User, MainDatabase};
use crate::schema::users::dsl::*;

pub fn routes() -> Vec<Route> {
	return routes![
		register_user
	];
}

#[derive(Deserialize, Serialize, Debug)]
struct Error {
	code: String,
	explanation: String,
}

type UserResult<T> = Result<Json<T>, status::Custom<Json<Error>>>;

#[derive(Serialize, Debug, Default)]
struct RegistrationResult {}

#[post("/register", data = "<register_user_data>")]
async fn register_user(database: MainDatabase, register_user_data: Json<RegisterUserData>) -> UserResult<RegistrationResult> {
	let data = register_user_data.0;
	let salt = SaltString::generate(&mut OsRng);
	let hasher = Argon2::default();
	let hashed_password = hasher.hash_password(data.password.as_bytes(), &salt).unwrap().to_string();
	let mut inserted_user_id = [0u8; 16];
	rand::thread_rng().fill(&mut inserted_user_id);
	loop {
		let query = users.filter(user_id.eq(inserted_user_id)).count();
		let count = database.run(move |conn| query.get_result::<i64>(conn)).await;
		match count {
			Ok(0) => {
				break;
			}
			Ok(_) => {
				rand::thread_rng().fill(&mut inserted_user_id);
			}
			Err(_) => {
				break;
			}
		}
	}
	let query = insert_into(users).values(User {
		user_id: Vec::from(inserted_user_id),
		username: data.username,
		password: hashed_password,
		email: data.email,
	});
	let inserted = database.run(move |c| { query.execute(c) }).await;
	match inserted {
		Ok(_) => {
			return Ok(Json::from(RegistrationResult {}));
		}
		Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
			return Err(status::Custom(Status::BadRequest, Json::from(Error { 
				code: String::from("AlreadyExists"), explanation: String::from("One of the unique parameters already exists")
			})));
		}
		Err(_) => {
			return Err(status::Custom(Status::InternalServerError, Json::from(Error {
				code: String::from("InternalError"), explanation: String::from("Unknown error. Contact administrator.")
			})));
		}
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterUserData {
	pub username: String,
	pub password: String,
	pub email: String
}

#[cfg(test)]
mod tests {
	use crate::tests_common;
	use super::*;

	#[test]
	fn register_test() {
		let client = tests_common::create_local_client();
		let request = client.post("/user/register").json(&RegisterUserData {
			email: String::from("email@example.com"),
			password: String::from("password1"),
			username: String::from("new_username"),
		});
		let repeat_request = request.clone();

		let success_response = request.dispatch();
		let unique_violation_response = repeat_request.dispatch();

		assert_eq!(success_response.status(), Status::Ok);
		assert_eq!(unique_violation_response.status(), Status::BadRequest);
		assert_eq!(unique_violation_response.into_json::<Error>().unwrap().code, "AlreadyExists");
	}
}