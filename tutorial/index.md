---
title:  'Prela Tutorial'
...

[Prela](https://prela-lang.org) is a new query language being developed
 at the [RePL lab](https://remy.wang) at UCLA.
The language is quite different from SQL, but its key ideas
 are very simple and (we think) elegant.
In this short tutorial, we will build a toy version of Prela
 in Python to understand its core principles.
By the end of this tutorial, you will understand how the following query works:

```python
(movie.where(company.s(country).eq("[us]")
             & keyword.eq("character-name-in-title"))
      .select(title & cast.s(person).s(alias).s(text)))
```

You can probably already guess what it's doing: the query finds every movie
 produced by an American company and has a character name in its title,
 and outputs the title along with the alias for each cast member.
Note that the [equivalent query in SQL](https://github.com/gregrahn/join-order-benchmark/blob/master/16b.sql) spans over 20 lines.

> [!NOTE]
> This tutorial uses [snip](https://remy.wang/snip/) to connect code cells into a notebook-like
> environment,[^1] so you can edit the code in one cell and see it reflected in a later cell!

[^1]: Different from e.g. Jupyter, snip always executes from the beginning from scratch to avoid corrupted state.

<script id="rel-class" type="text/plain">
class Rel:
    def __init__(self, pairs: list):
        self.pairs = pairs

    def select(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(select(r, s))

    def s(self, other):
        return self.select(other)

    def where(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(where(r, s))

    def __call__(self, x):
        return call(self.pairs, x)

    def eq(self, v):
        return Rel(eq(self.pairs, v))

    def __and__(self, other):
        return Rel(and_(self.pairs, other.pairs))

    def __repr__(self):
        if not self.pairs:
            return "(empty)"
        return "\n".join(f"{x}, {y}" for x, y in self.pairs)

    def __eq__(self, other):
        return isinstance(other, Rel) and set(self.pairs) == set(other.pairs)
</script>

The first special thing about Prela is that there are only *binary* relations,
 i.e., tables with two columns.
That may sound very limiting at first, but it's easy to "binarize" a wide table
 with multiple columns.
Suppose we have a table of movies:

| ID  | title         | year |
|-----|---------------|------|
| 646 | The Godfather | 1972 |
| 478 | Seven Samurai | 1954 |
| 583 | Casablanca    | 1942 |

We can "shred" the 3-column table into 3 binary relations, each mapping the row number to the column value:

```python
movie = Rel([(646, 0),
             (478, 1),
             (583, 2)])

title = Rel([(0, "The Godfather"),
             (1, "Seven Samurai"),
             (2, "Casablanca")])

year  = Rel([(0, 1972),
             (1, 1954),
             (2, 1942)])
```
<run-snip lang="python" session="tutorial" setup="rel-class" hide-run></run-snip>

The `movie`, `title`, and `year` relations above represent the `ID`, `title`, and `year`
  columns of the original table, respectively.
Note how the row number comes first in `title` and `year`, but second in `movie` (which is also not called `ID`).
The reason for this will become clear later.

The motivation for focusing on binary relations is that they generalize functions.
Functions are very powerful and are the building blocks of programs.
This is because functions *compose*, and the composition of binary relations turns out
 to generalize function composition.

That is all very abstract, so let's go back to our examples.
First, a binary relation is just a function that can "return" multiple results for every input.
Because our relations are all finite, we can implement "calling" a relation
 by building a dictionary, then lookig up the argument from the dict:

```python
def call(r, x):
  d = {}
  for k, v in r:
    d.setdefault(k, []).append(v)
  return d.get(x, [])
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

With a little bit of Python magic,[^2] we can call the relations like so: 

[^2]: We're just forwarding `Rel.__call__` to `call()`. 

```python
print(movie(646), title(0), year(0))
```
<run-snip lang="python" session="tutorial"></run-snip>

The results come back wrapped in lists because in general,
 a relation can map an input to multiple outputs.

We're now ready to introduce the first and most important
 operator in Prela, the relational composition.
Functional composition works by applying one function first, then applying
 the other one to the output.
Relation composition is similar, and just applies the second relation
 to every output of the first.
Relation composition is spelt as `select` in Prela:

```python
def select(r, s):
  return [ (x, z) for x, y in r for z in call(s, y) ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

In other words, we join `r` and `s` on the second column of `r`
 and the first column of `s`,
 then keep the first column of `r` and second column of `s`;
 in SQL: 

```sql
SELECT r.x, s.z 
  FROM r, s 
 WHERE r.y = s.y
```

Using our example, the query below composes `movie` with `title`
 to get a relation mapping each movie ID to its title:[^3]

[^3]: The `.select` method syntax uses the same trick of forwarding `Rel.select` to `select()`.

```python
print(movie.select(title))
```
<run-snip lang="python" session="tutorial"></run-snip>

Try changing `title` to `year` and see what you get.
The power of composition really shows when we chain together multiple `.select` calls.
Suppose we add a foreign key column mapping each movie to its production company,
 and another table for movie companies:

::::: {.columns}
::: {.column width="50%"}
| ID  | title | year | company |
|-----|-------|------|---------|
| ... | ...   | ...  | 0       |
| ... | ...   | ...  | 1       |
| ... | ...   | ...  | 2       |

:::
::: {.column width="50%"}
| ID | name         | country |
|----|--------------|---------|
| 0  | Paramount    | [us]    |
| 1  | Toho         | [jp]    |
| 2  | Warner Bros. | [us]    |

:::
:::::

Shredding these the same way gives us four more relations:

```python
company = Rel([(0, 0),
               (1, 1),
               (2, 2)])

id2row  = Rel([(0, 0),
               (1, 1),
               (2, 2)])

name    = Rel([(0, "Paramount"),
               (1, "Toho"),
               (2, "Warner Bros.")])

country = Rel([(0, "[us]"),
               (1, "[jp]"),
               (2, "[us]")])
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

Then, we can find the country of a movie's production company by a chain of `.select` calls,
 where we abbreviate with `.s`:

```python
print(movie.s(company).s(id2row).s(country))
```
<run-snip lang="python" session="tutorial"></run-snip>

Because joining via a foreign key almost always require "resolving" an ID to a row,
 Prela automatically inserts that step so one can write the following,[^4]
 which reads just like "a movie's company's country"!

[^4]: Here we cheat by using the row number as company IDs.

```python
print(movie.s(company).s(country))
```
<run-snip lang="python" session="tutorial"></run-snip>

Our company IDs happen to be their own row numbers, which makes `id2row` the
 identity relation --- try deleting `.s(id2row)` from the cell above and you
 will get the same answer.
This is also what happened in `cast.s(person).s(alias).s(text)` 
 on the last line of the snippet in the beginning of the tutorial.


So far every query has returned a single column of values.
To select *multiple* attributes, we introduce the `&` operator.

Where `.select` matches the second column of `r` against the first column of
 `s`, `&` joins `r` and `s` on the first column of *both*, then pairs up their
 second columns:

```python
def and_(r, s):
  return [ (x, (y, z)) for x, y in r for z in call(s, x) ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

So `title & year` maps every movie row to both of its attributes at once:

```python
print(title & year)
```
<run-snip lang="python" session="tutorial"></run-snip>

Note that the result is still a binary relation, `&` simply nests the values
 into a tuple.
That means we can keep composing it like any other relation, which is how a
 query returns more than one column:

```python
print(movie.select(title & year))
```
<run-snip lang="python" session="tutorial"></run-snip>

Next, we need a way to say *which* rows we want.
The predicate `.eq(v)` filters a relation, keeping only the pairs whose second
 column equals `v`:

```python
def eq(r, v):
  return [ (x, y) for x, y in r if y == v ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

On its own, `.eq` only narrows the relation it is applied to.
The query below still maps movie rows to countries, just no longer all of them:

```python
print(company.s(country).eq("[us]"))
```
<run-snip lang="python" session="tutorial"></run-snip>

Finally, the *restriction* operator `.where` takes a predicate like the one above and
 filters another relation with it.

```python
def where(r, s):
  keys = { k for k, _ in s }
  return [ (x, y) for x, y in r if y in keys ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

Handing our predicate to `.where` turns it into a filter on movies:

```python
american = company.s(country).eq("[us]")
print(movie.where(american))
```
<run-snip lang="python" session="tutorial"></run-snip>

Reading the two lines together gives us "movies where the company's country
 is [us]", which is once again very close to plain English.

One last trick: becuase `&` joins its arguments,
 it doubles as logical conjunction once nested inside a `.where`:

```python
print(movie.where(american & year.eq(1942)))
```
<run-snip lang="python" session="tutorial"></run-snip>

Only Casablanca is American *and* from 1942.
Putting it all together, `.select` then fetches whatever columns we want to
 see for the movies that survived the filter:

```python
print(movie.where(american & year.eq(1942)).select(title & year))
```
<run-snip lang="python" session="tutorial"></run-snip>

We can even push the predicate into the `select` clause
 for a cleaner query:

```python
print(movie.where(american).select(title & year.eq(1942)))
```
<run-snip lang="python" session="tutorial"></run-snip>

And that's pretty much the whole language!
Prela also supports grouping and aggregation, and we are working a full documentation for the language.
As an excercise,[^5] you can try to define the necessary relations so that the
 snippet at the top runs.
If you run into issues, try to break down the query - every subexpression in Prela is a valid query.

[^5]: A solution is hidden *somewhere* on this page ;)

```python
# keyword = ...
# ...

print(movie.where(company.s(country).eq("[us]")
                  & keyword.eq("character-name-in-title"))
          .select(title & cast.s(person).s(alias).s(text)))
```
<run-snip lang="python" session="tutorial"></run-snip>

<!-- Solution: `cast` is deliberately multi-valued, giving The Godfather two
     cast members and therefore two rows of output.

keyword = Rel([(0, "character-name-in-title"),
               (1, "samurai"),
               (2, "wwii")])

cast    = Rel([(0, 0),
               (0, 1),
               (2, 2)])

person  = Rel([(0, 0), (1, 1), (2, 2)])

alias   = Rel([(0, 0), (1, 1), (2, 2)])

text    = Rel([(0, "Brando, Marlon"),
               (1, "Pacino, Al"),
               (2, "Bogart, Humphrey")])

print(movie.where(company.s(country).eq("[us]")
                  & keyword.eq("character-name-in-title"))
           .select(title & cast.s(person).s(alias).s(text)))

which prints:

646, ('The Godfather', 'Brando, Marlon')
646, ('The Godfather', 'Pacino, Al')
-->

<script type="module" src="https://unpkg.com/@remywang/snip@0/snip.js"></script>
