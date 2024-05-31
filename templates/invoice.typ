#set page(
  background: [
    #image("tik.png")
  ],
  footer: [
    Laskut hyväksytään Tietokillan hallituksen kokouksissa.
    Ongelmatapauksissa ota yhteyttä rahastonhoitajaan: rahastonhoitaja\@tietokilta.fi
    Tarkemmat yhteystiedot löydät killan sivuilta.
  ],
)
#set text(lang: "fi")

#let writeline(length) = {
  line(length: length, start: (0pt, 1em))
}

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
*Katuosoite*: puuttuu \
*Postinumero ja -toimipaikka*: puuttuu \
*Puhelin*: puuttuu \
*E-mail*: #link("mailto:" + data.recipient_email) \

#colbreak()
= LASKU
*ID*: #data.id \
*Päivämäärä*: #data.creation_time \
]

== Tietokilta

*Aihe*: puuttuu \
*Perustelut*: puuttuu \

=== Erittely
#let rows = data.rows.map(it => ([#it.quantity #it.unit], [#it.product],
      [#(it.unit_price/100) euroa], [#(it.quantity*it.unit_price/100) euroa]))
#table(columns: (25%, 25%, 25%, 25%),
  table.header([*Määrä*], [*Tuote*], [*Hinta per*], [*Yhteensä*]),
  ..rows.flatten(),
  ..([], [], [], [*#(data.rows.map(r => r.unit_price*r.quantity).sum()/100) euroa*])
)

*IBAN-tilinumero*: #data.bank_account_number \

=== Muuta:

==== LIITTEET:
#data.attachments.map(a => a.filename).join(",")

#for file in data.attachments {
  pagebreak()
  image(file.filename)
}
