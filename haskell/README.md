# Prela in Haskell

This is a Haskell implementation of [Prela](../README.md), and it works slightly differently. This document will
assume familiarity with Prela's Rust implementation; we'll start with the "lowest level" concepts and build upwards.

The first thing to know is that this version of Prela is structured around *pull streams*, which produce
values as a consumer requests them. This is in contrast to *push streams*, where producers loop over their output, "pushing"
it outward to consumers.

The simplest encoding of a pull stream looks like this:

~~~haskell
data Step st k v
= Yield k v st
  | Skip st
  | Done

data Stream k v = forall st src. Stream
  { source       :: src
  , initialState :: src -> st
  , next         :: src -> st -> Step st k v
  }
~~~

`source` is the data used by the traversal (e.g., a column). Whenever a consumer asks the `Stream` to advance by calling its `next`, it will either `Yield` a value (in Prela, a row) or `Skip` without producing anything (which is important for operations like *filtering*). `Done` indicates that there are no more values to produce. `Yield` and `Skip` also produce a new state, `st`, that tells the stream how to resume. When scanning a column, for example, `st` would just be the index of the next row.

In reality, Prela's `Stream` type is more complicated. One efficient way to implement pull streams is to pass *continuations* to the stream representing "what to do" in the `Yield`, `Skip`, and `Done` cases. This avoids building new `Step` values each time the stream is advanced, reducing allocation (and therefore GC pauses down the line). Prela uses this representation:

~~~haskell
data Stream k v = forall st src. Stream
    { source       :: src
    , initialState :: src -> st
    , next         :: forall a. src
      -> st
      -> (k -> v -> st -> a) -- yield
      -> (st -> a)           -- skip
      -> a                   -- done
      -> a
    }
~~~

A `Stream` describes one traversal of a singular data source, but Prela queries need to combine and nest traversals, so we wrap the `Stream` type in a more general `Drive` type:

~~~haskell
data Drive k v
    = Lin (Stream k v)
    | forall k' v'. Bind
        (Drive k' v')
        (k' -> v' -> Drive k v)
    | Cat
        (Drive k v)
        (Drive k v)
~~~

`Drive` describes all the ways to traverse a `Stream`: `Lin` wraps a basic `Stream`, and `Cat` runs one `Drive` after another. `Bind` says that whenever the first `Drive` yields a row, that row is passed to a function that constructs a new `Drive`. The new `Drive` is traversed, and then the first `Drive` is resumed.

Sometimes, though, we want to select a subset of a relation, either getting the values associated with a given key or checking that such values exist. For that, we have `Probe`:

~~~haskell
data Probe k v = Probe
  { at     :: k -> Drive () v
  , exists :: k -> (v -> Bool) -> Bool
  }
~~~

`Probe` provides two facilities. `at` returns a `Drive` over such values (returning the unit type for `k`, since it is already known), while `exists` checks whether any of them satisfy a predicate, stopping as soon as the predicate is satisfied.

`Drive` and `Probe` are two different ways of accessing a relation, and many operations on relations work for both: filtering a `Drive` produces another `Drive`; filtering a `Probe` produces another `Probe`. To describe all the operations that work for both types, we use the *type class* `Mode`:

~~~haskell
class Mode q where
    filt
      :: (v -> Bool)
      -> q k v
      -> q k v

    compose
      :: q k middle
      -> Probe middle v
      -> q k v

    ...
~~~

`q` is any representation with a `Mode` instance: either a `Drive` or `Probe`, which each implement their version of the combinators in `Mode`, or a `Relation`, which doesn't impose an access mode yet:

~~~haskell
newtype Relation k v = Relation
  { use :: forall q. Mode q => q k v
  }
~~~

`Relation`s eventually need to be used as either `Drive` or `Probe`. To do this, we implement the additional type classes `Drivable` and `Probeable`:

~~~haskell
class Drivable q where
  drive :: q d r -> Drive d r

instance Drivable Drive where
  drive = id
instance Drivable Relation where
  drive = use

class Probeable q where
  probe :: q d r -> Probe d r   
  
  --- (... same instances for Probeable ...)
~~~

`Drive` is already `Drivable`, and `Probe` is already `Probeable`. When we `drive` a `Relation`, we ask it for its `Drive` form. Now we can write operations, or *combinators*, using the constructs we've defined:

~~~haskell
union :: (Drivable a, Drivable b) 
  => a k v
  -> b k v
  -> Drive k v
union l r = Cat (drive l) (drive r)
~~~

The type of `union` prescibes that both its arguments are `Drivable`, either `Relation`s or `Drive`s, and that the key and value types match. It returns a `Drive`. The implementation calls `drive` on both its left and right arguments to ensure that they are in the `Drive` form, then combines them using `Cat`. Similarly for `collect`:

~~~haskell
collect :: Drivable q => q k v -> [(k, v)]
collect r = collectD (drive r)

collectD :: Drive k v -> [(k, v)]
collectD (Lin s) = collectS s
collectD (Cat l r) = 
  collectD l ++ collectD r
--- (... same for Bind ...)

collectS :: Stream k v -> [(k, v)]
collectS (Stream src init next) =
  loop (init src)
  where
    loop st =
      next src st y s d

    y k v st' =
      (k, v) : loop st'

    s st' = loop st'

    d = []
~~~

`collect` has some additional complexity because it *consumes* a `Drive` and assembles its rows into a list, whereas `union` simply transforms two existing `Drive`s into one. `collect` recursively interprets the `Drive` until it reaches its `Lin` constructors, whereupon it repeatedly advances each `Stream` until that `Stream` reports `Done`.

Let's apply these concepts in a concrete query. Suppose we declare a movie database in which each movie
can refer to many cast and crew members:

~~~haskell
Schema "Database"
  [ entity "Movie" "movies"
      [ many "cast" (ref "Person")
      , many "crew" (ref "Person")
      ]
  , entity "Person" "people"
      [ one "name" str ]
  ]
~~~

`many` means that a movie may be associated with any number of people. From
this declaration, Prela generates the `Database` type and the `cast` and `crew`
accessors, whose types look like this:

~~~haskell
cast :: Database -> Relation Movie Person
crew :: Database -> Relation Movie Person
~~~

Finally, we can write a query:

~~~haskell
credits :: Database -> [(Movie, Person)]
credits db =
  collect $
    union
      (cast db)
      (crew db)
~~~

Unfortunately, I've kind of lied to you about how Prela works.