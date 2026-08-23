use bytes::{Buf, BufMut, BytesMut};

pub trait ByteBufExt {
    fn read_bool(&mut self) -> bool;
    fn read_string(&mut self) -> String;
    fn read_string_list(&mut self) -> Vec<String>;
    fn write_string(&mut self, s: &str);
    fn write_string_list(&mut self, list: &[String]);
}

impl ByteBufExt for BytesMut {
    fn read_bool(&mut self) -> bool {
        self.get_u8() != 0
    }

    fn read_string(&mut self) -> String {
        let len = self.get_i32() as usize;
        let bytes = self.split_to(len).to_vec();
        String::from_utf8(bytes).unwrap_or_else(|_| String::new())
    }

    fn read_string_list(&mut self) -> Vec<String> {
        let count = self.get_i32() as usize;
        (0..count).map(|_| self.read_string()).collect()
    }

    fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.put_i32(bytes.len() as i32);
        self.put_slice(bytes);
    }

    fn write_string_list(&mut self, list: &[String]) {
        self.put_i32(list.len() as i32);
        for s in list {
            self.write_string(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_buf_round_trip() {
        let mut buf = BytesMut::new();
        buf.write_string("hello");
        buf.write_string_list(&["a".to_string(), "b".to_string()]);
        buf.put_u8(1);
        buf.put_u8(0);

        assert_eq!(buf.read_string(), "hello");
        assert_eq!(buf.read_string_list(), vec!["a".to_string(), "b".to_string()]);
        assert!(buf.read_bool());
        assert!(!buf.read_bool());
        assert!(buf.is_empty());
    }
}
