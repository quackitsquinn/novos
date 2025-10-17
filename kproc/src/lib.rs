//! Custom procedural macros for testing and logging.
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod log_filter;
mod test;

/// Attribute macro to mark a function as a test.
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as syn::ItemFn);

    test::derive_test(input, attr)
}

/// Returns true if the given target metadata has been filtered out.
///
/// Implementation wise this generates a match statement that looks for targets declared in the LOG_FILTERS
/// environment variable, and returns false if a match is found.
#[proc_macro]
pub fn log_filter(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as syn::Expr);

    log_filter::derive_log_filter(input)
}
