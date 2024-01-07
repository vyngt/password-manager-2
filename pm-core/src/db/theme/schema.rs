// @generated automatically by Diesel CLI.

diesel::table! {
    color_scheme (id) {
        id -> Integer,
        name -> Text,
        primary -> Text,
        secondary -> Text,
        success -> Text,
        danger -> Text,
        warning -> Text,
        foreground -> Text,
        background -> Text,
        selection -> Text,
    }
}

diesel::table! {
    theme (id) {
        id -> Integer,
        color_scheme_id -> Nullable<Integer>,
    }
}

diesel::joinable!(theme -> color_scheme (color_scheme_id));

diesel::allow_tables_to_appear_in_same_query!(
    color_scheme,
    theme,
);
