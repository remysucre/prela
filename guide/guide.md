# Prela User Guide

> This guide is a draft — expect gaps as more chapters land. Every snippet
> below is runnable in place: press "Run" to compile and execute it against
> the real `prela` engine, no local setup required.

## Your first query

Every value in Prela is a binary relation. The two simplest ones you'll
build by hand are `VecRel` (a total, one-value-per-id relation) and
`Universe` (the identity relation over `0..n` — "all the ids").

Here, `dept` maps an employee id to a department name, and `name` maps it to
a person's name. The query below restricts the employee universe to ids
whose department is `"eng"`, then navigates each surviving id to its name:

```rust
use prela::engine::*;

fn main() {
    let dept: VecRel<&str> = VecRel::new(vec!["eng", "eng", "sales", "eng", "sales"]);
    let name: VecRel<&str> = VecRel::new(vec!["Ada", "Grace", "Alan", "Barbara", "Linus"]);
    let employees = Universe::new(5);

    // employees : (dept == "eng") → name
    let engineers = employees.with(dept.eq("eng")).select(&name);

    engineers.drive(|id, nm| println!("{id}: {nm}"));
}
```

<codapi-snippet sandbox="prela" editor="basic"></codapi-snippet>

`.with` is restriction (Julia/TAR's `:`) — keep ids whose value satisfies a
predicate. `.select` is composition (`→`) — navigate from one relation's
values into the next relation's domain. `.eq` is a comparison filter. These
three combinators, plus the product `∧`/`.and`, cover most everyday queries;
see the [operator precedence table](../README.md#query-language-operator-precedence)
in the README for how they nest.

<!-- TODO: next chapters — products & `.and`, restriction sets & `.with_in`,
     navigating typed schemas via `schema!`, materialization
     (`.collect::<HashIdx<_,_>>()` / `MatSet`), and a walkthrough of a real
     JOB query. -->
