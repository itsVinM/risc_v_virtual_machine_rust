use std::{
    env, fs,
    path::Path,
};

fn main() {
    let src = fs::read_to_string("src/cpu/decoder.rs").unwrap();
    let arms = parse_instr(&src);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("disassemble.rs");

    let mut code = String::from(
        "impl Inst {\n\
         pub fn disassemble_inner(\n\
         &self, reg_name: impl Fn(usize) -> &'static str\n\
         ) -> String {\n\
         match self {\n",
    );
    for (variant, fields, format_str) in &arms {
        code.push_str(&generate_arm(variant, fields, format_str));
    }
    code.push_str("        _ => String::new(),\n    }\n}\n}\n");

    fs::write(&dest, &code).unwrap();

    println!("cargo::rerun-if-changed=src/cpu/decoder.rs");
}

/// Parsed variant info
struct VariantDef {
    name: String,
    fields: Vec<FieldDef>,
}

enum FieldDef {
    Named(String),
    Unnamed(usize),
}

fn parse_instr(src: &str) -> Vec<(String, Vec<FieldDef>, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(fmt) = trimmed.strip_prefix("// instr(") {
            // Extract the string between quotes
            if let Some(end) = fmt.strip_suffix(')') {
                let fmt_str = end
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or("");
                if let Some(vd) = find_variant(&lines[i + 1..]) {
                    results.push((vd.name, vd.fields, fmt_str.to_string()));
                }
            }
        }
        i += 1;
    }
    results
}

fn find_variant(lines: &[&str]) -> Option<VariantDef> {
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        // Strip trailing comma
        let body = trimmed.strip_suffix(',').unwrap_or(trimmed).trim();

        // Unit variant: "Foo"
        if !body.contains('(') && !body.contains('{') {
            return Some(VariantDef {
                name: body.to_string(),
                fields: vec![],
            });
        }

        // Tuple variant: "Foo(u32)" or "Foo(u32, u32, u32)"
        if let Some(tuple_start) = body.find('(') {
            let name = body[..tuple_start].trim().to_string();
            let inner = &body[tuple_start + 1..];
            let inner = inner.strip_suffix(')').unwrap_or(inner);
            // Count commas (each comma + 1 = number of fields)
            let count = if inner.trim().is_empty() {
                0
            } else {
                inner.split(',').count()
            };
            let fields: Vec<FieldDef> = (0..count).map(FieldDef::Unnamed).collect();
            return Some(VariantDef { name, fields });
        }

        // Named struct variant: "Foo { rd: u8, rs1: u8 }"
        if let Some(brace_start) = body.find('{') {
            let name = body[..brace_start].trim().to_string();
            let inner = &body[brace_start + 1..];
            let inner = inner.strip_suffix('}').unwrap_or(inner);
            let fields: Vec<FieldDef> = inner
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .filter_map(|s| {
                    s.split_once(':')
                        .map(|(name, _)| FieldDef::Named(name.trim().to_string()))
                })
                .collect();
            return Some(VariantDef { name, fields });
        }
    }
    None
}

fn generate_arm(variant: &str, fields: &[FieldDef], fmt: &str) -> String {
    let tokens = tokenize(fmt);

    let mut pattern = format!("Inst::{}", variant);
    match fields.len() {
        0 => {}
        _ if fields.iter().all(|f| matches!(f, FieldDef::Named(_))) => {
            let names: Vec<String> = fields
                .iter()
                .map(|f| match f {
                    FieldDef::Named(n) => n.clone(),
                    FieldDef::Unnamed(i) => format!("_{}", i),
                })
                .collect();
            pattern += &format!(" {{ {} }}", names.join(", "));
        }
        _ => {
            let names: Vec<String> = (0..fields.len())
                .map(|i| format!("_{}", i))
                .collect();
            pattern += &format!("({})", names.join(", "));
        }
    }

    let mut args = Vec::new();
    let mut format_parts = Vec::new();

    for tok in &tokens {
        match tok {
            FormatToken::Text(t) => format_parts.push(t.clone()),
            FormatToken::Field { name, spec } => {
                let arg = field_access(name, fields);
                args.push(arg);
                match spec {
                    Some(s) => format_parts.push(format!("{{:{}}}", s)),
                    None => format_parts.push("{}".to_string()),
                }
            }
        }
    }

    let format_str = format_parts.concat();
    if args.is_empty() {
        format!("        {} => format!(\"{}\"),\n", pattern, format_str)
    } else {
        format!(
            "        {} => format!(\"{}\", {}),\n",
            pattern,
            format_str,
            args.join(", ")
        )
    }
}

fn field_access(name: &str, _fields: &[FieldDef]) -> String {
    if matches!(name, "rd" | "rs1" | "rs2") {
        format!("reg_name(*{} as usize)", name)
    } else {
        format!("*{}", name)
    }
}

enum FormatToken {
    Text(String),
    Field { name: String, spec: Option<String> },
}

fn tokenize(s: &str) -> Vec<FormatToken> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&&'{') {
                buf.push('{');
                chars.next();
                continue;
            }
            if !buf.is_empty() {
                tokens.push(FormatToken::Text(std::mem::take(&mut buf)));
            }
            let mut name = String::new();
            let mut fmt = None;
            let mut in_fmt = false;
            for c in &mut chars {
                if c == '}' {
                    break;
                }
                if c == ':' {
                    in_fmt = true;
                } else if in_fmt {
                    fmt.get_or_insert_with(String::new).push(c);
                } else {
                    name.push(c);
                }
            }
            tokens.push(FormatToken::Field { name, spec: fmt });
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() || tokens.is_empty() {
        tokens.push(FormatToken::Text(buf));
    }
    tokens
}
