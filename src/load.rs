use std::{collections::HashMap, fs};
use walkdir::WalkDir;

pub(super) fn load(locales_dir: &str) -> HashMap<String, Vec<String>> {
    let walker = WalkDir::new(locales_dir)
        .sort_by(|a, b| b.file_name().len().cmp(&a.file_name().len()))
        .into_iter();
    let mut global_fluent: Vec<String> = vec![];
    let mut files: HashMap<String, Vec<String>> = HashMap::new();
    for entry in walker.flatten() {
        let path = entry.path();
        let parts: Vec<&str> = path.iter().skip(1).map(|x| x.to_str().unwrap()).collect();
        match parts[..] {
            [filename] if filename.ends_with("ftl") => {
                global_fluent.push(fs::read_to_string(path).unwrap().parse::<String>().unwrap());
            }
            [lang, filename] if filename.ends_with("ftl") => {
                let id = lang.to_string();
                let file: String = fs::read_to_string(path).unwrap().parse().unwrap();
                files
                    .entry(id)
                    .and_modify(|v| v.push(file.clone()))
                    .or_insert_with(|| {
                        global_fluent
                            .iter()
                            .cloned()
                            .chain(vec![file])
                            .collect::<Vec<String>>()
                    });
            }
            [lang, region, filename] if filename.ends_with("ftl") => {
                let id = format!("{}-{}", lang, region);
                let file: String = fs::read_to_string(path).unwrap().parse().unwrap();
                let lang_key = lang.to_string();
                let lang_prev: Vec<String> = files.get(&lang_key).unwrap().to_vec();

                files
                    .entry(id)
                    .and_modify(|v| v.push(file.clone()))
                    .or_insert_with(|| {
                        global_fluent
                            .iter()
                            .cloned()
                            .chain(lang_prev)
                            .chain(vec![file])
                            .collect::<Vec<String>>()
                    });
            }
            _ => (),
        };
    }
    files
}
