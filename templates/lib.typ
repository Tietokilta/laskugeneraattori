#let price(number) = { ... }
#let percentage(number) = { ... }

#let writeline(length) = {
  line(length: length, start: (0pt, 1em))
}

#let generator_footer = [
  #v(1em)
  #align(right)[
    Laskugeneraattori #VERSION
    #link("https://github.com/Tietokilta/laskugeneraattori/commit/" + COMMIT_HASH)[
      #COMMIT_HASH.slice(0, 7)
    ]
  ]
]

#let default_page(extra_footer) = {
  set page(
    background: [
      image("/tik.png")
    ],
    footer: [
      #extra_footer
      #generator_footer
    ],
    footer-descent: -0.5em,
  )
}