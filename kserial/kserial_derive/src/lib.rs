use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::{DataStruct, DeriveInput, Field, Fields, Ident, Index};

#[proc_macro_derive(Validate)]
pub fn derive_validate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let mut output = proc_macro2::TokenStream::new();

    impl_validate(input, &mut output);

    TokenStream::from(output)
}

fn impl_validate(input: DeriveInput, output: &mut proc_macro2::TokenStream) {
    let name = input.ident;
    let data = input.data;

    let ds = match data {
        syn::Data::Struct(data) => data,
        _ => panic!("Validate can only be derived for structs"),
    };

    generate_validate_check(&ds.fields, output);
    generate_validate_implementation(&name, &ds, output);
}

fn generate_validate_check(fields: &Fields, output: &mut proc_macro2::TokenStream) {
    let mut checks = proc_macro2::TokenStream::new();

    for field in fields {
        let ty = field.ty.clone();
        checks.extend(quote! {
            assert_validate::<#ty>();
        });
    }

    output.extend(quote! {
            const _: fn() = || {
                fn assert_validate<T: crate::common::Validate>(){}
                #checks
            };
    });
}

fn generate_validate_implementation(name: &Ident, fields: &DataStruct, output: &mut TokenStream2) {
    let checks = field_struct_gen(
        |ident, _| {
            quote! {
                self.#ident.validate()
            }
        },
        fields,
    );

    output.extend(quote! {
        impl crate::common::Validate for #name {
            fn validate(&self) -> bool {
                #(#checks)&&*
            }
        }
    });
}

/// Generates a vector of TokenStream2 from the fields of a struct.
/// The `transform` function is applied to each field's identifier and the field itself.
/// This is useful for generating code based on the fields of a struct. The field identifier automatically works with both named and unnamed fields.
fn field_struct_gen(
    transform: fn(&TokenStream2, &Field) -> TokenStream2,
    input: &syn::DataStruct,
) -> Vec<TokenStream2> {
    match &input.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| {
                let ident = field.ident.as_ref().unwrap();
                transform(&ident.to_token_stream(), field)
            })
            .collect(),
        syn::Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let ident = Index::from(i);
                transform(&ident.to_token_stream(), field)
            })
            .collect(),
        syn::Fields::Unit => Vec::new(),
    }
}
