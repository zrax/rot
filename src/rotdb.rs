use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, Result};

use regex::Regex;
use once_cell::sync::Lazy;

pub struct RotDb {
    filename: String,
    values: HashMap<String, i64>,
    dirty: bool,
}

fn normalize_key(key: &str) -> String {
    static RE_SEPS: Lazy<Regex> = Lazy::new(|| Regex::new("(::|->)").unwrap());
    RE_SEPS.replace_all(key, ".").to_ascii_lowercase()
}

fn parse_db_line(filename: &str, text: &str) -> Option<(String, i64)> {
    let parts: Vec<&str> = text.splitn(2, ':').collect();
    if parts.len() != 2 {
        eprintln!("Invalid line format in {}: \"{}\"", filename, text);
        return None;
    }
    match parts[1].parse::<i64>() {
        Ok(value) => Some((parts[0].to_string(), value)),
        Err(_) => {
            eprintln!("Invalid value in {}: \"{}\"", filename, text);
            None
        }
    }
}

fn parse_zot_db(filename: &str) -> Result<HashMap<String, i64>> {
    let stream = File::open(filename)?;
    let values = BufReader::new(stream).lines()
        .filter_map(|line| {
            match line {
                Err(err) => {
                    eprintln!("Error reading line from {}:\n{}", filename, err);
                    None
                }
                Ok(text) => parse_db_line(filename, &text),
            }
        }).collect();

    Ok(values)
}

impl RotDb {
    pub fn new(filename_ref: &str) -> RotDb {
        let filename = filename_ref.to_owned();
        match parse_zot_db(&filename) {
            Ok(values) => RotDb { filename, values, dirty: false },
            Err(_) => {
                eprintln!("Initializing new zot db");
                RotDb { filename, values: HashMap::new(), dirty: false }
            }
        }
    }

    pub fn value(&self, key: &str) -> i64 {
        *self.values.get(&normalize_key(key))
                    .unwrap_or(&0)
    }

    pub fn increment(&mut self, key: &str) -> i64 {
        self.dirty = true;
        *self.values.entry(normalize_key(key))
                    .and_modify(|v| *v += 1)
                    .or_insert(1)
    }

    pub fn decrement(&mut self, key: &str) -> i64 {
        self.dirty = true;
        *self.values.entry(normalize_key(key))
                    .and_modify(|v| *v -= 1)
                    .or_insert(-1)
    }

    pub fn sync(&mut self) {
        if !self.dirty {
            return;
        }

        let mut stream = match File::create(&self.filename) {
            Ok(stream) => stream,
            Err(err) => {
                eprintln!("Could not open {} for writing:\n{}", self.filename, err);
                return;
            }
        };
        for (key, val) in &self.values {
            if let Some(err) = writeln!(stream, "{}:{}", &key, &val).err() {
                eprintln!("Could not write to {}:\n{}", self.filename, err);
                return;
            }
        }

        self.dirty = false;
    }
}

impl Drop for RotDb {
    fn drop(&mut self) {
        self.sync();
    }
}

#[test]
fn test_rotdb() {
    {
        // Start from a clean slate
        let _ = std::fs::remove_file("test.db");
        let mut db = RotDb::new("test.db");
        assert_eq!(db.increment("Foo::Bar"), 1);
        assert_eq!(db.decrement("Bar.foo"), -1);
        assert_eq!(db.value("Foo.BAR"), 1);
        assert_eq!(db.value("bar->foo"), -1);
        assert_eq!(db.value("Baz"), 0);
        assert_eq!(db.decrement("foo::bar"), 0);
        assert_eq!(db.increment("Foo::Bar"), 1);
    }
    {
        // Load previous database
        let db = RotDb::new("test.db");
        assert_eq!(db.value("Foo.BAR"), 1);
        assert_eq!(db.value("bar->foo"), -1);
        assert_eq!(db.value("Baz"), 0);
    }

    // Get rid of our test artifact
    let _ = std::fs::remove_file("test.db");
}
