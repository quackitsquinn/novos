use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse::Parse, parse_macro_input, Expr, Ident, Lit, LitStr, Token};

// Matches a key-value pair in the form `key = "value"`
const ATTR_REGEX: &str = r#"(?P<key>\S*) ?= ?\"(?P<value>.*?)\""#;

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

struct KeyValueList(Vec<KeyValue>);

impl Parse for KeyValueList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let pairs = syn::punctuated::Punctuated::<KeyValue, Token![,]>::parse_terminated(input)?;
        Ok(Self(pairs.into_iter().collect()))
    }
}

struct TestAttrib {
    human_name: String,
    rest: KeyValueList,
}

impl Parse for TestAttrib {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let human_name: Expr = input.parse()?;
        let rest = input.parse()?;
        match human_name {
            Expr::Lit(lit) => {
                if let syn::Lit::Str(s) = lit.lit {
                    Ok(Self {
                        human_name: s.value(),
                        rest,
                    })
                } else {
                    Err(syn::Error::new_spanned(
                        lit,
                        "Expected a string literal for the human name",
                    ))
                }
            }
            _ => Err(syn::Error::new_spanned(
                human_name,
                "Expected a string literal for the human name",
            )),
        }
    }
}

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as syn::ItemFn);
    // attr is in Ident = Lit, Ident = Lit format
    let attr = parse_macro_input!(attr as TestAttrib);

    let fn_name = input.sig.ident.clone();

    let human_name = Lit::Str(LitStr::new(attr.human_name.as_str(), Span::call_site()));

    let marker_name = Ident::new(&format!("__test_{}", fn_name), Span::call_site());

    let mut attributes = vec![];

    for KeyValue { key, value } in attr.rest.0 {
        attributes.push(quote::quote! {
            #key: #value,
        });
    }

    // Build the output, possibly using quasi-quotation
    let output = quote::quote! {
        #[test_case]
        #[allow(non_upper_case_globals)]
        static #marker_name: crate::testing::TestFunction = crate::testing::TestFunction {
            function: #fn_name,
            function_name: stringify!(#fn_name),
            human_name: #human_name,
            // Insert the attributes here
            #(#attributes)*
            ..crate::testing::TestFunction::const_default()
        };
        #[allow(unused)]
        #input
    };

    // Convert the output tokens back into a TokenStream
    output.into()
}
