use rocket::launch;

use rocket_sync_db_pools::database;

mod schema;
mod model;
mod user_routes;

/*fn main() {
    dotenvy::dotenv().unwrap();
    let mut database = diesel::SqliteConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();
    // database.set_instrumentation(|event: diesel::connection::InstrumentationEvent| {
    //     println!("{:#?}", event);
    // });
    let user_list = users.filter(
        email
            .eq("email@example.com")
            .and(username.eq("user"))
    ).load::<User>(&mut database).unwrap();
    println!("{:#?}", user_list);
}*/

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