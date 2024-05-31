use std::sync::Once;

use crate::api::app;
use crate::api::invoices::{CreateInvoice, CreateInvoiceRow, PopulatedInvoice};
use crate::models::NewAddress;

use axum::http::StatusCode;
use axum::Router;
use axum_test::multipart::MultipartForm;
use axum_test::TestServer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

static INIT: Once = Once::new();
async fn test_init() -> Router {
    INIT.call_once(|| {
        tracing_subscriber::registry()
            .with::<EnvFilter>(
                "laskugeneraattori=debug,tower_http=debug,axum::rejection=trace".into(),
            )
            .with(tracing_subscriber::fmt::layer())
            .init()
    });
    app().with_state(crate::database::new().await)
}
#[tokio::test]
async fn create() {
    let app = test_init().await;

    let body = CreateInvoice {
        rows: vec![
            CreateInvoiceRow {
                product: String::from("pleikkari"),
                quantity: 69,
                unit: String::from("kpl"),
                unit_price: 4200,
            },
            CreateInvoiceRow {
                product: String::from("xbox"),
                quantity: 1,
                unit: String::from("kpl"),
                unit_price: 4200,
            },
            CreateInvoiceRow {
                product: String::from("nintendo wii"),
                quantity: 2,
                unit: String::from("kpl"),
                unit_price: 4200,
            },
        ],
        attachments: vec![],
        recipient_name: "Velkoja".into(),
        recipient_email: "velkoja@velat.com".into(),
        subject: "Pelikonsoleita".into(),
        description: "Ostettiin pelikonsoleita ku ei ollu tarpeeks".into(),
        address: NewAddress {
            street: "Otakaari 18A 69".into(),
            city: "Espoo".into(),
            zip: "02jotain".into(),
        },
        bank_account_number: "ei ole".into(),
        phone_number: "+358401234567".into(),
    };

    let body = MultipartForm::new().add_text("data", serde_json::to_string(&body).unwrap());
    let server = TestServer::new(app).unwrap();

    let response = server.post("/invoices").multipart(body).await;

    assert_eq!(response.status_code(), StatusCode::CREATED);
}

#[tokio::test]
async fn list_all() {
    let app = test_init().await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/invoices").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn create_list_all() {
    let app = test_init().await;

    let body = CreateInvoice {
        recipient_name: "Velkoja".into(),
        recipient_email: "velkoja@velat.com".into(),
        address: NewAddress {
            street: "Otakaari 18A 69".into(),
            city: "Espoo".into(),
            zip: "02jotain".into(),
        },
        bank_account_number: "ei ole".into(),
        phone_number: "+358401234567".into(),
        subject: "Pelikonsoleita".into(),
        description: "Ostettiin pelikonsoleita ku ei ollu tarpeeks".into(),
        rows: vec![
            CreateInvoiceRow {
                product: String::from("pleikkari"),
                quantity: 69,
                unit: String::from("kpl"),
                unit_price: 4200,
            },
            CreateInvoiceRow {
                product: String::from("xbox"),
                quantity: 1,
                unit: String::from("kpl"),
                unit_price: 4200,
            },
            CreateInvoiceRow {
                product: String::from("nintendo wii"),
                quantity: 2,
                unit: String::from("kpl"),
                unit_price: 4200,
            },
        ],
        attachments: vec![],
    };

    let server = TestServer::new(app).unwrap();
    let body = MultipartForm::new().add_text("data", serde_json::to_string(&body).unwrap());

    let create_response = server.post("/invoices").multipart(body).await;
    assert_eq!(create_response.status_code(), StatusCode::CREATED);

    let list_response = server.get("/invoices").await;
    assert_eq!(list_response.status_code(), StatusCode::OK);

    let result = std::panic::catch_unwind(|| list_response.json::<Vec<PopulatedInvoice>>());
    assert!(result.is_ok_and(|invoices| !invoices.is_empty()));
}
