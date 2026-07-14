use super::TEST_COST_POOL_ID;
use serde_json::{Value, json};

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
        "cost_pool": TEST_COST_POOL_ID,
        "rows": [
            {
                "product": "Test Product",
                "unit_price": 1000
            }
        ]
    })
}

/// A well-formed id the CMS has never heard of, e.g. a cost pool deleted while the form was open
pub fn invoice_with_unknown_cost_pool() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["cost_pool"] = json!("507f1f77bcf86cd799439099");
    invoice
}

/// Not a CMS document id at all, so it never reaches the CMS
pub fn invoice_with_malformed_cost_pool_id() -> Value {
    let mut invoice = valid_invoice_json();
    invoice["cost_pool"] = json!("ei-olemassa");
    invoice
}

pub fn invoice_without_cost_pool() -> Value {
    let mut invoice = valid_invoice_json();
    invoice.as_object_mut().unwrap().remove("cost_pool");
    invoice
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
