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
  "cost_pool": "liikuntatoimikunta",
  "rows": [{ "product": "Product 1", "unit_price": 100 }],
  "attachment_descriptions": ["Attachment"]
}
```

## Reference numbers and cost pools

Every invoice is booked against a *toimikunta*, chosen by the person filing it. The
toimikunnat and their accounting accounts live in [`cost_pools.toml`](./cost_pools.toml), and
`GET /cost-pools` serves the list so that the frontend can populate its dropdown without
being redeployed whenever the list changes. The `cost_pool` field of an invoice must be one
of the `id`s from that list.

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

`cost_pool` is optional. An invoice that arrives without one is booked against account **4999**,
which does not exist in the bookkeeping, and is labelled *KOHDISTAMATON – toimikunta puuttuu* on
the PDF and in the notification email. This means an older frontend keeps working instead of
having every invoice rejected, while the treasurer still cannot pay it into the wrong place by
accident. An *unknown* `cost_pool` id, on the other hand, is a client bug and is rejected with a
422.
