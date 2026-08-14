# Prela in Haskell

This is a Haskell implementation of [Prela](../README.md), and it works slightly differently. This document will
assume familiarity with Prela's Rust implementation, and focus on the differences between the two implementations.
We'll start with the "lowest level" concepts and build upwards.

The first thing to know is that this version of Prela is structured around *pull streams*, which produce
values as a consumer requests them. This is in contrast to *push streams*, where producers loop over their output, "pushing"
it outward to consumers.

The simplest encoding of a pull stream looks like this:

~~~haskell
data Step st k v
= Yield k v st
| Skip st
| Done

data Producer k v = forall st src. Producer
  { source       :: src
  , initialState :: src -> st
  , next         :: src -> st -> Step st k v
  }
~~~

Whenever a consumer asks the `Producer` to advance, it will either `Yield` a value---in Prela, a row---or `Skip`s without producing one (e.g. when it gets filtered out). `Done` indicates that there are no more values to produce. `Yield` and `Skip` also produce a new state that tells the producer how to resume. When scanning a column, `state` would just be the index of the next row.

In reality, Prela's `Step` type is more complicated. One efficient way to implement pull streams is to pass *continuations* to the producer representing "what to do" in the `Yield`, `Skip`, and `Done` cases. This avoids building new `Step` values each time the producer is advanced. Prela uses this representation:

~~~haskell
data Producer k v = forall st src. Producer
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
