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
    }
}

diesel::table! {
    item_tags (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    items (id) {
        id -> Integer,
        name -> Text,
        url -> Text,
        username -> Text,
        password -> Text,
    }
}

diesel::table! {
    user (id) {
        id -> Integer,
        password -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    color_scheme,
    item_tags,
    items,
    user,
);
