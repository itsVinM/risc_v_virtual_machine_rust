use syn::{Data, DeriveInput, Fields, Ident, Type};

pub struct VariantInfo {
    pub name: Ident,
    pub fields: Vec<FieldInfo>,
    pub format: Option<String>,
}

pub enum FieldInfo {
    Named { name: Ident, ty: Type },
    Unnamed { index: usize, ty: Type },
}

pub fn parse_enum(input: &DeriveInput) -> Vec<VariantInfo> {
    let data = match &input.data {
        Data::Enum(e) => e,
        _ => panic!("Disassemble requires an enum"),
    };

    data.variants
        .iter()
        .map(|v| {
            let fields = match &v.fields {
                Fields::Named(named) => named
                    .named
                    .iter()
                    .map(|f| FieldInfo::Named {
                        name: f.ident.clone().unwrap(),
                        ty: f.ty.clone(),
                    })
                    .collect(),
                Fields::Unnamed(unnamed) => unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, f)| FieldInfo::Unnamed {
                        index: i,
                        ty: f.ty.clone(),
                    })
                    .collect(),
                Fields::Unit => vec![],
            };

            let format = v
                .attrs
                .iter()
                .find(|a| a.path().is_ident("instr"))
                .and_then(|a| {
                    let s: syn::LitStr = a.parse_args().unwrap();
                    Some(s.value())
                });

            VariantInfo {
                name: v.ident.clone(),
                fields,
                format,
            }
        })
        .collect()
}

pub fn is_register_name(name: &str) -> bool {
    matches!(name, "rd" | "rs1" | "rs2" | "uimm")
}
