use std::fmt::Write;
use std::fmt;
use std::string::{String as StdString};

use Table as TomlTable;
use Value::{self, String, Integer, Float, Boolean, Datetime, Array, Table};

struct Printer<'a, 'b:'a> {
    output: &'a mut fmt::Formatter<'b>,
    stack: Vec<&'a str>,
}

struct Key<'a>(&'a [&'a str]);

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            String(ref s) => write_str(f, s),
            Integer(i) => write!(f, "{}", i),
            Float(fp) => {
                try!(write!(f, "{}", fp));
                if fp % 1.0 == 0.0 { try!(write!(f, ".0")) }
                Ok(())
            }
            Boolean(b) => write!(f, "{}", b),
            Datetime(ref s) => write!(f, "{}", s),
            Table(ref t) => {
                let mut p = Printer { output: f, stack: Vec::new() };
                p.print(t)
            }
            Array(ref a) => {
                try!(write!(f, "["));
                for (i, v) in a.iter().enumerate() {
                    if i != 0 { try!(write!(f, ", ")); }
                    try!(write!(f, "{}", v));
                }
                write!(f, "]")
            }
        }
    }
}

fn write_str(f: &mut fmt::Formatter, s: &str) -> fmt::Result {
    try!(write!(f, "\""));
    for ch in s.chars() {
        match ch {
            '\u{8}' => try!(write!(f, "\\b")),
            '\u{9}' => try!(write!(f, "\\t")),
            '\u{a}' => try!(write!(f, "\\n")),
            '\u{c}' => try!(write!(f, "\\f")),
            '\u{d}' => try!(write!(f, "\\r")),
            '\u{22}' => try!(write!(f, "\\\"")),
            '\u{5c}' => try!(write!(f, "\\\\")),
            c if c < '\u{1f}' => {
                try!(write!(f, "\\u{:04}", ch as u32))
            }
            ch => try!(write!(f, "{}", ch)),
        }
    }
    write!(f, "\"")
}

impl<'a, 'b> Printer<'a, 'b> {
    fn print(&mut self, table: &'a TomlTable) -> fmt::Result {
        let mut space_out_first = false;
        for (k, v) in table.iter() {
            match *v {
                Table(..) => continue,
                Array(ref a) => {
                    if let Some(&Table(..)) = a.first() {
                        continue;
                    }
                }
                _ => {}
            }
            space_out_first = true;
            try!(writeln!(self.output, "{} = {}", Key(&[k]), v));
        }
        for (i, (k, v)) in table.iter().enumerate() {
            match *v {
                Table(ref inner) => {
                    self.stack.push(k);
                    if space_out_first || i != 0 {
                        try!(write!(self.output, "\n"));
                    }
                    try!(writeln!(self.output, "[{}]", Key(&self.stack)));
                    try!(self.print(inner));
                    self.stack.pop();
                }
                Array(ref inner) => {
                    match inner.first() {
                        Some(&Table(..)) => {}
                        _ => continue
                    }
                    self.stack.push(k);
                    for (j, inner) in inner.iter().enumerate() {
                        if space_out_first || i != 0 || j != 0 {
                            try!(write!(self.output, "\n"));
                        }
                        try!(writeln!(self.output, "[[{}]]", Key(&self.stack)));
                        match *inner {
                            Table(ref inner) => try!(self.print(inner)),
                            _ => panic!("non-heterogeneous toml array"),
                        }
                    }
                    self.stack.pop();
                }
                _ => {},
            }
        }
        Ok(())
    }
}

impl<'a> fmt::Display for Key<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, part) in self.0.iter().enumerate() {
            if i != 0 { try!(write!(f, ".")); }
            let ok = part.chars().all(|c| {
                match c {
                    'a' ... 'z' |
                    'A' ... 'Z' |
                    '0' ... '9' |
                    '-' | '_' => true,
                    _ => false,
                }
            });
            if ok {
                try!(write!(f, "{}", part));
            } else {
                try!(write_str(f, part));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(warnings)]
mod tests {
    use Value;
    use Value::{String, Integer, Float, Boolean, Datetime, Array, Table};
    use std::collections::BTreeMap;

    macro_rules! map( ($($k:expr => $v:expr),*) => ({
        let mut _m = BTreeMap::new();
        $(_m.insert($k.to_string(), $v);)*
        _m
    }) );

    #[test]
    fn simple_show() {
        assert_eq!(String("foo".to_string()).to_string(),
                   "\"foo\"");
        assert_eq!(Integer(10).to_string(),
                   "10");
        assert_eq!(Float(10.0).to_string(),
                   "10.0");
        assert_eq!(Float(2.4).to_string(),
                   "2.4");
        assert_eq!(Boolean(true).to_string(),
                   "true");
        assert_eq!(Datetime("test".to_string()).to_string(),
                   "test");
        assert_eq!(Array(vec![]).to_string(),
                   "[]");
        assert_eq!(Array(vec![Integer(1), Integer(2)]).to_string(),
                   "[1, 2]");
    }

    #[test]
    fn table() {
        assert_eq!(Table(map! { }).to_string(),
                   "");
        assert_eq!(Table(map! { "test" => Integer(2) }).to_string(),
                   "test = 2\n");
        assert_eq!(Table(map! {
                        "test" => Integer(2),
                        "test2" => Table(map! {
                            "test" => String("wut".to_string())
                        })
                   }).to_string(),
                   "test = 2\n\
                    \n\
                    [test2]\n\
                    test = \"wut\"\n");
        assert_eq!(Table(map! {
                        "test" => Integer(2),
                        "test2" => Table(map! {
                            "test" => String("wut".to_string())
                        })
                   }).to_string(),
                   "test = 2\n\
                    \n\
                    [test2]\n\
                    test = \"wut\"\n");
        assert_eq!(Table(map! {
                        "test" => Integer(2),
                        "test2" => Array(vec![Table(map! {
                            "test" => String("wut".to_string())
                        })])
                   }).to_string(),
                   "test = 2\n\
                    \n\
                    [[test2]]\n\
                    test = \"wut\"\n");
        assert_eq!(Table(map! {
                        "foo.bar" => Integer(2),
                        "foo\"bar" => Integer(2)
                   }).to_string(),
                   "\"foo\\\"bar\" = 2\n\
                    \"foo.bar\" = 2\n");
    }
}




/// Encodes an encodable value into a "pretty" TOML string.
///
/// This function expects the type given to represent a TOML table in some form.
/// If encoding encounters an error, then this function will fail the task.
/// 
/// "pretty" means the following features of the output are changed:
/// - strings with newlines characters (`\n`) will use the `'''` form
///     and span multiple lines
pub fn encode_str_pretty<'a>(tbl: TomlTable) -> Result<StdString, fmt::Error> {
    let mut out = StdString::new();
    {
        let mut pp = PrettyPrinter { output: &mut out, stack: Vec::new() };
        try!(pp.print(&tbl));
    }
    Ok(out)
}

fn write_pretty_str(f: &mut StdString, s: &str) -> fmt::Result {
    try!(write!(f, "'''\n"));
    for ch in s.chars() {
        match ch {
            '\u{8}' => try!(write!(f, "\\b")),
            '\u{9}' => try!(write!(f, "\\t")),
            '\u{c}' => try!(write!(f, "\\f")),
            '\u{d}' => try!(write!(f, "\\r")),
            '\u{22}' => try!(write!(f, "\\\"")),
            '\u{5c}' => try!(write!(f, "\\\\")),
            ch => try!(write!(f, "{}", ch)),
        }
    }
    write!(f, "'''")
}

// The only thing in this impl that wasn't copy/pasted is
// - I removed handling arrays of tables (panics)
// - I added pretty printing strings when the have a \n in them
impl<'a, 'b> PrettyPrinter<'a, 'b> {
    fn print(&mut self, table: &'a TomlTable) -> fmt::Result {
        let mut space_out_first = false;
        // print out the regular key/value pairs at the top,
        // including arrays of tables I guess? (who cares)
        for (k, v) in table.iter() {
            match *v {
                Value::Table(..) => continue,
                Value::Array(ref a) => {
                    if let Some(&Value::Table(..)) = a.first() {
                        // not supported in rst
                        panic!("attempting to serialize an array of tables!")
                    }
                }
                // super special case -- the whole reason this is here!
                Value::String(ref s) => {
                    if s.contains('\n') {
                        try!(write!(self.output, "{} = ", Key(&[k])));
                        try!(write_pretty_str(self.output, s));
                        try!(write!(self.output, "\n"));
                        space_out_first = true;
                        continue;
                    }
                }
                _ => {}
            }
            space_out_first = true;
            try!(writeln!(self.output, "{} = {}", Key(&[k]), v));
        }
        // now go through the table and format the other tables
        for (i, (k, v)) in table.iter().enumerate() {
            match *v {
                Value::Table(ref inner) => {
                    // store the stack so that we can write
                    // [table.foo.bar]
                    self.stack.push(k);
                    if space_out_first || i != 0 {
                        try!(write!(self.output, "\n"));
                    }
                    try!(writeln!(self.output, "[{}]", Key(&self.stack)));
                    try!(self.print(inner));
                    self.stack.pop();
                }
                _ => {},
            }
        }
        Ok(())
    }
}

/// pretty printer for making multi-line text prettier
/// uses a String instead of the formatter from before
struct PrettyPrinter<'a, 'b:'a> {
    output: &'b mut StdString,
    stack: Vec<&'a str>,
}

// #############################################################################
// Tests


#[test]
fn test_pretty() {
    // examples of the form (input, expected output). If expected output==None, 
    // then it == input
    let mut examples = vec![
// toml keeps pretty strings
(r##"[example]
a_first = "hello world"
b_second = '''
this is a little longer
yay, it looks good!
'''
"##, None),

// format with two tables
(r##"[a_first]
int = 7
long = '''
i like long text
it is nice
'''

[b_second]
int = 10
text = "this is some text"
"##, None),

// toml re-orders fields alphabetically
(r##"[example]
b_second = ''' woot '''
a_first = "hello world"
"##, 
Some(r##"[example]
a_first = "hello world"
b_second = " woot "
"##)),

// toml reorders tables alphabetically
("[b]\n[a]\n", Some("[a]\n\n[b]\n")),
];
    use Parser;

    for (i, (value, expected)) in examples.drain(..).enumerate() {
        let expected = match expected {
            Some(ref r) => r,
            None => value,
        };
        assert_eq!((i, encode_str_pretty(Parser::new(value).parse().unwrap()).unwrap()), 
                   (i, expected.to_string()));
    }
}
