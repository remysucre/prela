# Prela User Guide

## Introduction

Prela is organized around a fundamental principle:

    values are binary relations

A binary relation $R$ is a (multi)set of pairs $(k, v)$.
The first element of a pair is a _key_; the second is a _value_.

In Prela, queries are defined by composing binary relations via _relation combinators_.
Relation combinators are operators that map binary relations to binary relations.
For example, the composition operator $\to$ represents _relation composition_: $R \to S = \{(x, z) : \exists y R(x, y) \text{ and } S(y, z) \}$.

## Your first Prela query

The following Prela code sets up the Prela equivalent of an employee table and queries it for employees in the Algebra department.

```rust
use prela::engine::*;

fn main() {
    // define the data
    let employees = Universe::new(5);
    let name: VecRel<&str> = VecRel::new(vec!["Alan", "Alonzo", "Alfred", "Arend", "Remy"]);
    let dept: VecRel<&str> = VecRel::new(vec!["Computability", "Computability", "Algebra", "Algebra", "Algebra"]);

    // define the query
    let algebraists = employees
                          .with(dept.eq("Algebra"))
                        .select(name);

    // scan the query and print each row
    algebraists.drive(|id, n| println!("{id}: {n}"));
}
```

We start by defining three relations: `employees`, `name`, and `dept`.
`name` and `dept` are `VecRels`: they represent one-to-one maps from IDs to values.
`employees` is the identity map over employee IDs (represented by the `Universe` type).

Next, we define our query with

```rust
employees.with(dept.eq("Algebra")).select(&name)
```

This query restricts `employees` to those whose `dept` is equal to `"Algebra"`, then `select`s their `name`.
Let's examine how each combinator in the query works.
Recall that `dept` represents the following relation:

```
dept =
   {(0, "Computability"),
    (1, "Computability"),
    (2,       "Algebra"),
    (3,       "Algebra"),
    (4,       "Algebra")}
```

The `r.eq(v)` combinator retains those pairs in `r` whose second element is equal to `v`.
Thus,

```
dept.eq("Algebra") =
   {(2, "Algebra"),
    (3, "Algebra"),
    (4, "Algebra")}
```

`r.with(s)` retains those rows of `r` whose keys are also keys of `s`.
Hence,

```
employees.with(dept.eq("Algebra")) =
   {(2, 2),
    (3, 3),
    (4, 4)}
```

Finally, `r.select(s)` represents the _composition_ of `r` with `s`.
A pair `(x, z)` appears in `r.select(s)` iff there is a `y` such that `r(x, y)` and `s(y, z)`:

```
employees.with(dept.eq("Algebra")).select(&name) =
   {(2, Alfred),
    (3,  Arend),
    (4,   Remy)}
```

`algebraists` defines the relation of interest.
To actually do something with it, we `drive` a printing function over it: `r.drive(f)` applies `f` to each row of `r`.
Thus, `algebraists.drive(|id, n| println!("{id}: {n}"));` is equivalent to the SQL query

```sql
SELECT id, name FROM employees WHERE dept = 'Algebra'
```

## A More Complex Query

The combinators introduced thus far cannot define relations over multiple attributes.
To do this, we need a pairing, or product, combinator.
This role is played by `and`.
`r.and(s)` takes the relational product of `r` and `s`: the set of pairs `(x, (y, z))` where `r(x, y)` and `s(x, z)`.

Now, we can print the name and department of algebraists as follows:

```rust
employees
      .with(dept.eq("Algebra"))
    .select(dept.and(name))
     .drive(|_, (d, n)| println!("{n}: {d}"));
```

Next, we might want to count the number of employees per department.
Here is how we can do this in Prela:

```rust
employees
    .group_by(dept)
        .fold(0_i64, |a, _| a + 1)
       .drive(|d, c| println!("dept: {d}; count: {c}"));
```

`r.group_by(s)` composes the relational inverse of `s` with `r`.
That is, a row `(g, v)` occurs in `r.group_by(s)` just in case `(id, g)` is in `s` and `(id, v)` is in `r` for some `id`.
Interestingly, a bare (unaggregated) `group_by` is a well-defined relation in Prela, whereas SQL `GROUP BY` must always be aggregated.

Here, `employees.group_by(dept)` produces the following relation:

```
employees.group_by(dept) =
    {("Computability", 0),
     ("Computability", 1),
     ("Algebra",       2),
     ("Algebra",       3),
     ("Algebra",       4)}
```

To count the number of employees per department, we use `fold`.
`r.fold(base, f)` folds the function `f(acc, val)` into an accumulator initialized to `base`, separately for each key in `r`.
You can think of `fold` as maintaining a hashmap `h` from keys of `r` to accumulators initialized to `base`.
For each row `(x, y)` in `r`, `fold` updates the value of `h[x]` to `f(h[x], y)`.
Thus,

```
employees.group_by(dept).fold(0_i64, |a, _| a + 1) =
    {("Computability", 2),
     ("Algebra",       3)}
```

## Schema Definitions

So far, we've defined our data manually.
In most use cases, the data will live somewhere on disk.
Manually loading it would be tedious and error-prone.

To systematize data-loading, Prela exposes a `schema` macro.
The macro has two primary purposes.
First, it defines a `<schema_name>_init` function which you can call to load data from disk into Prela.
Second, it exposes _navigation handles_ that facilitate access to relations.
In addition, it does some technical bookkeeping the relevance of which will be examined later.

The @sec:prela-internals section and the `schema` documentation explain how all this works under the hood.
For now, we illustrate the use `schema` to set up a Prela application.
The following invocation sets up a schema much like the one assumed in the example queries above.

```rust
schema! {
    EMPLOYEE / EmployeeSchema / employee_init:
    Employee(employee) / EmployeeNav {
        pub dept: Dept,
        pub name: str
    }
    Dept / DeptNav {
        name: str
    }
}
```

The macro consists of a header,

```rust
    EMPLOYEE / EmployeeSchema / employee_init:
```

followed by a number of entity declarations

```rust
    Employee(employee) / EmployeeNav {
        pub dept: Department,
        pub name: str
    }
    Department / DepartmentNav {
        name: str
    }
```

The header consists mostly of bookkeeping declarations used internally by the macro but not exposed to the user.

> [!warn]
> The header structure is likely to change in future versions.

The entity declarations consist of:

- The entity name (`Employee`, `Dept`), optionally followed by that entity's universe (`(employee)`).
- An `<EntityName>Nav` struct name, used internally by the macro.
- A list of column declarations (`{ pub dept: Dept, pub name: str }`)

Entity universes and columns marked `pub` are directly available in the global namespace.
This allows us to write `employee.select(name)` instead of `Employee::employee.select(Employee::name)`.

Entities correspond to SQL tables.
Thus, the schema above is equivalent to declaring the following tables in SQL:

```SQL
CREATE TABLE employee (
    id   INTEGER PRIMARY KEY,
    dept INTEGER REFERENCES department(id),
    name TEXT
);

CREATE TABLE department (
    id   INTEGER PRIMARY KEY,
    name TEXT
);
```

Note that the `id` columns are implicit in Prela.

### Building the DB

To build the DB, first run the macro, then call the `init` function defined by the macro with a path to the data files.
`init` expects an `<Entity>_<columns_name>.bin` data file at the provided path for each `(Entity, column)` appearing in `schema`.

## From SQL to Prela

Since Prela is embedded in Rust, it is easy to write expressive folds.
Here is an implementation of the first TPC-H query in Prela, using all the features we've seen so far:

```rust
let grouped = lineitem
        .with(shipdate.le(19980902))
      .select(quantity.and(extendedprice).and(discount).and(tax))
    .group_by(returnflag.and(Lineitem::status))
        .fold(
              (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0_i64),
              |(qty, ext, di, dp, chg, n), (((q, e), dc), tx)| {
                  let dp_inc = e * (1.0 - dc);
                  let chg_inc = dp_inc * (1.0 + tx);
                  (qty + q, ext + e, di + dc, dp + dp_inc, chg + chg_inc, n + 1)
              },
        );
```
