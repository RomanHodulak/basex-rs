use std::cmp::min;
use std::io::Read;

/// Wraps a reader and escapes all bytes with special meaning as defined by
/// [conventions](https://docs.basex.org/wiki/Server_Protocol#Conventions).
///
/// All bytes that have any special meaning get prefixed by a single `0xFF` byte.
///
/// # Examples
/// ## Input
/// `[0, 1, 2, 3, 4, 0xFF]`
/// ## Output
/// `[0xFF, 0, 1, 2, 3, 4, 0xFF, 0xFF]`
pub(crate) struct EscapeReader<'a, R>
where
    R: Read,
{
    inner: &'a mut R,
    accumulator: Vec<u8>,
}

impl<'a, R> EscapeReader<'a, R>
where
    R: Read,
{
    pub(crate) fn new(inner: &'a mut R) -> Self {
        Self {
            inner,
            accumulator: vec![],
        }
    }
}

impl<R> Read for EscapeReader<'_, R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let accumulator_length = min(buf.len(), self.accumulator.len());

        for buf in buf.iter_mut().take(accumulator_length) {
            *buf = self.accumulator.pop().unwrap();
        }

        let stream_length = self.inner.read(&mut buf[accumulator_length..])?;
        let size = accumulator_length + stream_length;
        let escape_chars_count = buf[accumulator_length..size]
            .iter()
            .filter(|b| **b == 0 || **b == 0xFF)
            .count();
        let escaped_size = size + escape_chars_count;
        let mut shift = escape_chars_count;
        let mut next_skip = false;

        for i in (accumulator_length..escaped_size).rev() {
            if next_skip {
                next_skip = false;
                continue;
            }
            if i >= buf.len() {
                self.accumulator.push(buf[i - shift]);
            } else {
                buf[i] = buf[i - shift];
            }

            if buf[i - shift] == 0xFF || buf[i - shift] == 0 {
                if i <= buf.len() {
                    buf[i - 1] = 0xFF;
                } else {
                    self.accumulator.push(0xFF);
                }
                shift -= 1;
                next_skip = true;
            }
        }

        Ok(min(buf.len(), accumulator_length + stream_length + escape_chars_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{empty, Read};

    #[test]
    fn test_escaping_without_escape_bytes_leaves_buffer_intact() {
        let expected_bytes = [1u8, 2, 3, 4];
        let mut slice = &expected_bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes.to_vec(), actual_bytes);
    }

    #[test]
    fn test_escaping_with_escape_bytes() {
        let bytes = [1u8, 0, 9, 0xFF, 6];
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes = vec![1u8, 0xFF, 0, 9, 0xFF, 0xFF, 6];
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[test]
    fn test_escaping_only_escape_bytes() {
        let bytes = [0u8].repeat(4);
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes = [0xFF, 0u8].repeat(4);
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[test]
    fn test_escaping_only_escape_bytes_on_multiple_reading() {
        let bytes = [0u8].repeat(20);
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes = [0xFF, 0u8].repeat(20);
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[test]
    fn test_escaping_from_empty_reader_does_nothing() {
        let mut bytes = empty();
        let mut escaped = EscapeReader::new(&mut bytes);

        let expected_bytes: Vec<u8> = vec![];
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[test]
    fn test_escaping_into_empty_buffer_does_nothing() {
        let bytes = [1u8];
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes: [u8; 0] = [];
        let mut actual_bytes: [u8; 0] = [];
        escaped.read(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }
}
