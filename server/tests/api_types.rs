//! The API's types in TypeScript (work package 2.8): the file the frontend
//! reads them from is what the Rust types say today, and no type of the API
//! is missing from it. And the description of the HTTP interface (OpenAPI,
//! work package 5.23) in `server/openapi.json`, which GitHub Pages shows, is
//! what the code says today too.

use std::{fs, path::Path};

use regex::Regex;
use rueder::typescript::{FILE, generate};

/// Fails when a type of the API changed and `web/src/lib/api/types.ts` did
/// not. Run with `UPDATE_API_TYPES=1` to write the file anew.
#[test]
fn the_committed_types_are_current() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FILE);
    let generated = generate().expect("the types have names of their own");
    assert_current(&path, &generated);
}

/// Fails when a route, its description or a type it sends or reads changed
/// and `server/openapi.json` did not. Run with `UPDATE_API_TYPES=1` to write
/// the file anew.
#[test]
fn the_committed_description_is_current() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
    let generated = rueder::description()
        .to_pretty_json()
        .expect("the description is JSON")
        + "\n";
    assert_current(&path, &generated);
}

/// Writes the file with `UPDATE_API_TYPES`, and fails where it differs from
/// what the code says.
fn assert_current(path: &Path, generated: &str) {
    if std::env::var_os("UPDATE_API_TYPES").is_some() {
        fs::create_dir_all(path.parent().expect("the file lies in a directory")).expect("mkdir");
        fs::write(path, generated).expect("the file can be written");
    }
    let committed = fs::read_to_string(path).unwrap_or_default();
    let lines = committed.lines().zip(generated.lines()).enumerate();
    let differs = lines
        .clone()
        .find(|(_, (committed, generated))| committed != generated)
        .map_or_else(
            || format!("one has {} lines more", lines.count()),
            |(line, (committed, generated))| {
                format!("line {}: {committed:?} should be {generated:?}", line + 1)
            },
        );
    assert!(
        committed == generated,
        "{} is out of date ({differs}): run `UPDATE_API_TYPES=1 cargo test --test api_types` in \
         server/ and commit the file",
        path.display()
    );
}

/// The file is the same whichever compiler builds the server: `ts-rs` comes by
/// the parts of a type in an order that differs between compilers, so the
/// file has its own. A listed type stands first, its parts follow by name.
#[test]
fn the_types_stand_in_a_fixed_order() {
    let generated = generate().unwrap();
    let names: Vec<_> = generated
        .lines()
        .filter_map(|line| line.strip_prefix("export type "))
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    let after = |name: &str, count: usize| {
        let at = names.iter().position(|found| *found == name).unwrap();
        names[at..=at + count].to_vec()
    };
    assert_eq!(names[..3], ["Health", "Credentials", "Me"]);
    assert_eq!(after("Tree", 3), ["Tree", "Feed", "FetchAgent", "Folder"]);
    assert_eq!(after("Move", 2), ["Move", "DropPosition", "NodeRef"]);
    assert_eq!(
        after("Rule", 4),
        ["Rule", "RuleAction", "RuleField", "RuleMatch", "RuleScope"]
    );
    assert_eq!(after("ServerEvent", 1), ["ServerEvent", "FeedUnread"]);
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "every name once");
}

/// JSON that is not the page's API: what is stored of an article, what the
/// old database holds, what an OpenID provider says and what the Google Reader
/// API answers apps. Everything else that is written or read as JSON has to
/// be in the file.
const NOT_THE_API: [(&str, &str); 6] = [
    ("articles.rs", "Original"),
    ("articles.rs", "Version"),
    ("import.rs", "*"),
    ("oidc.rs", "*"),
    // The Google Reader API (`greader/`) answers apps of other makers, never
    // the page.
    ("mod.rs", "Tokens"),
    ("mod.rs", "UserInfo"),
];
/// Types that are only ever part of another, which spells their fields out.
const ONLY_INSIDE_ANOTHER: [(&str, &str); 1] = [("articles.rs", "Opened")];

/// A type that derives `Serialize` or `Deserialize`, as the source says it.
struct Json {
    file: String,
    name: String,
    /// Its name in TypeScript, if it derives `TS`.
    typescript: Option<String>,
}

fn json_types() -> Vec<Json> {
    let derive = Regex::new(
        r"#\[derive\(([^)]*)\)\]\s*((?:#\[[^\n]*\]\s*)*)(?:pub(?:\([a-z]+\))? )?(?:struct|enum) (\w+)",
    )
    .unwrap();
    let rename = Regex::new(r#"#\[ts\([^\n]*rename = "(\w+)""#).unwrap();
    let mut types = Vec::new();
    let mut directories = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            for found in derive.captures_iter(&source) {
                let (derives, attributes, name) = (&found[1], &found[2], &found[3]);
                let derives: Vec<_> = derives.split(',').map(str::trim).collect();
                if !derives.contains(&"Serialize") && !derives.contains(&"Deserialize") {
                    continue;
                }
                let renamed = rename
                    .captures(attributes)
                    .map(|renamed| renamed[1].to_owned());
                types.push(Json {
                    file: path.file_name().unwrap().to_string_lossy().into_owned(),
                    name: name.to_owned(),
                    typescript: derives
                        .contains(&"TS")
                        .then(|| renamed.unwrap_or_else(|| name.to_owned())),
                });
            }
        }
    }
    types
}

/// Whoever adds a type to the API cannot forget the frontend: a type that is
/// JSON derives `TS`, and its module lists it for the file.
#[test]
fn every_type_of_the_api_is_in_the_file() {
    let generated = generate().unwrap();
    let types = json_types();
    assert!(types.len() > 50, "the source is read: {}", types.len());
    let listed = |list: &[(&str, &str)], json: &Json| {
        list.iter()
            .any(|(file, name)| *file == json.file && (*name == "*" || *name == json.name))
    };
    for json in &types {
        let Json { file, name, .. } = json;
        match &json.typescript {
            None => assert!(
                listed(&NOT_THE_API, json),
                "{name} in {file} is JSON and does not derive TS: derive it and list the type in \
                 the module's `types`, or name it in NOT_THE_API"
            ),
            Some(typescript) => assert!(
                generated.contains(&format!("export type {typescript} ="))
                    != listed(&ONLY_INSIDE_ANOTHER, json),
                "{name} in {file} derives TS and is not in the file as {typescript}: list it in \
                 the module's `types`"
            ),
        }
    }
    // The lists of exceptions name what is there.
    for (file, name) in NOT_THE_API.iter().chain(&ONLY_INSIDE_ANOTHER) {
        let there = |json: &Json| json.file == *file && (*name == "*" || json.name == *name);
        assert!(types.iter().any(there), "{name} in {file} is gone");
    }
}

/// The file says what the server sends, so the names in it are the ones that
/// go over the wire.
#[test]
fn the_file_has_the_names_of_the_wire() {
    let generated = generate().unwrap();
    for part in [
        // Numbers of 64 bits are plain numbers.
        "\tid: number,\n\tpublished_at: string,\n",
        // A field with a name of its own in JSON.
        "\tmatch: RuleMatch,\n",
        "\ttype: string | null,\n",
        // What a request may leave out.
        "\tbefore?: number,\n",
        "\tunread?: boolean,\n",
        "\tparent_id?: number | null,\n",
        "\tis_admin?: boolean,\n",
        // An enum is its values in lower case, one with fields a union.
        r#"export type RuleAction = "hide" | "read";"#,
        "\t| {\n\t\t\"kind\": \"folder\",\n\t\t\"id\": number,\n\t}\n",
        "\t| {\n\t\t\"event\": \"resync\",\n\t\t\"data\": Record<symbol, never>,\n\t}\n",
        "\t\t\"event\": \"reading\",\n\t\t\"data\": {\n",
        "\t\t\tfeed: FeedUnread | null,\n",
        "\tfetch_agent: FetchAgent,\n",
    ] {
        assert!(generated.contains(part), "{part:?} is missing");
    }
    // What is stored and not sent has no place in it.
    assert!(!generated.contains("JsonValue") && !generated.contains("bigint"));
    assert!(!generated.contains(": any"), "nothing is left open");
}
