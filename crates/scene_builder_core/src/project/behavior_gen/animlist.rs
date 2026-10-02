use std::fmt;

#[derive(Debug, Clone)]
pub struct AnimLine {
    pub anim_type: String,
    pub options: Vec<String>,
    pub event: String,
    pub file: String,
    pub anim_objects: Vec<String>,
}

impl AnimLine {
    pub fn has_opt(&self, name: &str) -> bool {
        self.options.iter().any(|o| o.eq_ignore_ascii_case(name))
    }

    pub fn file_stem(&self) -> &str {
        self.file
            .strip_suffix(".hkx")
            .or_else(|| self.file.strip_suffix(".HKX"))
            .unwrap_or(&self.file)
    }

    pub fn is_acyclic(&self) -> bool {
        self.has_opt("a")
    }

    pub fn has_tn(&self) -> bool {
        self.options.iter().any(|o| {
            let lower = o.to_ascii_lowercase();
            lower == "tn"
                || (lower.starts_with("tn") && lower[2..].chars().all(|c| c.is_ascii_digit()))
        })
    }

    pub fn has_anim_objects(&self) -> bool {
        self.has_opt("o") || !self.anim_objects.is_empty()
    }
}

#[derive(Debug)]
pub struct AnimlistParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for AnimlistParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnimList line {}: {}", self.line, self.message)
    }
}

/// Parse FNIS AnimList text into animation lines (skips comments / Version / blanks).
pub fn parse_animlist(text: &str) -> Result<Vec<AnimLine>, AnimlistParseError> {
    let mut out = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('\'')
            || line.to_ascii_lowercase().starts_with("version")
        {
            continue;
        }
        out.push(parse_line(line, line_no)?);
    }
    Ok(out)
}

fn parse_line(line: &str, line_no: usize) -> Result<AnimLine, AnimlistParseError> {
    let err = |message: &str| AnimlistParseError {
        line: line_no,
        message: message.to_string(),
    };

    let mut tokens = line.split_whitespace();
    let type_tok = tokens.next().ok_or_else(|| err("empty line"))?;
    if type_tok.starts_with('-') {
        return Err(err("missing AnimType before options"));
    }

    let anim_type = type_tok.to_string();
    let next = tokens.next().ok_or_else(|| err("missing AnimEvent or options"))?;

    let (options, event) = if next.starts_with('-') {
        let opts: Vec<String> = next[1..]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let event = tokens
            .next()
            .ok_or_else(|| err("missing AnimEvent"))?
            .to_string();
        (opts, event)
    } else {
        (Vec::new(), next.to_string())
    };

    let file = tokens
        .next()
        .ok_or_else(|| err("missing AnimFile"))?
        .to_string();
    let anim_objects: Vec<String> = tokens.map(|s| s.to_string()).collect();

    Ok(AnimLine {
        anim_type,
        options,
        event,
        file,
        anim_objects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pplus_md() {
        let lines = parse_animlist(
            "b -md ct62B_Chaur_B_CageD_A2_S1 Chaur_B_CageD_A2_S1.hkx\nb -o,md,a,Tn ev file.hkx AO1 AO2\n",
        )
        .unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].anim_type, "b");
        assert!(lines[0].has_opt("md"));
        assert_eq!(lines[0].event, "ct62B_Chaur_B_CageD_A2_S1");
        assert!(lines[1].has_anim_objects());
        assert!(lines[1].is_acyclic());
        assert!(lines[1].has_tn());
        assert_eq!(lines[1].anim_objects, vec!["AO1", "AO2"]);
    }
}
