// `schema_simpl!` — a refactored twin of `schema!` (src/schema.rs): same
// surface syntax, same generated items, but with the internal duplication
// consolidated (one field-syntax walker, one field-type fact table, one
// nav-method emitter — see "Internal organization" below). Kept alongside
// the original for comparison; nothing in the crate invokes it outside its
// own tests. NOTE: needs `#![recursion_limit = "256"]` for large schemas.
//
// `schema!` — declarative typed schemas over the v2 binary cache.
//
// One declaration generates, per entity:
//   - a unit-struct entity tag (`pub struct Movie;`) — the phantom `E` of
//     `Id<E>`, so cross-entity compositions are COMPILE errors;
//   - a cols struct (same ident, inside the schema's module) holding the
//     entity's typed columns, loaded by the generated `init(cache_dir)`
//     from `<Entity>_<field>.bin` via the src/cache.rs v2 readers
//     (id-valued columns are bulk-reinterpreted to `Id<T>` through the
//     `repr(transparent)` layout — no per-element conversion);
//   - a paren-free leaf HANDLE per field: a ZST named by the field, living
//     in `<mod>::<Nav>` (the nav-trait ident doubles as the per-entity
//     handle namespace — macro_rules cannot concatenate idents), that
//     implements `engine::IntoQuery<Q = &'static VecRel/MultiRel<…>>`. Its
//     `iq` fetches the column from the OnceLock store ONCE, at plan
//     construction — the built plan contains only the `&'static` leaf, so
//     hot loops are identical to hand-built plans;
//   - the QUALIFIED spelling for EVERY field as an associated const on the
//     entity tag (`impl Info { pub const ty: TPCH-internal handle type }`),
//     so `Info::ty` is a value usable wherever a relation is expected;
//   - a BARE re-export of the handle (`pub use …::production_year;`) for
//     fields marked `pub` — explicit, like Julia's selective `@expose`,
//     because macro_rules cannot detect cross-entity name collisions.
//     CAVEAT (unit structs in patterns): a bare handle in scope captures
//     any same-named BINDING pattern — `let kind = …`, a closure param
//     `|part, _|`, a match arm — which then fails to compile ("interpreted
//     as a unit struct, not a new binding"). Rename such locals. This also
//     means `assert_eq!`/`assert_ne!` break in any module that glob-imports
//     a schema exporting bare `kind` (the core macros internally bind
//     `let kind`): test modules import schema names selectively;
//   - a bare universe HANDLE (`pub struct movie;` with
//     `IntoQuery<Q = Universe<Id<Movie>>>`) when the entity is declared
//     `Movie(movie)` — explicit because macro_rules cannot lowercase
//     idents. Universe size = the entity's first column's key count,
//     resolved at `iq` time;
//   - a NAVIGATION extension trait (`Movie(movie) / MovieNav`) with one
//     method per field, blanket-implemented for everything that RESOLVES
//     (via `IntoQuery`) to a query whose value type is the entity's id:
//       trait MovieNav: IntoQuery + Sized
//       where Self::Q: Query<R = Id<Movie>> {
//           fn title(self) -> Compose<Self::Q, &'static …> { … } // per field
//       }
//     so `cast.person().name()` spells the compose chain
//     `cast.select(Cast::person).select(Person::name)` — the leaf handle roots
//     the chain paren-free, and every later hop is a nav method. Coherence
//     is safe: same-named methods on different entities' nav traits have
//     disjoint receivers (a resolved query's `R` equals exactly ONE
//     `Id<E>`), so method resolution always finds a single applicable
//     trait. Predicate ROOTS are bare handles (`keyword`) or qualified
//     consts (`Entity::field`); everything after the root navigates.
//
// Field types: `str` → `VecRel<&'static str, Id<E>>`; `i64`/`f64` →
// `VecRel<i64/f64, Id<E>>`; a bare entity ident `Kind` →
// `VecRel<Id<Kind>, Id<E>>`; `Multi<T>` → `MultiRel<…, Id<E>>` likewise.
// The literal idents `str|i64|f64` are matched before the entity-ident
// case, so an entity cannot be named `str`/`i64`/`f64`.
//
// Filenames are `concat!(stringify!(Entity), "_", stringify!(field))`,
// verbatim — field names are filenames (no raw-keyword underscore
// trimming; spell keyword-ish fields differently, e.g. `ty` not `type`).
//
// The declaration is also regen's source of truth for WHAT the cache must
// contain: the macro emits a `pub const MANIFEST: &[(&str, &str, u32)]`
// of (entity, field, cache kind — src/format.rs `KIND_*`). regen's
// parquet→cache TRANSFORMATION logic (FK joins, multi-table splits) stays
// hand-written — it is parquet-specific — but after writing, regen checks
// its output file set and header kinds against the manifest, so a column
// regen produces that the schema doesn't declare (or vice versa, or with
// the wrong physical kind) fails the regen run loudly.
//
// Internal organization — two shared mechanisms replace what used to be
// per-consumer copies:
//   @walk — THE field-list walker, the only rule that parses field surface
//     syntax (`[pub] name : ty [<T>] ,`). A consumer `<cb>` implements two
//     arms — `@<cb> field …` (emit, then tail-call @walk with the rest)
//     and `@<cb> done …` — instead of re-implementing the pub-stripping
//     muncher. Consumers: cols, initent, mani(fest), consts, handle, nav.
//   @tyrow — THE field-type fact table: one row per field type carrying
//     (relation shape, cache loader, cache kind), CPS-dispatched to tiny
//     receivers (@colty_r / @load_r / @kind_r). Adding a field type = one
//     row (+ a @valty arm if it introduces a new value type).
// Walker recursion costs ~2 expansion frames per field (the manifest walks
// every field of the schema in one chain), so crates declaring large
// schemas set `#![recursion_limit = "256"]`.

#[macro_export]
macro_rules! schema_simpl {
    // ===== entry: SCHEMA_MOD / StorageStruct / init_fn : entities... =====
    ( $mod_:ident / $store:ident / $init:ident :
      $( $Ent:ident $( ( $uni:ident $($mode:ident)? ) )? / $Nav:ident { $($body:tt)* } )* ) => {

        $(
            #[allow(dead_code)]
            pub struct $Ent;
        )*

        /// Generated storage: one cols struct per entity, filled by `init`.
        #[allow(non_snake_case, dead_code)]
        pub struct $store {
            $( pub $Ent: $mod_::$Ent, )*
        }

        #[allow(non_snake_case, dead_code)]
        pub mod $mod_ {
            pub static STORE: ::std::sync::OnceLock<super::$store> =
                ::std::sync::OnceLock::new();
            $( $crate::schema_simpl::schema_simpl!{@walk cols ($Ent) [] $($body)*} )*
            $( $crate::schema_simpl::schema_simpl!(@handlemod $Ent; $Nav; $($body)*); )*
        }

        /// Load every column from `<cache_dir>/<Entity>_<field>.bin`.
        #[allow(dead_code)]
        pub fn $init(cache_dir: &::std::path::Path) {
            let loaded = $store {
                $( $Ent: $crate::schema_simpl::schema_simpl!{@walk initent (cache_dir $mod_ $Ent) [] $($body)*}, )*
            };
            if $mod_::STORE.set(loaded).is_err() {
                panic!(concat!(stringify!($init), ": schema already initialized"));
            }
        }

        $( $crate::schema_simpl::schema_simpl!(@entitykind $mod_; $Ent; [$($($mode)?)?] $($body)*); )*
        $( $crate::schema_simpl::schema_simpl!(@uni $mod_; $Ent; [$($uni $($mode)?)?]; $($body)*); )*
        $( $crate::schema_simpl::schema_simpl!{@walk consts ($mod_ $Ent $Nav) [] $($body)*} )*
        $( $crate::schema_simpl::schema_simpl!{@walk nav ($mod_ $Ent $Nav) [] $($body)*} )*
        $( $crate::schema_simpl::schema_simpl!(@primary $mod_; $Ent; $($body)*); )*

        $crate::schema_simpl::schema_simpl!(@manifest [] $( $Ent { $($body)* } )*);
    };

    // ===== @walk: the field-list walker ===================================
    // Single parser of the field surface syntax. The pub arm is tried first
    // because `$f:ident` would eat a leading `pub` (ident matches keywords).
    // Each field is handed to the consumer as
    //     @<cb> field <ctx> [acc] [pub?] f; t1 [(t2)]; then rest...
    // and the consumer tail-calls @walk with `rest`; list end dispatches
    // `@<cb> done <ctx> [acc]`. Recursion is brace-form (`schema!{…}`) so
    // the one walker works in item AND expr position (item-position paren
    // calls would need a trailing `;`, expr-position calls must not have one).
    (@walk $cb:ident $ctx:tt [$($acc:tt)*]) => {
        $crate::schema_simpl::schema_simpl!{@$cb done $ctx [$($acc)*]}
    };
    (@walk $cb:ident $ctx:tt [$($acc:tt)*]
      pub $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)?) => {
        $crate::schema_simpl::schema_simpl!{@$cb field $ctx [$($acc)*] [pub] $f; $t1 $(($t2))?; then $($($rest)*)?}
    };
    (@walk $cb:ident $ctx:tt [$($acc:tt)*]
      $f:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)?) => {
        $crate::schema_simpl::schema_simpl!{@$cb field $ctx [$($acc)*] [] $f; $t1 $(($t2))?; then $($($rest)*)?}
    };

    // ===== @tyrow: the field-type fact table ==============================
    // One row per field type, carrying every per-type fact:
    //     [RelKind value-type] [loader (fk target)] KIND_*
    // CPS-dispatched: `@tyrow <ty> => <recv> <args>` invokes
    // `@<recv> <facts> <args>`; each receiver picks the facts it needs.
    (@tyrow str => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [VecRel str] [load_strs_in] KIND_DENSE_STR $args)
    };
    (@tyrow i64 => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [VecRel i64] [load_i64_in] KIND_DENSE_I64 $args)
    };
    (@tyrow f64 => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [VecRel f64] [load_f64_in] KIND_DENSE_F64 $args)
    };
    (@tyrow Multi (str) => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [MultiRel str] [load_multi_strs_in] KIND_CSR_STR $args)
    };
    (@tyrow Multi (i64) => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [MultiRel i64] [load_multi_i64_in] KIND_CSR_WORDS $args)
    };
    (@tyrow Multi ($T:ident) => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [MultiRel (id $T)] [load_multi_ids_in] KIND_CSR_WORDS $args)
    };
    (@tyrow $T:ident => $k:ident $args:tt) => {
        $crate::schema_simpl::schema_simpl!(@$k [VecRel (fk $T)] [load_fk_in $T] KIND_DENSE_I64 $args)
    };

    // ===== field type → physical column type (receiver: @colty_r) ========
    // The parenthesized prefix is a `::`-joined path back to the scope
    // holding the entity tags: `()` at the invocation scope, `(super)` from
    // inside the schema module, `(super super)` from a handle module.
    (@colty ($($p:ident)*); $E:ident; $($t:tt)+) => {
        $crate::schema_simpl::schema_simpl!(@tyrow $($t)+ => colty_r (($($p)*) $E))
    };
    (@colty_r [$rel:ident $vt:tt] $ld:tt $kind:ident (($($p:ident)*) $E:ident)) => {
        $crate::engine::$rel<
            $crate::schema_simpl::schema_simpl!(@valty ($($p)*); $vt),
            $crate::engine::Id<$($p::)* $E>>
    };
    // a column's value type, path-prefixed like @colty
    (@valty ($($p:ident)*); str) => { &'static str };
    (@valty ($($p:ident)*); i64) => { i64 };
    (@valty ($($p:ident)*); f64) => { f64 };
    (@valty ($($p:ident)*); (id $T:ident)) => { $crate::engine::Id<$($p::)* $T> };
    (@valty ($($p:ident)*); (fk $T:ident)) => {
        <$($p::)* $T as $crate::engine::EntityKind>::Fk
    };

    // ===== field type → cache kind (receiver: @kind_r) ====================
    (@kind $($t:tt)+) => { $crate::schema_simpl::schema_simpl!(@tyrow $($t)+ => kind_r ()) };
    (@kind_r $shape:tt $ld:tt $kind:ident ()) => { $crate::format::$kind };

    // ===== field type → cache reader (receiver: @load_r) ==================
    (@load $dir:ident; $name:expr; $($t:tt)+) => {
        $crate::schema_simpl::schema_simpl!(@tyrow $($t)+ => load_r ($dir $name))
    };
    (@load_r $shape:tt [$ld:ident $($LT:ident)?] $kind:ident ($dir:ident $name:tt)) => {
        $crate::cache::$ld$(::<$LT, _>)?($dir, $name)
    };

    // ===== manifest: (entity, field, cache kind) for every column ========
    // Walks entities × fields into one accumulator (the walker ctx carries
    // the not-yet-walked entities). Field names are used verbatim —
    // `<Entity>_<field>` IS the cache filename.
    (@manifest [$($acc:tt)*]) => {
        /// Generated (entity, field, `format::KIND_*`) manifest — the file
        /// list + physical kinds this schema loads, consumed by `regen` to
        /// verify the cache it writes.
        #[allow(dead_code)]
        pub const MANIFEST: &[(&str, &str, u32)] = &[$($acc)*];
    };
    (@manifest [$($acc:tt)*] $Ent:ident { $($body:tt)* } $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@walk mani ($Ent [$($rest)*]) [$($acc)*] $($body)*}
    };
    (@mani field ($Ent:ident $rest:tt) [$($acc:tt)*] $p:tt
      $f:ident; $t1:tt $(($t2:tt))?; then $($frest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@walk mani ($Ent $rest)
            [$($acc)* (stringify!($Ent), stringify!($f),
                       $crate::schema_simpl::schema_simpl!(@kind $t1 $(($t2))?)),]
            $($frest)*}
    };
    (@mani done ($Ent:ident [$($rest:tt)*]) [$($acc:tt)*]) => {
        $crate::schema_simpl::schema_simpl!{@manifest [$($acc)*] $($rest)*}
    };

    // ===== cols struct (inside the schema module; entity tags are super::*)
    (@cols field ($Ent:ident) [$($acc:tt)*] $p:tt
      $f:ident; $t1:tt $(($t2:tt))?; then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@walk cols ($Ent)
            [$($acc)* pub $f: $crate::schema_simpl::schema_simpl!(@colty (super); $Ent; $t1 $(($t2))?),]
            $($rest)*}
    };
    (@cols done ($Ent:ident) [$($acc:tt)*]) => {
        pub struct $Ent { $($acc)* }
    };

    // ===== per-entity struct literal for init (expr position) ============
    (@initent field ($dir:ident $mod_:ident $Ent:ident) [$($acc:tt)*] $p:tt
      $f:ident; $t1:tt $(($t2:tt))?; then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@walk initent ($dir $mod_ $Ent)
            [$($acc)* $f: $crate::schema_simpl::schema_simpl!(@load $dir;
                concat!(stringify!($Ent), "_", stringify!($f)); $t1 $(($t2))?),]
            $($rest)*}
    };
    (@initent done ($dir:ident $mod_:ident $Ent:ident) [$($acc:tt)*]) => {
        $mod_::$Ent { $($acc)* }
    };

    // ===== Primary: elision support for entities with a scalar first field
    // Emits `impl Primary` (engine.rs) iff the FIRST field is str/i64/f64,
    // reusing the existing first-field column for `primary()`. Entity-ref /
    // Multi first fields (roots like Order, Lineitem) get no impl — so `.eq`
    // on their ids stays unavailable. `pub` is stripped first (the `$ff:ident`
    // matcher would otherwise eat it, same as @walk).
    (@primary $mod_:ident; $Ent:ident; pub $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!(@primary $mod_; $Ent; $($rest)*);
    };
    (@primary $mod_:ident; $Ent:ident; $ff:ident : str $(, $($rest:tt)*)?) => {
        $crate::schema_simpl::schema_simpl!(@primary_emit $mod_; $Ent; $ff; &'static str; str);
    };
    (@primary $mod_:ident; $Ent:ident; $ff:ident : i64 $(, $($rest:tt)*)?) => {
        $crate::schema_simpl::schema_simpl!(@primary_emit $mod_; $Ent; $ff; i64; i64);
    };
    (@primary $mod_:ident; $Ent:ident; $ff:ident : f64 $(, $($rest:tt)*)?) => {
        $crate::schema_simpl::schema_simpl!(@primary_emit $mod_; $Ent; $ff; f64; f64);
    };
    // first field is an entity ref or Multi<…> → no scalar primary.
    (@primary $mod_:ident; $Ent:ident; $ff:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)?) => {};
    (@primary_emit $mod_:ident; $Ent:ident; $ff:ident; $scalar:ty; $colkind:tt) => {
        impl $crate::engine::Primary for $Ent {
            type Scalar = $scalar;
            type Col = $crate::schema_simpl::schema_simpl!(@colty (); $Ent; $colkind);
            #[inline]
            fn primary() -> &'static Self::Col {
                &$mod_::STORE.get().expect("schema not initialized").$Ent.$ff
            }
        }
    };

    // ===== EntityKind: how an entity is addressed (dense Ident | dict) =====
    // DENSE (default): Fk = Id, Table = Ident — inlines away, so navs into a
    // dense entity are byte-identical to a direct column compose.
    (@entitykind $mod_:ident; $Ent:ident; [] $($body:tt)*) => {
        impl $crate::engine::EntityKind for $Ent {
            type Fk = $crate::engine::Id<$Ent>;
            type Table = $crate::engine::Ident<$Ent>;
            #[inline(always)] fn table() -> Self::Table { $crate::engine::Ident::new() }
        }
    };
    // NON-DENSE (`dict`): Fk = Key, Table = a `DictTable` built once (lazily)
    // from the entity's FIRST field — its `i64` external-id column.
    (@entitykind $mod_:ident; $Ent:ident; [dict] $f0:ident : $($rest:tt)*) => {
        impl $crate::engine::EntityKind for $Ent {
            type Fk = $crate::engine::Key<$Ent>;
            type Table = &'static $crate::engine::DictTable<$Ent>;
            #[inline]
            fn table() -> Self::Table {
                static T: ::std::sync::OnceLock<$crate::engine::DictTable<$Ent>>
                    = ::std::sync::OnceLock::new();
                T.get_or_init(|| $crate::engine::DictTable::from_i64(
                    &$mod_::STORE.get().expect("schema not initialized").$Ent.$f0.values))
            }
        }
    };
    // `sparse` is a DRIVE property (masked universe), not an addressing one —
    // such an entity is still dense-ADDRESSED, so its EntityKind is the default.
    (@entitykind $mod_:ident; $Ent:ident; [sparse] $($body:tt)*) => {
        $crate::schema_simpl::schema_simpl!(@entitykind $mod_; $Ent; [] $($body)*);
    };

    // ===== universe handle, sized by the FIRST declared field ============
    (@uni $mod_:ident; $Ent:ident; []; $($rest:tt)*) => {};
    // `dict` is an ADDRESSING property; the universe (drive over `0..n`) is
    // unaffected, so a dict entity gets the normal universe handle.
    (@uni $mod_:ident; $Ent:ident; [$uni:ident dict]; $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!(@uni $mod_; $Ent; [$uni]; $($rest)*);
    };
    (@uni $mod_:ident; $Ent:ident; [$uni:ident]; pub $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!(@uni $mod_; $Ent; [$uni]; $($rest)*);
    };
    (@uni $mod_:ident; $Ent:ident; [$uni:ident];
      $ff:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        /// Generated universe HANDLE — resolves (via `IntoQuery`) to the
        /// identity relation over the entity's dense id space (size = first
        /// column's key count, read at plan-construction time).
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Clone, Copy)]
        pub struct $uni;
        impl $crate::engine::IntoQuery for $uni {
            type Q = $crate::engine::Universe<$crate::engine::Id<$Ent>>;
            #[inline]
            fn iq(self) -> Self::Q {
                $crate::engine::Universe::new(
                    $mod_::STORE.get().expect("schema not initialized").$Ent.$ff.n_keys())
            }
        }
    };

    // ===== SPARSE universe handle: dense id space WITH holes (e.g. orders =====
    // ===== over sparse orderkeys). Resolves to a `SparseUniverse` whose ======
    // ===== drive skips holes; validity mask built lazily from the FIRST ======
    // ===== field (which must be an FK — `NO_ID` marks a hole slot). ==========
    (@uni $mod_:ident; $Ent:ident; [$uni:ident sparse]; pub $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!(@uni $mod_; $Ent; [$uni sparse]; $($rest)*);
    };
    (@uni $mod_:ident; $Ent:ident; [$uni:ident sparse];
      $ff:ident : $t1:tt $(< $t2:tt >)? $(, $($rest:tt)*)? ) => {
        /// Generated SPARSE universe HANDLE — resolves to a `SparseUniverse`
        /// whose `drive` enumerates only live slots (the orderkey-gap holes are
        /// masked out); `probe`/`member` keep the plain range check.
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Clone, Copy)]
        pub struct $uni;
        impl $crate::engine::IntoQuery for $uni {
            type Q = $crate::engine::SparseUniverse<$crate::engine::Id<$Ent>>;
            #[inline]
            fn iq(self) -> Self::Q {
                static MASK: ::std::sync::OnceLock<
                    $crate::engine::Bitset<$crate::engine::Id<$Ent>>> = ::std::sync::OnceLock::new();
                let store = $mod_::STORE.get().expect("schema not initialized");
                let n = store.$Ent.$ff.n_keys();
                let mask = MASK.get_or_init(||
                    $crate::engine::Bitset::<$crate::engine::Id<$Ent>>::validity(
                        &store.$Ent.$ff.values));
                $crate::engine::SparseUniverse::new(n, mask)
            }
        }
    };

    // ===== navigation trait: one compose method per field ================
    // Trait items can't be emitted incrementally into an open `trait { … }`,
    // so methods accumulate as tts and the trait + blanket impl are emitted
    // by the done arm. The three scalar literals are intercepted so a bare
    // entity ident (FK) falls through to the last arm; all five dispatch
    // arms forward to the ONE method emitter, @navmeth — the FK arm passes
    // its target `[T]`, which adds the entity-table hop (`Ident` for a
    // dense entity, so it inlines away; a non-dense entity gets a Key→Id
    // dictionary), keeping the result valued `Id<T>`.
    (@nav field $ctx:tt $acc:tt $p:tt $f:ident; str; then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@navmeth $ctx $acc $f; (str); []; $($rest)*}
    };
    (@nav field $ctx:tt $acc:tt $p:tt $f:ident; i64; then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@navmeth $ctx $acc $f; (i64); []; $($rest)*}
    };
    (@nav field $ctx:tt $acc:tt $p:tt $f:ident; f64; then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@navmeth $ctx $acc $f; (f64); []; $($rest)*}
    };
    (@nav field $ctx:tt $acc:tt $p:tt $f:ident; Multi ($t2:tt); then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@navmeth $ctx $acc $f; (Multi ($t2)); []; $($rest)*}
    };
    (@nav field $ctx:tt $acc:tt $p:tt $f:ident; $T:ident; then $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@navmeth $ctx $acc $f; ($T); [$T]; $($rest)*}
    };
    (@navmeth ($mod_:ident $Ent:ident $Nav:ident) [$($acc:tt)*]
      $f:ident; ($($t:tt)+); [$($T:ident)?]; $($rest:tt)*) => {
        $crate::schema_simpl::schema_simpl!{@walk nav ($mod_ $Ent $Nav)
            [$($acc)*
             #[allow(dead_code)]
             #[inline]
             fn $f(self) -> $crate::schema_simpl::schema_simpl!(@navret $Ent; ($($t)+) [$($T)?]) {
                 let q = $crate::engine::Compose {
                     a: self.iq(),
                     b: &$mod_::STORE.get().expect("schema not initialized").$Ent.$f,
                 };
                 $(let q = $crate::engine::Compose {
                     a: q,
                     b: <$T as $crate::engine::EntityKind>::table(),
                 };)?
                 q
             }]
            $($rest)*}
    };
    // nav method return type: the column compose, table-wrapped for FK
    (@navret $Ent:ident; ($($t:tt)+) []) => {
        $crate::engine::Compose<Self::Q,
            &'static $crate::schema_simpl::schema_simpl!(@colty (); $Ent; $($t)+)>
    };
    (@navret $Ent:ident; ($($t:tt)+) [$T:ident]) => {
        $crate::engine::Compose<
            $crate::engine::Compose<Self::Q,
                &'static $crate::schema_simpl::schema_simpl!(@colty (); $Ent; $($t)+)>,
            <$T as $crate::engine::EntityKind>::Table>
    };
    (@nav done ($mod_:ident $Ent:ident $Nav:ident) [$($acc:tt)*]) => {
        /// Generated navigation trait — for anything resolving (via
        /// `IntoQuery`) to a query valued in this entity's ids, one method
        /// per field composing with that field's column (`q.title()` ≡
        /// `q.select(Movie::title)`). Blanket-implemented; same-named methods
        /// on other entities' nav traits don't clash because the resolved
        /// receivers' `R = Id<E>` bounds are disjoint.
        #[allow(dead_code)]
        pub trait $Nav: $crate::engine::IntoQuery + Sized
        where Self::Q: $crate::engine::Query<R = $crate::engine::Id<$Ent>>
        {
            $($acc)*
        }
        impl<T: $crate::engine::IntoQuery + Sized> $Nav for T
        where T::Q: $crate::engine::Query<R = $crate::engine::Id<$Ent>> {}
    };

    // ===== leaf handles: one paren-free ZST per field =====================
    // The nav-trait ident doubles as the per-entity handle namespace (a
    // module inside the schema module), because macro_rules cannot mint
    // fresh idents: `Movie.title`'s handle type is `<mod>::MovieNav::title`.
    // The PUBLIC spellings — `Movie::title` (assoc const, every field) and
    // bare `title` (re-export, `pub` fields) — are generated by @consts.
    (@handlemod $Ent:ident; $Nav:ident; $($body:tt)*) => {
        /// Generated per-field leaf handles — ZSTs resolving (via
        /// `IntoQuery::iq`, one OnceLock fetch at plan construction) to the
        /// `&'static` column relation. Internal: spell them `Entity::field`
        /// or (for `pub` fields) bare `field`.
        #[allow(non_camel_case_types, dead_code)]
        pub mod $Nav {
            $crate::schema_simpl::schema_simpl!{@walk handle ($Ent) [] $($body)*}
        }
    };
    (@handle field ($Ent:ident) $acc:tt $p:tt
      $f:ident; $t1:tt $(($t2:tt))?; then $($rest:tt)*) => {
        #[derive(Clone, Copy)]
        pub struct $f;
        impl $crate::engine::IntoQuery for $f {
            type Q = &'static $crate::schema_simpl::schema_simpl!(@colty (super super); $Ent; $t1 $(($t2))?);
            #[inline]
            fn iq(self) -> Self::Q {
                &super::STORE.get().expect("schema not initialized").$Ent.$f
            }
        }
        $crate::schema_simpl::schema_simpl!{@walk handle ($Ent) [] $($rest)*}
    };
    (@handle done $ctx:tt $acc:tt) => {};

    // ===== public handle spellings: qualified const for every field; =====
    // ===== the captured `pub` marker itself becomes the `pub use` bare ====
    // ===== re-export (the `$( … $p use … )?` expands only for pub fields) =
    (@consts field ($mod_:ident $Ent:ident $Nav:ident) $acc:tt [$($p:tt)?]
      $f:ident; $t1:tt $(($t2:tt))?; then $($rest:tt)*) => {
        impl $Ent {
            #[allow(non_upper_case_globals, dead_code)]
            pub const $f: $mod_::$Nav::$f = $mod_::$Nav::$f;
        }
        $( #[allow(unused_imports)] $p use $mod_::$Nav::$f; )?
        $crate::schema_simpl::schema_simpl!{@walk consts ($mod_ $Ent $Nav) [] $($rest)*}
    };
    (@consts done $ctx:tt $acc:tt) => {};
}

pub use schema_simpl;

// ===== tests — a tiny schema over a generated v2 cache dir ===============

#[cfg(test)]
mod tests {
    use crate::engine::*;
    use crate::format::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    // Three entities exercising every field-type arm: dense str, dense
    // i64, FK (entity ident), Multi<entity>, Multi<str>.
    schema_simpl! { TESTS / TestSchema / test_init:
        Film(film) / FilmNav { pub ftitle: str, pub year: i64, genre: Genre, tags: Multi<Tag> }
        Genre / GenreNav { gname: str, ty: str }
        Tag / TagNav { tag: str, films: Multi<Film> }
    }


    pub(super) fn write_v2(dir: &PathBuf, name: &str, head: [u8; HEADER_LEN], payload: &[u8]) {
        let mut f = File::create(dir.join(format!("{name}.bin"))).unwrap();
        f.write_all(&head).unwrap();
        f.write_all(payload).unwrap();
    }

    pub(super) fn dense_str(vals: &[&str]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        let mut off = 0u32;
        payload.extend_from_slice(&off.to_le_bytes());
        for v in vals {
            off += v.len() as u32;
            payload.extend_from_slice(&off.to_le_bytes());
        }
        for v in vals {
            payload.extend_from_slice(v.as_bytes());
        }
        (header(KIND_DENSE_STR, vals.len() as u64, off as u64), payload)
    }

    pub(super) fn dense_words(vals: &[u64]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        (header(KIND_DENSE_I64, vals.len() as u64, 0), payload)
    }

    fn csr_words(offsets: &[u32], vals: &[u64]) -> ([u8; HEADER_LEN], Vec<u8>) {
        let mut payload = Vec::new();
        for o in offsets {
            payload.extend_from_slice(&o.to_le_bytes());
        }
        payload.resize(align8(HEADER_LEN + payload.len()) - HEADER_LEN, 0);
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        (header(KIND_CSR_WORDS, (offsets.len() - 1) as u64, vals.len() as u64), payload)
    }

    #[test]
    fn schema_macro_loads_types_and_navigates() {
        let dir = std::env::temp_dir()
            .join(format!("prela_schema_simpl_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // films: 0 "Alien" 1979 genre 1 tags {0}, 1 "Blade" 1998 genre 0 tags {0, 1}
        let (h, p) = dense_str(&["Alien", "Blade"]);
        write_v2(&dir, "Film_ftitle", h, &p);
        let (h, p) = dense_words(&[1979, 1998]);
        write_v2(&dir, "Film_year", h, &p);
        let (h, p) = dense_words(&[1, 0]);
        write_v2(&dir, "Film_genre", h, &p);
        let (h, p) = csr_words(&[0, 1, 3], &[0, 0, 1]);
        write_v2(&dir, "Film_tags", h, &p);
        // genres: 0 "drama"/"main", 1 "horror"/"sub"
        let (h, p) = dense_str(&["drama", "horror"]);
        write_v2(&dir, "Genre_gname", h, &p);
        let (h, p) = dense_str(&["main", "sub"]);
        write_v2(&dir, "Genre_ty", h, &p);
        // tags: 0 "cult" films {0, 1}, 1 "noir" films {1}
        let (h, p) = dense_str(&["cult", "noir"]);
        write_v2(&dir, "Tag_tag", h, &p);
        let (h, p) = csr_words(&[0, 2, 3], &[0, 1, 1]);
        write_v2(&dir, "Tag_films", h, &p);

        test_init(&dir);

        // universe size = first column's key count (the universe HANDLE
        // resolves to the `Universe` value via `iq`)
        assert_eq!(film.iq().n, 2);

        // typed composition across three entities, in navigation form:
        // a predicate ROOT is a paren-free handle (qualified `Film::genre`,
        // bare `year` for pub fields); every later hop is a nav method
        // (`.gname()` ≡ `.select(Genre::gname)` via the generated GenreNav).
        let q = film
            .with(Film::genre.gname().eq("horror"))
            .with(year.lt(1990))
            .ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Alien"]);

        // primary-field ELISION: `Film::genre.eq("horror")` auto-navigates to
        // Genre's primary (gname) — identical result to the explicit
        // `.gname().eq(..)` above. Genre is `Primary` (first field `gname:
        // str`); the scalar `year.lt(1990)` is the identity (Field) case.
        let q = film
            .with(Film::genre.eq("horror"))
            .with(year.lt(1990))
            .ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Alien"]);

        // Multi<entity> column + nav through Tag's tag column
        let q = film.with(Film::tags.tag().eq("noir")).ftitle();
        let mut got = Vec::new();
        q.drive(|_, t| got.push(t));
        assert_eq!(got, vec!["Blade"]);

        // same-named nav methods on different entities resolve by the
        // receiver's RESOLVED value type: Tag::films is Film-valued, so
        // `.year()` picks FilmNav; the chain navigates Film → Genre → gname.
        let mut got = Vec::new();
        Tag::films.year().probe(Id::new(0), |y| got.push(y));
        assert_eq!(got, vec![1979, 1998]);
        let mut got = Vec::new();
        Tag::films.genre().gname().probe(Id::new(1), |g| got.push(g));
        assert_eq!(got, vec!["drama"]);

        // field names are filenames verbatim (`ty` → Genre_ty.bin); a
        // handle in leaf (non-chain) position resolves explicitly via `iq`
        let mut got = Vec::new();
        Genre::ty.iq().probe(Id::new(0), |v| got.push(v));
        assert_eq!(got, vec!["main"]);

        // typed ids round-trip the bulk reinterpret: Film_genre words → Id<Genre>
        let mut got = Vec::new();
        Film::genre.iq().probe(Id::<Film>::new(1), |g| got.push(g));
        assert_eq!(got, vec![Id::<Genre>::new(0)]);

        // the generated manifest names every column with its cache kind
        assert_eq!(MANIFEST, &[
            ("Film", "ftitle", KIND_DENSE_STR),
            ("Film", "year", KIND_DENSE_I64),
            ("Film", "genre", KIND_DENSE_I64),
            ("Film", "tags", KIND_CSR_WORDS),
            ("Genre", "gname", KIND_DENSE_STR),
            ("Genre", "ty", KIND_DENSE_STR),
            ("Tag", "tag", KIND_DENSE_STR),
            ("Tag", "films", KIND_CSR_WORDS),
        ]);
    }

}

// A schema with a NON-DENSE (`dict`) entity, in its own module (each `schema!`
// emits a `MANIFEST`, so two can't share a module). `Studio` is addressed by an
// external `id` (100/205/9899 — not dense rows); its first field is that id
// column, from which the `DictTable` is built. `Movie.studio` is a FK STORING
// those external keys, navigated through the table.
#[cfg(test)]
mod dict_tests {
    use crate::engine::*;
    use crate::format::*;
    use super::tests::{write_v2, dense_words, dense_str};

    schema_simpl! { DICTT / DictSchema / dictt_init:
        Movie(movie) / MovieNav { studio: Studio, year: i64 }
        Studio(studio dict) / StudioNav { id: i64, sname: str }
    }

    #[test]
    fn dict_entity_loads_and_navigates() {
        let dir = std::env::temp_dir()
            .join(format!("prela_dict_simpl_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // Studio: EXTERNAL ids 100/205/9899 at dense rows 0/1/2; names. The id
        // column is what the DictTable inverts (external id → row).
        let (h, p) = dense_words(&[100, 205, 9899]);
        write_v2(&dir, "Studio_id", h, &p);
        let (h, p) = dense_str(&["Warner", "A24", "Mubi"]);
        write_v2(&dir, "Studio_sname", h, &p);
        // Movie.studio: FK storing the external KEYS (205, 100) — NOT row ids.
        let (h, p) = dense_words(&[205, 100]);
        write_v2(&dir, "Movie_studio", h, &p);
        let (h, p) = dense_words(&[2008, 1999]);
        write_v2(&dir, "Movie_year", h, &p);

        dictt_init(&dir);

        // movie.studio().sname() — `.studio()` crosses Studio's DictTable (built
        // lazily from `Studio.id`), then `.sname()` reads the column. The FK is
        // a non-dense Key, resolved to a row by the table.
        let q = movie.studio().sname();
        let mut got = Vec::new();
        q.drive(|m, n| got.push((m.idx(), n)));
        got.sort();
        // movie 0 → studio key 205 → row 1 → "A24"; movie 1 → key 100 → row 0 → "Warner"
        assert_eq!(got, vec![(0, "A24"), (1, "Warner")]);

        // the FK column genuinely stores a non-dense Key (not a row Id): the raw
        // handle `Movie::studio` resolves to the Key column, un-followed.
        let mut keys = Vec::new();
        movie.select(Movie::studio).drive(|m, k: Key<Studio>| keys.push((m.idx(), k.0)));
        keys.sort();
        assert_eq!(keys, vec![(0, 205), (1, 100)]);
    }
}
