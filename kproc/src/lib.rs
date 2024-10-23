use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, token::Token, Expr, Ident, Lit, LitStr, Token};

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
    rest: Option<KeyValueList>,
}

fn assert_expr_is_str_lit(expr: &Expr) -> syn::Result<&LitStr> {
    match expr {
        Expr::Lit(lit) => {
            if let Lit::Str(s) = &lit.lit {
                Ok(s)
            } else {
                Err(syn::Error::new_spanned(
                    lit,
                    "Expected a string literal for the human name",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "Expected a string literal for the human name",
        )),
    }
}

impl Parse for TestAttrib {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let human_name: Expr = input.parse()?;
        let human_name_str = assert_expr_is_str_lit(&human_name)?.value();
        if input.parse::<Token![,]>().is_ok() {
            let rest: KeyValueList = input.parse()?;
            Ok(Self {
                human_name: human_name_str,
                rest: Some(rest),
            })
        } else {
            if !input.is_empty() {
                // There's more input so error
                Err(syn::Error::new_spanned(
                    input.cursor().token_stream(),
                    "Expected a comma followed by key-value pairs",
                ))
            } else {
                Ok(Self {
                    human_name: human_name_str,
                    rest: None,
                })
            }
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
    if let Some(rest) = &attr.rest {
        for KeyValue { key, value } in rest.0.iter() {
            attributes.push(quote::quote! {
                #key: #value,
            });
        }
    }

    // Build the output, possibly using quasi-quotation
    quote::quote! {
        #[test_case]
        #[allow(non_upper_case_globals)]
        static #marker_name: crate::testing::TestFunction = crate::testing::TestFunction {
            function: #fn_name,
            function_name: stringify!(#fn_name),
            human_name: #human_name,
            // Insert the attributes here
            #(#attributes),*
            ..crate::testing::TestFunction::const_default()
        };
        #[allow(unused)]
        #input
    }
    .into()
}
