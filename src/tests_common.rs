use std::env;

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use rocket::{fairing::AdHoc, http::Status, local::{asynchronous, blocking::Client}};

use crate::{model::User, rocket, schema::users::dsl::*, user_routes::RegisterUserData, MainDatabase};


pub const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!("./migrations");

fn migration_fairing() -> AdHoc {
	return AdHoc::on_ignite("Migrate memory database", |rocket| async {
		let database = MainDatabase::get_one(&rocket).await.unwrap();
		database.run(|conn| {
			conn.run_pending_migrations(MIGRATIONS).unwrap();
		}).await;
		return rocket;
	});
}

pub fn create_local_client() -> Client {
	env::set_var("ROCKET_DATABASES", "{main={url=\":memory:\"}}");
	let rocket = rocket().attach(migration_fairing());
	let client = Client::tracked(rocket).unwrap();
	return client;
}

pub async fn create_local_async_client() -> asynchronous::Client {
	env::set_var("ROCKET_DATABASES", "{main={url=\":memory:\"}}");
	let rocket = rocket().attach(migration_fairing());
	let client = asynchronous::Client::tracked(rocket).await.unwrap();
	return client;
}

pub async fn setup_user(client: &asynchronous::Client) -> User {
	let response = client.post("/user/register").json(&RegisterUserData {
		email: String::from("email@example.com"),
		password: String::from("password1"),
		username: String::from("new_username"),
	}).dispatch().await;
	assert_eq!(response.status(), Status::Ok);
	let database = MainDatabase::get_one(client.rocket()).await.unwrap();
	let created_user = database.run(|conn| users.filter(username.eq("new_username")).first::<User>(conn)).await.unwrap();
	return created_user;
}

#[rocket::async_test]
async fn verify_user_creation() {
	let client = create_local_async_client().await;
	let created_user = setup_user(&client).await;
	assert_eq!(created_user.username, "new_username");
	assert_eq!(created_user.email, "email@example.com");
}