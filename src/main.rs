use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{read_dir, File};
use std::io::{BufReader, Write};

#[derive(Serialize, Deserialize)]
struct Data {
    description: String,
    image: String,
    name: String,
    dna: String,
    uid: String,
    generator: String,
    attributes: Vec<Attribute>,
}

#[derive(Serialize, Deserialize)]
struct Attribute {
    trait_type: String,
    value: String,
}

struct Builder {
    traits: BTreeMap<String, Vec<String>>,
    data: Vec<(String, Vec<u8>)>,
}

struct Finished {
    dna: String,
    data: Vec<u8>,
}

fn convert_code_safe(name: &str) -> String {
    let res = if name == "1:1" {
        "Unique".to_string()
    } else {
        name.replace(" ", "")
            .replace(":", "_")
            .replace("-", "_")
            .replace("'", "")
            .replace("$", "S")
            .replace("=", "is")
            .replace("∞", "Infinity")
            .replace("+", "Plus")
    };

    let re = regex::Regex::new(r"^\d").unwrap();
    if re.is_match(&res) {
        format!("_{res}")
    } else {
        res
    }
}

fn main() {
    let mut builder = Builder {
        traits: Default::default(),
        data: vec![],
    };

    // Initialize builder
    let mut assets_order = vec![];
    for entry in read_dir("../derpies-assets/constantine/output/erc721 metadata").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        // Read all nfts
        if path.is_file() {
            let file = File::open(path).unwrap();
            let reader = BufReader::new(file);
            let data: Data = serde_json::from_reader(reader).unwrap();

            // Build the current data
            let mut indexes = vec![];
            for attr in data.attributes {
                // Try to find existing attribute - this should all be built on the first item
                let index = if let Some(values) = builder.traits.get_mut(&attr.trait_type) {
                    // Try to find current trait or insert it
                    if let Some(found) = values.iter().position(|value| value == &attr.value) {
                        found
                    } else {
                        let i = values.len();
                        values.push(attr.value.clone());
                        i
                    }
                } else {
                    assets_order.push(attr.trait_type.clone());
                    builder.traits.insert(attr.trait_type, vec![attr.value]);
                    0
                };

                indexes.push(index as u8);
            }

            builder.data.push((data.dna, indexes));
        }
    }

    // Build file
    let mut file = File::create("./output.rs").unwrap();

    // Build traits
    for (name, attrs) in builder.traits.iter() {
        let obj_name = convert_code_safe(name);
        let items = attrs.len();
        file.write(format!("pub static {obj_name}: [&str; {items}] = [").as_bytes()).unwrap();
        for attr in attrs {
            file.write(format!("\"{attr}\",").as_bytes()).unwrap();
        }
        file.write(b"];\n").unwrap();
    }

    // Build convert function
    file.write(b"use serde::{Deserialize, Serialize};\n").unwrap();
    file.write(b"#[derive(Serialize, Deserialize)]\nstruct Attribute {\n\ttrait_type: String,\n\tvalue: String,\n}\n").unwrap();
    file.write(b"impl Attribute {\n\tpub fn new(trait_type: &str, value: &str) -> Self {\n\t\tSelf { trait_type: trait_type.to_string(), value: value.to_string() }\n\t}\n}\n").unwrap();
    file.write(b"pub fn read_item(index: usize) -> (String, Vec<Attribute>) {\tlet data = ATTRS[index];\n\n\tlet mut attrs = vec![\n").unwrap();

    for (i, name) in assets_order.iter().enumerate() {
        let obj_name = convert_code_safe(name);
        file.write(format!("\t\tAttribute::new(\"{name}\", {obj_name}[data[{i}] as usize]),\n").as_bytes()).unwrap();
    }

    file.write(b"];\n\t(DNAS[index].to_string(), attrs)\n}\n").unwrap();

    // Build dnas data
    let items = builder.data.len();
    file.write(format!("pub static DNAS: [&str; {items}] = [").as_bytes()).unwrap();
    for (dna, _) in builder.data.iter() {
        file.write(format!("\"{dna}\",").as_bytes()).unwrap();
    }
    file.write(b"];\n").unwrap();

    // Build attrs data
    let attr_len = builder.data.first().unwrap().1.len();
    file.write(format!("pub const ATTRS: [[u8; {attr_len}]; {items}] = [").as_bytes()).unwrap();
    for (_, attrs) in builder.data.iter() {
        file.write(b"[").unwrap();
        for i in attrs {
            file.write(format!("{i},").as_bytes()).unwrap();
        }
        file.write(b"],").unwrap();
    }
    file.write(b"];\n").unwrap();

}
