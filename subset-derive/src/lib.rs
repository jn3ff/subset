#[cfg(feature = "functions")]
pub(crate) mod registry;
#[cfg(feature = "functions")]
pub(crate) mod rewrite;
pub(crate) mod subset;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(Subset, attributes(subset))]
pub fn subset_derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as syn::DeriveInput);
    let expanded = subset::impl_subset(derive_input);
    expanded.into()
}
