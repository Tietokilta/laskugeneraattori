#import "lib.typ": price, percentage, page_background, generator_footer

#let extra_footer = [
  Tietokilta ry *ei ole* arvonlisäverovelvollinen.
  Ongelmatapauksissa ota yhteyttä rahastonhoitajaan:
  #link("mailto:rahastonhoitaja@tietokilta.fi").
  Tarkemmat yhteystiedot löydät killan sivuilta.
]

#set page(
  background: page_background,
  footer: [#extra_footer #generator_footer],
  footer-descent: -0.5em,
)

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