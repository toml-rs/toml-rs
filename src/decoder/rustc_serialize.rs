use rustc_serialize;
use std::mem;

use super::{Decoder, DecodeError};
use super::DecodeErrorKind::*;
use {Value, Table};

impl rustc_serialize::Decoder for Decoder {
    type Error = DecodeError;
    fn read_nil(&mut self) -> Result<(), DecodeError> {
        match self.toml {
            Some(Value::String(ref s)) if s.len() == 0 => {}
            Some(Value::String(..)) => return Err(self.err(NilTooLong)),
            ref found => return Err(self.mismatch("string", found)),
        }
        self.toml.take();
        Ok(())
    }
    fn read_usize(&mut self) -> Result<usize, DecodeError> {
        self.read_i64().map(|i| i as usize)
    }
    fn read_u64(&mut self) -> Result<u64, DecodeError> {
        self.read_i64().map(|i| i as u64)
    }
    fn read_u32(&mut self) -> Result<u32, DecodeError> {
        self.read_i64().map(|i| i as u32)
    }
    fn read_u16(&mut self) -> Result<u16, DecodeError> {
        self.read_i64().map(|i| i as u16)
    }
    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        self.read_i64().map(|i| i as u8)
    }
    fn read_isize(&mut self) -> Result<isize, DecodeError> {
        self.read_i64().map(|i| i as isize)
    }
    fn read_i64(&mut self) -> Result<i64, DecodeError> {
        match self.toml {
            Some(Value::Integer(i)) => { self.toml.take(); Ok(i) }
            ref found => Err(self.mismatch("integer", found)),
        }
    }
    fn read_i32(&mut self) -> Result<i32, DecodeError> {
        self.read_i64().map(|i| i as i32)
    }
    fn read_i16(&mut self) -> Result<i16, DecodeError> {
        self.read_i64().map(|i| i as i16)
    }
    fn read_i8(&mut self) -> Result<i8, DecodeError> {
        self.read_i64().map(|i| i as i8)
    }
    fn read_bool(&mut self) -> Result<bool, DecodeError> {
        match self.toml {
            Some(Value::Boolean(b)) => { self.toml.take(); Ok(b) }
            ref found => Err(self.mismatch("bool", found)),
        }
    }
    fn read_f64(&mut self) -> Result<f64, DecodeError> {
        match self.toml {
            Some(Value::Float(f)) => Ok(f),
            ref found => Err(self.mismatch("float", found)),
        }
    }
    fn read_f32(&mut self) -> Result<f32, DecodeError> {
        self.read_f64().map(|f| f as f32)
    }
    fn read_char(&mut self) -> Result<char, DecodeError> {
        let ch = match self.toml {
            Some(Value::String(ref s)) if s.chars().count() == 1 =>
                s.chars().next().unwrap(),
            ref found => return Err(self.mismatch("string", found)),
        };
        self.toml.take();
        Ok(ch)
    }
    fn read_str(&mut self) -> Result<String, DecodeError> {
        match self.toml.take() {
            Some(Value::String(s)) => Ok(s),
            found => {
                let err = Err(self.mismatch("string", &found));
                self.toml = found;
                err
            }
        }
    }

    // Compound types:
    fn read_enum<T, F>(&mut self, _name: &str, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        f(self)
    }

    fn read_enum_variant<T, F>(&mut self, names: &[&str], mut f: F)
        -> Result<T, DecodeError>
        where F: FnMut(&mut Decoder, usize) -> Result<T, DecodeError>
    {
        let mut first_error = None;
        for i in 0..names.len() {
            let mut d = self.sub_decoder(self.toml.clone(), "");
            match f(&mut d, i) {
                Ok(t) => { self.toml = d.toml; return Ok(t) }
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }
        Err(first_error.unwrap_or_else(|| self.err(NoEnumVariants)))
    }
    fn read_enum_variant_arg<T, F>(&mut self, _a_idx: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        f(self)
    }

    fn read_enum_struct_variant<T, F>(&mut self, _names: &[&str], _f: F)
        -> Result<T, DecodeError>
        where F: FnMut(&mut Decoder, usize) -> Result<T, DecodeError>
    {
        panic!()
    }
    fn read_enum_struct_variant_field<T, F>(&mut self,
                                            _f_name: &str,
                                            _f_idx: usize,
                                            _f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        panic!()
    }

    fn read_struct<T, F>(&mut self, _s_name: &str, _len: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        match self.toml {
            Some(Value::Table(..)) => {
                let ret = try!(f(self));
                match self.toml {
                    Some(Value::Table(Table(ref t, _,))) if t.len() == 0 => {}
                    _ => return Ok(ret)
                }
                self.toml.take();
                Ok(ret)
            }
            ref found => Err(self.mismatch("table", found)),
        }
    }
    fn read_struct_field<T, F>(&mut self, f_name: &str, _f_idx: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        let field = format!("{}", f_name);
        let toml = match self.toml {
            Some(Value::Table(Table(ref mut table, _))) => {
                table.remove(&field)
                    .or_else(|| table.remove(&f_name.replace("_", "-")))
            },
            ref found => return Err(self.mismatch("table", found)),
        };
        let mut d = self.sub_decoder(toml, f_name);
        let ret = try!(f(&mut d));
        if let Some(value) = d.toml {
            if let Some(Value::Table(Table(ref mut table, _))) = self.toml {
                table.insert(field, value);
            }
        }
        Ok(ret)
    }

    fn read_tuple<T, F>(&mut self, tuple_len: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        self.read_seq(move |d, len| {
            assert!(len == tuple_len,
                    "expected tuple of length `{}`, found tuple \
                         of length `{}`", tuple_len, len);
            f(d)
        })
    }
    fn read_tuple_arg<T, F>(&mut self, a_idx: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        self.read_seq_elt(a_idx, f)
    }

    fn read_tuple_struct<T, F>(&mut self, _s_name: &str, _len: usize, _f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        panic!()
    }
    fn read_tuple_struct_arg<T, F>(&mut self, _a_idx: usize, _f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        panic!()
    }

    // Specialized types:
    fn read_option<T, F>(&mut self, mut f: F)
        -> Result<T, DecodeError>
        where F: FnMut(&mut Decoder, bool) -> Result<T, DecodeError>
    {
        match self.toml {
            Some(..) => f(self, true),
            None => f(self, false),
        }
    }

    fn read_seq<T, F>(&mut self, f: F) -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder, usize) -> Result<T, DecodeError>
    {
        let len = match self.toml {
            Some(Value::Array(ref arr)) => arr.len(),
            None => 0,
            ref found => return Err(self.mismatch("array", found)),
        };
        let ret = try!(f(self, len));
        match self.toml {
            Some(Value::Array(ref mut arr)) => {
                arr.retain(|slot| slot.as_integer() != Some(0));
                if arr.len() != 0 { return Ok(ret) }
            }
            _ => return Ok(ret)
        }
        self.toml.take();
        Ok(ret)
    }
    fn read_seq_elt<T, F>(&mut self, idx: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        let toml = match self.toml {
            Some(Value::Array(ref mut arr)) => {
                mem::replace(&mut arr[idx], Value::Integer(0))
            }
            ref found => return Err(self.mismatch("array", found)),
        };
        let mut d = self.sub_decoder(Some(toml), "");
        let ret = try!(f(&mut d));
        match d.toml {
            Some(toml) => match self.toml {
                Some(Value::Array(ref mut arr)) => arr[idx] = toml,
                _ => {}
            },
            _ => {}
        }
        Ok(ret)
    }

    fn read_map<T, F>(&mut self, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder, usize) -> Result<T, DecodeError>
    {
        let len = match self.toml {
            Some(Value::Table(Table(ref table, _))) => table.len(),
            ref found => return Err(self.mismatch("table", found)),
        };
        let ret = try!(f(self, len));
        self.toml.take();
        Ok(ret)
    }
    fn read_map_elt_key<T, F>(&mut self, idx: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        match self.toml {
            Some(Value::Table(ref table)) => {
                match table.0.iter().skip(idx).next() {
                    Some((key, _)) => {
                        let val = Value::String(format!("{}", key));
                        f(&mut self.sub_decoder(Some(val), &**key))
                    }
                    None => Err(self.err(ExpectedMapKey(idx))),
                }
            }
            ref found => Err(self.mismatch("table", found)),
        }
    }
    fn read_map_elt_val<T, F>(&mut self, idx: usize, f: F)
        -> Result<T, DecodeError>
        where F: FnOnce(&mut Decoder) -> Result<T, DecodeError>
    {
        match self.toml {
            Some(Value::Table(ref table)) => {
                match table.0.iter().skip(idx).next() {
                    Some((_, value)) => {
                        // XXX: this shouldn't clone
                        f(&mut self.sub_decoder(Some(value.clone()), ""))
                    }
                    None => Err(self.err(ExpectedMapElement(idx))),
                }
            }
            ref found => Err(self.mismatch("table", found)),
        }
    }

    fn error(&mut self, err: &str) -> DecodeError {
        DecodeError {
            field: self.cur_field.clone(),
            kind: ApplicationError(format!("{}", err))
        }
    }
}
