pub mod business {
    pub mod usecases {
        pub mod theme {
            pub mod create_color_scheme;
            pub mod delete_color_scheme;
            pub mod get_color_scheme;
            pub mod get_current_theme;
            pub mod list_color_schemes;
            pub mod test_utils;
            pub mod update_color_scheme;
            pub mod update_theme;
        }
        pub mod utilities {
            pub mod generate_password;
            pub mod test_utils;
        }
        pub mod vault {
            pub mod change_key_vault;
            pub mod create_vault_item;
            pub mod delete_vault_item;
            pub mod get_vault_item;
            pub mod list_vault_items;
            pub mod test_utils;
            pub mod unlock_vault;
            pub mod update_vault_item;
        }
    }
}
