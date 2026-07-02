// A small "company" database, declared with prela's `schema!` macro
// (see prela/rust/src/schema.rs for what each declaration expands to) and
// seeded directly in memory — no binary cache, no regen step. Swap `load()`
// below for your own entities/data to try out different shapes.
//
// Four entities exercise every field kind the macro supports:
//   str / i64 / f64 scalars, an entity FK (Employee.dept), a Multi<entity>
//   (Employee.skills, Project.members) and a Multi<str> (Employee.certs).
//
// Field-name collisions across entities can't both be bare: `budget` and
// `pname` collide, so they stay qualified (`Department::budget`,
// `Project::pname`); everything else is exposed bare.

use prela::engine::*;
use prela::schema::schema;

schema! { DB / Store / _unused_cache_init:
    Department(department) / DepartmentNav {
        pub dname: str,
        budget: f64,
    }
    Skill(skill) / SkillNav {
        pub sname: str,
    }
    Employee(employee) / EmployeeNav {
        pub name: str,
        pub age: i64,
        pub salary: f64,
        pub dept: Department,
        pub skills: Multi<Skill>,
        pub certs: Multi<str>,
    }
    Project(project) / ProjectNav {
        pname: str,
        budget: f64,
        pub lead: Employee,
        pub members: Multi<Employee>,
    }
}

/// Build one CSR-backed `MultiRel` from a per-key list of values. Small-data
/// helper for hand-written playgrounds; production schemas get their CSR
/// arrays straight from the cache (see `cache.rs`).
fn csr<R: Copy + 'static, D: Dense>(rows: Vec<Vec<R>>) -> MultiRel<R, D> {
    let mut offsets = Vec::with_capacity(rows.len() + 1);
    let mut values = Vec::new();
    offsets.push(0u32);
    for row in &rows {
        values.extend_from_slice(row);
        offsets.push(values.len() as u32);
    }
    MultiRel::from_csr(Vec::leak(offsets), Vec::leak(values))
}

/// Seed the schema's global store directly — the in-memory analog of the
/// generated `_unused_cache_init(cache_dir)`. Call once before running
/// queries.
pub fn load() {
    let store = Store {
        Department: DB::Department {
            dname: VecRel::new(vec!["Engineering", "Sales", "Marketing"]),
            budget: VecRel::new(vec![5_000_000.0, 2_000_000.0, 1_500_000.0]),
        },
        Skill: DB::Skill {
            sname: VecRel::new(vec!["Rust", "Python", "SQL", "Negotiation", "SEO"]),
        },
        Employee: DB::Employee {
            name: VecRel::new(vec!["Alice", "Bob", "Carol", "Dave", "Erin", "Frank"]),
            age: VecRel::new(vec![34, 29, 41, 26, 37, 45]),
            salary: VecRel::new(vec![
                120_000.0, 95_000.0, 110_000.0, 88_000.0, 130_000.0, 105_000.0,
            ]),
            dept: VecRel::new(vec![
                Id::new(0), // Alice   -> Engineering
                Id::new(0), // Bob     -> Engineering
                Id::new(1), // Carol   -> Sales
                Id::new(2), // Dave    -> Marketing
                Id::new(0), // Erin    -> Engineering
                Id::new(1), // Frank   -> Sales
            ]),
            skills: csr(vec![
                vec![Id::new(0), Id::new(2)], // Alice:  Rust, SQL
                vec![Id::new(1), Id::new(2)], // Bob:    Python, SQL
                vec![Id::new(3)],             // Carol:  Negotiation
                vec![Id::new(4)],             // Dave:   SEO
                vec![Id::new(0), Id::new(1)], // Erin:   Rust, Python
                vec![Id::new(3), Id::new(2)], // Frank:  Negotiation, SQL
            ]),
            certs: csr(vec![
                vec!["AWS Certified"],
                vec![],
                vec!["PMP"],
                vec![],
                vec!["AWS Certified", "Kubernetes"],
                vec![],
            ]),
        },
        Project: DB::Project {
            pname: VecRel::new(vec!["Atlas", "Nimbus", "Phoenix"]),
            budget: VecRel::new(vec![800_000.0, 300_000.0, 450_000.0]),
            lead: VecRel::new(vec![Id::new(0), Id::new(2), Id::new(4)]),
            members: csr(vec![
                vec![Id::new(0), Id::new(1), Id::new(4)], // Atlas:   Alice, Bob, Erin
                vec![Id::new(2), Id::new(5)],             // Nimbus:  Carol, Frank
                vec![Id::new(4), Id::new(3), Id::new(1)], // Phoenix: Erin, Dave, Bob
            ]),
        },
    };
    if DB::STORE.set(store).is_err() {
        panic!("schema already initialized");
    }
}
