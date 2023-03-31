extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DeriveInput};

#[proc_macro_derive(Build, attributes(required))]
pub fn build_macro_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_build_macro(&ast)
}

fn impl_build_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (required_fields, optional_fields) = partition_fields(&ast.data);

    let required_params = required_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        quote! { #field_name: impl Into<#field_type> }
    });

    let required_assignments = required_fields.iter().map(|field| {
        let field_name = &field.ident;
        quote! { #field_name: #field_name.into() }
    });

    let optional_methods = optional_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        quote! {
            pub fn #field_name(mut self, #field_name: impl Into<#field_type>) -> Self {
                self.#field_name = #field_name.into();
                self
            }
        }
    });

    let expanded = quote! {
        impl #name {
            pub fn new(#(#required_params),*) -> Self {
                Self {
                    #(#required_assignments,)*
                    ..Default::default()
                }
            }

            #(#optional_methods)*
        }
    };

    TokenStream::from(expanded)
}

fn partition_fields(data: &syn::Data) -> (Vec<syn::Field>, Vec<syn::Field>) {
    let fields = match data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("Only structs are supported for the Build macro."),
    };

    fields
        .iter()
        .cloned()
        .partition(|field| field.attrs.iter().any(|attr| attr.path.is_ident("required")))
}
