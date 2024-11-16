// @generated automatically by Diesel CLI.

diesel::table! {
    courses (code) {
        code -> Text,
        name -> Text,
        unit_id -> Integer,
    }
}

diesel::table! {
    units (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::joinable!(courses -> units (unit_id));

diesel::allow_tables_to_appear_in_same_query!(
    courses,
    units,
);
