#![crate_type = "lib"]
#![feature(macro_rules)]

use std::collections::HashMap;

pub use parser::{Parser, Error};

mod parser;
#[cfg(test)]
mod test;

pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(String),
    Array(Array),
    Table(Table),
}

pub type Array = Vec<Value>;
pub type Table = HashMap<String, Value>;

impl Value {
    fn same_type(&self, other: &Value) -> bool {
        match (self, other) {
            (&String(..), &String(..)) |
            (&Integer(..), &Integer(..)) |
            (&Float(..), &Float(..)) |
            (&Boolean(..), &Boolean(..)) |
            (&Datetime(..), &Datetime(..)) |
            (&Array(..), &Array(..)) |
            (&Table(..), &Table(..)) => true,

            _ => false,
        }
    }

    fn type_str(&self) -> &'static str {
        match *self {
            String(..) => "string",
            Integer(..) => "integer",
            Float(..) => "float",
            Boolean(..) => "boolean",
            Datetime(..) => "datetime",
            Array(..) => "array",
            Table(..) => "table",
        }
    }
}
