pub(crate) mod subset;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Derive macro
#[proc_macro_derive(Subset, attributes(subset))]
pub fn subset_derive(input: TokenStream) -> TokenStream {
    // Parse TokenStream into DeriveInput
    let derive_input = parse_macro_input!(input as syn::DeriveInput);

    // Call the internal function that returns proc_macro2::TokenStream
    let expanded = subset::impl_subset(derive_input);

    // Convert proc_macro2::TokenStream -> proc_macro::TokenStream for Rust
    expanded.into()
}
