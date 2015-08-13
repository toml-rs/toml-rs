use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::collections::hash_map::{Entry};
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::iter::Map;
use std::slice::Iter;
use std::fmt::Write;

pub mod parser;

struct TraversalPosition<'a> {
    direct: Option<&'a mut ValuesMap>,
    indirect: &'a mut HashMap<String, IndirectChild>
}

impl<'a> TraversalPosition<'a> {
    fn from_indirect(map: &mut HashMap<String, IndirectChild>)
                     -> TraversalPosition {
        TraversalPosition {
            direct: None,
            indirect: map 
        }
    }
}

// Main table representing the whole document.
// This structure preserves TOML document and its markup.
// Internally, a document is split in the following way:
//         +
//         |- values
//  a="b"  +
//         +
//  [foo]  |
//  x="y"  |- container_list
//  [bar]  |
//  c="d"  +
//         +
//         |- trail
//         +
pub struct RootTable {
    values: ValuesMap,
    // List of containers: tables and arrays that are present in the document.
    // Stored in the order they appear in the document.
    container_list: Vec<Rc<RefCell<Container>>>,
    // Index for quick traversal.
    container_index: HashMap<String, IndirectChild>,
    trail: String
}

impl RootTable {
    fn new() -> RootTable {
        RootTable {
            values: ValuesMap::new(),
            container_list: Vec::new(),
            container_index: HashMap::new(),
            trail: String::new(),
        }
    }

    // Converts editable document to a simplified representation.
    fn simplify(self) -> super::Table {
        self.values
            .simplify().into_iter()
            .chain(as_simplified_vec(&self.container_index))
            .collect()
    }

    pub fn serialize(&self, buf: &mut String) {
        self.values.serialize(buf);
        for table in self.container_list.iter() {
            table.borrow().serialize(buf);
        }
        buf.push_str(&*self.trail);
    }

    fn traverse(&mut self) -> TraversalPosition {
        TraversalPosition {
            direct: Some(&mut self.values),
            indirect: &mut self.container_index
        }
    }
}

// Order-preserving map of values that are directly contained in
// a root table, table or an array.
// A map is represented in the following way:
//         +
//         |- kvp_list[0]
//  a="b"  +
//         +
//         |- kvp_list[1]
//  x="y"  +
//         +
//         |- trail
//         +
struct ValuesMap {
    // key-value pairs stored in the order they appear in a document
    kvp_list: Vec<(Key, Rc<RefCell<FormattedValue>>)>,
    // Index for quick traversal.
    kvp_index: HashMap<String, Rc<RefCell<FormattedValue>>>,
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

    fn insert(&mut self, key: Key, value: FormattedValue) -> bool {
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

// Value plus leading and trailing auxiliary text.
// a =     "qwertyu"   \n
//    +---++-------++---+
//      |      |      |
//    lead   value  trail
struct FormattedValue {
    value: Value,
    // auxiliary text between the equality sign and the value
    lead: String,
    // auxiliary text after the value, up to and including the first newline
    trail: String
}

impl FormattedValue {
    fn new(lead: String, v: Value) -> FormattedValue {
        FormattedValue {
            value: v,
            lead: lead,
            trail: String::new()
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
    Array { values: Vec<FormattedValue>, trail: String },
    InlineTable { values: ContainerData, trail: String }
}

impl Value {
    fn new_table(map: ValuesMap, trail: String) -> Value {
        Value::InlineTable { 
            values: ContainerData {
                direct: map,
                indirect: HashMap::new()
            },
            trail: trail
        }
    }

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

    fn as_table(&mut self) -> &mut ContainerData {
        match *self {
            Value::InlineTable { ref mut values, .. } => values,
            _ => panic!()
        }
    }

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
                values.direct.serialize_inline(buf);
                buf.push_str(trail);
                buf.push('}');
            }
        }
    }
}

// Entry in the document index. This index is used for traversal (which is
// heavily used during parsing and adding new elements) and does not preserve
// ordering, just the structure 
// Some examples:
//  [a.b]
//  x="y"
// Document above contains single implicit table [a], which in turn contains
// single explicit table [a.b].
//  [a.b]
//  x="y"
//  [a]
// Document above contains single explicit table [a], which in turn contains
// single explicit table [a.b].
enum IndirectChild {
    ImplicitTable(HashMap<String, IndirectChild>),
    ExplicitTable(Rc<RefCell<Container>>),
    Array(Vec<Rc<RefCell<Container>>>)
}

impl IndirectChild {
    fn as_implicit(&mut self) -> &mut HashMap<String, IndirectChild> {
        if let IndirectChild::ImplicitTable(ref mut m) = *self { m }
        else { panic!() }
    }

    fn to_implicit(self) -> HashMap<String, IndirectChild> {
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

    fn is_implicit(&self) -> bool {
        match *self {
            IndirectChild::ImplicitTable (..) => true,
            _ => false
        }
    }
}

struct Container {
    data: ContainerData,
    // Path to the table, eg:
    //  [   a   .   b   ]
    //   +-----+ +-----+
    //      |       |
    //   keys[0] keys[1]
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

// Direct children are key-values that appear below container declaration,
// indirect children are are all other containers that are logically defined
// inside the container. For example:
//  [a]
//  x="y"
// [a.b]
// q="w"
// In the document above, table container [a] contains single direct child
// (x="y") and single indirect child (table container [a.b])
struct ContainerData {
    direct: ValuesMap,
    indirect: HashMap<String, IndirectChild>
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

    fn traverse(&mut self) -> TraversalPosition {
        TraversalPosition {
            direct: Some(&mut self.direct),
            indirect: &mut self.indirect
        }
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
    fn new(lead: String, key: (String, Option<String>), trail: String) -> Key {
        Key {
            escaped: key.0,
            raw: key.1,
            lead: lead,
            trail: trail,
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

fn as_simplified_vec(map: &HashMap<String, IndirectChild>)
                     -> Vec<(String, super::Value)> {
    map.iter().map(|(k, c)|(k.clone(), c.simplify())).collect()
}

trait Serializable {
    fn serialize(&self, buf: &mut String);
}

#[cfg(test)]
mod tests {
    use Parser;

    macro_rules! test_round_trip {
        ($name: ident, $text: expr) => (
            #[test]
            fn $name() {
                let mut p = Parser::new($text);
                let table = p.parse_doc().unwrap();
                let mut buf = String::new();
                table.serialize(&mut buf);
                if $text != buf {
                    panic!(format!("expected:\n{}\nactual:\n{}\n", $text, buf));
                }
            }
        )
    }

    test_round_trip!(empty, "  #asd \n ");
    test_round_trip!(single_table, "  #asd\t  \n [a]\n \t \n\n  #asdasdad\n ");
    test_round_trip!(root_key, " a = \"b\" \n ");
    test_round_trip!(array_with_values, " #as \n  \n  [[ a .b ]] \n  a = 1\n ");
    test_round_trip!(escaped, " str = \"adas \\\"Quote me\\\".sdas\" ");
    test_round_trip!(literal_string, " str = 'C:\\Users\\nodejs\\templates' ");
    test_round_trip!(array_empty," foo = [   ] ");
    test_round_trip!(array_non_empty, " foo = [ 1 , 2 ] ");
    test_round_trip!(array_trailing_comma, " foo = [ 1 , 2 , ] ");
    test_round_trip!(integer_with_sign, " foo = +10 ");
    test_round_trip!(underscore_integer, " foo = 1_000 ");
    test_round_trip!(inline_table, "\n a = { x = \"foo\"  , y = \"bar\"\t } ");
}