use time::OffsetDateTime;

/// Marks the payment as one created by this service, so that the accounting software can
/// tell it apart from references originating elsewhere
const SOURCE_FLAG: &str = "1337";

/// The number of digits reserved for making the reference unique
const UNIQUE_DIGITS: u32 = 11;

fn check_digit(base: &str) -> Option<u8> {
    if base.is_empty() || !base.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let weights = [7u32, 3, 1];
    let sum = base
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| u32::from(b - b'0') * weights[i % weights.len()])
        .sum::<u32>();

    Some(((10 - (sum % 10)) % 10) as u8)
}

/// Checks that the value is a syntactically valid Finnish reference number
pub fn is_valid(value: &str) -> bool {
    if value.len() < 4 || value.len() > 20 || !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let (base, check) = value.split_at(value.len() - 1);
    let Some(expected) = check_digit(base) else {
        return false;
    };

    check
        .chars()
        .next()
        .and_then(|c| c.to_digit(10))
        .is_some_and(|d| d as u8 == expected)
}

/// Generates a 20-digit Finnish reference number of the form
/// `<account><SOURCE_FLAG><unique><check digit>`, where the leading four digits tell the
/// accounting software which account the payment belongs to.
///
/// The unique part is derived from the current time and wraps every 100 seconds; a
/// collision would require two invoices to be created exactly 100 seconds apart to the
/// nanosecond, and would be harmless anyway.
pub fn generate(account: &str) -> String {
    let nanos = OffsetDateTime::now_utc()
        .unix_timestamp_nanos()
        .unsigned_abs();
    let unique = nanos % 10u128.pow(UNIQUE_DIGITS);

    let base = format!(
        "{account}{SOURCE_FLAG}{unique:0width$}",
        width = UNIQUE_DIGITS as usize
    );
    let check = check_digit(&base).expect("BUG: generated an invalid reference number base");

    format!("{base}{check}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_digit_matches_known_references() {
        assert!(is_valid("1232"));
        assert!(is_valid("11111111111111111117"));
        assert!(!is_valid("1233"));
        assert!(!is_valid("12a2"));
        assert!(!is_valid("123"), "too short to be a reference number");
        assert!(!is_valid(""));
    }

    #[test]
    fn generated_reference_encodes_the_account_and_the_source_flag() {
        let reference = generate("4212");

        assert_eq!(reference.len(), 20);
        assert!(reference.starts_with("42121337"));
        assert!(is_valid(&reference));
    }

    // The accounts now come from the CMS, so any four-digit account that does not start with a
    // zero has to produce a valid reference
    #[test]
    fn every_possible_account_generates_a_valid_reference() {
        for account in 1000..=9999 {
            let account = account.to_string();
            let reference = generate(&account);
            assert!(is_valid(&reference), "invalid reference for {account}");
            assert!(reference.starts_with(&account));
        }
    }
}
