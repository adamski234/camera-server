// @generated automatically by Diesel CLI.

diesel::table! {
    device (device_id) {
        device_id -> Binary,
        mac_address -> Binary,
        auth_key -> Binary,
        registration_first_stage -> Bool,
        user_id -> Binary,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Binary,
        username -> Text,
        password -> Text,
        email -> Text,
    }
}

diesel::joinable!(device -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    device,
    users,
);
