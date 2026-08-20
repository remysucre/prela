//> using scala 3.8.4

// Row-kind tags. Distinct types at compile time (so e.g. a `PersonRow` can't
// be composed with a relation expecting a `CastRow`), plain `Int`s at
// runtime (`opaque type` erases to the underlying type, zero overhead).
opaque type CompanyRow = Int
object CompanyRow { def apply(i: Int): CompanyRow = i }
opaque type CastRow = Int
object CastRow { def apply(i: Int): CastRow = i }
opaque type PersonRow = Int
object PersonRow { def apply(i: Int): PersonRow = i }
opaque type AliasRow = Int
object AliasRow { def apply(i: Int): AliasRow = i }

/** A binary relation from K to V: a generalization of a function K -> V
  * that allows multiple V's per K. Every "column" of every table is one
  * of these -- see tutorial/prela.py for the reference (Python) version,
  * and ../rust/ for the real (CPS-compiled) implementation.
  *
  * Combinators (`WHERE`, `SELECT`, `AND`) are capitalized to read like SQL
  * keywords; tables and columns (`movie`, `company`, ...) stay lowercase,
  * like SQL identifiers. Scala doesn't care about case in identifiers --
  * the only place it's semantically special is inside `match` patterns --
  * so this split is purely a naming convention here, not a language rule.
  */
final class Rel[K, V](val rows: Seq[(K, V)]) extends Selectable:

  /** Relational composition `r SELECT s` ("r's s"): SQL's
    *   SELECT r.x, s.z FROM r, s WHERE r.y = s.y
    */
  infix def SELECT[V2](that: Rel[V, V2]): Rel[K, V2] =
    new Rel(for (x, y) <- rows; (y2, z) <- that.rows if y == y2 yield (x, z))

  /** Product: two relations sharing a key, `K -> V` and `K -> V2`,
    * become `K -> (V, V2)`. */
  infix def AND[V2](that: Rel[K, V2]): Rel[K, (V, V2)] =
    new Rel(for (x, y) <- rows; (x2, z) <- that.rows if x == x2 yield (x, (y, z)))

  /** Left semijoin: keeps rows of `r` whose value is a key of `that`.
    * Plays the role of SQL's WHERE clause. Named `WHERE`, not `with`
    * (which the Rust implementation is stuck with because `where` is
    * reserved there -- it isn't in Scala).
    */
  infix def WHERE[V2](that: Rel[V, V2]): Rel[K, V] =
    val keys = that.rows.map(_._1).toSet
    new Rel(rows.filter((_, y) => keys.contains(y)))

  /** Equality predicate on the value column, spelled with the real `==`.
    * `Any` already declares a final `==(that: Any): Boolean`, but ours
    * takes a `V`, not an `Any`, so the two coexist as overloads -- the
    * compiler picks whichever one's parameter type actually matches the
    * argument at each call site. `company.country == "[us]"` resolves to
    * this one; `rel1 == rel2` (comparing two `Rel`s) still falls through
    * to `Any`'s, which calls our `equals` override below. */
  infix def ==(v: V): Rel[K, V] = new Rel(rows.filter(_._2 == v))

  /** `r.foo` is sugar for `r SELECT foo`: `foo` is looked up by name in
    * the global `Schema` and composed on. This is `scala.Selectable`'s
    * doing: for a type like `Cast[K]` below, which refines `Rel[K,CastRow]`
    * with `def person: Person[K]`, the compiler rewrites `cast.person`
    * into `cast.selectDynamic("person")` and *statically* types the
    * result as `Person[K]` -- taking our word for it from the refinement,
    * not from anything this method actually returns. So this is compile-time
    * checked at the use site (`cast.person.alias` fails to typecheck if
    * `Person[K]` doesn't declare `.alias`), even though the name -> relation
    * lookup itself happens at runtime.
    */
  def selectDynamic(name: String): Rel[K, Any] =
    SELECT(Schema.columns(name).asInstanceOf[Rel[V, Any]])

  override def equals(other: Any): Boolean = other match
    case that: Rel[?, ?] => rows.toSet == that.rows.toSet
    case _                => false
  override def hashCode: Int = rows.toSet.hashCode
  override def toString: String = s"Rel(${rows.mkString(", ")})"

object Rel:
  def apply[K, V](rows: (K, V)*): Rel[K, V] = new Rel(rows)

object Schema:
  var columns: Map[String, Rel[Any, Any]] = Map.empty
  /** Registers `r` under `name` for dynamic field lookup, and returns it
    * unchanged so this can wrap a `val` definition. */
  def register[K, V](name: String, r: Rel[K, V]): Rel[K, V] =
    columns += name -> r.asInstanceOf[Rel[Any, Any]]
    r

// One refinement alias per row kind, each naming its reachable fields and
// what you land on next. `K` (the row you started navigating from) is left
// free, so e.g. `Cast[K]` fits a `cast` relation out of any table.
// A `val` needs one of these ascribed only where a query dots into it
// directly; intermediate steps (`person`, `alias`, ...) inherit their
// return type from the *previous* hop's refinement, not from how they
// were declared.
type Company[K] = Rel[K, CompanyRow] { def country: Rel[K, String] }
type Cast[K]    = Rel[K, CastRow]    { def person: Person[K] }
type Person[K]  = Rel[K, PersonRow]  { def alias: Alias[K] }
type Alias[K]   = Rel[K, AliasRow]   { def text: Rel[K, String] }

@main def demo(): Unit =
  // movie is the identity relation over movie ids
  val movie = Rel(0 -> 0, 1 -> 1, 2 -> 2, 3 -> 3)
  val title = Rel(0 -> "The Cape Canaveral Monsters", 1 -> "Escape from Planet Earth",
                   2 -> "Alien Vengeance", 3 -> "Nice Movie")

  // movie -> production company row -> country. Casting to `Company[Int]`
  // is what makes `.country` available on `company`; see selectDynamic above.
  val company: Company[Int] = Schema.register("company",
    Rel[Int, CompanyRow](0 -> CompanyRow(0), 1 -> CompanyRow(1), 2 -> CompanyRow(2), 3 -> CompanyRow(3))
  ).asInstanceOf[Company[Int]]
  Schema.register("country", Rel[CompanyRow, String](
    CompanyRow(0) -> "[us]", CompanyRow(1) -> "[us]", CompanyRow(2) -> "[de]", CompanyRow(3) -> "[us]"))

  // movie -> keyword (a movie can have several)
  val keyword = Rel(
    0 -> "character-name-in-title", 0 -> "cape-canaveral",
    1 -> "character-name-in-title", 1 -> "space",
    2 -> "alien",
    3 -> "boring",
  )

  // movie -> cast row (several per movie) -> person -> alias -> text
  val cast: Cast[Int] = Schema.register("cast",
    Rel[Int, CastRow](0 -> CastRow(0), 0 -> CastRow(1), 1 -> CastRow(2), 3 -> CastRow(3))
  ).asInstanceOf[Cast[Int]]
  Schema.register("person", Rel[CastRow, PersonRow](
    CastRow(0) -> PersonRow(10), CastRow(1) -> PersonRow(11), CastRow(2) -> PersonRow(12), CastRow(3) -> PersonRow(13)))
  Schema.register("alias", Rel[PersonRow, AliasRow](
    PersonRow(10) -> AliasRow(100), PersonRow(11) -> AliasRow(101), PersonRow(12) -> AliasRow(102), PersonRow(13) -> AliasRow(103)))
  Schema.register("text", Rel[AliasRow, String](
    AliasRow(100) -> "Scott Peters", AliasRow(101) -> "Jack Nicholson", AliasRow(102) -> "Linnea Quigley", AliasRow(103) -> "Nobody"))

  // JOB query 16b (see fig:JOB-query-16b in prela.lyx): capitalized
  // combinators, dotted field access, and a real `==` -- about as close to
  // the SQL it replaces as Scala syntax gets:
  //
  //   SELECT title, alias FROM movie
  //   WHERE company's country = 'us' AND keyword = 'character-name-in-title'
  //
  val result = (
         movie WHERE ((company.country == "[us]") AND 
                      (keyword == "character-name-in-title"))
              SELECT (title AND cast.person.alias.text)
  )

  println(result)
