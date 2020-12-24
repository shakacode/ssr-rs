// This is slightly modified version of formatter taken from
// https://github.com/xd009642/tarpaulin/blob/fd7059131cf7f838a68e681a9ddeefb26a8adf7c/src/report/safe_json.rs
// All copyrights belong to the author of this implementation.

use std::{default::Default, io};

use serde_json::{
    ser::{CharEscape, CompactFormatter, Formatter},
    Serializer,
};

struct JsonFormatter(CompactFormatter);

impl Default for JsonFormatter {
    fn default() -> Self {
        JsonFormatter(CompactFormatter)
    }
}

impl Formatter for JsonFormatter {
    fn write_string_fragment<W: ?Sized>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: io::Write,
    {
        let mut start = 0;
        let mut code_length = 0;
        for char in fragment.chars() {
            code_length += char.len_utf8();
            let escape = match char {
                '<' | '>' | '&' => CharEscape::AsciiControl(char as u8),
                _ => continue,
            };
            if start < code_length - 1 {
                self.0
                    .write_string_fragment(writer, &fragment[start..code_length - 1])?;
            }

            self.write_char_escape(writer, escape)?;

            start = code_length;
        }

        if start < code_length {
            self.0.write_string_fragment(writer, &fragment[start..])?;
        }

        Ok(())
    }
}

pub fn to_vec<T: serde::Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut writer = Vec::new();
    let mut ser = Serializer::with_formatter(&mut writer, JsonFormatter::default());
    value.serialize(&mut ser)?;
    Ok(writer)
}
