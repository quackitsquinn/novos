use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Expr, Lit, LitStr};

pub fn derive_log_filter(metadata: Expr) -> TokenStream {
    let log_filters = log_filters();
    quote! {
        match #metadata {
            #(#log_filters => false,)*
            _ => true,}
    }
    .into()
}

fn log_filters() -> Vec<LitStr> {
    let filters = std::env::var("LOG_FILTERS").unwrap_or_default();

    filters
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| LitStr::new(s, Span::call_site()))
        .collect()
}
