use std::{fs::File, io::Write};

use proc_macro::TokenStream;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, Expr, Ident, ItemFn, Token};

struct TestAttributes {
    human_name: Expr,
    rest: Option<KeyValues>,
}

impl Parse for TestAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let human_name: Expr = input.parse()?;
        let rest = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            human_name: human_name,
            rest,
        })
    }
}

struct KeyValue {
    key: Ident,
    value: Expr,
}

impl Parse for KeyValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(Self { key, value })
    }
}
struct KeyValues(Vec<KeyValue>);

impl KeyValues {
    fn into_struct_format(&self) -> Vec<proc_macro2::TokenStream> {
        self.0
            .iter()
            .map(|KeyValue { key, value }| {
                quote::quote! {
                    #key: #value
                }
            })
            .collect()
    }
}

impl Parse for KeyValues {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let pairs = Punctuated::<KeyValue, Token![,]>::parse_terminated(input)?;
        Ok(Self(pairs.into_iter().collect()))
    }
}

pub fn derive_test(input: ItemFn, attributes: TokenStream) -> TokenStream {
    let attributes = parse_macro_input!(attributes as TestAttributes);
    let fn_name = &input.sig.ident.clone();
    let marker_name = Ident::new(
        &format!("__kproc_test_{}", fn_name.to_string()),
        input.sig.ident.span(),
    );
    let struct_format = attributes
        .rest
        .as_ref()
        .map_or_else(|| vec![], |rest| rest.into_struct_format());
    let human_name = attributes.human_name;

    let expanded = quote::quote! {
        #[test_case]
        #[doc(hidden)]
        #[allow(non_snake_case)]
        static #marker_name: crate::testing::TestFunction = crate::testing::TestFunction {
            function: #fn_name,
            function_name: stringify!(#fn_name),
            human_name: #human_name,
            #(#struct_format,)*
            ..crate::testing::TestFunction::const_default()
        };
        #input
    };
    // Write the expanded code to a file for debugging. We ignore the error because it's not important.
    let _ = File::create(format!("target/expand/{}.rs", fn_name.to_string())).map(|mut file| {
        file.write_all(expanded.to_string().as_bytes()).unwrap();
    });
    expanded.into()
}
