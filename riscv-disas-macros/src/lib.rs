use proc_macro::TokenStream;

mod parse;
mod gen;

#[proc_macro_derive(Disassemble, attributes(instr))]
pub fn derive_disassemble(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    gen::generate(&input).into()
}
