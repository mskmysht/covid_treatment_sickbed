pub fn to_half_digits(fd: &str) -> Option<String> {
    fd.chars()
        .map(|c| {
            match c {
                // convert FULLWIDTH DIGIT ZERO ~ NINE to ascii 0-9
                '\u{FF10}'..='\u{FF19}' => {
                    let k = u32::try_from(c).unwrap() - 0xFF10 + 0x0030;
                    char::from_u32(k)
                }
                '\u{0030}'..='\u{0039}' => Some(c),
                _ => None,
            }
        })
        .collect::<Option<String>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_half_digits() {
        assert_eq!(to_half_digits("０"), Some("0".to_string()));
        assert_eq!(to_half_digits("９"), Some("9".to_string()));
        assert_eq!(to_half_digits("0"), Some("0".to_string()));
        assert_eq!(to_half_digits("9"), Some("9".to_string()));
        assert_eq!(to_half_digits("１１"), Some("11".to_string()));
        assert_eq!(to_half_digits("1１1１1"), Some("11111".to_string()));
    }
}
