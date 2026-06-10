module Prela

# Prela — algebraic-relational queries, staged.
#
# A query is a typed tree of relation combinators (the whole plan lives in the
# type). Running one has three phases, with state and codegen each pinned to
# exactly one place:
#
#   1. build      operators assemble the *logical* plan — pure data, no state.
#   2. prepare    `prepare(engine, q)` lowers it to the *physical* plan: every
#                 node is rewritten for its access mode (driven vs probed), and
#                 every index/cache a node needs is built eagerly, right there,
#                 from its already-prepared children. Physical nodes hold only
#                 concrete, immutable state — no Union{Nothing,…}, no laziness.
#   3. scan       `scan(engine, pq, sink)` drives the physical plan into a
#                 sink. ALL scans — the index builds inside `prepare` and the
#                 final result scan — go through the same engine.
#
# An `Engine` is anything that implements `scan`. Two are provided:
#
#   Interp()  — value-level CPS: `drive`/`probe` closure chains (interp.jl).
#               The executable spec, and the subject of inlining experiments.
#   Staged()  — type-level CPS: a `@generated` walk of the physical plan's
#               type emits one fused flat loop nest (staged.jl). The default.
#
# CPS protocol (value level):
#   drive(q, k)        — call k(x, y) for every pair q produces
#   probe(q, x, k)     — call k(y) for every y related to key x
#   member(s, x)::Bool — domain/membership test
#
# Operators (low→high precedence):
#   →  composition  | ∨ union | ∧ intersection | ==,<,~,…  predicates
#   ⊗/× product (tightest) | :  restriction | -  difference

include("algebra.jl")   # the language: entities, leaves, nodes, surface syntax
include("interp.jl")    # engine: value-level CPS (drive/probe closure chains)
include("plan.jl")      # engines + prepare: mode lowering, eager state builds
include("staged.jl")    # engine: type-level CPS (@generated fused loops)
include("schema.jl")    # @entity / @declare / @expose, sealing

end # module
