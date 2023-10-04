use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedLine {
    Nothing,
    Increment(String),
    Decrement(String),
    Query(String),
}

fn parsed_from(op: &str, ident: &str) -> ParsedLine {
    match op {
        "++" => ParsedLine::Increment(ident.to_string()),
        "--" => ParsedLine::Decrement(ident.to_string()),
        "?" => ParsedLine::Query(ident.to_string()),
        _ => ParsedLine::Nothing,
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
        ParsedLine::Nothing
    }
}

#[test]
fn test_parser() {
    assert_eq!(parse_line(""), ParsedLine::Nothing);
    assert_eq!(parse_line("Hello, world!"), ParsedLine::Nothing);
    assert_eq!(parse_line("// ++empty"), ParsedLine::Nothing);
    assert_eq!(parse_line("// --empty"), ParsedLine::Nothing);
    assert_eq!(parse_line("// ?empty"), ParsedLine::Nothing);
    assert_eq!(parse_line("/* ++empty */"), ParsedLine::Nothing);
    assert_eq!(parse_line("/* --empty */"), ParsedLine::Nothing);
    assert_eq!(parse_line("/* ?empty */"), ParsedLine::Nothing);

    assert_eq!(parse_line("++foo"), ParsedLine::Increment("foo".to_string()));
    assert_eq!(parse_line("foo++"), ParsedLine::Increment("foo".to_string()));
    assert_eq!(parse_line("--foo"), ParsedLine::Decrement("foo".to_string()));
    assert_eq!(parse_line("foo--"), ParsedLine::Decrement("foo".to_string()));
    assert_eq!(parse_line("?foo"), ParsedLine::Query("foo".to_string()));

    assert_eq!(parse_line("++Foo::Bar"), ParsedLine::Increment("Foo::Bar".to_string()));
    assert_eq!(parse_line("++Foo->Bar"), ParsedLine::Increment("Foo->Bar".to_string()));
    assert_eq!(parse_line("++Foo.Bar"), ParsedLine::Increment("Foo.Bar".to_string()));
    assert_eq!(parse_line("++Foo..Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("++Foo:Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("++Foo:::Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("++Foo :: Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("++Foo: :Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("+ +Foo::Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("+Foo::Bar"), ParsedLine::Nothing);
    assert_eq!(parse_line("+-Foo::Bar"), ParsedLine::Nothing);

    assert_eq!(parse_line("Foo::Bar++"), ParsedLine::Increment("Foo::Bar".to_string()));
    assert_eq!(parse_line("Foo->Bar++"), ParsedLine::Increment("Foo->Bar".to_string()));
    assert_eq!(parse_line("Foo.Bar++"), ParsedLine::Increment("Foo.Bar".to_string()));
    assert_eq!(parse_line("Foo..Bar++"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo:Bar++"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo:::Bar++"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo :: Bar++"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo: :Bar++"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo::Bar+ +"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo::Bar+"), ParsedLine::Nothing);
    assert_eq!(parse_line("Foo::Bar+-"), ParsedLine::Nothing);

    assert_eq!(parse_line("  ++  foo  "), ParsedLine::Increment("foo".to_string()));
    assert_eq!(parse_line("  foo  ++  "), ParsedLine::Increment("foo".to_string()));
    assert_eq!(parse_line("  --  foo  "), ParsedLine::Decrement("foo".to_string()));
    assert_eq!(parse_line("  foo  --  "), ParsedLine::Decrement("foo".to_string()));
    assert_eq!(parse_line("  ?  foo  "), ParsedLine::Query("foo".to_string()));

    assert_eq!(parse_line(" /* junk */ ++ /* junk */ foo /* junk */ // junk"),
               ParsedLine::Increment("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ foo /* junk */ ++ /* junk */ // junk"),
               ParsedLine::Increment("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ -- /* junk */ foo /* junk */ // junk"),
               ParsedLine::Decrement("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ foo /* junk */ -- /* junk */ // junk"),
               ParsedLine::Decrement("foo".to_string()));
    assert_eq!(parse_line(" /* junk */ ? /* junk */ foo /* junk */ // junk"),
               ParsedLine::Query("foo".to_string()));
    assert_eq!(parse_line("/*junk*/++/*junk*/foo::bar/*junk*///junk"),
               ParsedLine::Increment("foo::bar".to_string()));
    assert_eq!(parse_line("+/* junk */+foo:/* junk */:bar // junk"),
               ParsedLine::Increment("foo::bar".to_string()));
}
