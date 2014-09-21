use std::fmt;

use {Value, String, Integer, Float, Boolean, Datetime, Array, Table, TomlTable};

struct Printer<'a, 'b:'a> {
    output: &'a mut fmt::Formatter<'b>,
    stack: Vec<&'a str>,
}

impl fmt::Show for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            String(ref s) => {
                try!(write!(f, "\""));
                for ch in s.as_slice().chars() {
                    match ch {
                        '\u0008' => try!(write!(f, "\\b")),
                        '\u0009' => try!(write!(f, "\\t")),
                        '\u000a' => try!(write!(f, "\\n")),
                        '\u000c' => try!(write!(f, "\\f")),
                        '\u000d' => try!(write!(f, "\\r")),
                        '\u0022' => try!(write!(f, "\\\"")),
                        '\u005c' => try!(write!(f, "\\\\")),
                        ch => try!(write!(f, "{}", ch)),
                    }
                }
                write!(f, "\"")
            }
            Integer(i) => write!(f, "{}", i),
            Float(fp) => {
                try!(write!(f, "{}", fp));
                if fp % 1.0 == 0.0 { try!(write!(f, ".0")) }
                Ok(())
            }
            Boolean(b) => write!(f, "{}", b),
            Datetime(ref s) => write!(f, "{}", s),
            Array(ref a) => {
                match a.as_slice().head() {
                   Some(&Table(..)) =>
                       try!(write!(f, "invalid toml array of tables")),
                   _ => {}
                }
                write!(f, "{}", a)
            }
            Table(ref t) => {
                let mut p = Printer { output: f, stack: Vec::new() };
                p.print(t)
            }
        }
    }
}

impl<'a, 'b> Printer<'a, 'b> {
    fn print(&mut self, table: &'a TomlTable) -> fmt::Result {
        for (k, v) in table.iter() {
            match *v {
                Table(..) => continue,
                Array(ref a) => {
                    match a.as_slice().head() {
                        Some(&Table(..)) => continue,
                        _ => {}
                    }
                }
                _ => {}
            }
            try!(writeln!(self.output, "{} = {}", k, v));
        }
        for (k, v) in table.iter() {
            match *v {
                Table(ref inner) => {
                    self.stack.push(k.as_slice());
                    try!(writeln!(self.output, "\n[{}]",
                                  self.stack.connect(".")));
                    try!(self.print(inner));
                    self.stack.pop();
                }
                Array(ref inner) => {
                    match inner.as_slice().head() {
                        Some(&Table(..)) => {}
                        _ => continue
                    }
                    self.stack.push(k.as_slice());
                    for inner in inner.iter() {
                        try!(writeln!(self.output, "\n[[{}]]",
                                      self.stack.connect(".")));
                        match *inner {
                            Table(ref inner) => try!(self.print(inner)),
                            _ => fail!("non-heterogeneous toml array"),
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

#[cfg(test)]
#[allow(warnings)]
mod tests {
    use {Value, String, Integer, Float, Boolean, Datetime, Array, Table};
    use std::collections::TreeMap;

    macro_rules! map( ($($k:expr: $v:expr),*) => ({
        let mut _m = TreeMap::new();
        $(_m.insert($k.to_string(), $v);)*
        _m
    }) )

    #[test]
    fn simple_show() {
        assert_eq!(String("foo".to_string()).to_string().as_slice(),
                   "\"foo\"")
        assert_eq!(Integer(10).to_string().as_slice(),
                   "10")
        assert_eq!(Float(10.0).to_string().as_slice(),
                   "10.0")
        assert_eq!(Float(2.4).to_string().as_slice(),
                   "2.4")
        assert_eq!(Boolean(true).to_string().as_slice(),
                   "true")
        assert_eq!(Datetime("test".to_string()).to_string().as_slice(),
                   "test")
        assert_eq!(Array(vec![]).to_string().as_slice(),
                   "[]")
        assert_eq!(Array(vec![Integer(1), Integer(2)]).to_string().as_slice(),
                   "[1, 2]")
    }

    #[test]
    fn table() {
        assert_eq!(Table(map! { }).to_string().as_slice(),
                   "")
        assert_eq!(Table(map! { "test": Integer(2) }).to_string().as_slice(),
                   "test = 2\n")
        assert_eq!(Table(map! {
                        "test": Integer(2),
                        "test2": Table(map! {
                            "test": String("wut".to_string())
                        })
                   }).to_string().as_slice(),
                   "test = 2\n\
                    \n\
                    [test2]\n\
                    test = \"wut\"\n")
        assert_eq!(Table(map! {
                        "test": Integer(2),
                        "test2": Table(map! {
                            "test": String("wut".to_string())
                        })
                   }).to_string().as_slice(),
                   "test = 2\n\
                    \n\
                    [test2]\n\
                    test = \"wut\"\n")
        assert_eq!(Table(map! {
                        "test": Integer(2),
                        "test2": Array(vec![Table(map! {
                            "test": String("wut".to_string())
                        })])
                   }).to_string().as_slice(),
                   "test = 2\n\
                    \n\
                    [[test2]]\n\
                    test = \"wut\"\n")
    }
}
