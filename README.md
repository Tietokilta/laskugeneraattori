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
  "rows": [{ "product": "Product 1", "unit_price": 100 }],
  "attachment_descriptions": ["Attachment"]
}
```
