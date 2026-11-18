Column.Load(key_filter, val_filter) -> (MultiRel, (keys_bloom, vals_bloom)):
    reader = scan(file, cols=[key_col, val_col],
                  filters=[key_col ∈ key_filter, val_col ∈ val_filter, pushed predicates])
    rel = MultiRel(); kb = Bloom(); vb = Bloom()
    for batch in reader:
        rel.add(batch[key_col], batch[val_col])
        kb.insert(batch[key_col]);  vb.insert(batch[val_col])
    return rel, (kb, vb)

Compose{a, b}.Load(key_filter, val_filter):          # a: D→M, b: M→R, join a.2 = b.1
    â, (ak, av) = a.Load(key_filter, All)            # top-down: a first
    b̂, (bk, bv) = b.Load(av, val_filter)             # b's KEYS ⊆ a's VALUES
    return Compose{â, b̂}, (ak, bv)                   # my D-side is a's, my R-side is b's

Restrict{a, b}.Load(key_filter, val_filter):         # a: D→M, b: M→X, keep a's pairs with value ∈ dom b
    b̂, (bk, _)  = b.Load(All, All)                   # the with-argument FIRST
    â, (ak, av) = a.Load(key_filter, val_filter AND bk)   # b's KEYS filter a's VALUES
    return Restrict{â, b̂}, (ak, av)                  # result is a's columns


Filter{a, p}.Load(key_filter, val_filter):
    â, blooms = a.Load(key_filter, val_filter AND p)  # p pushed into the scan if it's a PredicateExpr
    return Filter{â, p}, blooms


     



pub trait ParquetType: Copy + 'static {
    fn accepts(t: &DataType) -> bool;                 // startup check against the footer
    fn from_array(a: &ArrayRef, out: &mut Vec<Self>); // load: Arrow column → Rust values
}
// i64   accepts Int32 | Int64   (o_shippriority is int32, l_linenumber int64; widen both)
// f64   accepts Float64
// Str   accepts Utf8            (strings must be copied into leaked/owned storage, see below)
// Id<E> accepts Int32 | Int64   (keys: c_nationkey is int32, c_custkey int64; widen to i64,
//                                 as keys_i64 in join_tree.rs does)

pub struct Catalog { dir: PathBuf, footers: HashMap<String, Arc<Schema>> }

impl Catalog {
    pub fn open(dir: &str) -> Self;

    /// key_col → val_col in one file, checked against its footer.
    pub fn col<E, T: ParquetType>(&mut self, file: &str, key_col: &str, val_col: &str) -> ColumnRef<E, T>;

    /// The identity relation over the file's key: key_col → key_col.
    pub fn key<E>(&mut self, file: &str, key_col: &str) -> ColumnRef<E, Id<E>> {
        self.col(file, key_col, key_col)
    }
}


#[derive(IntoQuery)]
pub struct Nation {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub region: ColumnRef<Self, Id<Region>>,
    pub customers: ColumnRef<Self, Id<Customer>>,   // reverse FK, see below
}

#[derive(IntoQuery)]
pub struct Customer {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub nation: ColumnRef<Self, Id<Nation>>,
    pub acctbal: ColumnRef<Self, f64>,
    pub mktsegment: ColumnRef<Self, Str>,
}

pub fn tpch(c: &mut Catalog) -> TpchFiles {
    TpchFiles {
        nation: Nation {
            id:        c.key("nation", "n_nationkey"),
            name:      c.col("nation", "n_nationkey", "n_name"),
            region:    c.col("nation", "n_nationkey", "n_regionkey"),
            customers: c.col("customer", "c_nationkey", "c_custkey"),   // reverse FK
        },
        customer: Customer {
            id:         c.key("customer", "c_custkey"),
            name:       c.col("customer", "c_custkey", "c_name"),
            nation:     c.col("customer", "c_custkey", "c_nationkey"),
            acctbal:    c.col("customer", "c_custkey", "c_acctbal"),
            mktsegment: c.col("customer", "c_custkey", "c_mktsegment"),
        },
        // region, supplier, part, partsupp, orders, lineitem follow the same pattern
    }
}


---------
Switching to a mutation than a returning design. 

Currently, what the .load() does it effectively produce an in-memory AST of the query which can then be .drive()n right after and one that works for with prela's engine. 

I now want to switch to a design where rather than returning this kind of AST, I want .load() to simply add/mutate the columnRefs objects with the actual in-memory multi-rel that it corresponds to after loading. So to enact this modification, we should, in addition to actually returning a Node (along with filters), it should ALSO populate a ColumnRef.sink , so all we have to do is actually give columnref a .sink attribute where it will be a MultiRel type and it will be the same base-columns. 

After which, I want to create a method which will return an-in memory db object which will just go and collect the sinks 

For example this code comes from queries.rs: this is working with an *in-memory** db object 
pub fn entries(db: &'static Job) -> Vec<Entry> {
    let Movie { title, kind, production_year, episode_nr, keyword, aka,
                company, cast, info, data, complete_cast, link, linked_by, .. } = &db.movie;
    let Cast { person, role, note: cast_note, character, .. } = &db.cast;
    let Person { name: person_name, gender, alias, bio, name_pcode_cf, .. } = &db.person;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
    let PersonInfo { info: personinfo_info, ty: personinfo_ty, note: personinfo_note, .. } = &db.person_info;
    let MovieLink { target, ty: link_ty, .. } = &db.movie_link;
    let CompleteCast { status, subject, .. } = &db.complete_cast;


    This cide comes from trialLoader.rs and is working with a loaded db object (prior to any data actually popoulated)

     let mut catalog = Catalog::open("../data/tpch/parquet");
    // Leaked to `&'static` (like src/main.rs does with its db), so the `&ColumnRef`s
    // below borrow for the whole program, as `Filter`'s `F: 'static` requires.
    let db: &'static TpchFiles = Box::leak(Box::new(tpch(&mut catalog)));
    let Region { name: region_name, .. } = &db.region;
    let Nation { name: nation_name, region, customers, .. } = &db.nation;
    let Customer { name: customer_name, nation, acctbal, mktsegment, .. } = &db.customer;
    let Part { name: part_name, retailprice, supplies, .. } = &db.part;
    let Partsupp { supplycost, part, .. } = &db.partsupp;
    let _ = (region_name, region, customer_name, nation, mktsegment, nation_name, customers, acctbal);
    let _ = (retailprice, part);

    Notice how close this syntax is. I would like to just have 
    db_memory = db.to_memory() # some method which after calling load it will just return the loaded columns which will then allow us to write new query plans that are different but with the loaded relation. The idea is that an optimal query loading plan and an in-memory execution plan might look different, so we want to have this customsiablity where we are free to use our populated/loaded relations as we please. (This applies still to a single query because different queries on loaded data will be diff -- but this is still an important distinction and thats why I need this mutation method.)