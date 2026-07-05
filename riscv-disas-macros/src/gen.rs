use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::parse::{parse_enum, FieldInfo, VariantInfo};

struct FormatToken {
    text: String,
    field: Option<(String, Option<String>)>,
}

fn tokenize_format(s: &str) -> Vec<FormatToken> {
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
                tokens.push(FormatToken { text: std::mem::take(&mut buf), field: None });
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
            tokens.push(FormatToken { text: String::new(), field: Some((name, fmt)) });
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() || tokens.is_empty() {
        tokens.push(FormatToken { text: buf, field: None });
    }
    tokens
}

fn field_expr(field_name: &str, fields: &[FieldInfo]) -> Option<TokenStream> {
    let field_ident = format_ident!("{}", field_name);

    for f in fields {
        let (ident, is_reg): (proc_macro2::Ident, bool) = match f {
            FieldInfo::Named { name, .. } => {
                let name_str = name.to_string();
                (name.clone(), matches!(name_str.as_str(), "rd" | "rs1" | "rs2"))
            }
            FieldInfo::Unnamed { index, .. } => (format_ident!("_{}", index), false),
        };
        if ident == field_ident {
            if is_reg {
                return Some(quote! { reg_name(*#ident as usize) });
            } else {
                return Some(quote! { *#ident });
            }
        }
    }
    None
}

fn generate_variant(v: &VariantInfo) -> TokenStream {
    let fmt = v.format.as_ref().unwrap();
    let tokens = tokenize_format(fmt);

    let rebuilt: String = tokens
        .iter()
        .map(|t| {
            if let Some((_field, fmt_opt)) = &t.field {
                match fmt_opt {
                    Some(f) => format!("{{:{}}}", f),
                    None => "{}".to_string(),
                }
            } else {
                t.text.clone()
            }
        })
        .collect();

    let args: Vec<TokenStream> = tokens
        .iter()
        .filter_map(|t| {
            t.field.as_ref().and_then(|(fn_name, _)| field_expr(fn_name, &v.fields))
        })
        .collect();

    let pat = build_pattern(&v);
    quote! {
        #pat => format!(#rebuilt, #(#args),*),
    }
}

fn build_pattern(v: &VariantInfo) -> TokenStream {
    let name = &v.name;
    match v.fields.len() {
        0 => quote! { Inst::#name },
        _ => {
            let field_pats: Vec<TokenStream> = v
                .fields
                .iter()
                .map(|f| match f {
                    FieldInfo::Named { name, .. } => quote! { #name },
                    FieldInfo::Unnamed { index, .. } => {
                        let ident = format_ident!("_{}", index);
                        quote! { #ident }
                    }
                })
                .collect();

            if v.fields.iter().all(|f| matches!(f, FieldInfo::Named { .. })) {
                quote! { Inst::#name { #(#field_pats),* } }
            } else {
                quote! { Inst::#name ( #(#field_pats),* ) }
            }
        }
    }
}

pub fn generate(input: &DeriveInput) -> TokenStream {
    let variants = parse_enum(input);

    let arms: Vec<TokenStream> = variants
        .iter()
        .filter(|v| v.format.is_some())
        .map(generate_variant)
        .collect();

    quote! {
        impl Inst {
            pub fn disassemble_inner(&self, reg_name: impl Fn(usize) -> &'static str) -> String {
                match self {
                    #(#arms)*
                    _ => String::new(),
                }
            }
        }
    }
}
