extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Discriminant)]
pub fn discriminant_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let attrs = ast.attrs;

    if let Data::Enum(data_enum) = ast.data {
        let variant_impls = data_enum.variants.into_iter().map(|variant| {
            let variant_name = &variant.ident;
            let fields = &variant.fields;

            match fields {
                Fields::Unit => {
                    quote! {
                        impl From<#variant_name> for #name {
                            fn from(value: #variant_name) -> Self {
                                Self::#variant_name
                            }
                        }

                        impl std::convert::TryFrom<#name> for #variant_name {
                            type Error = ();

                            fn try_from(value: #name) -> Result<Self, Self::Error> {
                                if let #name::#variant_name = value {
                                    Ok(#variant_name)
                                } else {
                                    Err(())
                                }
                            }
                        }

                        #(#attrs)*
                        struct #variant_name;
                    }
                }
                _ => {
                    let field_name = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();
                    let field_type = fields.iter().map(|field| &field.ty).collect::<Vec<_>>();

                    quote! {
                        impl From<#variant_name> for #name {
                            fn from(value: #variant_name) -> Self {
                                Self::#variant_name {
                                    #(#field_name: value.#field_name),*
                                }
                            }
                        }

                        impl std::convert::TryFrom<#name> for #variant_name {
                            type Error = ();

                            fn try_from(value: #name) -> Result<Self, Self::Error> {
                                if let #name::#variant_name { #(#field_name),* } = value {
                                    Ok(#variant_name {
                                        #(#field_name),*
                                    })
                                } else {
                                    Err(())
                                }
                            }
                        }

                        #(#attrs)*
                        struct #variant_name {
                            #(#field_name: #field_type),*
                        }
                    }
                }
            }
        });

        let output = quote! {
            #(#variant_impls)*
        };

        TokenStream::from(output)
    } else {
        panic!("Discriminant can only be derived for enums.")
    }
}
