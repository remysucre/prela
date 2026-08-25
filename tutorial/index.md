---
title:  'Prela in 11 Lines of Code'
...

[Prela](https://prela-lang.org) is a new query language being developed
 at UCLA [RePL](https://remy.wang).
The language is quite different from SQL, but its key ideas
 are very simple and (in our opinion) elegant.
In this short tutorial, we will build a toy version of Prela
 in Python to understand its core principles.
By the end of this tutorial, you will understand how the following query works:

```python
movie.where(company.s(country).eq("[us]") &
            keyword.eq("character-name-in-title"))
    .select(title & cast.s(person).s(alias).s(text))
```

You can probably already guess what it's doing: the query finds every movie
 produced by an American company and has a character name in its title,
 and outputs the title along with the alias for each cast member.
Note that the [equivalent query in SQL](https://github.com/gregrahn/join-order-benchmark/blob/master/16b.sql) spans over 20 lines.

> [!TIP]
> This tutorial uses [snip](https://remy.wang/snip/) to connect code cells into a notebook-like
> environment,[^1] changes made in one cell are reflected in later cells.

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

    def __iter__(self):
        return iter(self.pairs)

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

We can decompose the 3-column table into 3 binary relations, each mapping the row number to the column value:

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
Functions are powerful because they *compose*,
 making them the building blocks of programs.
A function maps every input to a unique output,
 where as a relation can map an input to multiple different outputs.
In a sense, a relation can be viewed as a *nondeterministic* function.

That is all very abstract, so let's go back to our examples.
To keep things simple, we will focus on
 relations mapping every input to exactly one output, 
 i.e., they all happen to be functions.
"Calling" a relation then boils down to turning that relation
 into a dictionary and looking up the value: 

```python
print(dict(movie)[646], dict(title)[0], dict(year)[0])
```
<run-snip lang="python" session="tutorial"></run-snip>

We're now ready to introduce the first and most important
 operator in Prela, the relational composition.
Functional composition works by applying one function first, then applying
 the other one to the output.
Relation composition is similar, and just applies the second relation
 to the output of the first, skipping the inputs `s` has no output for.
Relation composition is spelt as `select` in Prela:

```python
def select(r, s):
  d = dict(s)
  return [ (x, d[y]) for x, y in r if y in d ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

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

Decomposing the same way gives us four more relations:

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

This is also what happened in `cast.s(person).s(alias).s(text)` 
 on the last line of the snippet in the beginning of the tutorial.


So far every query has returned a single column of values.
To select *multiple* attributes, we introduce the `&` operator.

Where `.select` matches the second column of `r` against the first column of
 `s`, `&` joins `r` and `s` on the first column of *both*, then pairs up their
 second columns:

```python
def and_(r, s):
  d = dict(s)
  return [ (x, (y, d[x])) for x, y in r if x in d ]
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
  d = dict(s)
  return [ (x, y) for x, y in r if y in d ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

Handing our predicate to `.where` turns it into a filter on movies:

```python
print(movie.where(company.s(country).eq("[us]")))
```
<run-snip lang="python" session="tutorial"></run-snip>

This reads right off the code: "movies where the company's country is [us]".

The query is getting long, so let's refactor it: 

```python
american = company.s(country).eq("[us]")
print(movie.where(american))
```
<run-snip lang="python" session="tutorial"></run-snip>

Wait, did we just create a [CTE](https://www.postgresql.org/docs/current/queries-with.html)
 with a plain Python variable?
Yes! This is possible because Prela queries are made up of operators,
 and every subexpression is a valid query.

How do we have multiple conditions?
A happy accident is that, 
 becuase `&` joins its arguments,
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
Prela also supports grouping and aggregation,
 and other common operators.
We are working a full documentation for the language,
 so for now you can refer to our [paper](https://arxiv.org/abs/2607.26356) for more details.
As an excercise,[^5] you can try to define the necessary relations so that the
 snippet at the top runs.

[^5]: A solution is hidden *somewhere* on this page ;)

```python
# keyword = ...
# ...

print(movie.where(company.s(country).eq("[us]") &
                  keyword.eq("character-name-in-title"))
          .select(title & cast.s(person).s(alias).s(text)))
```
<run-snip lang="python" session="tutorial"></run-snip>

A self-contained Python program for our toy Prela can be found [here](https://github.com/remysucre/prela/blob/main/tutorial/prela.py).

<!-- Solution:

keyword = Rel([(0, "character-name-in-title"),
               (1, "samurai"),
               (2, "wwii")])

cast    = Rel([(0, 0), (1, 1), (2, 2)])

person  = Rel([(0, 0), (1, 1), (2, 2)])

alias   = Rel([(0, 0), (1, 1), (2, 2)])

text    = Rel([(0, "Brando, Marlon"),
               (1, "Pacino, Al"),
               (2, "Bogart, Humphrey")])

print(movie.where(company.s(country).eq("[us]") &
                  keyword.eq("character-name-in-title"))
           .select(title & cast.s(person).s(alias).s(text)))

which prints:

646, ('The Godfather', 'Brando, Marlon')
-->

<script type="module" src="https://unpkg.com/@remywang/snip@0/snip.js"></script>
