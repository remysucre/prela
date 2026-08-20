class Rel:
    def __init__(self, pairs: list):
        self.pairs = pairs

    def select(self, other):
        r = self.pairs
        s = other.pairs
        return Rel(select(r, s))

    def where(self, other):
        return Rel(where(r, s))

    def __repr__(self):
        return f"Rel({self.pairs!r})"

    def __eq__(self, other):
        return isinstance(other, Rel) and set(self.pairs) == set(other.pairs)

movie = Rel([(0, 0), (1, 1), (2, 2)])
title = Rel([(0, "Jaws"), (1, "Alien"), (2, "Tron")])

def select(r, s):
  return [ (x, z) for x, yr in r for ys, z in s if yr == ys ]

print(movie.select(title))
