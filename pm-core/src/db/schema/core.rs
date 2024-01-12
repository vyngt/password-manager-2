// @generated automatically by Diesel CLI.

diesel::table! {
    item (id) {
        id -> BigInt,
        name -> Text,
        url -> Text,
        username -> Text,
        password -> Text,
    }
}

diesel::table! {
    program (id) {
        id -> Integer,
        version -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(item, program,);
