# Staged Haskell query redesign

## The short version

We want staged Haskell queries to read like ordinary Haskell expressions while
remaining recognizably Prela.

This is a change to the **way a plan is represented and written**, not an
invitation to rewrite the plan. If the author writes a restriction followed by
a composition and a fold, compilation must preserve that restriction,
composition, and fold in that order. Prela deliberately gives the author control
over the plan; the Haskell front end must not quietly become an optimizer.

The intended pipeline is:

```text
ordinary typed Haskell plan
        ↓  structural lowering
existing staged Stream/Lookup implementation
        ↓  code generation and fusion
tight runtime loops
```

The middle layer may fuse away iterator plumbing, as it already does, but it may
not invent a different relational or physical plan.

## Why change the current surface?

The existing staged executor is not the main problem. It already generates fast
loops and has useful distinctions between scanning a relation and probing it by
key.

The problem is that query definitions currently expose too much of that
implementation. They contain:

- `Stream`, `Lookup`, and `SMode` types;
- `Q.stream` and `Q.keyed` conversions;
- `Gen` actions and large `do` blocks for runtime materialization;
- `Q.Scalar`, `Q.pair`, and `Q.onPair` for staged scalar expressions; and
- deeply nested `compose` calls for foreign-key navigation.

Those details make the query look like code-generation machinery rather than a
Prela plan. The Rust embedding hides most of the same executor distinctions
behind one compositional query vocabulary.

For example, the important idea in Q4 is simple:

1. find the order IDs of late lineitems;
2. store those IDs in a bitset;
3. restrict the date-filtered orders by that bitset;
4. group by priority; and
5. count each group.

The Haskell source should make those five ideas prominent. It should not make
the reader mentally execute `Gen`, choose `Q.keyed`, and unwrap a materialized
relation before seeing the plan.

## What “staying true to Prela” means

The following are design constraints, not optional implementation details.

### The written query is the plan

Compilation must not automatically:

- reorder joins or products;
- reorder predicates;
- push filters through other operators;
- choose a different join order;
- deduplicate a bag;
- introduce or remove materialization;
- replace a hash fold with a dense fold;
- replace a set with a bitset; or
- merge two textually separate computations as a common subexpression.

Any of those transformations may be useful, but in Prela they belong in an
explicit alternative query written by the programmer.

### Algebra operators lower one-for-one

The new representation needs direct counterparts for the existing algebra:

| Plan operation | Meaning | Existing staged lowering |
|---|---|---|
| `compose a b` | relational composition | `Ops.compose` |
| `restrict a p` | keep values accepted by `p` | `Ops.restrict` |
| `prod a b` | product on a shared domain | `Ops.prod` |
| `diff a b` | remove keys present in `b` | `Ops.diff` |
| `groupBy rows key` | re-key rows by `key` | `Ops.groupBy` |
| `filt predicate a` | scalar value filter | `Ops.filt` |
| `mapValues f a` | scalar value mapping | `Ops.mapv` |

Lowering may select the stream or keyed implementation demanded by the
surrounding operator. That is execution mode selection, not plan optimization.
For example, the right side of `restrict` is naturally lowered to a keyed probe,
preserving the existing fused `probeAny` fast path.

### Physical operations stay visible

A query must explicitly say when it wants:

- a general grouped fold;
- a dense fold with a stated universe;
- a bitset with a stated universe;
- a hash index or other materialized lookup; or
- a scalar computation shared across subsequent work.

The compiler implements the selected operation; it does not select one on the
author's behalf.

### Bag and order behavior stay unchanged

The new representation must preserve multiplicity, traversal order where the
old operator defines one, and the distinction between membership-only union and
enumerable bag concatenation. Similar-looking relational expressions are not
interchangeable when they differ on duplicates or enumeration.

## The three times involved

Staging is easier to reason about when the three phases are kept separate.

### 1. Plan construction

Template Haskell evaluates an ordinary Haskell function which constructs a
typed plan. This is where we want normal `let`, `where`, functions, and static
type checking.

No data is scanned at this time.

### 2. Lowering and code generation

The compiler walks the plan and invokes the existing staged executor
combinators. Materializer nodes introduce generated runtime bindings here.

No query data is scanned at this time either; the result is Haskell code which
will do so later.

### 3. Query execution

The generated function receives the loaded schema and runs the emitted loops.
The plan representation and lowering logic do not exist in this runtime code.

This separation is why a plan data type need not make queries slower. It is
compile-time data, not a runtime interpreter. Performance depends on whether
lowering emits the same loops and materializers as the old query.

## The important sharing problem

This is the unresolved part of the design.

Consider an ordinary, unmaterialized relation used twice:

```haskell
let europeanParts = ...
in prod (compose europeanParts a)
        (compose europeanParts b)
```

Prela may intentionally scan `europeanParts` twice. A normal Haskell `let` names
the plan description; it must not silently add a cache.

Now consider a fold or bitset used twice:

```haskell
let minimumCost = fold ...
in prod (compose rows minimumCost)
        (compose otherRows minimumCost)
```

The materialized fold must be constructed once and probed twice. Emitting the
fold twice changes both the physical plan and its cost.

Therefore the compiler must distinguish:

- reuse of a logical relation description, which does not imply
  materialization; and
- reuse of the result of an explicitly materializing node, which must retain
  its one-build scope.

A naive recursive AST lowerer does not guarantee this. Each traversal of the
same fold subtree can emit a new fold. Conversely, automatic
common-subexpression elimination would be wrong because it could cache ordinary
relations the author meant to scan independently.

We should not settle the public representation until it has a precise answer to
this problem. Possible mechanisms include explicit typed materialization
bindings, stable identities only on materializer nodes, or a scoped plan form
which represents “build once, use this reference below.” The choice should be
judged by semantic clarity first and surface convenience second.

## What the compiler is allowed to do for speed

The following implementation improvements preserve the written plan:

- lower a driven relation to `Stream` and a probed relation to `Lookup`;
- fuse filters, maps, compositions, and membership tests into producer loops;
- use the existing `probeAny` early-exit path;
- inline schema leaves and scalar functions during generation;
- bind explicitly materialized storage once;
- keep strict scalar and tuple components unboxed where the backend already
  does so; and
- retain `-fno-full-laziness` at generated splice sites to prevent allocation
  regressions.

These are implementations of a fixed plan. They do not choose a different
plan.

## Idiomatic physical plans in the current TPC-H suite

The current `TPCH.StagedQueries` module now follows a small set of explicit
planning idioms. They are query-author choices, not compiler rewrites.

### Hoist a selective dimension predicate before a large fact aggregate

Q20 first builds a bitset of forest-named parts, then restricts the 1994
lineitem fold with it. The `(part, supplier)` table consequently contains only
groups the final partsupp predicate can use. At SF1 this reduced Q20 allocation
from about 272 MB to 3.7 MB and warm time from about 222 ms to 41 ms.

Q2 similarly precomputes eligible parts and European suppliers. Its minimum
fold sees only qualifying partsupp rows instead of aggregating all European
offers.

This is not a universal rule. Equivalent hoists tried in Q3 and Q8 were slower
than their existing fact-side short-circuit plans and were removed.

### Use a dense fold when a validated id is already the slot

Q10 and Q15 group by customer and supplier ids respectively. Their extents are
known, so the queries explicitly choose `denseFold` instead of paying for hash
keys and collision probes. Q21 likewise uses order and supplier extents for its
two dense reductions.

### Keep top-k inside the generated plan

`Q.topK` is an explicit bounded materializer. It consumes a stream into a
mutable buffer capped at `k`, orders the retained rows, and returns a stream so
later projections can continue normally.

Q2, Q10, and Q18 rank narrow rows first and attach wide strings only to winners.
Q3 and Q21 also move their SQL limits out of the unstaged renderer. Renderers
now format already ordered, already bounded results instead of collecting and
sorting every candidate.

### Put cheap/selective conjuncts first

Products and restrictions probe left-to-right and short-circuit. Q12 tests its
1994 receipt-date range before ship mode and the two cross-column date
comparisons. This is visible in the written plan and approximately halved its
warm SF1 time.

### Pack bounded state when the alternatives are small

Q21 needs unseen / one supplier / multiple suppliers for both all lines and
late lines. Encoding both state machines into one `Int` halves the order-sized
dense storage. Its query allocation fell from about 194 MB to 98 MB.

Across all 22 queries these retained rewrites reduced post-reference-fix
allocation from about 1.41 GB to 0.98 GB and warm SF1 runtime from about 2.61 s
to 2.16--2.18 s. All queries continue to match their recorded oracles. Detailed
measurements and the earlier leaf diagnosis are in
[`STAGED_FOREIGN_KEY_ALLOCATION.md`](STAGED_FOREIGN_KEY_ALLOCATION.md).

## Proposed development sequence

### 1. Freeze the semantic reference

Keep the existing Rust queries and old staged Haskell TPC-H queries available.
They document both relational meaning and important explicit physical choices.
Do not delete them during the representation experiment.

### 2. Write down the core typed plan vocabulary

Model the Prela operators directly, including their drive/probe capabilities and
bag behavior. Avoid convenience constructs which only make sense after an
optimizer rewrite.

### 3. Solve explicit materializer sharing

Demonstrate both of these in small tests:

- an ordinary relation referenced twice emits two traversals and no cache;
- one explicitly materialized fold or bitset referenced twice emits one build
  and two probes.

This must be solved before migrating large queries.

### 4. Implement structural lowering

Each plan constructor should have an evident lowering equation to the existing
staged operator. There should be no rewrite phase.

### 5. Test representative plans

Use small staged tests for quick iteration:

- a Q6-shaped fused scan for filters, product, composition, and scalar fold;
- a Q4-shaped bitset query for build-once membership;
- a Q21-shaped dense fold for explicit physical state and repeated probes; and
- duplicate-sensitive examples for bag semantics.

### 6. Inspect generated code and allocations

For the representative plans, compare generated structure, runtime allocation,
and timings with the existing staged implementation. The target is the same
physical plan and no measurable runtime regression.

The first allocation audit found and fixed one concrete backend issue: driven
traversal of word-backed foreign-key columns allocated about 32--33 bytes per
successful read because optional values survived across nested producer
continuations. Generated one-valued references now use a direct physical leaf.
See [`STAGED_FOREIGN_KEY_ALLOCATION.md`](STAGED_FOREIGN_KEY_ALLOCATION.md) for
the measurements, implementation, results, and regression tests.

### 7. Migrate queries gradually

Only after the semantic and performance checks pass should TPC-H queries move
to the new representation. A migrated query should remain visibly comparable
to its Rust counterpart and to the old Haskell plan.

The full TPC-H build is too slow for the normal edit loop. Use the focused
staged test suite during development and run the complete oracle suite at
milestones.

## Status of the current prototype

`Prela.PullStaged.Plan` is an experiment, not the settled design. It demonstrates
that pure typed plan nodes can lower into the existing executor and pass small
fold and bitset tests. It does **not** yet provide a satisfactory general answer
for materializer identity and reuse.

Accordingly:

- do not remove the old TPC-H queries yet;
- do not treat the prototype surface as final;
- do not add optimization or rewrite passes; and
- solve and test sharing before expanding the prototype.

The desired outcome is modest but important: queries should look like normal
typed Haskell descriptions of Prela plans, while the generated runtime program
remains the same fast program the author explicitly asked for.
