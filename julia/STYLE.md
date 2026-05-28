# Prela query parens style guide

Julia's actual operator precedence for our DSL (high → low):

```
∧, ∨     ←  multiplication level, VERY tight
:        ←  range level
==, <, >, ≤, ≥, ~, ≁, in, !=  ←  comparison level
→, ←, ⩘  ←  arrow level, loosest
```

Verified via:
- `a == b ∧ c`  parses as  `a == (b ∧ c)`        →  `∧` tighter than `==`
- `a : b == c`  parses as  `(a : b) == c`        →  `:` tighter than `==`
- `a : b ∧ c`   parses as  `a : (b ∧ c)`         →  `∧` tighter than `:`
- `a → b : c`   parses as  `a → (b : c)`         →  `:` tighter than `→`
- `a : b → c`   parses as  `(a : b) → c`         →  `:` tighter than `→`

## Rules — when parens are NEEDED

1. **Around every comparison predicate inside an `∧`/`∨` chain.**
   ```julia
   (Info.type == "countries") ∧ (production_year > 2008)
   ```
   Without the parens, `Info.type == "countries" ∧ (production_year > 2008)` would parse as
   `Info.type == ("countries" ∧ (production_year > 2008))` — the `∧` binds tighter than `==`,
   so the `"countries"` becomes one operand of `∧`, triggering `askeys(::String)`.

2. **Around each conjunct in an `∧` chain** when the conjunct itself contains a navigation
   (`→`) or filter (`:`), since `→` and `:` are looser than `∧`:
   ```julia
   (cast → person → Person.name) ∧ (kind == "movie")
   ```

3. **The outer query-block paren** — `(movie ... → title)` — so Julia treats the
   multi-line expression as a single Julia expression.

4. **Tuples and function args** — `("Germany", "German")`, `in (49, 14, …)`, etc.

5. **Disjunctions inside conjunctions** (note: `∨` is at multiplication level too, so
   the disjunction itself doesn't need a wrap — only its operands do):
   ```julia
   ((cond_a) ∨ (cond_b)) ∧ (cond_c)
   ```
   The outer `((...) ∨ (...))` paren keeps the disjunction together as one conjunct.

## Rules — when parens are NOT needed

1. **Around nested `:`/`→`**. `(a → b) : c` and `a → b : c` (which Julia parses as
   `a → (b : c)`) emit the **same set of pairs** under CPS. Compose+Filter and
   Filter+Compose differ in node-tree shape but collapse to the same drive sequence —
   both walk the same path through the relations and apply the same predicate. Same
   story for `(a → b) → c` vs `a → b → c` (Compose is associative through drive).
   So write whichever spelling reads better.

2. **Around an `∧`/`∨` chain itself** — `∧` is at multiplication level, so it doesn't need
   wrapping. The chain itself stands alone:
   ```julia
   # BAD (extra wrap):
   (info → ((Info.type == "countries") ∧ (Info.info == "Germany")))
   # GOOD:
   (info → (Info.type == "countries") ∧ (Info.info == "Germany"))
   ```
   The `→` arg is `(c1) ∧ (c2)`; `∧` is tighter than `→`, so `→` correctly takes the whole
   conjunction as its rhs.

3. **Around a single navigation when it's already the rhs of an arrow**:
   ```julia
   # BAD:
   info → (Info.type == "release dates") → (Info.info)
   # GOOD:
   info → (Info.type == "release dates") → Info.info
   ```
   The final `Info.info` doesn't need parens.

4. **Around the rhs of `:` when it's a binary expression that doesn't conflict**:
   The `:` rhs binds tightly with `∧` so the `∧` chain after `:` is automatically grouped.

## Examples

### Clean

```julia
(movie
    : (keyword ~ r"sequel") ∧
      (info → (Info.info == "Bulgaria")) ∧
      (production_year > 2010)
    → title)
```

### Cleaned q22a (target)

```julia
(movie
    : (info → (Info.type == "countries")
            ∧ (Info.info in ("Germany", "German", "USA", "American"))) ∧
      (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
      (production_year > 2008) ∧
      (kind in ("movie", "episode"))
    → title
    × (data : (Data.data < "7.0") ∧ (Data.type == "rating") → Data.data)
    × (company : (Company.note ≁ r"\(USA\)")
               ∧ (Company.note ~ r"\(200.*\)")
               ∧ (Company.country != "[us]")
               ∧ (Company.type == "production companies") → Company.name))
```

Notes:
- The `(info → (cond1) ∧ (cond2))` form has NO extra wrap around the `∧` — `∧` binds tightly enough.
- Each comparison conjunct keeps its own parens: `(Info.type == "countries")`, `(Info.info in (...))`.
- The outer `(info → ...)` paren is the conjunct boundary in the surrounding `∧` chain.
