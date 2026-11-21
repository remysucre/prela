#import "style.typ": noun

#let tar-figure = context {
  let ox = 5.3
  let oy = 1.75
  let P(x, y) = ((x + ox) * 1cm, (oy - y) * 1cm)
  let inset = 0.3333em
  let node(x, y, body, boxed: false, size: 9pt) = {
    let content = text(size: size, body)
    let m = measure(box(inset: inset, content))
    (
      x: x, y: y, body: content, boxed: boxed,
      w: m.width / 1cm, h: m.height / 1cm,
    )
  }
  let north(n) = (n.x, n.y + n.h / 2)
  let south(n) = (n.x, n.y - n.h / 2)
  let east(n) = (n.x + n.w / 2, n.y)
  let west(n) = (n.x - n.w / 2, n.y)

  let n2 = node(-4.3, 1.49, [SQL])
  let n1 = node(-4.3, 0.49, [ORM])
  let n6 = node(-4.31, -0.5, [Programmer])
  let n3 = node(0, 1.49, [Relational Algebra])
  let n4 = node(0, 0.5, [Physical Plan])
  let n5 = node(0, -0.5, [Columnar Storage])
  let n7 = node(-2.4, -0.5, noun[TAR], boxed: true)
  let n8 = node(-2.4, 0.75, [optimize (future work)], size: 7pt)

  let head(tip, from, stealth: false) = {
    let (tx, ty) = P(..tip)
    let (fx, fy) = P(..from)
    let dx = (tx - fx) / 1pt
    let dy = (ty - fy) / 1pt
    let len = calc.sqrt(dx * dx + dy * dy)
    let ux = dx / len
    let uy = dy / len
    let l = if stealth { 4.5 } else { 3.5 }
    let wdt = if stealth { 2.2 } else { 1.8 }
    let back = if stealth { 3.2 } else { l }
    let bx = tx / 1pt - ux * l
    let by = ty / 1pt - uy * l
    let mx = tx / 1pt - ux * back
    let my = ty / 1pt - uy * back
    place(polygon(
      fill: black,
      stroke: none,
      (tx, ty),
      ((bx - uy * wdt) * 1pt, (by + ux * wdt) * 1pt),
      (mx * 1pt, my * 1pt),
      ((bx + uy * wdt) * 1pt, (by - ux * wdt) * 1pt),
    ))
  }
  let arrow(a, b, stealth: false) = {
    place(line(start: P(..a), end: P(..b), stroke: 0.5pt))
    head(b, a, stealth: stealth)
  }

  box(width: 6.7cm, height: 2.6cm, {
    for n in (n1, n2, n3, n4, n5, n6, n7, n8) {
      let (px, py) = P(n.x - n.w / 2, n.y + n.h / 2)
      place(dx: px, dy: py, box(
        inset: inset,
        stroke: if n.boxed { 0.5pt } else { none },
        radius: if n.boxed { 3pt } else { 0pt },
        n.body,
      ))
    }
    arrow(north(n1), south(n2))
    arrow(east(n2), west(n3))
    arrow(south(n3), north(n4))
    arrow(south(n4), north(n5))
    arrow(north(n6), south(n1))
    arrow(east(n6), west(n7), stealth: true)
    arrow(east(n7), west(n5), stealth: true)
    let top = north(n7)
    place(curve(
      stroke: (thickness: 0.5pt, dash: "densely-dotted"),
      curve.move(P(..top)),
      curve.cubic(P(-2.8, 0), P(-2.9, 0.5), P(-2.4, 0.5)),
      curve.cubic(P(-1.9, 0.5), P(-2, 0), P(..top)),
    ))
    head(top, (-2, 0), stealth: true)
  })
}
