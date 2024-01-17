// @generated automatically by Diesel CLI.

diesel::table! {
    color_scheme (id) {
        id -> BigInt,
        name -> Text,
        primary -> Text,
        secondary -> Text,
        success -> Text,
        danger -> Text,
        warning -> Text,
        foreground -> Text,
        background -> Text,
    }
}

diesel::table! {
    theme (id) {
        id -> BigInt,
        color_scheme_id -> BigInt,
    }
}

diesel::joinable!(theme -> color_scheme (color_scheme_id));

diesel::allow_tables_to_appear_in_same_query!(color_scheme, theme,);
