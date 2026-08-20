---
title:  'Prela Tutorial'
...

<script id="rel-class" type="text/plain">
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
</script>


Let's load some data: 

```python
movie = Rel([(0, 0), (1, 1), (2, 2)])
title = Rel([(0, "Jaws"), (1, "Alien"), (2, "Tron")])
```
<run-snip lang="python" session="tutorial" setup="rel-class" hide-run></run-snip>

`.select` is relation composition:

```python
def select(r, s):
  return [ (x, z) for x, yr in r for ys, z in s if yr == ys ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

For example: 

```python
print(movie.select(title))
```
<run-snip lang="python" session="tutorial"></run-snip>

<script type="module" src="https://unpkg.com/@remywang/snip@0/snip.js"></script>
