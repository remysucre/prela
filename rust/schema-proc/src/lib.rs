// `schema_proc!` — a procedural-macro implementation of `schema!`
// (prela's src/schema.rs). Same surface syntax, same generated items;
// see schema.rs for the full documentation of what is generated and why.
//
// Where `schema!` must thread everything through tt-munchers (macro_rules
// cannot hold structured state), this version parses the declaration into
// a small AST once and walks it with plain Rust — each generated item is
// one straightforward `quote!` block.
//
// LIMITATION: proc macros have no `$crate`, so generated paths are spelled
// `crate::engine` / `crate::cache` / `crate::format` — the macro is only
// usable from within the `prela` crate itself.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parenthesized, parse_macro_input, Ident, Result, Token};

// ===== AST ================================================================

struct Schema {
    mod_: Ident,
    store: Ident,
    init: Ident,
    entities: Vec<Entity>,
}

struct Entity {
    tag: Ident,
    /// `Movie(movie)` → Some(movie): generate a universe handle.
    uni: Option<Ident>,
    mode: Mode,
    nav: Ident,
    fields: Vec<Field>,
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Dense,
    Dict,
    Sparse,
}

struct Field {
    is_pub: bool,
    name: Ident,
    ty: FieldTy,
}

enum FieldTy {
    Str,
    I64,
    F64,
    /// bare entity ident — a foreign key.
    Fk(Ident),
    Multi(MultiTy),
}

enum MultiTy {
    Str,
    I64,
    Fk(Ident),
}

// ===== parsing ============================================================

impl Parse for Schema {
    fn parse(input: ParseStream) -> Result<Self> {
        let mod_: Ident = input.parse()?;
        input.parse::<Token![/]>()?;
        let store: Ident = input.parse()?;
        input.parse::<Token![/]>()?;
        let init: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let mut entities = Vec::new();
        while !input.is_empty() {
            entities.push(input.parse()?);
        }
        Ok(Schema {
            mod_,
            store,
            init,
            entities,
        })
    }
}

impl Parse for Entity {
    fn parse(input: ParseStream) -> Result<Self> {
        let tag: Ident = input.parse()?;
        let (mut uni, mut mode) = (None, Mode::Dense);
        if input.peek(syn::token::Paren) {
            let inner;
            parenthesized!(inner in input);
            uni = Some(inner.parse::<Ident>()?);
            if !inner.is_empty() {
                let m: Ident = inner.parse()?;
                mode = match m.to_string().as_str() {
                    "dict" => Mode::Dict,
                    "sparse" => Mode::Sparse,
                    _ => return Err(syn::Error::new(m.span(), "expected `dict` or `sparse`")),
                };
            }
        }
        input.parse::<Token![/]>()?;
        let nav: Ident = input.parse()?;
        let body;
        braced!(body in input);
        let mut fields = Vec::new();
        while !body.is_empty() {
            fields.push(body.parse()?);
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        if fields.is_empty() && (uni.is_some() || mode == Mode::Dict) {
            return Err(syn::Error::new(
                tag.span(),
                "a universe or `dict` entity needs at least one field (universe \
                 size / dict table come from the first column)",
            ));
        }
        Ok(Entity {
            tag,
            uni,
            mode,
            nav,
            fields,
        })
    }
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        let is_pub = input.peek(Token![pub]);
        if is_pub {
            input.parse::<Token![pub]>()?;
        }
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        Ok(Field { is_pub, name, ty })
    }
}

impl Parse for FieldTy {
    fn parse(input: ParseStream) -> Result<Self> {
        // `str`/`i64`/`f64` are primitive NAMES, not keywords, so they parse
        // as plain idents — matched here before the entity-ident case, same
        // precedence as schema!'s macro arms.
        let id: Ident = input.parse()?;
        Ok(match id.to_string().as_str() {
            "str" => FieldTy::Str,
            "i64" => FieldTy::I64,
            "f64" => FieldTy::F64,
            "Multi" => {
                input.parse::<Token![<]>()?;
                let inner: Ident = input.parse()?;
                let m = match inner.to_string().as_str() {
                    "str" => MultiTy::Str,
                    "i64" => MultiTy::I64,
                    _ => MultiTy::Fk(inner),
                };
                input.parse::<Token![>]>()?;
                FieldTy::Multi(m)
            }
            _ => FieldTy::Fk(id),
        })
    }
}

// ===== per-field-type facts =====

impl FieldTy {
    /// Physical column type. `prefix` is the `::`-path back to the scope
    /// holding the entity tags (empty at invocation scope, `super::` inside
    /// the schema module, `super::super::` inside a handle module).
    fn colty(&self, prefix: &TokenStream2, ent: &Ident) -> TokenStream2 {
        let e = quote!(crate::engine::Id<#prefix #ent>);
        match self {
            FieldTy::Str => quote!(crate::engine::VecRel<&'static str, #e>),
            FieldTy::I64 => quote!(crate::engine::VecRel<i64, #e>),
            FieldTy::F64 => quote!(crate::engine::VecRel<f64, #e>),
            FieldTy::Fk(t) => quote!(
                crate::engine::VecRel<<#prefix #t as crate::engine::EntityKind>::Fk, #e>
            ),
            FieldTy::Multi(MultiTy::Str) => quote!(crate::engine::MultiRel<&'static str, #e>),
            FieldTy::Multi(MultiTy::I64) => quote!(crate::engine::MultiRel<i64, #e>),
            FieldTy::Multi(MultiTy::Fk(t)) => {
                quote!(crate::engine::MultiRel<crate::engine::Id<#prefix #t>, #e>)
            }
        }
    }

    /// Cache kind for the MANIFEST row.
    fn kind(&self) -> TokenStream2 {
        match self {
            FieldTy::Str => quote!(crate::format::KIND_DENSE_STR),
            FieldTy::I64 | FieldTy::Fk(_) => quote!(crate::format::KIND_DENSE_I64),
            FieldTy::F64 => quote!(crate::format::KIND_DENSE_F64),
            FieldTy::Multi(MultiTy::Str) => quote!(crate::format::KIND_CSR_STR),
            FieldTy::Multi(_) => quote!(crate::format::KIND_CSR_WORDS),
        }
    }

    /// Cache-reader call for init (emitted at invocation scope, where the
    /// entity tags are in scope bare).
    fn loader(&self, fname: &str) -> TokenStream2 {
        match self {
            FieldTy::Str => quote!(crate::cache::load_strs_in(cache_dir, #fname)),
            FieldTy::I64 => quote!(crate::cache::load_i64_in(cache_dir, #fname)),
            FieldTy::F64 => quote!(crate::cache::load_f64_in(cache_dir, #fname)),
            FieldTy::Fk(t) => quote!(crate::cache::load_fk_in::<#t, _>(cache_dir, #fname)),
            FieldTy::Multi(MultiTy::Str) => {
                quote!(crate::cache::load_multi_strs_in(cache_dir, #fname))
            }
            FieldTy::Multi(MultiTy::I64) => {
                quote!(crate::cache::load_multi_i64_in(cache_dir, #fname))
            }
            FieldTy::Multi(MultiTy::Fk(_)) => {
                quote!(crate::cache::load_multi_ids_in(cache_dir, #fname))
            }
        }
    }

    /// The scalar type for a `Primary` impl, if this type supports one.
    fn primary_scalar(&self) -> Option<TokenStream2> {
        match self {
            FieldTy::Str => Some(quote!(&'static str)),
            FieldTy::I64 => Some(quote!(i64)),
            FieldTy::F64 => Some(quote!(f64)),
            _ => None,
        }
    }
}

// ===== codegen ============================================================

#[proc_macro]
pub fn schema_proc(input: TokenStream) -> TokenStream {
    let schema = parse_macro_input!(input as Schema);
    expand(&schema).into()
}

fn expand(s: &Schema) -> TokenStream2 {
    let Schema {
        mod_,
        store,
        init,
        entities,
    } = s;
    let tags: Vec<_> = entities.iter().map(|e| &e.tag).collect();

    let cols_structs = entities.iter().map(|e| cols_struct(e));
    let handle_mods = entities.iter().map(|e| handle_mod(e));
    let init_ents = entities.iter().map(|e| init_entity(mod_, e));
    let entity_kinds = entities.iter().map(|e| entity_kind(mod_, e));
    let universes = entities.iter().filter_map(|e| universe(mod_, e));
    let consts = entities.iter().map(|e| consts(mod_, e));
    let nav_traits = entities.iter().map(|e| nav_trait(mod_, e));
    let primaries = entities.iter().filter_map(|e| primary(mod_, e));
    let manifest_rows = entities.iter().flat_map(|e| {
        let ent_s = e.tag.to_string();
        e.fields.iter().map(move |f| {
            let (ent_s, f_s, kind) = (ent_s.clone(), f.name.to_string(), f.ty.kind());
            quote!((#ent_s, #f_s, #kind),)
        })
    });

    quote! {
        /// `Entity` structs
        #( #[allow(dead_code)] pub struct #tags; )*

        /// Generated storage: one cols struct per entity, filled by init.
        #[allow(non_snake_case, dead_code)]
        pub struct #store {
            #( pub #tags: #mod_::#tags, )*
        }

        #[allow(non_snake_case, dead_code)]
        pub mod #mod_ {
            pub static STORE: ::std::sync::OnceLock<super::#store> =
                ::std::sync::OnceLock::new();
            #( #cols_structs )*
            #( #handle_mods )*
        }

        /// Load every column from `<cache_dir>/<Entity>_<field>.bin`.
        #[allow(dead_code)]
        pub fn #init(cache_dir: &::std::path::Path) {
            let loaded = #store { #( #init_ents, )* };
            if #mod_::STORE.set(loaded).is_err() {
                panic!(concat!(stringify!(#init), ": schema already initialized"));
            }
        }

        #( #entity_kinds )*
        #( #universes )*
        #( #consts )*
        #( #nav_traits )*
        #( #primaries )*

        /// Generated (entity, field, `format::KIND_*`) manifest — the file
        /// list + physical kinds this schema loads, consumed by `regen` to
        /// verify the cache it writes.
        #[allow(dead_code)]
        pub const MANIFEST: &[(&str, &str, u32)] = &[ #( #manifest_rows )* ];
    }
}

/// Cols struct, inside the schema module (entity tags are `super::*`).
fn cols_struct(e: &Entity) -> TokenStream2 {
    let tag = &e.tag;
    let prefix = quote!(super::);
    let fields = e.fields.iter().map(|f| {
        let (name, ty) = (&f.name, f.ty.colty(&prefix, tag));
        quote!(pub #name: #ty,)
    });
    quote! {
        pub struct #tag { #( #fields )* }
    }
}

/// Per-field leaf handles — ZSTs resolving (via `IntoQuery::iq`, one
/// OnceLock fetch at plan construction) to the `&'static` column relation.
fn handle_mod(e: &Entity) -> TokenStream2 {
    let (tag, nav) = (&e.tag, &e.nav);
    let prefix = quote!(super::super::);
    let handles = e.fields.iter().map(|f| {
        let (name, ty) = (&f.name, f.ty.colty(&prefix, tag));
        quote! {
            #[derive(Clone, Copy)]
            pub struct #name;
            impl crate::engine::IntoQuery for #name {
                type Q = &'static #ty;
                #[inline]
                fn iq(self) -> Self::Q {
                    &super::STORE.get().expect("schema not initialized").#tag.#name
                }
            }
        }
    });
    quote! {
        #[allow(non_camel_case_types, dead_code)]
        pub mod #nav { #( #handles )* }
    }
}

/// One `Ent: mod::Ent { field: loader, … }` struct-literal entry for init.
fn init_entity(mod_: &Ident, e: &Entity) -> TokenStream2 {
    let tag = &e.tag;
    let fields = e.fields.iter().map(|f| {
        let name = &f.name;
        let loader = f.ty.loader(&format!("{}_{}", tag, name));
        quote!(#name: #loader,)
    });
    quote!(#tag: #mod_::#tag { #( #fields )* })
}

/// EntityKind: how the entity is addressed. Dense (default, incl. `sparse`
/// — a drive property, not an addressing one): Fk = Id, Table = Ident, which
/// inlines away. `dict`: Fk = Key, Table = a `DictTable` built once (lazily)
/// from the entity's FIRST field — its `i64` external-id column.
fn entity_kind(mod_: &Ident, e: &Entity) -> TokenStream2 {
    let tag = &e.tag;
    if e.mode == Mode::Dict {
        let f0 = &e.fields[0].name;
        quote! {
            impl crate::engine::EntityKind for #tag {
                type Fk = crate::engine::Key<#tag>;
                type Table = &'static crate::engine::DictTable<#tag>;
                #[inline]
                fn table() -> Self::Table {
                    static T: ::std::sync::OnceLock<crate::engine::DictTable<#tag>>
                        = ::std::sync::OnceLock::new();
                    T.get_or_init(|| crate::engine::DictTable::from_i64(
                        &#mod_::STORE.get().expect("schema not initialized").#tag.#f0.values))
                }
            }
        }
    } else {
        quote! {
            impl crate::engine::EntityKind for #tag {
                type Fk = crate::engine::Id<#tag>;
                type Table = crate::engine::Ident<#tag>;
                #[inline(always)] fn table() -> Self::Table { crate::engine::Ident::new() }
            }
        }
    }
}

/// Universe handle (if declared), sized by the FIRST field's key count.
/// `sparse` gets a `SparseUniverse` whose drive skips hole slots (validity
/// mask built lazily from the first field — an FK where `NO_ID` marks holes).
fn universe(mod_: &Ident, e: &Entity) -> Option<TokenStream2> {
    let uni = e.uni.as_ref()?;
    let (tag, ff) = (&e.tag, &e.fields[0].name);
    Some(if e.mode == Mode::Sparse {
        quote! {
            #[allow(non_camel_case_types, dead_code)]
            #[derive(Clone, Copy)]
            pub struct #uni;
            impl crate::engine::IntoQuery for #uni {
                type Q = crate::engine::SparseUniverse<crate::engine::Id<#tag>>;
                #[inline]
                fn iq(self) -> Self::Q {
                    static MASK: ::std::sync::OnceLock<
                        crate::engine::Bitset<crate::engine::Id<#tag>>> = ::std::sync::OnceLock::new();
                    let store = #mod_::STORE.get().expect("schema not initialized");
                    let n = store.#tag.#ff.n_keys();
                    let mask = MASK.get_or_init(||
                        crate::engine::Bitset::<crate::engine::Id<#tag>>::validity(
                            &store.#tag.#ff.values));
                    crate::engine::SparseUniverse::new(n, mask)
                }
            }
        }
    } else {
        quote! {
            #[allow(non_camel_case_types, dead_code)]
            #[derive(Clone, Copy)]
            pub struct #uni;
            impl crate::engine::IntoQuery for #uni {
                type Q = crate::engine::Universe<crate::engine::Id<#tag>>;
                #[inline]
                fn iq(self) -> Self::Q {
                    crate::engine::Universe::new(
                        #mod_::STORE.get().expect("schema not initialized").#tag.#ff.n_keys())
                }
            }
        }
    })
}

/// Public handle spellings: a qualified assoc const (`Ent::field`) for every
/// field, plus a bare `pub use` re-export for `pub` fields.
fn consts(mod_: &Ident, e: &Entity) -> TokenStream2 {
    let (tag, nav) = (&e.tag, &e.nav);
    let items = e.fields.iter().map(|f| {
        let name = &f.name;
        let reexport = f.is_pub.then(|| {
            quote! {
                #[allow(unused_imports)]
                pub use #mod_::#nav::#name;
            }
        });
        quote! {
            impl #tag {
                #[allow(non_upper_case_globals, dead_code)]
                pub const #name: #mod_::#nav::#name = #mod_::#nav::#name;
            }
            #reexport
        }
    });
    quote!(#( #items )*)
}

/// Navigation trait: one compose method per field, blanket-implemented for
/// anything resolving to a query valued in this entity's ids. FK fields
/// additionally cross the target's entity table (`Ident` for dense — inlines
/// away; a Key→Id dictionary for `dict`).
fn nav_trait(mod_: &Ident, e: &Entity) -> TokenStream2 {
    let (tag, nav) = (&e.tag, &e.nav);
    let prefix = quote!();
    let methods = e.fields.iter().map(|f| {
        let (name, colty) = (&f.name, f.ty.colty(&prefix, tag));
        let col = quote!(&#mod_::STORE.get().expect("schema not initialized").#tag.#name);
        if let FieldTy::Fk(t) = &f.ty {
            quote! {
                #[allow(dead_code)]
                #[inline]
                fn #name(self) -> crate::engine::Compose<
                    crate::engine::Compose<Self::Q, &'static #colty>,
                    <#t as crate::engine::EntityKind>::Table>
                {
                    crate::engine::Compose {
                        a: crate::engine::Compose { a: self.iq(), b: #col },
                        b: <#t as crate::engine::EntityKind>::table(),
                    }
                }
            }
        } else {
            quote! {
                #[allow(dead_code)]
                #[inline]
                fn #name(self) -> crate::engine::Compose<Self::Q, &'static #colty> {
                    crate::engine::Compose { a: self.iq(), b: #col }
                }
            }
        }
    });
    quote! {
        #[allow(dead_code)]
        pub trait #nav: crate::engine::IntoQuery + Sized
        where Self::Q: crate::engine::Query<R = crate::engine::Id<#tag>>
        {
            #( #methods )*
        }
        impl<T: crate::engine::IntoQuery + Sized> #nav for T
        where T::Q: crate::engine::Query<R = crate::engine::Id<#tag>> {}
    }
}

/// `impl Primary` iff the FIRST field is scalar (str/i64/f64), reusing that
/// column for `primary()`. Entity-ref / Multi first fields get no impl.
fn primary(mod_: &Ident, e: &Entity) -> Option<TokenStream2> {
    let f0 = e.fields.first()?;
    let scalar = f0.ty.primary_scalar()?;
    let (tag, ff) = (&e.tag, &f0.name);
    let colty = f0.ty.colty(&quote!(), tag);
    Some(quote! {
        impl crate::engine::Primary for #tag {
            type Scalar = #scalar;
            type Col = #colty;
            #[inline]
            fn primary() -> &'static Self::Col {
                &#mod_::STORE.get().expect("schema not initialized").#tag.#ff
            }
        }
    })
}
