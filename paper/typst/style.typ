#let acm-purple = cmyk(55%, 100%, 0%, 15%)
#let acm-dark-blue = cmyk(100%, 58%, 0%, 21%)

#let noun(body) = text(font: ("Linux Biolinum O", "Helvetica Neue"), body)
#let c(..keys) = keys.pos().map(k => cite(label(k))).join()
#let runin(title, body) = { v(0.6em); par(first-line-indent: 0pt)[#emph(title). #body] }
#let njoin = math.class("binary", "⋈")

#let rel-table(..args) = {
  set text(top-edge: "ascender", bottom-edge: "descender")
  table(
    stroke: 0.5pt,
    inset: (x: 5pt, y: 0.5pt),
    align: center + horizon,
    ..args,
  )
}

#let header-gap(n) = range(n).map(_ => table.cell(inset: 0pt, stroke: (x: none), box(height: 2pt)))

#let prela-keywords = "with|and|s|eq|select|gt|lt|filt|reduce|fold|inv|group_by|rx|map|gather|drive|probe|for|in|len|drv|prb"

#let paper(title: none, authors: (), abstract: none, body) = {
  set document(title: title, author: authors.map(a => repr(a.name)))
  set page(
    paper: "us-letter",
    margin: (x: 0.75in, top: 0.85in, bottom: 1in),
    columns: 2,
  )
  set columns(gutter: 24pt)
  set text(font: ("Linux Libertine O", "Libertinus Serif"), size: 9pt, lang: "en")
  set par(justify: true, leading: 0.5em, spacing: 0.5em, first-line-indent: (amount: 1em, all: false))

  show raw: set text(font: "DejaVu Sans Mono", size: 0.97em)
  show raw.where(block: true): it => block(above: 9pt, below: 9pt, it)
  show footnote.entry: it => { show raw: set text(size: 9pt * 0.97 * 0.8 - 1pt); it }
  show math.equation: set text(font: "Libertinus Math")
  show raw.where(block: true): set align(left)
  show raw.where(block: true): set par(justify: false)
  show raw.where(block: false, lang: none): it => {
    show regex("\\b(" + prela-keywords + ")\\b"): set text(weight: "bold")
    it
  }

  show link: it => if type(it.dest) == str { text(fill: acm-dark-blue, it) } else { it }
  show ref: it => { show regex("\\d+(\\.\\d+)*"): set text(fill: acm-purple); it }
  show cite: it => { show regex("\\d+"): set text(fill: acm-purple); it }

  set heading(numbering: "1.1")
  show heading: set text(size: 9pt)
  show heading.where(level: 1): it => block(above: 1.4em, below: 0.8em, {
    set text(weight: "bold", size: 11pt)
    if it.numbering != none { counter(heading).display(it.numbering); h(1em) }
    it.body
  })
  show heading.where(level: 2): it => block(above: 1.2em, below: 0.7em, {
    set text(weight: "bold", size: 10pt)
    if it.numbering != none { counter(heading).display(it.numbering); h(1em) }
    it.body
  })

  set figure(placement: auto, gap: 0.8em)
  show figure.where(kind: table): set figure.caption(position: top)
  show figure.caption: set text(size: 8.5pt)
  show figure.caption: it => align(center, box(align(left, [*#it.supplement~#context it.counter.display(it.numbering)#it.separator*#it.body])))
  set figure.caption(separator: [: ])

  show bibliography: set text(size: 7pt)
  show bibliography: set par(leading: 3.4pt, spacing: 3.4pt)

  place(top + center, float: true, scope: "parent", clearance: 2.5em, {
    set par(justify: false, first-line-indent: 0pt)
    align(center, text(size: 14.4pt, weight: "bold", title))
    v(1.2em)
    grid(
      columns: (1fr,) * authors.len(),
      ..authors.map(a => align(center, {
        text(size: 11pt, a.name)
        linebreak()
        text(size: 9pt, [#a.affiliation \ #a.department \ #a.location])
      })),
    )
  })

  if abstract != none {
    block(below: 1.4em, {
      set par(first-line-indent: 0pt)
      text(weight: "bold", size: 11pt, [Abstract])
      v(0.4em)
      abstract
    })
  }

  body
}

#let appendix(body) = {
  counter(heading).update(0)
  set heading(numbering: "A.1", supplement: [Appendix])
  body
}
