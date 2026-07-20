mod string_enum;

use proc_macro::TokenStream;

#[proc_macro_derive(StringEnum)]
pub fn derive_string_enum_macro(input: TokenStream) -> TokenStream {
    string_enum::derive_string_enum(input)
}
