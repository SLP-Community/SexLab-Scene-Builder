use std::collections::HashMap;
use std::path::Path;

/// One Animation() block from an SLAnimGenerate source .txt
#[derive(Debug, Clone, Default)]
pub struct SourceAnim {
    pub id: String,
    pub name: String,
    /// actor index (1-based in file) -> default object string (space-separated AO names)
    pub actor_objects: HashMap<usize, String>,
    /// actor index -> stage number (1-based) -> object override
    pub stage_objects: HashMap<usize, HashMap<usize, String>>,
}

pub fn parse_slanim_source(text: &str) -> Result<Vec<SourceAnim>, String> {
    let mut anims = Vec::new();
    let mut rest = text;

    while let Some(start) = find_animation_start(rest) {
        let from = &rest[start..];
        let open_rel = from.find('(').ok_or_else(|| {
            "Animation without '(' in SLAnim source".to_string()
        })?;
        let body = extract_paren_body(from).ok_or_else(|| {
            "Unclosed Animation( in SLAnim source".to_string()
        })?;
        anims.push(parse_animation_body(body)?);
        let end = start + open_rel + 1 + body.len() + 1;
        rest = &rest[end.min(rest.len())..];
    }

    Ok(anims)
}

pub fn parse_slanim_source_file(path: &Path) -> Result<Vec<SourceAnim>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_slanim_source(&text)
}

/// Convert space-separated AO names to SLSB comma-separated anim_obj.
pub fn objects_to_anim_obj(objects: &str) -> String {
    objects
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn find_animation_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 9 <= bytes.len() {
        if &bytes[i..i + 9] == b"Animation" {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let mut j = i + 9;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if before_ok && j < bytes.len() && bytes[j] == b'(' {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Given text starting at `Animation`, return the inside of the following `(...)`.
fn extract_paren_body(from_animation: &str) -> Option<&str> {
    let open = from_animation.find('(')?;
    let bytes = from_animation.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = open;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => in_str = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&from_animation[open + 1..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_animation_body(body: &str) -> Result<SourceAnim, String> {
    let mut anim = SourceAnim::default();
    anim.id = find_kw_string(body, "id").unwrap_or_default();
    anim.name = find_kw_string(body, "name").unwrap_or_default();
    if anim.id.is_empty() {
        return Err("Animation() missing id=".into());
    }

    // actorN=Type(..., object="...")
    for (actor_idx, call) in find_actor_calls(body) {
        if let Some(obj) = find_kw_string(&call, "object") {
            if !obj.is_empty() {
                anim.actor_objects.insert(actor_idx, obj);
            }
        }
    }

    // aN_stage_params=[ Stage(n, object="..."), ... ]
    for (actor_idx, list_body) in find_stage_param_lists(body) {
        for (stage_num, stage_body) in find_stage_calls(&list_body) {
            if let Some(obj) = find_kw_string(&stage_body, "object") {
                anim.stage_objects
                    .entry(actor_idx)
                    .or_default()
                    .insert(stage_num, obj);
            }
        }
    }

    Ok(anim)
}

fn find_kw_string(text: &str, key: &str) -> Option<String> {
    let pattern = format!("{}=", key);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + pattern.len() <= bytes.len() {
        if text[i..].starts_with(&pattern) {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            if before_ok {
                let after = &text[i + pattern.len()..];
                let trimmed = after.trim_start();
                if let Some(s) = parse_quoted_string(trimmed) {
                    return Some(s);
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_quoted_string(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.first()? != &b'"' {
        return None;
    }
    let mut out = String::new();
    let mut i = 1;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' && i + 1 < bytes.len() {
            out.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if c == '"' {
            return Some(out);
        }
        out.push(c);
        i += 1;
    }
    None
}

fn find_actor_calls(body: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 5 < bytes.len() {
        if &bytes[i..i + 5] == b"actor" {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let mut j = i + 5;
            let mut num = 0usize;
            let mut saw_digit = false;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                saw_digit = true;
                num = num * 10 + (bytes[j] - b'0') as usize;
                j += 1;
            }
            if before_ok && saw_digit && j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                // TypeName(...)
                if let Some(paren_at) = body[j..].find('(') {
                    let from_type = &body[j..];
                    if let Some(inner) = extract_paren_body_generic(from_type, paren_at) {
                        out.push((num, inner.to_string()));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

fn extract_paren_body_generic(text: &str, open_idx: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = open_idx;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => in_str = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[open_idx + 1..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn find_stage_param_lists(body: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    // aN_stage_params=
    while i + 2 < bytes.len() {
        if bytes[i] == b'a' && (i == 0 || !is_ident_byte(bytes[i - 1])) {
            let mut j = i + 1;
            let mut num = 0usize;
            let mut saw_digit = false;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                saw_digit = true;
                num = num * 10 + (bytes[j] - b'0') as usize;
                j += 1;
            }
            let suffix = b"_stage_params";
            if saw_digit && j + suffix.len() <= bytes.len() && &bytes[j..j + suffix.len()] == suffix
            {
                j += suffix.len();
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'=' {
                    j += 1;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'[' {
                        if let Some(inner) = extract_bracket_body(&body[j..]) {
                            out.push((num, inner.to_string()));
                        }
                    }
                }
            }
        }
        i += 1;
    }
    out
}

fn extract_bracket_body(from_open: &str) -> Option<&str> {
    let bytes = from_open.as_bytes();
    if bytes.first()? != &b'[' {
        return None;
    }
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&from_open[1..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn find_stage_calls(list_body: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = list_body.as_bytes();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        if &bytes[i..i + 5] == b"Stage" {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let mut j = i + 5;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if before_ok && j < bytes.len() && bytes[j] == b'(' {
                if let Some(inner) = extract_paren_body_generic(&list_body[i..], j - i) {
                    if let Some(stage_num) = parse_leading_int(inner) {
                        out.push((stage_num, inner.to_string()));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

fn parse_leading_int(text: &str) -> Option<usize> {
    let trimmed = text.trim_start();
    let mut num = 0usize;
    let mut saw = false;
    for c in trimmed.chars() {
        if c.is_ascii_digit() {
            saw = true;
            num = num * 10 + (c as u8 - b'0') as usize;
        } else {
            break;
        }
    }
    if saw {
        Some(num)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_staff_objects() {
        let src = r#"
Animation(
    id="B_FMastStaff",
    name="F Masturbation Staff",
    actor1=Female(),
		a1_stage_params=[
		Stage(1,  object="AO_BStaff2", animvars="AVbHumanoidFootIKDisable"),
		Stage(2,  object="AO_BStaff2"),	
		Stage(3,  object="AO_BStaff2 AOMBallMNodeR AO_BMCsA"),
    ],		
)
"#;
        let anims = parse_slanim_source(src).unwrap();
        assert_eq!(anims.len(), 1);
        let a = &anims[0];
        assert_eq!(a.id, "B_FMastStaff");
        let stages = a.stage_objects.get(&1).unwrap();
        assert_eq!(stages.get(&1).unwrap(), "AO_BStaff2");
        assert_eq!(
            stages.get(&3).unwrap(),
            "AO_BStaff2 AOMBallMNodeR AO_BMCsA"
        );
        assert_eq!(
            objects_to_anim_obj(stages.get(&3).unwrap()),
            "AO_BStaff2,AOMBallMNodeR,AO_BMCsA"
        );
    }

    #[test]
    fn parses_actor_level_object() {
        let src = r#"
Animation(
    id="B_ACG",
    name="Armbinder Cowgirl",
    actor1=Female(add_cum=Vaginal, object="AO_BArmbinder"),
			a1_stage_params=[
		Stage(1,  animvars="AVbHumanoidFootIKDisable"),
    ],		
    actor2=Male(strap_on=True),
)
"#;
        let anims = parse_slanim_source(src).unwrap();
        assert_eq!(anims[0].actor_objects.get(&1).unwrap(), "AO_BArmbinder");
        assert!(anims[0].actor_objects.get(&2).is_none());
    }

    #[test]
    fn parses_billyy_human_reference_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Billyy_Human.txt");
        if !path.exists() {
            return;
        }
        let anims = parse_slanim_source_file(&path).unwrap();
        assert_eq!(anims.len(), 128);
        let staff = anims.iter().find(|a| a.id == "B_FMastStaff").unwrap();
        assert_eq!(
            staff.stage_objects.get(&1).unwrap().get(&3).unwrap(),
            "AO_BStaff2 AOMBallMNodeR AO_BMCsA"
        );
    }

    #[test]
    fn parses_billyy_dd_reference_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Billyy_HumanDD.txt");
        if !path.exists() {
            return;
        }
        let anims = parse_slanim_source_file(&path).unwrap();
        assert_eq!(anims.len(), 60);
        let acg = anims.iter().find(|a| a.id == "B_ACG").unwrap();
        assert_eq!(acg.actor_objects.get(&1).unwrap(), "AO_BArmbinder");
    }
}
