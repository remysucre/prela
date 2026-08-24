---
title:  'Prela Tutorial'
...

Hello! If you've been looking to learn more about
[Prela](https://github.com/remysucre/prela), a new query language
developed by the [Wang Lab](https://remy.wang/) at UCLA, you've come to
the right place! Prela is a clean, efficient language that outdoes many
query language competitors.

In this interactive tutorial, we'll go step by step through Prela's
operators, explaining the idea behind each one and showing exactly what
it computes. Prela queries are Rust code, so they need to be compiled
ahead of time and can't be run live in a page like this one. Instead, for
each operator below we've paired a small, honest reimplementation in
plain Python, code you can read, run, and edit right here in your
browser, to make each operator's semantics concrete instead of abstract.

We're aiming to explain even the technical parts in as simple and
easy-to-read way as possible. If something feels like a lot, that's on us
to fix, not on you.

Before you start, it's helpful to have some familiarity with:

- Python
- SQL

> [!NOTE]
> This tutorial is designed to act like a Jupyter notebook, changes in a
> cell may affect following cells!

Every operator below is really two things: a plain function that does the
actual computation on lists of `(key, value)` pairs, and a small `Rel`
class that wraps those pairs so you can chain operators with dot-syntax,
`r.select(s)` instead of `select(r, s)`. Here is a portion of the `Rel`
class, so you can get a feel of how the functions we'll be looking at in
the rest of the tutorial fit logically together:

```python
class Rel:
    def __init__(self, pairs: list):
        self.pairs = pairs

    def select(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(select(r, s))

    def where(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(where(r, s))

    def __repr__(self):
        return f"Rel({self.pairs!r})"

    def __eq__(self, other):
        return isinstance(other, Rel) and set(self.pairs) == set(other.pairs)
```

Each method unwraps two `Rel`s down to their raw pair-lists
(`self.pairs`, `other.pairs`), calls the real function (the one you'll
actually see explained below), and wraps the result back into a `Rel` so
it can be chained further.

> [!NOTE]
> `r.select(s)` and `select(r, s)` end up doing the same work, but
> they're not the same function. When you call a method with
> `object.method(arg)`, Python automatically passes `object` in as the
> method's first parameter (conventionally called `self`), you never
> write it yourself. So, `movie.select(title)` really means "call
> `.select()` with `self=movie` and `other=title`", it just looks like
> one argument because the first one is supplied by the dot.

<script id="rel-class" type="text/plain">
class Rel:
    def __init__(self, pairs: list):
        self.pairs = pairs

    def select(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(select(r, s))

    def where(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(where(r, s))

    def eq(self, v):
        return Rel(eq(self.pairs, v))

    def ne(self, v):
        return Rel(ne(self.pairs, v))

    def gt(self, v):
        return Rel(gt(self.pairs, v))

    def lt(self, v):
        return Rel(lt(self.pairs, v))

    def ge(self, v):
        return Rel(ge(self.pairs, v))

    def le(self, v):
        return Rel(le(self.pairs, v))

    def between(self, lo, hi):
        return Rel(between(self.pairs, lo, hi))

    def __repr__(self):
        return f"Rel({self.pairs!r})"

    def __eq__(self, other):
        return isinstance(other, Rel) and set(self.pairs) == set(other.pairs)
</script>


Let's load some data: 

```python
movie = Rel([(0, 0),
             (1, 1),
             (2, 2)])
title = Rel([(0, "Jaws"),
             (1, "Alien"),
             (2, "Tron")])
```
<run-snip lang="python" session="tutorial" setup="rel-class" hide-run></run-snip>

`.select` is relation composition. It's Prela's unified equivalent of
SQL's projection and join, since a foreign key is just another relation
to compose with:

```python
def select(r, s):
  result = []
  for x, yr in r:
    for ys, z in s:
      if yr == ys:
        result.append((x, z))
  return result
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

This would look like `SELECT r.x, s.z FROM r, s WHERE r.y = s.y` in SQL.
However, unlike SQL, which splits "read a column" (`SELECT`) and "walk a
foreign key" (`JOIN ... ON`) into two different syntaxes, Prela treats
both as the same operation: chaining two `.select()` calls does the work
of a `JOIN ... ON` plus a `SELECT`.

For example: 

```python
print(movie.select(title))
```
<run-snip lang="python" session="tutorial"></run-snip>

Moving on: the method you'll likely be using most often is the `.where()`
method (`.with()` in Prela). `.where()` is the filtering method, and is a
great example showing Prela's readability. Imagine this query:
*movie.where(title has an "o")* (not real syntax, just the idea). In the
English language, this can be written:

Find all movies with an "o" in their title.

Reading the code is essentially reading English! Here's how it works:

```python
def where(r, s):
  keys = { y for y, _ in s }
  return [ (x, y) for x, y in r if y in keys ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

In SQL: `SELECT r.x, r.y FROM r WHERE r.y IN (SELECT x FROM s)`. Note that
`.where()` only restricts which rows of `r` you keep, and doesn't pull in
any of `s`'s own columns, that's what `.select` is for. The two are often
chained together: narrow down with `.where()`, then `.select` across to
fetch a related column.

For example, keep only the titles that contain the letter "o", once
you've got the set of titles you're checking against, the actual
restriction is just one line:

```python
has_o = Rel([("Tron", 1)])
print(title.where(has_o))
```
<run-snip lang="python" session="tutorial"></run-snip>

We'll start with the most basic comparators. `.eq()`, short for equals,
and `.ne()`, short for not equals, test for equivalency, narrowing a
relation down to just the pairs whose value matches (or doesn't match)
whatever you pass in:

```python
def eq(r, v):
  return [ (x, y) for x, y in r if y == v ]

def ne(r, v):
  return [ (x, y) for x, y in r if y != v ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

This would look like `title = 'Tron'` in SQL. On its own, `.eq()` just
narrows the relation, remember `.where()`? That's how we turn a narrowed
relation into WHERE-clause behavior: `.eq()` picks out the row we care
about, and `.where()` uses that to filter which rows of some other
relation we keep.

```python
print(title.eq("Tron"))
```
<run-snip lang="python" session="tutorial"></run-snip>

And chained together with `.where()`, keeping only the movie whose title
is exactly "Tron":

```python
print(movie.where(title.eq("Tron")))
```
<run-snip lang="python" session="tutorial"></run-snip>

`.ne()` is `.eq()`'s counterpart, same idea, opposite direction:

```python
print(title.ne("Tron"))
```
<run-snip lang="python" session="tutorial"></run-snip>

Next up are the relational comparators: `.gt()`, `.lt()`, `.ge()`,
`.le()`. Yup, you guessed it, greater than, less than, greater than or
equal to, less than or equal to. We're comparing every value in the
relation against the argument:

```python
def gt(r, v):
  return [ (x, y) for x, y in r if y > v ]

def lt(r, v):
  return [ (x, y) for x, y in r if y < v ]

def ge(r, v):
  return [ (x, y) for x, y in r if y >= v ]

def le(r, v):
  return [ (x, y) for x, y in r if y <= v ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

`movie`'s values are already plain numbers, so we can use it directly,
no text column needed this time. `movie.gt(0)` would look like
`SELECT * FROM movie WHERE movie.y > 0` in SQL:

```python
print(movie.gt(0))
```
<run-snip lang="python" session="tutorial"></run-snip>

`.lt()`, `.ge()`, and `.le()` all work exactly the same way, just with
the comparison flipped:

```python
print(movie.lt(2))
print(movie.ge(1))
print(movie.le(1))
```
<run-snip lang="python" session="tutorial"></run-snip>

`.during(lo, hi)` checks a half-open range: `lo <= x < hi`, inclusive of
the start, exclusive of the end. That's exactly the shape you want for
date buckets: "Q3 1993" is `date.during(19930701, 19931001)`, everything
from July 1st up to, but not including, October 1st.

> [!NOTE]
> This particular example needs a date-valued column, which our toy
> dataset doesn't have, so it's shown for reference only and isn't a
> live cell below.

```python
def during(r, lo, hi):
  return [ (x, y) for x, y in r if lo <= y < hi ]
```

`.between(lo, hi)` checks a closed range: `lo <= x <= hi`, inclusive on
both ends, matching SQL's `BETWEEN` exactly. Unlike `.during()` above,
this one works fine on our own data, since `movie`'s values are perfectly
good numbers to test a closed range against:

```python
def between(r, lo, hi):
  return [ (x, y) for x, y in r if lo <= y <= hi ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

In SQL: `SELECT * FROM movie WHERE movie.y BETWEEN 0 AND 1`.

```python
print(movie.between(0, 1))
```
<run-snip lang="python" session="tutorial"></run-snip>

This is especially worth contrasting explicitly since they look almost
identical: `.during(lo, hi)` excludes `hi`, `.between(lo, hi)` includes
it. In other words: `.during()` is built for "up to the start of the next
period," `.between()` is built for "up to and including this exact
value."

<script type="module" src="https://unpkg.com/@remywang/snip@0/snip.js"></script>

Try removing `(2, "Tron")` from `title` and re-run the cell above.
