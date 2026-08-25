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

q = movie.where(company.s(country).eq("[us]")).select(title & year.eq(1942))
print(q)
