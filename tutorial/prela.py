"""A toy implementation of Prela, the query language of binary relations.

Companion to the tutorial in index.md; see https://prela-lang.org.
Every relation here maps each input to at most one output, i.e. it is a
partial function, which is what lets the operators be plain dict lookups.
"""


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


# ---------------------------------------------------------------- operators

def select(r, s):
  d = dict(s)
  return [ (x, d[y]) for x, y in r if y in d ]

def and_(r, s):
  d = dict(s)
  return [ (x, (y, d[x])) for x, y in r if x in d ]

def eq(r, v):
  return [ (x, y) for x, y in r if y == v ]

def where(r, s):
  d = dict(s)
  return [ (x, y) for x, y in r if y in d ]


# ------------------------------------------------------------------- data

movie = Rel([(646, 0),
             (478, 1),
             (583, 2)])

title = Rel([(0, "The Godfather"),
             (1, "Seven Samurai"),
             (2, "Casablanca")])

year  = Rel([(0, 1972),
             (1, 1954),
             (2, 1942)])

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

american = company.s(country).eq("[us]")


if __name__ == "__main__":
    print(dict(movie)[646], dict(title)[0], dict(year)[0])
    print()
    print(movie.select(title))
    print()
    print(movie.s(company).s(id2row).s(country))
    print()
    print(movie.s(company).s(country))
    print()
    print(title & year)
    print()
    print(movie.select(title & year))
    print()
    print(company.s(country).eq("[us]"))
    print()
    print(movie.where(company.s(country).eq("[us]")))
    print()
    print(movie.where(american))
    print()
    print(movie.where(american & year.eq(1942)))
    print()
    print(movie.where(american & year.eq(1942)).select(title & year))
    print()
    print(movie.where(american).select(title & year.eq(1942)))
