#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidOctalError;

pub fn read_ascii_octal(o: &[u8]) -> Result<u64, InvalidOctalError> {
    let mut res = 0;
    for b in o {
        res *= 8;
        match b {
            b'0'..=b'7' => res += (b - b'0') as u64,
            _ => return Err(InvalidOctalError),
        }
    }
    Ok(res)
}

pub fn trim_nulls(b: &[u8]) -> &[u8] {
    let idx = b
        .iter()
        .enumerate()
        .find_map(|(i, &b)| (b == 0).then_some(i))
        .unwrap_or(b.len());
    &b[..idx]
}

pub fn read_ascii_octal_trim(o: &[u8]) -> Result<u64, InvalidOctalError> {
    read_ascii_octal(trim_nulls(o))
}

#[cfg(test)]
mod tests {
    #[test]
    fn read_ascii_octal() {
        let octal = [b'3', b'0', b'0', b'7', b'1'];
        let value = super::read_ascii_octal(&octal).unwrap();
        assert_eq!(value, 12345);
    }

    #[test]
    fn trim_nulls() {
        let b = [1, 2, 3, 4, 5, 0, 0, 0, 0];
        let trimmed = super::trim_nulls(&b);
        assert_eq!(trimmed, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn trim_nulls_no_nulls() {
        let b = [1, 2, 3, 4, 5];
        let trimmed = super::trim_nulls(&b);
        assert_eq!(trimmed, b);
    }
}
