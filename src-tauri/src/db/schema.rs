// @generated automatically by Diesel CLI.

diesel::table! {
    items (id) {
        id -> Integer,
        name -> Text,
        url -> Text,
        username -> Text,
        password -> Text,
    }
}
