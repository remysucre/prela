// A prela playground: a small in-memory "company" database (see schema.rs)
// plus a handful of example queries to get a feel for the combinator API.
// Add your own entities to schema.rs and your own queries below.
//
// Operator quick reference (engine.rs::QueryExt + schema!-generated nav
// methods — everything roots on IntoQuery, so paren-free leaf HANDLES like
// `employee`, `dept`, `Project::pname` mix freely with plan nodes):
//   .select(b)     composition (→)              .and(b)   product (∧ / ×)
//   .with(s)       restriction (:), keep rows whose value is a member of `s`
//   .eq/.ne/.gt/.lt/.ge/.le/.is_in/.rx/.nrx      predicates
//   .group_by(k)   re-key by a probeable relation, paired with:
//   .fold / .dense_fold / .buf_fold              per-key reduce (▷)
//   .field()       navigation methods generated per schema field — e.g.
//                  `lead.age()` ≡ `lead.select(Employee::age)`
//   .eq on an entity-valued query auto-navigates to that entity's PRIMARY
//   (first-declared scalar) field — `dept.eq("Engineering")` ≡
//   `dept.dname().eq("Engineering")`.
//
// Run with: cargo run

mod schema;

use prela::engine::*;
use schema::*;

fn main() {
    schema::load();

    // println!("== Engineers earning over $100k ==");
    // employee
    //     .with(dept.eq("Engineering").and(salary.gt(100_000.0)))
    //     .select(name)
    //     .drive(|_, n| println!("  {n}"));
    //
    // println!("\n== Employees who know Rust ==");
    // employee
    //     .with(skills.eq("Rust"))
    //     .select(name)
    //     .drive(|_, n| println!("  {n}"));
    //
    // println!("\n== Employees with an AWS certification ==");
    // employee
    //     .with(certs.rx("AWS"))
    //     .select(name)
    //     .drive(|_, n| println!("  {n}"));
    //
    // println!("\n== Projects led by an employee over 35 ==");
    // project
    //     .with(lead.age().gt(35))
    //     .select(Project::pname)
    //     .drive(|_, p| println!("  {p}"));
    //
    // println!("\n== Projects Bob is a member of ==");
    // project
    //     .with(members.eq("Bob"))
    //     .select(Project::pname)
    //     .drive(|_, p| println!("  {p}"));
    //
    // println!("\n== Total salary by department ==");
    // salary
    //     .group_by(dept)
    //     .dense_fold(department.iq().n, 0.0_f64, |a, s| a + s)
    //     .drive(|d, total| {
    //         dname.iq().probe(d, |dn| println!("  {dn:<12} ${total:>12.2}"));
    //     });

    println!("== Engineers earning over $100k ==");
    employee
        .with(salary.gt(100_000.0))
        .select(name)
        .drive(|_, y| println!("{y}"));
    println!("\n== Employees who know Rust ==");
    println!("\n== Employees with an AWS certification ==");
    println!("\n== Projects led by an employee over 35 ==");
}
