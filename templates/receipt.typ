#import "lib.typ": price, percentage, default_page

#default_page([
  Tietokilta ry *ei ole* arvonlisäverovelvollinen.
  Ongelmatapauksissa ota yhteyttä rahastonhoitajaan:
  #link("mailto:rahastonhoitaja@tietokilta.fi").
  Tarkemmat yhteystiedot löydät killan sivuilta.
])

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