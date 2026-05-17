Everything in Prela is a relation of arity at most 2.
Such a relation can be thought of as a (finite) function
 that can return multiple results for the same input.

Prela is typed, and the same name can be used for different relations thanks to type inference.

Let's look at what the JOB schema looks like in Prela:

```
movie: Movie

info: Movie -> Info

Info : {
  val : String,
  type : InfoType,
  note : String  
}
```

Compare with SQL:

```
CREATE TABLE movie_info (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    info_type_id integer NOT NULL,
    info text NOT NULL,
    note text
);
```

There is an abstract `Movie` type instead of the integer ID `movie_id`,
 and we're explicit that `info` returns the `Info` of a `Movie`.
The "struct" `Info` is in fact shorthand for declaring 3 relations:
 `val: Info -> String, type: Info -> InfoType, note: Info -> String`.
This allows us to "access fields" with simple relational composition (introduced formally soon):
 `movie.info.note`.

Sequential composition / join `.`: `r . s` with `r: x -> y, s: y -> z` is the relational composition
 `t: x -> z`.
If `s: y`, then `t: x -> y` (range restriction); if `r: y`, then `t: y -> z` (domain restriction).
If they are both unary, then same as intersection.

Intersection `&`: `r: x -> y & s: x -> z` is the intersections of their keys, i.e. a set over `x`.

Select `:`: same as sequential composition, but requiring lhs to be unary (domain reistriction).

Where `|`: `s | r = r : s`

Predicates are applied to the range of each relation:
 `r < 3` with `r: x -> y` filters `r` by `y < 3`.

Because the fundamental data model of Prela is over unary/binary relations,
 creating "tuples" requires a bit more machinery.

Parallel composition `,`: `r: x -> y, s: x -> z` returns `t: x -> w` where `w` is a fresh entity
 that represents tuples over `y, z`, and creates new relations `r: w -> y`, `s: w -> z`
 such that `(r, s).r` is the same as `r` but with domain restricted to those shared with `s`,
 and similarly `(r, s).s` is the same as `s` with domain restricted to those shared with `r`.
Parallel composition works over multiple arguments, e.g. `(r, s, t)`
 with `r: x -> y, s: x -> z, t: x -> w` returns `x -> u`
 and creates `r: u -> y` s.t. `(r, s, t).r` is `r` with restricted domain,
 and similar for `s, t`. 

Return `!`: `r! = r.val`

Aggregation `min, max, sum, ...`: `agg(r)` where `r: x -> y` groups by `x` and aggregates over `y`.

Here's JOB q22c:

```
movie.(
    info.(type = 'countries' & val in ('Germany', 'German', 'USA', 'American'))
  & keyword in ('murder', 'murder-in-title', 'blood', 'violence')
  : title.(production_year > 2008 & kind in ('movie', 'episode'))!
  , data.(val < '8.5' & type = 'rating')!
  , company.(
       note not like '%(USA)%' &
       note like '%(200%)%' &
       country != '[us]' & 
       type = 'production companies'
    ).name
)
```
