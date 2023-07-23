use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedLine {
    Nothing,
    Increment(String),
    Decrement(String),
    Query(String),
}
use ParsedLine::*;

fn parsed_from(op: &str, ident: &str) -> ParsedLine {
    match op {
        "++" => Increment(ident.to_string()),
        "--" => Decrement(ident.to_string()),
        "?" => Query(ident.to_string()),
        _ => Nothing,
    }
}

pub fn parse_line(line: &str) -> ParsedLine {
    static RE_CLEAN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:/\*(?:[^/]|/[^*])*\*/|//.*)").unwrap()
    });
    static RE_PREOP: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*(\+\+|--|\?)\s*([A-Za-z_][A-Za-z0-9_]*(?:(?:\.|->|::)[A-Za-z_][A-Za-z0-9_]*)*)[\s;]*$").unwrap()
    });
    static RE_POSTOP: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*(?:(?:\.|->|::)[A-Za-z_][A-Za-z0-9_]*)*)\s*(\+\+|--)[\s;]*$").unwrap()
    });

    let clean = RE_CLEAN.replace_all(line, "");
    if let Some(pre_caps) = RE_PREOP.captures(&clean) {
        parsed_from(&pre_caps[1], &pre_caps[2])
    } else if let Some(post_caps) = RE_POSTOP.captures(&clean) {
        parsed_from(&post_caps[2], &post_caps[1])
    } else {
        Nothing
    }
}

#[test]
fn test_parser() {
    assert_eq!(parse_line(""), Nothing);
    assert_eq!(parse_line("Hello, world!"), Nothing);
    assert_eq!(parse_line("// ++empty"), Nothing);
    assert_eq!(parse_line("// --empty"), Nothing);
    assert_eq!(parse_line("// ?empty"), Nothing);
    assert_eq!(parse_line("/* ++empty */"), Nothing);
    assert_eq!(parse_line("/* --empty */"), Nothing);
    assert_eq!(parse_line("/* ?empty */"), Nothing);

    assert_eq!(parse_line("++foo"), Increment("foo".to_string()));
    assert_eq!(parse_line("foo++"), Increment("foo".to_string()));
    assert_eq!(parse_line("--foo"), Decrement("foo".to_string()));
    assert_eq!(parse_line("foo--"), Decrement("foo".to_string()));
    assert_eq!(parse_line("?foo"), Query("foo".to_string()));

    assert_eq!(parse_line("++Foo::Bar"), Increment("Foo::Bar".to_string()));
    assert_eq!(parse_line("++Foo->Bar"), Increment("Foo->Bar".to_string()));
    assert_eq!(parse_line("++Foo.Bar"), Increment("Foo.Bar".to_string()));
    assert_eq!(parse_line("++Foo..Bar"), Nothing);
    assert_eq!(parse_line("++Foo:Bar"), Nothing);
    assert_eq!(parse_line("++Foo:::Bar"), Nothing);
    assert_eq!(parse_line("++Foo :: Bar"), Nothing);
    assert_eq!(parse_line("++Foo: :Bar"), Nothing);
    assert_eq!(parse_line("+ +Foo::Bar"), Nothing);
    assert_eq!(parse_line("+Foo::Bar"), Nothing);
    assert_eq!(parse_line("+-Foo::Bar"), Nothing);

    assert_eq!(parse_line("Foo::Bar++"), Increment("Foo::Bar".to_string()));
    assert_eq!(parse_line("Foo->Bar++"), Increment("Foo->Bar".to_string()));
    assert_eq!(parse_line("Foo.Bar++"), Increment("Foo.Bar".to_string()));
    assert_eq!(parse_line("Foo..Bar++"), Nothing);
    assert_eq!(parse_line("Foo:Bar++"), Nothing);
    assert_eq!(parse_line("Foo:::Bar++"), Nothing);
    assert_eq!(parse_line("Foo :: Bar++"), Nothing);
    assert_eq!(parse_line("Foo: :Bar++"), Nothing);
    assert_eq!(parse_line("Foo::Bar+ +"), Nothing);
    assert_eq!(parse_line("Foo::Bar+"), Nothing);
    assert_eq!(parse_line("Foo::Bar+-"), Nothing);

    assert_eq!(parse_line("  ++  foo  "), Increment("foo".to_string()));
    assert_eq!(parse_line("  foo  ++  "), Increment("foo".to_string()));
    assert_eq!(parse_line("  --  foo  "), Decrement("foo".to_string()));
    assert_eq!(parse_line("  foo  --  "), Decrement("foo".to_string()));
    assert_eq!(parse_line("  ?  foo  "), Query("foo".to_string()));

    assert_eq!(parse_line(" /* junk */ ++ /* junk */ foo /* junk */ // junk"),
               Increment("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ foo /* junk */ ++ /* junk */ // junk"),
               Increment("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ -- /* junk */ foo /* junk */ // junk"),
               Decrement("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ foo /* junk */ -- /* junk */ // junk"),
               Decrement("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ ? /* junk */ foo /* junk */ // junk"),
               Query("foo".to_string()));
    assert_eq!(parse_line("/*junk*/++/*junk*/foo::bar/*junk*///junk"),
               Increment("foo::bar".to_string()));
    assert_eq!(parse_line("+/* junk */+foo:/* junk */:bar // junk"),
               Increment("foo::bar".to_string()));
}
