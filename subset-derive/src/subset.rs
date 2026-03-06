use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Data, DeriveInput, Field, Ident, LitStr, Result, Token,
};

struct SubsetAttr {
    from: LitStr,
}

impl Parse for SubsetAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "from" {
            return Err(input.error("expected `from`"));
        }
        input.parse::<Token![=]>()?;
        let from: LitStr = input.parse()?;
        Ok(SubsetAttr { from })
    }
}

pub fn impl_subset(input: DeriveInput) -> TokenStream2 {
    let struct_name = input.ident;

    let source_type_ident: Ident = match input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("subset"))
        .and_then(|attr| {
            attr.parse_args::<SubsetAttr>()
                .ok()
                .map(|a| Ident::new(&a.from.value(), a.from.span()))
        }) {
        Some(id) => id,
        None => {
            return syn::Error::new_spanned(
                &struct_name,
                "Expected #[subset(from = \"SourceType\")]",
            )
            .to_compile_error();
        }
    };

    let fields_iter = match input.data {
        Data::Struct(ref data_struct) => data_struct.fields.iter().map(|f| {
            let target_ident = f
                .ident
                .as_ref()
                .expect("named fields only (tuple/unnamed not supported)");
            let rhs = field_rhs_tokens(f, target_ident);
            quote! { #target_ident: #rhs }
        }),
        _ => {
            return syn::Error::new_spanned(
                &struct_name,
                "Subset can only be derived on structs",
            )
            .to_compile_error();
        }
    };

    quote! {
        impl From<#source_type_ident> for #struct_name {
            fn from(source: #source_type_ident) -> Self {
                Self { #(#fields_iter),* }
            }
        }

        impl subset::Subset<#source_type_ident> for #struct_name {}
    }
}

/// Build the RHS tokens for a field assignment:
/// - default: `source.<target_field>`
/// - alias:   `source.<alias_ident>`
/// - path:    `source.<seg0>.<seg1>...`
fn field_rhs_tokens(field: &Field, target_ident: &Ident) -> TokenStream2 {
    // Look for #[subset(...)] on the field
    let mut alias_lit: Option<LitStr> = None;
    let mut path_lit: Option<LitStr> = None;

    for attr in &field.attrs {
        if attr.path().is_ident("subset") {
            let res: Result<()> = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("alias") {
                    let lit: LitStr = meta.value()?.parse()?;
                    alias_lit = Some(lit);
                    Ok(())
                } else if meta.path.is_ident("path") {
                    let lit: LitStr = meta.value()?.parse()?;
                    path_lit = Some(lit);
                    Ok(())
                } else {
                    Err(meta.error("unsupported subset attribute; expected `alias` or `path`"))
                }
            });
            // If parse_nested_meta fails, return a compile error at the attribute
            if let Err(e) = res {
                return e.to_compile_error();
            }
        }
    }

    if let Some(lit) = alias_lit {
        let alias_ident = Ident::new(&lit.value(), lit.span());
        quote!( source.#alias_ident )
    } else if let Some(lit) = path_lit {
        // Split "a.b.c" into identifiers and build chained access: source.a.b.c
        let segs: Vec<Ident> = lit
            .value()
            .split('.')
            .map(|s| Ident::new(s, lit.span()))
            .collect();

        let mut ts: TokenStream2 = quote!(source);
        for seg in segs {
            ts = quote!( #ts.#seg );
        }
        ts
    } else {
        // Default: same name in source and target
        quote!( source.#target_ident )
    }
}
