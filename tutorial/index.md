---
title:  'Prela Tutorial'
...

> [!TIP]
> Changes made in any cell will be reflected when a later cell runs.

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
  result = []
  for x, yr in r:
    for ys, z in s:
      if yr == ys:
        result.append((x, z))
  return result
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

It's basically `SELECT r.x, s.z FROM r, s WHERE r.y = s.y`

For example: 

```python
print(movie.select(title))
```
<run-snip lang="python" session="tutorial"></run-snip>

`.where` is restriction:

```python
def where(r, s):
  keys = { y for y, _ in s }
  return [ (x, y) for x, y in r if y in keys ]
```
<run-snip lang="python" session="tutorial" hide-run></run-snip>

For example:

```python
print(movie.where(title))
```
<run-snip lang="python" session="tutorial"></run-snip>

<script type="module" src="https://unpkg.com/@remywang/snip@0/snip.js"></script>

Try removing `(2, "Tron")` from `title` and re-run the cell above.
