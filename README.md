# Laskugeneraattori

This is the laskugeneraattoori backend written in Rust.

The application is based on [axum](https://github.com/tokio-rs/axum).
PDF generation is based on [typst](https://github.com/typst/typst).

## Configuration

The following variables can be configured in the environment (or the .env file)

```sh
# VARIABLE="default value"

PORT=3000
BIND_ADDR=127.0.0.1
ALLOWED_ORIGINS= # comma separated list of urls
CMS_URL= # required, base url of the CMS the cost pools are read from, e.g. https://tietokilta.fi
MAILGUN_URL=
MAILGUN_USER=
MAILGUN_PASSWORD=
MAILGUN_TO=
MAILGUN_FROM=
MAILGUN_DISABLE= # disable mailgun, e.g. for local testing
```

## Running laskugeneraattori

To run without mailgun (e.g. for local testing), set env variable `MAILGUN_DISABLE=true`. The resulting pdf is saved to temp folder, path can be found from the server output.

### With cargo

```sh
cargo run
```

### With Docker

```sh
docker build . -t laskugeneraattori
docker run laskugeneraattori
```

## Sending invoices with curl

Especially in local development/testing, it is useful to be able to send invoices via curl.

For example:

```sh
curl -v -F data="$(cat invoice.json)" -F attachments="@file1.pdf" http://localhost:3000/invoices
```

With `invoice.json` being something like

```json
{
  "recipient_name": "Test Name",
  "recipient_email": "test@example.com",
  "address": {
    "street": "Street name",
    "city": "Espoo",
    "zip": "02150"
  },
  "phone_number": "+358401234567",
  "subject": "Subject",
  "description": "Description",
  "bank_account_number": "FI1410093000123458",
  "cost_pool": "507f1f77bcf86cd799439011",
  "rows": [{ "product": "Product 1", "unit_price": 100 }],
  "attachment_descriptions": ["Attachment"]
}
```

## Reference numbers and cost pools

Every invoice is booked against a *toimikunta*, chosen by the person filing it. The toimikunnat
and their accounting accounts are maintained in the CMS, in the `cost-pools` collection, so that
adding or renaming one needs no deploy of either the site or this service. The `cost_pool` field
of an invoice is the id of such a document, and this service looks it up from
`$CMS_URL/api/cost-pools/{id}` to find the account.

The server then generates the invoice's Finnish reference number as

```
4212 1337 04713915823 4
│    │    │           └─ check digit (7-3-1 weighting)
│    │    └───────────── derived from the current time, makes the reference unique
│    └────────────────── constant, marks the payment as created by laskugeneraattori
└─────────────────────── the account of the chosen toimikunta
```

The reference travels in the bank barcode on the PDF, so when the treasurer pays the invoice
the accounting software can route the payment to the right account on its own.

`cost_pool` is optional. An invoice whose toimikunta cannot be established — none was sent, the
CMS does not know the id, or the CMS is unreachable — is booked against account **4999**, which
does not exist in the bookkeeping, and is labelled *KOHDISTAMATON – toimikunta puuttuu* on the
PDF and in the notification email. Nothing about the CMS can stop someone from filing an
invoice; the treasurer just has to assign those by hand. A `cost_pool` that is not a CMS
document id at all is a client bug and is rejected with a 422.
