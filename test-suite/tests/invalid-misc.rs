extern crate toml;

#[test]
fn bad() {
    fn bad(s: &str) {
        assert!(s.parse::<toml::Value>().is_err());
    }

    bad("a = 01");
    bad("a = 1__1");
    bad("a = 1_");
    bad("''");
    bad("a = 9e99999");

    bad("a = \"\u{7f}\"");
    bad("a = '\u{7f}'");

    bad("a = -0x1");
    bad("a = 0x-1");

    // Dotted keys.
    bad("a.b.c = 1
         a.b = 2
        ");
    bad("a = 1
         a.b = 2");
    bad("a = {k1 = 1, k1.name = \"joe\"}")
}
