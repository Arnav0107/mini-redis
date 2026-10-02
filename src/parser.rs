use bytes::{Buf, BytesMut};

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Invalid(&'static str),
}

fn find_crlf(buf: &[u8], from: usize) -> Option<usize> {
    buf.get(from..)? //Give me the portion of the buffer starting at from.
        .windows(2)
        .position(|w| w == b"\r\n") // finds where crlf error
        .map(|p| p + from) // converts the position back to the original buffer's index.
}

fn read_int(buf: &[u8], pos: usize, prefix: u8) -> Result<Option<(i64, usize)>, ParseError> {
    if pos >= buf.len() {
        return Ok(None);
    }
    //If we don't have enough data yet:

    if buf[pos] != prefix {
        return Err(ParseError::Invalid("unexpected type byte"));
    }

    let end = match find_crlf(buf, pos + 1) {
        Some(e) => e,
        None => return Ok(None),
    };

    let n = std::str::from_utf8(&buf[pos + 1..end])
        .map_err(|_| ParseError::Invalid("non-utf8 length"))?
        .parse::<i64>()
        .map_err(|_| ParseError::Invalid("bad number"))?;
    Ok(Some((n, end + 2)))
}

pub fn parse(buf: &mut BytesMut) -> Result<Option<Vec<String>>, ParseError> {
    //This function parses one complete Redis command.
    let data = &buf[..];
    let (count, mut pos) = match read_int(data, 0, b'*')? {
        Some(v) => v,
        None => return Ok(None),
    };
    if count < 0 {
        return Err(ParseError::Invalid("negative array length"));
    }

    let mut args = Vec::with_capacity((count as usize).min(1024));
    for _ in 0..count {
        let (len, start) = match read_int(data, pos, b'$')? {
            Some(v) => v,
            None => return Ok(None),
        };
        if len < 0 {
            return Err(ParseError::Invalid("negative bulk length"));
        }
        let end = start + len as usize;

        if data.len() < end + 2 {
            return Ok(None); // body or trailing \r\n not here yet
        }
        if &data[end..end + 2] != b"\r\n" {
            //After every bulk string, RESP requires:
            return Err(ParseError::Invalid("missing CRLF after bulk string"));
        }
        let s = String::from_utf8(data[start..end].to_vec())
            .map_err(|_| ParseError::Invalid("non-utf8 bulk string"))?;
        args.push(s);
        pos = end + 2;
    }
    buf.advance(pos); // consume only after a full successful parse
    Ok(Some(args))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> BytesMut {
        BytesMut::from(s.as_bytes())
    }

    #[test]
    fn parses_ping() {
        let mut b = buf("*1\r\n$4\r\nPING\r\n");
        assert_eq!(parse(&mut b), Ok(Some(vec!["PING".to_string()])));
        assert!(b.is_empty());
    }

    #[test]
    fn parses_set() {
        let mut b = buf("*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
        assert_eq!(
            parse(&mut b),
            Ok(Some(vec!["SET".into(), "foo".into(), "bar".into()]))
        );
    }

    #[test]
    fn incomplete_returns_None_and_keeps_buffer() {
        let mut b = buf("*3\r\n$3\r\nSET\r\n$3\r\nfo");
        let before = b.len();
        assert_eq!(parse(&mut b), Ok(None));
        assert_eq!(b.len(), before);
    }

    #[test]
    fn two_commands_in_one_buffer() {
        let mut b = buf("*1\r\n$4\r\nPING\r\n*2\r\n$4\r\nECHO\r\n$2\r\nhi\r\n");
        assert_eq!(parse(&mut b), Ok(Some(vec!["PING".into()])));
        assert_eq!(parse(&mut b), Ok(Some(vec!["ECHO".into(), "hi".into()])));
        assert_eq!(parse(&mut b), Ok(None));
    }

    #[test]
    fn rejects_garbage() {
        let mut b = buf("hello\r\n");
        assert!(parse(&mut b).is_err());
    }
}
