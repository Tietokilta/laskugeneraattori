#let price(number) = {
    let num_as_str = str(number)
    let whole_nums="0"
    if num_as_str.len() > 2 {
        whole_nums = num_as_str.slice(0, -2)
    }
    let rem = "00"
    if num_as_str.len() == 1 {
        rem = "0" + num_as_str
    } else if num_as_str.len() >= 2 {
        rem = num_as_str.slice(-2)
    }
    whole_nums+","+rem
}
#let percentage(number) = {
    let num_as_str = str(number)
    let whole_nums="0"
    if num_as_str.len() > 1 {
        whole_nums = num_as_str.slice(0, -1)
    }
    let rem = "0"
    if num_as_str.len() >= 1 {
        rem = num_as_str.slice(-1)
    }
    whole_nums+","+rem
}
#let writeline(length) = {
  line(length: length, start: (0pt, 1em))
}
#let page_background = [
  #image("/tik.png")
]
#let generator_footer = [
  #v(1em)
  #align(right)[
    Laskugeneraattori #VERSION
    #link("https://github.com/Tietokilta/laskugeneraattori/commit/" + COMMIT_HASH)[
      #COMMIT_HASH.slice(0, 7)
    ]
  ]
]
