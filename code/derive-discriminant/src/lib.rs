extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Discriminant)]
pub fn discriminant_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_discriminant_macro(ast)
}

fn impl_discriminant_macro(ast: DeriveInput) -> TokenStream {
    let name = &ast.ident;

    // all non-doc attributes
    let global_attrs: Vec<_> = ast
        .attrs
        .into_iter()
        .filter(|attr| !attr.path().is_ident("doc"))
        .collect();

    let Data::Enum(data_enum) = ast.data else {
        panic!("Discriminant can only be derived for enums");
    };

    let variant_names: Vec<_> = data_enum
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .collect();

    // implementation for the .cast() method to cast into a trait object
    // this requires nightly
    let cast_method = quote! {
        impl #name {
            fn cast<U: ?Sized>(self) -> Box<U> where #(#variant_names: ::core::marker::Unsize<U>),* {
                let value = self;
                // TODO: use a singular match expression
                #(
                    let value = match #variant_names::try_from(value) {
                        Ok(v) => {
                            let x = Box::new(v);
                            return x;
                        }
                        Err(v) => v,
                    };
                )*

                unreachable!();
            }
        }
    };

    let variant_impls = data_enum.variants.into_iter().map(|variant| {
        let variant_name = &variant.ident;
        let fields = &variant.fields;
        let variant_attrs = variant.attrs;

        let is_variant_name: syn::Ident = {
            let lowercase = variant_name.to_string().to_lowercase();
            let name = format!("is_{}", lowercase);
            syn::parse_str(&name).expect("failed to parse variant name")
        };

        match fields {
            Fields::Unit => {
                quote! {
                    impl From<#variant_name> for #name {
                        fn from(value: #variant_name) -> Self {
                            Self::#variant_name
                        }
                    }

                    impl std::convert::TryFrom<#name> for #variant_name {
                        type Error = #name;

                        fn try_from(value: #name) -> Result<Self, Self::Error> {
                            if let #name::#variant_name = value {
                                Ok(#variant_name)
                            } else {
                                Err(value)
                            }
                        }
                    }

                    impl #name {
                        pub fn #is_variant_name(&self) -> bool {
                            matches!(self, Self::#variant_name)
                        }
                    }

                    #(#global_attrs)*
                    #(#variant_attrs)*
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
                        type Error = #name;

                        fn try_from(value: #name) -> Result<Self, Self::Error> {
                            if let #name::#variant_name { #(#field_name),* } = value {
                                Ok(#variant_name {
                                    #(#field_name),*
                                })
                            } else {
                                Err(value)
                            }
                        }
                    }

                    impl #name {
                        pub fn #is_variant_name(&self) -> bool {
                            matches!(self, Self::#variant_name { .. })
                        }
                    }

                    #(#global_attrs)*
                    #(#variant_attrs)*
                    struct #variant_name {
                        #(#field_name: #field_type),*
                    }
                }
            }
        }
    });

    let output = quote! {
        #(#variant_impls)*
        #cast_method
    };

    TokenStream::from(output)
}
