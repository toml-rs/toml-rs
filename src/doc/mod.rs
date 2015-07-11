use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::collections::hash_map::{Entry};
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::iter::Map;
use std::slice::Iter;
use std::fmt::Write;

pub mod parser;

// Main table representing the whole document.
// This structure preserves TOML document and its markup.
pub struct RootTable {
    values: ValuesMap,
    // List of containers: tables and arrays that are present in the document.
    // Stored in the order they appear in the document.
    table_list: Vec<Rc<RefCell<Container>>>,
    // Index over contained structure for quick traversal.
    table_index: IndirectChildrenMap,
    // Trailing auxiliary text.
    trail: String
}

impl RootTable {
    fn new() -> RootTable {
        RootTable {
            values: ValuesMap::new(),
            table_list: Vec::new(),
            table_index: HashMap::new(),
            trail: String::new(),
        }
    }

    // Converts editable document to a simplified representation.
    fn simplify(self) -> super::Table {
        self.values
            .simplify().into_iter()
            .chain(as_simplified_vec(&self.table_index))
            .collect()
    }

    pub fn serialize(&self, buf: &mut String) {
        self.values.serialize(buf);
        for table in self.table_list.iter() {
            table.borrow().serialize(buf);
        }
        buf.push_str(&*self.trail);
    }
}

// Indexed map of values that are directly contained in a table.
struct ValuesMap {
    kvp_list: Vec<(Key, Rc<RefCell<Formatted<Value>>>)>,
    kvp_index: HashMap<String, Rc<RefCell<Formatted<Value>>>>,
    trail: String
}

impl ValuesMap {
    fn new() -> ValuesMap {
        ValuesMap {
            kvp_list: Vec::new(),
            kvp_index: HashMap::new(),
            trail: String::new()
        }
    }

    fn insert(&mut self, key: Key, value: Formatted<Value>) -> bool {
        let value = Rc::new(RefCell::new(value));
        match self.kvp_index.entry(key.escaped.clone()) {
            Entry::Occupied(_) => return false,
            Entry::Vacant(entry) => {
                entry.insert(value.clone())
            }
        };
        self.kvp_list.push((key,value));
        true
    }

    fn set_last_value_trail(&mut self, s: String) {
        self.kvp_list.last().as_ref().unwrap().1.borrow_mut().trail = s;
    }

    fn simplify(&self) -> Vec<(String, super::Value)> {
        self.kvp_list
            .iter()
            .map(|&(ref k, ref v)| {
                (k.escaped.clone(), v.borrow().value.as_simple_value())
            })
            .collect()
    }

    fn serialize(&self, buf: &mut String) {
        for &(ref key, ref value) in self.kvp_list.iter() {
            key.serialize(buf);
            buf.push('=');
            value.borrow().serialize(buf);
        }
    }

    fn serialize_inline(&self, buf: &mut String) {
        for (idx, &(ref key, ref value)) in self.kvp_list.iter().enumerate() {
            key.serialize(buf);
            buf.push('=');
            value.borrow().serialize(buf);
            if idx < self.kvp_list.len() - 1 {
                buf.push(',');
            }
        }
    }
}

struct Formatted<T: Serializable> {
    value: T,
    lead: String,
    trail: String
}

impl<T: Serializable> Formatted<T> {
    fn new(lead: String, v: T) -> Formatted<T> {
        Formatted {
            value: v,
            lead: lead,
            trail: String::new()
        }
    }

    fn map<U: Serializable, F: Fn(T) -> U>(self, f: F) -> Formatted<U> {
        Formatted {
            value: f(self.value),
            lead: self.lead,
            trail: self.trail
        }
    }

    fn serialize(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        self.value.serialize(buf);
        buf.push_str(&*self.trail);
    }
}

enum Value {
    String { escaped: String, raw: String },
    Integer { parsed: i64, raw: String },
    Float { parsed: f64, raw: String },
    Boolean(bool),
    Datetime(String),
    Array { values: Vec<Formatted<Value>>, trail: String },
    InlineTable { values: ValuesMap, trail: String }
}

impl Value {
    fn as_simple_value(&self) -> super::Value {
        match self {
            &Value::String { ref escaped, .. } => {
                super::Value::String(escaped.clone())
            }
            &Value::Integer { parsed, .. } => super::Value::Integer(parsed),
            &Value::Float { parsed, .. } => super::Value::Float(parsed),
            &Value::Boolean(x) => super::Value::Boolean(x),
            &Value::Datetime(ref x) => super::Value::Datetime(x.clone()),
            &Value::Array { ref values, .. } => {
                let values = values
                    .iter()
                    .map(|fv| fv.value.as_simple_value())
                    .collect();
                super::Value::Array(values)
            }
            &Value::InlineTable { ref values, .. } => { 
                super::Value::Table(values.simplify().into_iter().collect())
            },
        }
    }

    fn type_str(&self) -> &'static str {
        match *self {
            Value::String {..} => "string",
            Value::Integer {..} => "integer",
            Value::Float {..} => "float",
            Value::Boolean(..) => "boolean",
            Value::Datetime(..) => "datetime",
            Value::Array {..} => "array",
            Value::InlineTable {..} => "table",
        }
    }

    fn is_table(&self) -> bool{
        match *self {
            Value::InlineTable {..} => true,
            _ => false
        }
    }

    fn as_table(&mut self) -> &mut ValuesMap {
        match *self {
            Value::InlineTable { ref mut values, .. } => values,
            _ => panic!()
        }
    }
}

impl Serializable for Value {
    fn serialize(&self, buf: &mut String) {
        match *self {
            Value::String { ref raw, .. } => buf.push_str(raw),
            Value::Integer { ref raw, .. } => buf.push_str(raw),
            Value::Float { ref raw, .. } => buf.push_str(raw),
            Value::Boolean(b) => buf.push_str(if b {"true"} else {"false"}),
            Value::Datetime(ref s) => buf.push_str(s),
            Value::Array { ref values, ref trail } => {
                buf.push('[');
                for (idx, value) in values.iter().enumerate() {
                    value.serialize(buf);
                    if idx != values.len() - 1 { buf.push(',') }
                }
                buf.push_str(trail);
                buf.push(']');
            }
            Value::InlineTable { ref values, ref trail } => {
                buf.push('{');
                values.serialize_inline(buf);
                buf.push_str(trail);
                buf.push('}');
            }
        }
    }
}

enum IndirectChild {
    ImplicitTable(IndirectChildrenMap),
    ExplicitTable(Rc<RefCell<Container>>),
    Array(Vec<Rc<RefCell<Container>>>)
}

impl IndirectChild {
    fn as_implicit(&mut self) -> &mut IndirectChildrenMap {
        if let IndirectChild::ImplicitTable(ref mut m) = *self { m }
        else { panic!() }
    }

    fn to_implicit(self) -> IndirectChildrenMap {
        if let IndirectChild::ImplicitTable(m) = self { m }
        else { panic!() }
    }

    fn simplify(&self) -> super::Value {
        match self {
            &IndirectChild::ImplicitTable(ref m) => {
                let kvp_vec = as_simplified_vec(m);
                super::Value::Table(kvp_vec.into_iter().collect())
            }
            &IndirectChild::ExplicitTable(ref m) => m.borrow().simplify(),
            &IndirectChild::Array(ref vec) => {
                let values = vec
                    .iter()
                    .map(|m| m.borrow().simplify())
                    .collect();
                super::Value::Array(values)
            }
        }
    }
}

struct Container {
    data: ContainerData,
    keys: Vec<Key>,
    kind: ContainerKind,
    lead: String,
}

impl Container {
    fn new_array(data: ContainerData, ks: Vec<Key>, lead: String)
                     -> Container {
        Container { 
            data: data,
            keys: ks,
            lead: lead,
            kind: ContainerKind::Array,
        }
    }

    fn new_table(data: ContainerData, ks: Vec<Key>, lead: String)
                     -> Container {
        Container { 
            data: data,
            keys: ks,
            lead: lead,
            kind: ContainerKind::Table,
        }
    }

    fn serialize(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        if self.keys.len() > 0 {
            match self.kind {
                ContainerKind::Table => buf.push_str("["),
                ContainerKind::Array => buf.push_str("[["),
            }
            for (i, key) in self.keys.iter().enumerate() {
                key.serialize(buf);
                if i < self.keys.len() - 1 { buf.push('.') }
            }
            match self.kind {
                ContainerKind::Table => buf.push_str("]"),
                ContainerKind::Array => buf.push_str("]]"),
            }
        }
        self.data.serialize(buf);
    }

    fn simplify(&self) -> super::Value {
        super::Value::Table(self.data.simplify().into_iter().collect())
    }
}

struct ContainerData {
    direct: ValuesMap,
    indirect: IndirectChildrenMap
}

impl ContainerData {
    fn new() -> ContainerData {
        ContainerData {
            direct: ValuesMap::new(),
            indirect: HashMap::new()
        }
    }

    fn serialize(&self, buf: &mut String) {
        self.direct.serialize(buf);
    }
    fn simplify(&self) -> Vec<(String, super::Value)> {
        self.direct
            .simplify()
            .into_iter()
            .chain(as_simplified_vec(&self.indirect))
            .collect()
    }
}

#[derive(PartialEq, Eq, Hash)]
enum ContainerKind {
    Table,
    Array,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct Key {
    escaped: String,
    raw: Option<String>,
    lead: String,
    trail: String
}

impl Key {
    fn new(lead: String, key: (String, Option<String>), trail: &str)
               -> Key{
        Key {
            escaped: key.0,
            raw: key.1,
            lead: lead,
            trail: trail.to_string(),
        }
    }

    fn serialize(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        match self.raw {
            Some(ref str_buf) => buf.push_str(&*str_buf),
            None => buf.push_str(&*self.escaped)
        };
        buf.push_str(&*self.trail);
    }
}

type IndirectChildrenMap = HashMap<String, IndirectChild>;

fn as_simplified_vec(map: &IndirectChildrenMap) -> Vec<(String, super::Value)> {
    map.iter().map(|(k, c)|(k.clone(), c.simplify())).collect()
}

trait Serializable {
    fn serialize(&self, buf: &mut String);
}

#[cfg(test)]
mod tests {
    use Parser;

    macro_rules! round_trip {
        ($text: expr) => ({
            let mut p = Parser::new($text);
            let table = p.parse_doc().unwrap();
            let mut buf = String::new();
            table.serialize(&mut buf);
            if $text != buf {
                panic!(format!("expected:\n{}\nactual:\n{}\n", $text, buf));
            }
        })
    }

    #[test]
    fn empty() {
        round_trip!("  #asd \n ")
    }
    #[test]
    fn single_table() {
        round_trip!("  #asd\t  \n [a]\n \t \n\n  #asdasdad\n ")
    }
    #[test]
    fn root_key() {
        round_trip!(" a = \"b\" \n ")
    }
    #[test]
    fn array_with_values() {
        round_trip!(" #asd \n  \n   [[ a . b ]]  \n  \n  a = 1 \n \n ")
    }
    #[test]
    fn escaped() {
        round_trip!(" str = \"adas \\\"You can quote me\\\".sdas\" ")
    }
    #[test]
    fn literal_string() {
        round_trip!(" str = 'C:\\Users\\nodejs\\templates' ")
    }
    #[test]
    fn array_empty() { 
        round_trip!(" foo = [   ] ")
    }
    #[test]
    fn array_non_empty() {
        round_trip!(" foo = [ 1 , 2 ] ")
    }
    #[test]
    fn array_trailing_comma() {
        round_trip!(" foo = [ 1 , 2 , ] ")
    }
    #[test]
    fn integer_with_sign() {
        round_trip!(" foo = +10 ")
    }
    #[test]
    fn underscore_integer() {
        round_trip!(" foo = 1_000 ")
    }
    #[test]
    fn inline_table() {
        round_trip!("\n a = { x = \"foo\"  , y = \"bar\"\t } ")
    }
}