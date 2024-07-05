#![feature(assert_matches)]
#![feature(generic_arg_infer)]
#![feature(f128)]
#![feature(f16)]

use device_connector::DeviceBridge;
use rocket::launch;
use rocket_sync_db_pools::database;

mod schema;
mod model;
mod user_routes;
mod auth;
mod device_connector;

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
		.attach(DeviceBridge::fairing(3333))
        /*.mount("/swagger-ui", make_swagger_ui(&SwaggerUIConfig {
            url: "../openapi.json".to_owned(),
            ..Default::default()
        }))*/
}