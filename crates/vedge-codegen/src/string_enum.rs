use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

pub fn derive_string_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let Data::Enum(data_enum) = input.data else {
        return syn::Error::new_spanned(&name, "#[derive(StringEnum)] can only be used on enums")
            .to_compile_error()
            .into();
    };

    let variants: Vec<_> = data_enum.variants.iter().map(|v| &v.ident).collect();

    let variant_strs: Vec<String> = data_enum
        .variants
        .iter()
        .map(|v| v.ident.to_string())
        .collect();

    let expanded = quote! {
        impl ::std::string::ToString for #name {
            fn to_string(&self) -> String {
                match self {
                    #(Self::#variants => #variant_strs.to_string(),)*
                }
            }
        }

        impl ::std::convert::TryFrom<&str> for #name {
            type Error = String;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                match s {
                    #( #variant_strs => Ok(Self::#variants), )*
                    _ => Err(format!("Invalid {}: {}", stringify!(#name), s)),
                }
            }
        }
    };

    expanded.into()
}
