use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::collections::hash_map::{Entry};
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::iter::Map;
use std::slice::Iter;

use parser::{ParserError};
use {Table};

pub type IndirectChildrenMap = HashMap<String, IndirectChild>;

fn convert_indirect_map(map: &IndirectChildrenMap) -> Vec<(String, super::Value)> {
    map.iter().map(|(k, c)|(k.clone(), c.convert())).collect()
}

pub struct RootTable {
    pub values: KvpMap,
    pub table_list: Vec<Rc<RefCell<Container>>>,
    pub table_index: IndirectChildrenMap,
    lead: String,
    trail: String
} impl RootTable {
    pub fn new() -> RootTable {
        RootTable {
            values: KvpMap::new(),
            table_list: Vec::new(),
            table_index: HashMap::new(),
            lead: String::new(),
            trail: String::new(),
        }
    }
    pub fn convert(self) -> Table {
        self.values.convert().into_iter().chain(convert_indirect_map(&self.table_index)).collect()
    }
    pub fn set_trail(&mut self, t: &str) {
        self.trail = t.to_string()
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
        match self.kvp_index.entry(key.key.clone()) {
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
        self.kvp_list.iter().map(|&(ref k, ref v)| (k.key.clone(), v.borrow().value.as_value())).collect()
    }
}

pub struct Formatted<T> {
    pub value: T,
    pub lead: String,
    pub trail: String
} impl<T> Formatted<T> {
    pub fn new(lead: String, v: T) -> Formatted<T> {
        Formatted {
            value: v,
            lead: lead,
            trail: String::new()
        }
    }

    pub fn map<U, F:Fn(T)->U>(self, f:F) -> Formatted<U> {
        Formatted {
            value: f(self.value),
            lead: self.lead,
            trail: self.trail
        }
    }
}

pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(String),
    Array(Vec<Formatted<Value>>),
    InlineTable(KvpMap),
} impl Value {
    pub fn as_value(&self) -> super::Value {
        match self {
            &Value::String(ref x) => super::Value::String(x.clone()),
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
            Value::String(..) => "string",
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
    lead: String,
    trail: String
} impl Container {

    pub fn new_array(data: ContainerData, ks: Vec<Key>, s: String) -> Container{
        Container::new(ContainerKind::Array, data, ks, s)
    }

    pub fn new_table(data: ContainerData, ks: Vec<Key>, s: String) -> Container{
        Container::new(ContainerKind::Table, data, ks, s)
    }

    fn new(kind: ContainerKind, data: ContainerData, ks: Vec<Key>, s: String)
           -> Container {
        Container {
            data: data,
            keys: ks,
            kind: kind,
            lead: s,
            trail: String::new()
        }
    }

    fn print(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        if self.keys.len() > 0 {
            for (i, key) in self.keys.iter().enumerate() {
                key.print(buf);
                if i < self.keys.len() - 1 { buf.push('.') }
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
    pub key: String,
    lead: String,
    trail: String
} impl Key {
    pub fn new(lead: String, key: String, trail: &str) -> Key {
        Key {
            lead: lead,
            key: key,
            trail: trail.to_string(),
        }
    }

    fn print(&self, buf: &mut String) {
        buf.push_str(&*self.lead);
        buf.push_str(&*self.key);
        buf.push_str(&*self.trail);
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        unimplemented!()
    }
}