#import "lib.typ": price, writeline, page_background, generator_footer

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

#set text(lang: "fi")

#move(dx: -10%, dy: -5%, box(
  width: 120%,
  inset: 1em,
  stroke: black,
)[
  #let year = datetime.today().year()
  == Rahastonhoitajan merkintöjä:
  #stack(dir: ltr)[Hyväksytty][
    #writeline(5em)
  ][.][
    #writeline(5em)
  ][.#year][
    #h(1em) TiKH:n kokouksessa
  ][
    #writeline(5em)
  ][/#year kohdistettavaksi tilille][
    #writeline(5em)
  ]
  #stack(dir: ltr)[Maksettu][
    #writeline(5em)
  ][.][
    #writeline(5em)
  ][.#year Pankkitili][
    #writeline(5em)
  ][Käteinen][
    #writeline(5em)
  ][#h(2em) TOSITE][
    #writeline(5em)
  ]
])

#columns(2)[
*Laskuttajan nimi*: #data.recipient_name \
*Katuosoite*: #data.address.street \
*Postinumero ja -toimipaikka*: #data.address.zip #data.address.city \
*Puhelin*: #link("tel:" + data.phone_number) \
*E-mail*: #link("mailto:" + data.recipient_email) \

#colbreak()
= LASKU
*Päivämäärä*: #datetime.today().display("[day padding:zero].[month padding:zero].[year]") \
]

== Tietokilta

*Aihe*: #data.subject \
*Perustelut*: #data.description \

=== Erittely
#let rows = data.rows.map(it => ([#it.product],[#price(it.unit_price) €]))
#table(columns: (75%, 25%),
  align: (left, right),
  table.header([*Kuitti/Tuote*], [*Summa*]),
  ..rows.flatten(),
  ..([],[*#price(data.rows.map(r => r.unit_price).sum()) €*])
)

*IBAN-tilinumero*: #data.bank_account_number \

*Pankkiviivakoodi*: #data.barcode \


=== LIITTEET
#table(columns: (1fr, 2fr),
  table.header([*Tiedosto*], [*Kuvaus*]),
  ..data.attachments
    .zip(data.attachment_descriptions)
    .map(((a, d)) => 
      // NOTE: add breakpoints to the string
      // so that it can be wrapped to multiple lines
      (a.filename.codepoints().map(x => x + sym.zws).join(), d)
    ).flatten()
)

#for file in data.attachments {
  if regex("(?i)\.(jpg|jpeg|png|gif|svg)$") in file.filename {
    pagebreak()
    image("/attachments/" + file.filename)
  }
}
