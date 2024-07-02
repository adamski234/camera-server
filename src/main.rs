use rocket::launch;

use rocket_sync_db_pools::database;

mod schema;
mod model;
mod user_routes;
mod auth;

#[cfg(test)]
mod tests_common;

#[database("main")]
pub struct MainDatabase(diesel::SqliteConnection);

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().unwrap();

    rocket::build()
        .mount("/user", user_routes::routes())
        .attach(MainDatabase::fairing())
        /*.mount("/swagger-ui", make_swagger_ui(&SwaggerUIConfig {
            url: "../openapi.json".to_owned(),
            ..Default::default()
        }))*/
}