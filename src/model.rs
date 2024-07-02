use crate::schema::*;
use diesel::prelude::*;

#[derive(Debug, PartialEq, Queryable, Identifiable, Selectable, Insertable)]
#[diesel(table_name = users)]
#[diesel(primary_key(user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
	pub user_id: Vec<u8>,
	pub username: String,
	pub password: String,
	pub email: String,
}

#[derive(Debug, PartialEq, Queryable, Identifiable, Selectable, Associations)]
#[diesel(table_name = device)]
#[diesel(primary_key(device_id))]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Device {
	pub device_id: Vec<u8>,
	pub mac_address: Vec<u8>,
	pub auth_key: Vec<u8>,
	pub registration_first_stage: bool,
	pub user_id: Vec<u8>,
}