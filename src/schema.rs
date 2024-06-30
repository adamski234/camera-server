// @generated automatically by Diesel CLI.

diesel::table! {
    authenticationtoken (token) {
        token -> Text,
        user_id -> Binary,
        expired_date -> Timestamp,
    }
}

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

diesel::joinable!(authenticationtoken -> users (user_id));
diesel::joinable!(device -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    authenticationtoken,
    device,
    users,
);
