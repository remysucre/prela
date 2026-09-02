//! `#[derive(IntoQuery)]` — make a schema struct drivable as a query.
//!
//! A schema entity is a struct that reads like a SQL table: a primary-key
//! column and the attribute columns, each a relation keyed by the entity's
//! ids. `Self` names the entity in field types, so the entity is spelled
//! once.
//!
//! ```ignore
//! #[derive(IntoQuery)]
//! pub struct Movie {
//!     #[primary_key]
//!     pub id: Key<Self>,                // the identity column: id → id
//!     pub title: Col<Self, Str>,        // id → title
//!     pub company: Set<Self, Id<Company>>,
//! }
//! ```
//!
//! The derive requires exactly one field marked `#[primary_key]` and
//! generates
//!
//! ```ignore
//! impl IntoQuery for &Movie {
//!     type Q = Key<Movie>;              // the marked field's type, `Self` resolved
//!     fn iq(self) -> Self::Q { self.id }
//! }
//! ```
//!
//! which is what lets the struct drive directly: `QueryExt` is
//! blanket-implemented over `IntoQuery`, so `db.movie.select(..)` resolves
//! by autoref on `&Movie` and yields the stored key column — no accessor,
//! no recomputation from another column's length. There is no coherence
//! clash with `impl<Q: Query> IntoQuery for Q` because the entity struct is
//! not itself a `Query`.
//!
//! Whether the id space is dense or has holes is a fact about the schema,
//! so it is stated by the author in the primary key's type, not by the
//! macro: `Key<Self>` for a dense entity, `SparseKey<Self>` for a gappy
//! one. The derive only asks that the type be `Copy` and a `Query`.

use proc_macro::TokenStream;
use proc_macro2::{Group, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// The named fields of a non-generic struct, or a compile error.
fn named_fields<'a>(
    input: &'a DeriveInput,
    derive: &str,
) -> Result<&'a syn::punctuated::Punctuated<syn::Field, syn::Token![,]>, TokenStream> {
    let err = |msg: String| {
        Err(syn::Error::new_spanned(&input.ident, msg)
            .to_compile_error()
            .into())
    };
    if !input.generics.params.is_empty() {
        return err(format!("{derive}: an entity struct takes no generics"));
    }
    match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => Ok(&f.named),
            _ => err(format!("{derive} needs a struct with named fields")),
        },
        _ => err(format!("{derive} applies to a struct")),
    }
}

/// `Self` → `name` throughout a type's tokens. Field types are written
/// `Key<Self>`; inside `impl IntoQuery for &Movie`, `Self` would be `&Movie`.
fn subst_self(ts: TokenStream2, name: &syn::Ident) -> TokenStream2 {
    ts.into_iter()
        .map(|tt| match tt {
            TokenTree::Ident(i) if i == "Self" => TokenTree::Ident(name.clone()),
            TokenTree::Group(g) => {
                let mut out = Group::new(g.delimiter(), subst_self(g.stream(), name));
                out.set_span(g.span());
                TokenTree::Group(out)
            }
            other => other,
        })
        .collect()
}

#[proc_macro_derive(IntoQuery, attributes(primary_key))]
pub fn derive_into_query(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let fields = match named_fields(&input, "IntoQuery") {
        Ok(f) => f,
        Err(e) => return e,
    };
    let marked: Vec<&syn::Field> = fields
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.path().is_ident("primary_key")))
        .collect();
    let key = match marked.as_slice() {
        [f] => *f,
        [] => {
            return syn::Error::new_spanned(
                name,
                "IntoQuery needs one field marked `#[primary_key]`, e.g. `pub id: Key<Self>`",
            )
            .to_compile_error()
            .into();
        }
        [_, second, ..] => {
            return syn::Error::new_spanned(second, "only one field may be `#[primary_key]`")
                .to_compile_error()
                .into();
        }
    };
    let key_ident = key.ident.as_ref().unwrap();
    let key_ty = key.ty.clone();
    let key_ty = subst_self(quote!(#key_ty), name);

    quote! {
        impl ::prela::engine::IntoQuery for &#name {
            type Q = #key_ty;
            #[inline(always)]
            fn iq(self) -> Self::Q {
                self.#key_ident
            }
        }
    }
    .into()
}
