//! `#[derive(IntoQuery)]` — make a schema struct drivable as a query.
//!
//! A schema entity is a struct that reads like a SQL table: its first field
//! is the key, and the remaining fields are its columns, each a relation
//! keyed by the entity's ids.
//!
//! ```ignore
//! #[derive(IntoQuery)]
//! pub struct Movie {
//!     pub key: Universe<Id<Movie>>,     // the identity column: id → id
//!     pub title: Col<Movie, Str>,       // id → title
//!     pub kind: Col<Movie, Id<Kind>>,   // id → kind
//! }
//! ```
//!
//! The derive requires a field named `key` and generates
//!
//! ```ignore
//! impl IntoQuery for &Movie {
//!     type Q = Universe<Id<Movie>>;     // the declared type of `key`
//!     fn iq(self) -> Self::Q { self.key }
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
//! so it is stated by the author in the type of `key`, not by the macro:
//! `key: Universe<Id<Part>>` for a dense entity, `key:
//! SparseUniverse<Id<Order>>` for a gappy one. The derive only asks that
//! the type be `Copy` and a `Query`.
//!
use proc_macro::TokenStream;
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

#[proc_macro_derive(IntoQuery)]
pub fn derive_into_query(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let fields = match named_fields(&input, "IntoQuery") {
        Ok(f) => f,
        Err(e) => return e,
    };
    let Some(key) = fields
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "key"))
    else {
        return syn::Error::new_spanned(
            name,
            "IntoQuery needs a `key` field holding the id space, \
             e.g. `pub key: Universe<Id<Self>>`",
        )
        .to_compile_error()
        .into();
    };
    let key_ty = &key.ty;

    quote! {
        impl ::prela::engine::IntoQuery for &#name {
            type Q = #key_ty;
            #[inline(always)]
            fn iq(self) -> Self::Q {
                self.key
            }
        }
    }
    .into()
}
