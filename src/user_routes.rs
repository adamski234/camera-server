use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use diesel::{insert_into, result::DatabaseErrorKind, ExpressionMethods, QueryDsl, RunQueryDsl};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;
use rocket::{http::Status, post, response::status, routes, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthToken, model::User, MainDatabase};
use crate::schema::users::dsl::*;

pub fn routes() -> Vec<Route> {
	return routes![
		register_user,
		login
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
		Err(error) => {
			eprintln!("{:?}", error);
			return Err(status::Custom(Status::InternalServerError, Json::from(Error {
				code: String::from("InternalError"), explanation: String::from("Unknown error. Contact administrator.")
			})));
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct LoginResult {
	token: String
}

#[post("/login", data = "<login_data>")]
async fn login(database: MainDatabase, login_data: Json<LoginUserData>) -> UserResult<LoginResult> {
	let login_data = login_data.0;
	let query = users.filter(username.eq(login_data.username));
	let user = database.run(move |conn| query.get_result::<User>(conn)).await;
	match user {
		Ok(user) => {
			match PasswordHash::new(&user.password) {
				Ok(hash) => {
					match Argon2::default().verify_password(login_data.password.as_bytes(), &hash) {
						Ok(_) => {
							let token_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is not set");
							let expiry_time = Utc::now() + Duration::hours(4);
							let header = Header::new(jsonwebtoken::Algorithm::HS512);
							let token = encode(&header, &AuthToken::new(user.username, expiry_time), &EncodingKey::from_secret(token_secret.as_bytes())).unwrap();
							return Ok(Json::from(LoginResult { token }));
						}
						Err(argon2::password_hash::Error::Password) => {
							return Err(status::Custom(Status::Unauthorized, Json::from(Error {
								code: String::from("InvalidPassword"), explanation: String::from("Invalid password for the user.")
							})));
						}
						Err(error) => {
							eprintln!("{:?}", error);
							return Err(status::Custom(Status::InternalServerError, Json::from(Error {
								code: String::from("InternalError"), explanation: String::from("Unknown password verification error. Contact administrator.")
							})));
						}
					}
				}
				Err(error) => {
					eprintln!("{:?}", error);
					return Err(status::Custom(Status::InternalServerError, Json::from(Error {
						code: String::from("InternalError"), explanation: String::from("Unknown password hashing error. Contact administrator.")
					})));
				}
			}
		}
		Err(diesel::result::Error::NotFound) => {
			return Err(status::Custom(Status::BadRequest, Json::from(Error {
				code: String::from("UserNotFound"), explanation: String::from("User with specified username does not exist"),
			})));
		}
		Err(error) => {
			eprintln!("{:?}", error);
			return Err(status::Custom(Status::InternalServerError, Json::from(Error {
				code: String::from("InternalError"), explanation: String::from("Unknown error. Contact administrator.")
			})));
		}
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginUserData {
	pub username: String,
	pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterUserData {
	pub username: String,
	pub password: String,
	pub email: String
}

#[cfg(test)]
mod tests {
	use jsonwebtoken::{decode, DecodingKey, Validation};

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

	#[rocket::async_test]
	async fn login_test() {
		let client = tests_common::create_local_async_client().await;
		let user = tests_common::setup_user(&client).await;
		let request = client.post("/user/login").json(&LoginUserData { 
			password: user.password.clone(),
			username: user.username.clone()
		});
		let response = request.dispatch().await;
		assert_eq!(response.status(), Status::Ok);
		
		let token = response.into_json::<LoginResult>().await.unwrap().token;
		let token_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is not set");
		let decoded_token = decode::<AuthToken>(&token, &DecodingKey::from_secret(token_secret.as_bytes()), &Validation::new(jsonwebtoken::Algorithm::HS512)).unwrap();
		println!("{:#?}", decoded_token.header);
		let token_data = decoded_token.claims;
		assert_eq!(token_data.username, user.username);

		let request = client.post("/user/login").json(&LoginUserData { password: user.password.clone() + "_wrong", username: user.username.clone() });
		let response = request.dispatch().await;
		assert_eq!(response.status(), Status::Unauthorized);
		let return_code = response.into_json::<Error>().await.unwrap();
		assert_eq!(return_code.code, "InvalidPassword");

		let request = client.post("/user/login").json(&LoginUserData { password: user.password.clone(), username: user.username.clone() + "_no_exist" });
		let response = request.dispatch().await;
		assert_eq!(response.status(), Status::BadRequest);
		let return_code = response.into_json::<Error>().await.unwrap();
		assert_eq!(return_code.code, "UserNotFound");
	}
}