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

#set page(
  background: [
    #image("/tik.png")
  ],
  footer: [
    Tietokilta ry *ei ole* arvonlisäverovelvollinen. Ongelmatapauksissa ota yhteyttä rahastonhoitajaan: #link("mailto:rahastonhoitaja@tietokilta.fi").
    Tarkemmat yhteystiedot löydät killan sivuilta.

    #v(1em)
    #align(right)[Laskugeneraattori #VERSION #link("https://github.com/Tietokilta/laskugeneraattori/commit/" + COMMIT_HASH)[#COMMIT_HASH.slice(0, 7)]]
  ],
  footer-descent: -0.5em,
)
#set text(lang: "fi")

= KUITTI
#v(10pt)

#columns(2)[
*Yhdistyksen nimi*: Tietokilta ry \
*Osoite*: Konemiehentie 2,
          02150 Espoo \
*Y-tunnus*: 1790346-8 \
*Päivämäärä*: #datetime.today().display("[day padding:zero].[month padding:zero].[year]") \
*Kellonaika*: #datetime.today().display("[hour padding:zero]:[minute padding:zero]") \
*Tunniste*: #data.receipt_number \

#colbreak()
*Maksajan nimi*: #data.purchaser_name \
*Sähköposti*: #link("mailto:" + data.purchaser_email) \
]

#v(40pt)
== Tuotteet

=== Erittely
#let rows = data.rows.map(it => ([#it.product],[#percentage(it.vat) %],[#price(it.unit_price) €]))
#table(columns: (50%, 25%, 25%),
  align: (left, right, right),
  table.header([*Kuitti/Tuote*], [*ALV*], [*Summa*]),
  ..rows.flatten(),
  ..([],[],[*#price(data.rows.map(r => r.unit_price).sum()) €*])
)