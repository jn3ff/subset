use std::collections::HashMap;

use proc_macro2::Span;
use syn::{
    Expr, Ident, Member,
    visit_mut::{self, VisitMut},
};

/// Reverse field mapping: source field path → target (subset) field name.
/// E.g., "metadata.followers" → "followers" (path mapping)
/// E.g., "followers" → "aliased_followers" (alias mapping)
pub type FieldMapping = HashMap<String, String>;

pub struct FieldRewriter {
    pub mappings: FieldMapping,
}

/// Extract the field access chain from an expression rooted at `self`.
/// Returns `Some(["a", "b"])` for `self.a.b`, `Some([])` for bare `self`, `None` otherwise.
fn extract_self_field_path(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Field(field_expr) => {
            if let Member::Named(ident) = &field_expr.member {
                if let Some(mut base_path) = extract_self_field_path(&field_expr.base) {
                    base_path.push(ident.to_string());
                    return Some(base_path);
                }
            }
            None
        }
        Expr::Path(expr_path) if expr_path.path.is_ident("self") => Some(vec![]),
        _ => None,
    }
}

impl VisitMut for FieldRewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Some(path_parts) = extract_self_field_path(expr) {
            if !path_parts.is_empty() {
                let path_str = path_parts.join(".");
                if let Some(new_field) = self.mappings.get(&path_str) {
                    let new_ident = Ident::new(new_field, Span::call_site());
                    *expr = syn::parse_quote!(self.#new_ident);
                    return;
                }
            }
        }
        // No match — recurse into children
        visit_mut::visit_expr_mut(self, expr);
    }
}
