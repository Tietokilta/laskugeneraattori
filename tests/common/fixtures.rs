use serde_json::{json, Value};

pub fn valid_invoice_json() -> Value {
    json!({
        "recipient_name": "Test User",
        "recipient_email": "test@example.com",
        "address": {
            "street": "Test Street 1",
            "city": "Helsinki",
            "zip": "00100"
        },
        "bank_account_number": "FI21 1234 5600 0007 85",
        "subject": "Test Invoice",
        "description": "Test description for invoice",
        "phone_number": "+358401234567",
        "attachment_descriptions": [],
        "rows": [
            {
                "product": "Test Product",
                "unit_price": 1000
            }
        ]
    })
}

pub fn invoice_with_invalid_iban() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["bank_account_number"] = json!("INVALID_IBAN");
    invoice
}

pub fn invoice_with_invalid_phone() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["phone_number"] = json!("not-a-phone");
    invoice
}

pub fn invoice_with_empty_rows() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["rows"] = json!([]);
    invoice
}

pub fn invoice_with_long_subject() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["subject"] = json!("A".repeat(200));
    invoice
}

pub fn invoice_with_empty_subject() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["subject"] = json!("");
    invoice
}

pub fn invoice_with_negative_price() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["rows"] = json!([{
        "product": "Test",
        "unit_price": -100
    }]);
    invoice
}

pub fn invoice_with_zero_price() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["rows"] = json!([{
        "product": "Test",
        "unit_price": 0
    }]);
    invoice
}

pub fn invoice_with_multiple_rows() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["rows"] = json!([
        { "product": "Product 1", "unit_price": 1000 },
        { "product": "Product 2", "unit_price": 2500 },
        { "product": "Product 3", "unit_price": 500 }
    ]);
    invoice
}

pub fn invoice_with_attachment_descriptions(descriptions: Vec<&str>) -> Value {
    let mut invoice = valid_invoice_json();
    invoice["attachment_descriptions"] = json!(descriptions);
    invoice
}
