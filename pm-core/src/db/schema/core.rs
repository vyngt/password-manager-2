// @generated automatically by Diesel CLI.

diesel::table! {
    it_work (id) {
        id -> Integer,
    }
}

diesel::table! {
    item (id) {
        id -> BigInt,
        name -> Text,
        url -> Text,
        username -> Text,
        password -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(it_work, item,);
