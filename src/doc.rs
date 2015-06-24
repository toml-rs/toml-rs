use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::collections::hash_map::{Entry};
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::iter::Map;
use std::slice::Iter;
use std::fmt::Write;

use parser::{ParserError};
use {Table};

pub type IndirectChildrenMap = HashMap<String, IndirectChild>;

fn convert_indirect_map(map: &IndirectChildrenMap) -> Vec<(String, super::Value)> {
    map.iter().map(|(k, c)|(k.clone(), c.convert())).collect()
}

pub trait Printable {
    fn print(&self, buf: &mut String);
}

pub struct RootTable {
    pub values: KvpMap,
    pub table_list: Vec<Rc<RefCell<Container>>>,
    pub table_index: IndirectChildrenMap,
    pub trail: String,
} impl RootTable {
    pub fn new() -> RootTable {
        RootTable {
            values: KvpMap::new(),
            table_list: Vec::new(),
            table_index: HashMap::new(),
            trail: String::new(),
        }
    }
    pub fn convert(self) -> Table {
        self.values.convert().into_iter().chain(convert_indirect_map(&self.table_index)).collect()
    }

    pub fn print(&self, buf: &mut String) {
        self.values.print(buf);
        for table in self.table_list.iter() {
            table.borrow().print(buf);
        }
        buf.push_str(&*self.trail);
    }
}

pub struct KvpMap {
    kvp_list: Vec<(Key, Rc<RefCell<Formatted<Value>>>)>,
    pub kvp_index: HashMap<String, Rc<RefCell<Formatted<Value>>>>,
    trail: String
} impl KvpMap {
    pub fn new() -> KvpMap {
        KvpMap {
            kvp_list: Vec::new(),
            kvp_index: HashMap::new(),
            trail: String::new()
        }
    }
    pub fn set_trail(&mut self, s: &str) { self.trail = s.to_string() }

    pub fn contains_key(&self, s:&str) -> bool {
        self.kvp_index.contains_key(&*s)
    }

    pub fn insert(&mut self, key: Key, value: Formatted<Value>) -> bool {
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

    pub fn set_last_value_trail(&mut self, t: &str) {
        self.kvp_list.last().as_ref().unwrap().1.borrow_mut().trail 
            = t.to_string();
    }

    fn convert(&self) -> Vec<(String, super::Value)> {
        self.kvp_list.iter().map(|&(ref k, ref v)| (k.escaped.clone(), v.borrow().value.as_value())).collect()
    }

    fn print(&self, buf: &mut String) {
        for &(ref key, ref value) in self.kvp_list.iter() {
            key.print(buf);
            buf.push('=');
            value.borrow().print(buf);
        }
    }
}

pub struct Formatted<T> where T: Printable {
    pub value: T,
    pub lead: String,
    pub trail: String
} impl<T:Printable> Formatted<T> {
    pub fn new(lead: String, v: T) -> Formatted<T> {
        Formatted {
            value: v,
            lead: lead,
            trail: String::new()
        }
    }

    pub fn map<U:Printable, F:Fn(T)->U>(self, f:F) -> Formatted<U> {
        Formatted {
            value: f(self.value),
            lead: self.lead,
            trail: self.trail
        }
    }

    fn print(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        self.value.print(buf);
        buf.push_str(&*self.trail);
    }
}

pub enum Value {
    String{ raw: String, escaped: String },
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(String),
    Array(Vec<Formatted<Value>>),
    InlineTable(KvpMap),
} impl Value {
    pub fn as_value(&self) -> super::Value {
        match self {
            &Value::String{ escaped: ref x, .. } => super::Value::String(x.clone()),
            &Value::Integer(x) => super::Value::Integer(x),
            &Value::Float(x) => super::Value::Float(x),
            &Value::Boolean(x) => super::Value::Boolean(x),
            &Value::Datetime(ref x) => super::Value::Datetime(x.clone()),
            &Value::Array(ref vec) => super::Value::Array(vec.iter().map(|fv| fv.value.as_value()).collect()),
            &Value::InlineTable(ref x) => { 
                super::Value::Table(x.convert().into_iter().collect())
            },
        }
    }

    pub fn type_str(&self) -> &'static str {
        match *self {
            Value::String{..} => "string",
            Value::Integer(..) => "integer",
            Value::Float(..) => "float",
            Value::Boolean(..) => "boolean",
            Value::Datetime(..) => "datetime",
            Value::Array(..) => "array",
            Value::InlineTable(..) => "table",
        }
    }

    pub fn is_table(&self) -> bool{
        match *self {
            Value::InlineTable(_) => true,
            _ => false
        }
    }

    pub fn as_table(&mut self) -> &mut KvpMap {
        match *self {
            Value::InlineTable(ref mut x) => x,
            _ => panic!()
        }
    }
} impl Printable for Value {
    fn print(&self, buf: &mut String) {
        match *self {
            Value::String{ raw: ref s, .. } => {
                buf.push_str(&*s);
            }
            Value::Integer(s) => { write!(buf, "{}", s).unwrap(); }
            _ => panic!()
        }
    }
}

pub enum IndirectChild {
    ImplicitTable(IndirectChildrenMap),
    ExplicitTable(Rc<RefCell<Container>>),
    Array(Vec<Rc<RefCell<Container>>>)
} impl IndirectChild {
    pub fn as_implicit(&mut self) -> &mut IndirectChildrenMap {
        if let IndirectChild::ImplicitTable(ref mut m) = *self { m }
        else { panic!() }
    }
    pub fn to_implicit(self) -> IndirectChildrenMap {
        if let IndirectChild::ImplicitTable(m) = self { m }
        else { panic!() }
    }

    fn convert(&self) -> super::Value {
        match self {
            &IndirectChild::ImplicitTable(ref m)
                => super::Value::Table(convert_indirect_map(m).into_iter().collect()),
            &IndirectChild::ExplicitTable(ref m) => m.borrow().convert(),
            &IndirectChild::Array(ref vec) => super::Value::Array(vec.iter().map(|m| m.borrow().convert()).collect()),
        }
    }
}

pub struct Container {
    pub data: ContainerData,
    pub keys: Vec<Key>,
    kind: ContainerKind,
    pub lead: String,
} impl Container {

    pub fn new_array(data: ContainerData, ks: Vec<Key>, lead: String)
                     -> Container {
        Container::new(ContainerKind::Array, data, ks, lead)
    }

    pub fn new_table(data: ContainerData, ks: Vec<Key>, lead: String)
                     -> Container {
        Container::new(ContainerKind::Table, data, ks, lead)
    }

    fn new(kind: ContainerKind, data: ContainerData, ks: Vec<Key>, lead: String)
           -> Container {
        Container {
            data: data,
            keys: ks,
            kind: kind,
            lead: lead,
        }
    }

    fn print(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        if self.keys.len() > 0 {
            match self.kind {
                ContainerKind::Table => buf.push_str("["),
                ContainerKind::Array => buf.push_str("[["),
            }
            for (i, key) in self.keys.iter().enumerate() {
                key.print(buf);
                if i < self.keys.len() - 1 { buf.push('.') }
            }
            match self.kind {
                ContainerKind::Table => buf.push_str("]"),
                ContainerKind::Array => buf.push_str("]]"),
            }
        }
        self.data.print(buf);
    }

    fn convert(&self) -> super::Value {
        super::Value::Table(self.data.convert().into_iter().collect())
    }
}

pub struct ContainerData {
    pub direct: KvpMap,
    pub indirect: IndirectChildrenMap
} impl ContainerData {
    pub fn new() -> ContainerData {
        ContainerData {
            direct: KvpMap::new(),
            indirect: HashMap::new()
        }
    }

    fn print(&self, buf: &mut String) {
        self.direct.print(buf);
    }
    fn convert(&self) -> Vec<(String, super::Value)> {
        self.direct.convert().into_iter().chain(convert_indirect_map(&self.indirect)).collect()
    }
}


#[derive(PartialEq, Eq, Hash)]
enum ContainerKind {
    Table,
    Array,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Key {
    pub escaped: String,
    pub raw: Option<String>,
    lead: String,
    trail: String
} impl Key {
    pub fn new(lead: String, key: (String, Option<String>), trail: &str) -> Key{
        Key {
            escaped: key.0,
            raw: key.1,
            lead: lead,
            trail: trail.to_string(),
        }
    }

    fn print(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        match self.raw {
            Some(ref str_buf) => buf.push_str(&*str_buf),
            None => buf.push_str(&*self.escaped)
        };
        buf.push_str(&*self.trail);
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use Parser;

    macro_rules! round_trip {
        ($text: expr) => ({
            let mut p = Parser::new($text);
            let table = p.parse_doc().unwrap();
            let mut buf = String::new();
            table.print(&mut buf);
            if $text != buf {
                panic!(format!("expected:\n{}\nactual:\n{}\n", $text, buf));
            }
        })
    }

    #[test]
    fn empty() { round_trip!("  #asd \n ") }
    #[test]
    fn single_table() {round_trip!("  #asd\t  \n [a]\n \t \n\n  #asdasdad\n ")}
    #[test]
    fn root_key() { round_trip!(" a = \"b\" \n ") }
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

}