use std::collections::HashMap;
use std::path::Path;

/// One AnimList line's AnimObjects, keyed later by event name (lowercase).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FnisAnimObjects {
    pub anim_objs: Vec<String>,
}

/// Parse a FNIS / Pandora AnimList into event → AnimObject names.
///
/// Tokens after the `.hkx` file are treated as AnimObjects (author hand-edits).
/// Event names are stored lowercased for matching.
pub fn parse_fnis_list(text: &str) -> HashMap<String, FnisAnimObjects> {
    let mut out: HashMap<String, FnisAnimObjects> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        let splits: Vec<&str> = line.split_whitespace().collect();
        if splits.is_empty() {
            continue;
        }
        let first = splits[0].to_ascii_lowercase();
        if first == "version" || first.ends_with("version") {
            continue;
        }

        let mut anim_file: Option<&str> = None;
        let mut anim_event: Option<&str> = None;
        let mut anim_objects: Vec<String> = Vec::new();

        for (i, split) in splits.iter().enumerate() {
            if split.len() <= 1 {
                continue;
            }
            if split.starts_with('-') {
                continue;
            }
            if split.to_ascii_lowercase().contains(".hkx") {
                anim_file = Some(split);
                if i > 0 {
                    anim_event = Some(splits[i - 1]);
                }
            } else if anim_event.is_some() {
                let obj = (*split).to_string();
                if !anim_objects.iter().any(|o| o.eq_ignore_ascii_case(&obj)) {
                    anim_objects.push(obj);
                }
            }
        }

        let (Some(_), Some(event)) = (anim_file, anim_event) else {
            continue;
        };
        if anim_objects.is_empty() {
            continue;
        }
        out.insert(
            event.to_ascii_lowercase(),
            FnisAnimObjects {
                anim_objs: anim_objects,
            },
        );
    }
    out
}

pub fn parse_fnis_list_file(path: &Path) -> Result<HashMap<String, FnisAnimObjects>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(parse_fnis_list(&text))
}

/// Match a project event id against FNIS map keys (exact, or strip 4-char SLSB hash prefix).
/// Returns `(map_key, objects)`.
pub fn lookup_fnis_objects<'a>(
    map: &'a HashMap<String, FnisAnimObjects>,
    event: &str,
) -> Option<(&'a str, &'a FnisAnimObjects)> {
    let key = event.to_ascii_lowercase();
    if let Some((k, v)) = map.get_key_value(&key) {
        return Some((k.as_str(), v));
    }
    // SLSB events are often `{4-char-hash}{classicEvent}`
    if key.len() > 4 {
        let rest = &key[4..];
        if rest.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
            if let Some((k, v)) = map.get_key_value(rest) {
                return Some((k.as_str(), v));
            }
        }
    }
    // Classic list event may include a hash the project event already has — try ±4 len suffix
    for (k, v) in map {
        if key.ends_with(k.as_str()) || k.ends_with(key.as_str()) {
            if key.len() >= 4 && (key.len() == k.len() + 4 || k.len() == key.len() + 4) {
                return Some((k.as_str(), v));
            }
        }
    }
    None
}

pub fn objects_to_anim_obj(objs: &[String]) -> String {
    objs.iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_classic_o_line_anim_objects() {
        let text = r#"
Version V1.0

s -o,AVbHumanoidFootIKDisable B_Billyy_ChairDildo_A1_S1 Billyy_ChairDildo_A1_S1.hkx AOChairA AOShockyDogDildoB
+ -o B_Billyy_ChairDildo_A1_S2 Billyy_ChairDildo_A1_S2.hkx AOChairA AOShockyDogDildoB
b -md yhd9plain_a1 plain_a1.hkx
"#;
        let map = parse_fnis_list(text);
        assert_eq!(map.len(), 2);
        let a1 = map.get("b_billyy_chairdildo_a1_s1").unwrap();
        assert_eq!(
            a1.anim_objs,
            vec!["AOChairA".to_string(), "AOShockyDogDildoB".to_string()]
        );
        assert!(map.get("yhd9plain_a1").is_none());
    }

    #[test]
    fn parses_slsb_b_line_with_hash_event() {
        let text =
            "b -o,md yhd9sap_highbenchfuck_a1 sap_highbenchfuck_a1.hkx sap_highbench\r\n";
        let map = parse_fnis_list(text);
        let e = map.get("yhd9sap_highbenchfuck_a1").unwrap();
        assert_eq!(e.anim_objs, vec!["sap_highbench".to_string()]);
    }

    #[test]
    fn lookup_strips_slsb_hash_prefix() {
        let mut map = HashMap::new();
        map.insert(
            "b_billyy_chairdildo_a1_s1".into(),
            FnisAnimObjects {
                anim_objs: vec!["AOChairA".into()],
            },
        );
        let hit = lookup_fnis_objects(&map, "yhd9B_Billyy_ChairDildo_A1_S1").unwrap();
        assert_eq!(hit.0, "b_billyy_chairdildo_a1_s1");
        assert_eq!(hit.1.anim_objs, vec!["AOChairA".to_string()]);
    }
}
