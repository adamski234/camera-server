use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use diesel::{RunQueryDsl, insert_into};
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
	let query = insert_into(users).values(User {
		user_id: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15],
		username: data.username,
		password: hashed_password,
		email: data.email,
	});
	let inserted = database.run(move |c| { query.execute(c) }).await;
	match inserted {
		Ok(_) => {
			return Ok(Json::from(RegistrationResult {}));
		}
		Err(_) => {
			return Err(status::Custom(Status::BadRequest, Json::from(Error { code: String::from("AlreadyExists"), explanation: String::from("One of the unique parameters already exists") })));
		}
	}
}

#[derive(Deserialize, Serialize, Debug)]
struct RegisterUserData {
	username: String,
	password: String,
	email: String
}