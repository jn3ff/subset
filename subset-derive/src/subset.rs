use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Field, Ident, LitStr, Result,
};
#[cfg(feature = "functions")]
use syn::Token;

#[cfg(feature = "functions")]
use std::collections::HashMap;
#[cfg(feature = "functions")]
use syn::visit_mut::VisitMut;
#[cfg(feature = "functions")]
use crate::{registry, rewrite::FieldRewriter};

/// Parsed struct-level attributes: `from` and `functions`.
struct StructAttrs {
    from: String,
    from_span: proc_macro2::Span,
    #[cfg(feature = "functions")]
    functions: Vec<String>,
}

/// Parse all `#[subset(...)]` attributes on the struct.
fn parse_struct_attrs(attrs: &[syn::Attribute]) -> Result<StructAttrs> {
    let mut from: Option<(String, proc_macro2::Span)> = None;
    #[cfg(feature = "functions")]
    let mut functions: Vec<String> = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("subset") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("from") {
                let lit: LitStr = meta.value()?.parse()?;
                from = Some((lit.value(), lit.span()));
                Ok(())
            } else if meta.path.is_ident("functions") {
                #[cfg(not(feature = "functions"))]
                return Err(meta.error(
                    "the `functions` attribute requires the \"functions\" feature: \
                     subset = { features = [\"functions\"] }",
                ));

                #[cfg(feature = "functions")]
                {
                    let value = meta.value()?;
                    let lookahead = value.lookahead1();
                    if lookahead.peek(syn::token::Bracket) {
                        let content;
                        syn::bracketed!(content in value);
                        while !content.is_empty() {
                            let lit: LitStr = content.parse()?;
                            functions.push(lit.value());
                            if !content.is_empty() {
                                content.parse::<Token![,]>()?;
                            }
                        }
                    } else {
                        let lit: LitStr = value.parse()?;
                        functions.push(lit.value());
                    }
                    Ok(())
                }
            } else {
                // Ignore unknown attrs here — field-level attrs (alias, path, generate)
                // are handled separately in field_rhs_tokens.
                Ok(())
            }
        })?;
    }

    let (from_str, from_span) = from.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "Expected #[subset(from = \"SourceType\")]")
    })?;

    Ok(StructAttrs {
        from: from_str,
        from_span,
        #[cfg(feature = "functions")]
        functions,
    })
}

#[cfg(feature = "functions")]
/// Build the reverse field mapping: source_access_path → subset_field_name.
/// Used for rewriting method bodies.
fn build_reverse_mapping(data: &Data) -> HashMap<String, String> {
    let mut mapping = HashMap::new();

    let Data::Struct(data_struct) = data else {
        return mapping;
    };

    for field in &data_struct.fields {
        let target_name = match field.ident.as_ref() {
            Some(id) => id.to_string(),
            None => continue,
        };

        let mut alias = None;
        let mut path = None;
        let mut is_generate = false;

        for attr in &field.attrs {
            if attr.path().is_ident("subset") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("alias") {
                        let lit: LitStr = meta.value()?.parse()?;
                        alias = Some(lit.value());
                    } else if meta.path.is_ident("path") {
                        let lit: LitStr = meta.value()?.parse()?;
                        path = Some(lit.value());
                    } else if meta.path.is_ident("generate") {
                        is_generate = true;
                        let _: LitStr = meta.value()?.parse()?;
                    }
                    Ok(())
                });
            }
        }

        if is_generate {
            continue; // generated fields have no source mapping
        }

        let source_key = if let Some(alias_name) = alias {
            alias_name
        } else if let Some(path_str) = path {
            path_str
        } else {
            target_name.clone()
        };

        mapping.insert(source_key, target_name);
    }

    mapping
}

#[cfg(feature = "functions")]
fn parse_function_ref(func_ref: &str, default_type: &str) -> (String, String) {
    if let Some(method) = func_ref.strip_prefix("from::") {
        (default_type.to_string(), method.to_string())
    } else if let Some(pos) = func_ref.rfind("::") {
        let type_name = &func_ref[..pos];
        let method_name = &func_ref[pos + 2..];
        (type_name.to_string(), method_name.to_string())
    } else {
        // Bare method name — assume source type
        (default_type.to_string(), func_ref.to_string())
    }
}

#[cfg(feature = "functions")]
fn generate_methods(
    struct_name: &Ident,
    source_type_name: &str,
    function_refs: &[String],
    reverse_mapping: &HashMap<String, String>,
) -> TokenStream2 {
    if function_refs.is_empty() {
        return quote! {};
    }

    let mut methods: Vec<TokenStream2> = Vec::new();

    for func_ref in function_refs {
        let (type_name, method_name) = parse_function_ref(func_ref, source_type_name);

        let method = match registry::get(&struct_name.to_string(), &type_name, &method_name) {
            Some(m) => m,
            None => {
                return syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "subset: method `{}::{}` not found. \
                         Ensure it is defined as an inherent impl in a .rs file \
                         within this crate.",
                        type_name, method_name
                    ),
                )
                .to_compile_error();
            }
        };

        // Clone and rewrite field accesses in the method body
        let mut rewritten = method.clone();
        let mut rewriter = FieldRewriter {
            mappings: reverse_mapping.clone(),
        };
        rewriter.visit_impl_item_fn_mut(&mut rewritten);

        // Register the rewritten method so downstream subsets can find it
        registry::register(&struct_name.to_string(), &method_name, &rewritten);

        methods.push(quote! { #rewritten });
    }

    quote! {
        impl #struct_name {
            #(#methods)*
        }
    }
}

pub fn impl_subset(input: DeriveInput) -> TokenStream2 {
    let struct_name = input.ident;

    let struct_attrs = match parse_struct_attrs(&input.attrs) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

    let source_type_ident = Ident::new(&struct_attrs.from, struct_attrs.from_span);

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
            return syn::Error::new_spanned(&struct_name, "Subset can only be derived on structs")
                .to_compile_error();
        }
    };

    #[cfg(feature = "functions")]
    let method_impls = {
        let reverse_mapping = build_reverse_mapping(&input.data);
        generate_methods(
            &struct_name,
            &struct_attrs.from,
            &struct_attrs.functions,
            &reverse_mapping,
        )
    };
    #[cfg(not(feature = "functions"))]
    let method_impls = quote! {};

    quote! {
        impl From<#source_type_ident> for #struct_name {
            fn from(from: #source_type_ident) -> Self {
                Self { #(#fields_iter),* }
            }
        }

        impl subset::Subset<#source_type_ident> for #struct_name {}

        #method_impls
    }
}

/// Build the RHS tokens for a field assignment:
/// - default: `source.<target_field>`
/// - alias:   `source.<alias_ident>`
/// - path:    `source.<seg0>.<seg1>...`
fn field_rhs_tokens(field: &Field, target_ident: &Ident) -> TokenStream2 {
    let mut alias_lit: Option<LitStr> = None;
    let mut path_lit: Option<LitStr> = None;
    let mut generate_lit: Option<LitStr> = None;

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
                } else if meta.path.is_ident("generate") {
                    let lit: LitStr = meta.value()?.parse()?;
                    generate_lit = Some(lit);
                    Ok(())
                } else {
                    // Skip struct-level attrs that might appear (functions, from)
                    Ok(())
                }
            });
            if let Err(e) = res {
                return e.to_compile_error();
            }
        }
    }

    let set_count =
        alias_lit.is_some() as u8 + path_lit.is_some() as u8 + generate_lit.is_some() as u8;
    if set_count > 1 {
        return syn::Error::new_spanned(
            field,
            "only one of `alias`, `path`, or `generate` may be specified per field",
        )
        .to_compile_error();
    }

    if let Some(lit) = generate_lit {
        let expr: TokenStream2 = lit.value().parse().unwrap_or_else(|_| {
            syn::Error::new(lit.span(), "failed to parse `generate` expression").to_compile_error()
        });
        expr
    } else if let Some(lit) = alias_lit {
        let alias_ident = Ident::new(&lit.value(), lit.span());
        quote!( from.#alias_ident )
    } else if let Some(lit) = path_lit {
        let segs: Vec<Ident> = lit
            .value()
            .split('.')
            .map(|s| Ident::new(s, lit.span()))
            .collect();

        let mut ts: TokenStream2 = quote!(from);
        for seg in segs {
            ts = quote!( #ts.#seg );
        }
        ts
    } else {
        quote!( from.#target_ident )
    }
}
