use proc_macro::TokenStream;
use syn::parse_macro_input;

mod test;

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as syn::ItemFn);

    test::derive_test(input, attr)
}
