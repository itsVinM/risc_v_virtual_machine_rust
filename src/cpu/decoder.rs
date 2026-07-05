/// All RISC-V instruction formats decoded via bit manipulation (no string parsing).
/// Disassembly format strings are provided in `// instr("...")` comments and
/// processed by build.rs to generate `disassemble_inner()`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inst {
    // RV64I — Lui/Auipc handled manually (imm >> 12 transform)
    Lui   { rd: u8, imm: i64 },
    // A extension (atomics) — no disassembly yet
    AmoaddW { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    AmoswapW { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    LrW { rd: u8, rs1: u8, aq: bool, rl: bool },
    ScW { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    LrD { rd: u8, rs1: u8, aq: bool, rl: bool },
    ScD { rd: u8, rs1: u8, rs2: u8, aq: bool, rl: bool },
    // instr("auipc   {rd}, 0x{imm:x}")
    Auipc { rd: u8, imm: i64 },
    // instr("jal     {rd}, {imm:+}")
    Jal   { rd: u8, imm: i64 },
    // instr("jalr    {rd}, {imm}({rs1})")
    Jalr  { rd: u8, rs1: u8, imm: i64 },
    // instr("beq     {rs1}, {rs2}, {imm:+}")
    Beq   { rs1: u8, rs2: u8, imm: i64 },
    // instr("bne     {rs1}, {rs2}, {imm:+}")
    Bne   { rs1: u8, rs2: u8, imm: i64 },
    // instr("blt     {rs1}, {rs2}, {imm:+}")
    Blt   { rs1: u8, rs2: u8, imm: i64 },
    // instr("bge     {rs1}, {rs2}, {imm:+}")
    Bge   { rs1: u8, rs2: u8, imm: i64 },
    // instr("bltu    {rs1}, {rs2}, {imm:+}")
    Bltu  { rs1: u8, rs2: u8, imm: i64 },
    // instr("bgeu    {rs1}, {rs2}, {imm:+}")
    Bgeu  { rs1: u8, rs2: u8, imm: i64 },
    // instr("lb      {rd}, {imm}({rs1})")
    Lb    { rd: u8, rs1: u8, imm: i64 },
    // instr("lh      {rd}, {imm}({rs1})")
    Lh    { rd: u8, rs1: u8, imm: i64 },
    // instr("lw      {rd}, {imm}({rs1})")
    Lw    { rd: u8, rs1: u8, imm: i64 },
    // instr("ld      {rd}, {imm}({rs1})")
    Ld    { rd: u8, rs1: u8, imm: i64 },
    // instr("lbu     {rd}, {imm}({rs1})")
    Lbu   { rd: u8, rs1: u8, imm: i64 },
    // instr("lhu     {rd}, {imm}({rs1})")
    Lhu   { rd: u8, rs1: u8, imm: i64 },
    // instr("lwu     {rd}, {imm}({rs1})")
    Lwu   { rd: u8, rs1: u8, imm: i64 },
    // instr("sb      {rs2}, {imm}({rs1})")
    Sb    { rs1: u8, rs2: u8, imm: i64 },
    // instr("sh      {rs2}, {imm}({rs1})")
    Sh    { rs1: u8, rs2: u8, imm: i64 },
    // instr("sw      {rs2}, {imm}({rs1})")
    Sw    { rs1: u8, rs2: u8, imm: i64 },
    // instr("sd      {rs2}, {imm}({rs1})")
    Sd    { rs1: u8, rs2: u8, imm: i64 },
    // Addi — handled manually (li/mv pseudo-instructions)
    Addi  { rd: u8, rs1: u8, imm: i64 },
    // instr("slti    {rd}, {rs1}, {imm}")
    Slti  { rd: u8, rs1: u8, imm: i64 },
    // instr("sltiu   {rd}, {rs1}, {imm}")
    Sltiu { rd: u8, rs1: u8, imm: i64 },
    // instr("xori    {rd}, {rs1}, {imm}")
    Xori  { rd: u8, rs1: u8, imm: i64 },
    // instr("ori     {rd}, {rs1}, {imm}")
    Ori   { rd: u8, rs1: u8, imm: i64 },
    // instr("andi    {rd}, {rs1}, {imm}")
    Andi  { rd: u8, rs1: u8, imm: i64 },
    // instr("slli    {rd}, {rs1}, {shamt}")
    Slli  { rd: u8, rs1: u8, shamt: u8 },
    // instr("srli    {rd}, {rs1}, {shamt}")
    Srli  { rd: u8, rs1: u8, shamt: u8 },
    // instr("srai    {rd}, {rs1}, {shamt}")
    Srai  { rd: u8, rs1: u8, shamt: u8 },
    // instr("add     {rd}, {rs1}, {rs2}")
    Add   { rd: u8, rs1: u8, rs2: u8 },
    // instr("sub     {rd}, {rs1}, {rs2}")
    Sub   { rd: u8, rs1: u8, rs2: u8 },
    // instr("sll     {rd}, {rs1}, {rs2}")
    Sll   { rd: u8, rs1: u8, rs2: u8 },
    // instr("slt     {rd}, {rs1}, {rs2}")
    Slt   { rd: u8, rs1: u8, rs2: u8 },
    // instr("sltu    {rd}, {rs1}, {rs2}")
    Sltu  { rd: u8, rs1: u8, rs2: u8 },
    // instr("xor     {rd}, {rs1}, {rs2}")
    Xor   { rd: u8, rs1: u8, rs2: u8 },
    // instr("srl     {rd}, {rs1}, {rs2}")
    Srl   { rd: u8, rs1: u8, rs2: u8 },
    // instr("sra     {rd}, {rs1}, {rs2}")
    Sra   { rd: u8, rs1: u8, rs2: u8 },
    // instr("or      {rd}, {rs1}, {rs2}")
    Or    { rd: u8, rs1: u8, rs2: u8 },
    // instr("and     {rd}, {rs1}, {rs2}")
    And   { rd: u8, rs1: u8, rs2: u8 },
    // RV64I W-suffix (32-bit ops sign-extended)
    // instr("addiw   {rd}, {rs1}, {imm}")
    Addiw { rd: u8, rs1: u8, imm: i64 },
    // instr("slliw   {rd}, {rs1}, {shamt}")
    Slliw { rd: u8, rs1: u8, shamt: u8 },
    // instr("srliw   {rd}, {rs1}, {shamt}")
    Srliw { rd: u8, rs1: u8, shamt: u8 },
    // instr("sraiw   {rd}, {rs1}, {shamt}")
    Sraiw { rd: u8, rs1: u8, shamt: u8 },
    // instr("addw    {rd}, {rs1}, {rs2}")
    Addw  { rd: u8, rs1: u8, rs2: u8 },
    // instr("subw    {rd}, {rs1}, {rs2}")
    Subw  { rd: u8, rs1: u8, rs2: u8 },
    // instr("sllw    {rd}, {rs1}, {rs2}")
    Sllw  { rd: u8, rs1: u8, rs2: u8 },
    // instr("srlw    {rd}, {rs1}, {rs2}")
    Srlw  { rd: u8, rs1: u8, rs2: u8 },
    // instr("sraw    {rd}, {rs1}, {rs2}")
    Sraw  { rd: u8, rs1: u8, rs2: u8 },
    // RV64M
    // instr("mul     {rd}, {rs1}, {rs2}")
    Mul    { rd: u8, rs1: u8, rs2: u8 },
    // instr("mulh    {rd}, {rs1}, {rs2}")
    Mulh   { rd: u8, rs1: u8, rs2: u8 },
    // instr("mulhsu  {rd}, {rs1}, {rs2}")
    Mulhsu { rd: u8, rs1: u8, rs2: u8 },
    // instr("mulhu   {rd}, {rs1}, {rs2}")
    Mulhu  { rd: u8, rs1: u8, rs2: u8 },
    // instr("div     {rd}, {rs1}, {rs2}")
    Div    { rd: u8, rs1: u8, rs2: u8 },
    // instr("divu    {rd}, {rs1}, {rs2}")
    Divu   { rd: u8, rs1: u8, rs2: u8 },
    // instr("rem     {rd}, {rs1}, {rs2}")
    Rem    { rd: u8, rs1: u8, rs2: u8 },
    // instr("remu    {rd}, {rs1}, {rs2}")
    Remu   { rd: u8, rs1: u8, rs2: u8 },
    // instr("mulw    {rd}, {rs1}, {rs2}")
    Mulw   { rd: u8, rs1: u8, rs2: u8 },
    // instr("divw    {rd}, {rs1}, {rs2}")
    Divw   { rd: u8, rs1: u8, rs2: u8 },
    // instr("divuw   {rd}, {rs1}, {rs2}")
    Divuw  { rd: u8, rs1: u8, rs2: u8 },
    // instr("remw    {rd}, {rs1}, {rs2}")
    Remw   { rd: u8, rs1: u8, rs2: u8 },
    // instr("remuw   {rd}, {rs1}, {rs2}")
    Remuw  { rd: u8, rs1: u8, rs2: u8 },
    // System
    // instr("ecall")
    Ecall,
    // instr("ebreak")
    Ebreak,
    // instr("fence")
    Fence,
    // instr("csrrw   {rd}, 0x{csr:03x}, {rs1}")
    Csrrw  { rd: u8, rs1: u8, csr: u16 },
    // instr("csrrs   {rd}, 0x{csr:03x}, {rs1}")
    Csrrs  { rd: u8, rs1: u8, csr: u16 },
    // instr("csrrc   {rd}, 0x{csr:03x}, {rs1}")
    Csrrc  { rd: u8, rs1: u8, csr: u16 },
    // instr("csrrwi  {rd}, 0x{csr:03x}, {uimm}")
    Csrrwi { rd: u8, uimm: u8, csr: u16 },
    // instr("csrrsi  {rd}, 0x{csr:03x}, {uimm}")
    Csrrsi { rd: u8, uimm: u8, csr: u16 },
    // instr("csrrci  {rd}, 0x{csr:03x}, {uimm}")
    Csrrci { rd: u8, uimm: u8, csr: u16 },
    // Mret / Sret
    // instr("mret")
    Mret,
    // instr("sret")
    Sret,
    // instr("illegal {_0:#010x}")
    Illegal(u32),
}

include!(concat!(env!("OUT_DIR"), "/disassemble.rs"));

// BIT FIELD HELPERS: extract bits from a raw instruction word (u32).
#[inline(always)] fn bits(x: u32, lo: u32, hi: u32) -> u32 { (x >> lo) & ((1 << (hi - lo + 1)) - 1) }
#[inline(always)] fn bit(x: u32, n: u32) -> u32 { (x >> n) & 1 }

// Sign-extend: takes a value and the position of its sign bit, extends to i64
#[inline(always)] fn sign_ext(x: u32, bit_pos: u32) -> i64 {
    let shift = 63 - bit_pos;
    ((x as i64) << shift) >> shift
}

// IMMEDIATE DECODERS

fn imm_i(raw: u32) -> i64 { sign_ext(bits(raw, 20, 31), 11) }
fn imm_s(raw: u32) -> i64 { sign_ext((bits(raw, 25, 31) << 5) | bits(raw, 7, 11), 11) }
fn imm_b(raw: u32) -> i64 {
    let imm = (bit(raw, 31) << 12) | (bit(raw, 7) << 11)
            | (bits(raw, 25, 30) << 5) | (bits(raw, 8, 11) << 1);
    sign_ext(imm, 12)
}
fn imm_u(raw: u32) -> i64 { sign_ext(raw & 0xFFFF_F000, 31) }
fn imm_j(raw: u32) -> i64 {
    let imm = (bit(raw, 31) << 20) | (bits(raw, 12, 19) << 12)
            | (bit(raw, 20) << 11) | (bits(raw, 21, 30) << 1);
    sign_ext(imm, 20)
}

// DECODER: takes a raw 32-bit instruction word and returns a decoded Inst enum variant.
pub fn decode(raw: u32) -> Inst {
    let opcode = raw & 0x7F;
    let rd     = bits(raw,  7, 11) as u8;
    let rs1    = bits(raw, 15, 19) as u8;
    let rs2    = bits(raw, 20, 24) as u8;
    let funct3 = bits(raw, 12, 14);
    let funct7 = bits(raw, 25, 31);
    let csr    = bits(raw, 20, 31) as u16;

    match opcode {
        0x37 => Inst::Lui   { rd, imm: imm_u(raw) },
        0x17 => Inst::Auipc { rd, imm: imm_u(raw) },
        0x6F => Inst::Jal   { rd, imm: imm_j(raw) },
        0x67 => Inst::Jalr  { rd, rs1, imm: imm_i(raw) },

        0x63 => match funct3 {
            0x0 => Inst::Beq  { rs1, rs2, imm: imm_b(raw) },
            0x1 => Inst::Bne  { rs1, rs2, imm: imm_b(raw) },
            0x4 => Inst::Blt  { rs1, rs2, imm: imm_b(raw) },
            0x5 => Inst::Bge  { rs1, rs2, imm: imm_b(raw) },
            0x6 => Inst::Bltu { rs1, rs2, imm: imm_b(raw) },
            0x7 => Inst::Bgeu { rs1, rs2, imm: imm_b(raw) },
            _   => Inst::Illegal(raw),
        },

        0x03 => match funct3 {
            0x0 => Inst::Lb  { rd, rs1, imm: imm_i(raw) },
            0x1 => Inst::Lh  { rd, rs1, imm: imm_i(raw) },
            0x2 => Inst::Lw  { rd, rs1, imm: imm_i(raw) },
            0x3 => Inst::Ld  { rd, rs1, imm: imm_i(raw) },
            0x4 => Inst::Lbu { rd, rs1, imm: imm_i(raw) },
            0x5 => Inst::Lhu { rd, rs1, imm: imm_i(raw) },
            0x6 => Inst::Lwu { rd, rs1, imm: imm_i(raw) },
            _   => Inst::Illegal(raw),
        },

        0x23 => match funct3 {
            0x0 => Inst::Sb { rs1, rs2, imm: imm_s(raw) },
            0x1 => Inst::Sh { rs1, rs2, imm: imm_s(raw) },
            0x2 => Inst::Sw { rs1, rs2, imm: imm_s(raw) },
            0x3 => Inst::Sd { rs1, rs2, imm: imm_s(raw) },
            _   => Inst::Illegal(raw),
        },

        0x13 => {
            let shamt = bits(raw, 20, 25) as u8;
            match funct3 {
                0x0 => Inst::Addi  { rd, rs1, imm: imm_i(raw) },
                0x2 => Inst::Slti  { rd, rs1, imm: imm_i(raw) },
                0x3 => Inst::Sltiu { rd, rs1, imm: imm_i(raw) },
                0x4 => Inst::Xori  { rd, rs1, imm: imm_i(raw) },
                0x6 => Inst::Ori   { rd, rs1, imm: imm_i(raw) },
                0x7 => Inst::Andi  { rd, rs1, imm: imm_i(raw) },
                0x1 => Inst::Slli  { rd, rs1, shamt },
                0x5 => if funct7 >> 1 == 0x10 { Inst::Srai { rd, rs1, shamt } }
                       else                    { Inst::Srli { rd, rs1, shamt } },
                _   => Inst::Illegal(raw),
            }
        },

        0x33 => match (funct7, funct3) {
            (0x00, 0x0) => Inst::Add  { rd, rs1, rs2 },
            (0x20, 0x0) => Inst::Sub  { rd, rs1, rs2 },
            (0x00, 0x1) => Inst::Sll  { rd, rs1, rs2 },
            (0x00, 0x2) => Inst::Slt  { rd, rs1, rs2 },
            (0x00, 0x3) => Inst::Sltu { rd, rs1, rs2 },
            (0x00, 0x4) => Inst::Xor  { rd, rs1, rs2 },
            (0x00, 0x5) => Inst::Srl  { rd, rs1, rs2 },
            (0x20, 0x5) => Inst::Sra  { rd, rs1, rs2 },
            (0x00, 0x6) => Inst::Or   { rd, rs1, rs2 },
            (0x00, 0x7) => Inst::And  { rd, rs1, rs2 },
            (0x01, 0x0) => Inst::Mul    { rd, rs1, rs2 },
            (0x01, 0x1) => Inst::Mulh   { rd, rs1, rs2 },
            (0x01, 0x2) => Inst::Mulhsu { rd, rs1, rs2 },
            (0x01, 0x3) => Inst::Mulhu  { rd, rs1, rs2 },
            (0x01, 0x4) => Inst::Div    { rd, rs1, rs2 },
            (0x01, 0x5) => Inst::Divu   { rd, rs1, rs2 },
            (0x01, 0x6) => Inst::Rem    { rd, rs1, rs2 },
            (0x01, 0x7) => Inst::Remu   { rd, rs1, rs2 },
            _           => Inst::Illegal(raw),
        },

        0x1B => {
            let shamt = bits(raw, 20, 24) as u8;
            match funct3 {
                0x0 => Inst::Addiw { rd, rs1, imm: imm_i(raw) },
                0x1 => Inst::Slliw { rd, rs1, shamt },
                0x5 => if funct7 == 0x20 { Inst::Sraiw { rd, rs1, shamt } }
                       else              { Inst::Srliw { rd, rs1, shamt } },
                _   => Inst::Illegal(raw),
            }
        },

        0x3B => match (funct7, funct3) {
            (0x00, 0x0) => Inst::Addw  { rd, rs1, rs2 },
            (0x20, 0x0) => Inst::Subw  { rd, rs1, rs2 },
            (0x00, 0x1) => Inst::Sllw  { rd, rs1, rs2 },
            (0x00, 0x5) => Inst::Srlw  { rd, rs1, rs2 },
            (0x20, 0x5) => Inst::Sraw  { rd, rs1, rs2 },
            (0x01, 0x0) => Inst::Mulw  { rd, rs1, rs2 },
            (0x01, 0x4) => Inst::Divw  { rd, rs1, rs2 },
            (0x01, 0x5) => Inst::Divuw { rd, rs1, rs2 },
            (0x01, 0x6) => Inst::Remw  { rd, rs1, rs2 },
            (0x01, 0x7) => Inst::Remuw { rd, rs1, rs2 },
            _           => Inst::Illegal(raw),
        },

         0x2F => {
            let funct5 = bits(raw, 27, 31);
            let aq = bit(raw, 26) != 0;
            let rl = bit(raw, 25) != 0;
            match (funct5, funct3) {
                (0x00, 0x2) => Inst::AmoaddW { rd, rs1, rs2, aq, rl },
                (0x01, 0x2) => Inst::AmoswapW { rd, rs1, rs2, aq, rl },
                (0x02, 0x2) => Inst::LrW { rd, rs1, aq, rl },
                (0x03, 0x2) => Inst::ScW { rd, rs1, rs2, aq, rl },
                (0x02, 0x3) => Inst::LrD { rd, rs1, aq, rl },
                (0x03, 0x3) => Inst::ScD { rd, rs1, rs2, aq, rl },
                _ => Inst::Illegal(raw),
            }
        }

         0x73 => match funct3 {
             0x0 => match raw {
                 0x0000_0073 => Inst::Ecall,
                 0x0010_0073 => Inst::Ebreak,
                 0x1020_0073 => Inst::Sret,
                 0x1050_0073 => Inst::Fence,
                 0x3020_0073 => Inst::Mret,
                 _ if raw & 0xFE007FFF == 0x12000073 => Inst::Fence,
                 _           => Inst::Illegal(raw),
            },
            0x1 => Inst::Csrrw  { rd, rs1,       csr },
            0x2 => Inst::Csrrs  { rd, rs1,       csr },
            0x3 => Inst::Csrrc  { rd, rs1,       csr },
            0x5 => Inst::Csrrwi { rd, uimm: rs1, csr },
            0x6 => Inst::Csrrsi { rd, uimm: rs1, csr },
            0x7 => Inst::Csrrci { rd, uimm: rs1, csr },
            _   => Inst::Illegal(raw),
        },

        0x0F => Inst::Fence,
        _    => Inst::Illegal(raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x33
    }
    fn rw(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x3B
    }
    fn i(imm: u32, rs1: u32, funct3: u32, rd: u32, opcode: u32) -> u32 {
        (imm << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    }
    fn iw(imm: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (imm << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x1B
    }
    fn s(imm: u32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
        let imm_hi = (imm >> 5) & 0x7F;
        let imm_lo = imm & 0x1F;
        (imm_hi << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (imm_lo << 7) | 0x23
    }
    fn b(imm: u32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
        let bit12 = (imm >> 12) & 1;
        let bit11 = (imm >> 11) & 1;
        let bits_10_5 = (imm >> 5) & 0x3F;
        let bits_4_1 = (imm >> 1) & 0xF;
        (bit12 << 31) | (bits_10_5 << 25) | (rs2 << 20) | (rs1 << 15)
            | (funct3 << 12) | (bits_4_1 << 8) | (bit11 << 7) | 0x63
    }
    fn u(imm: u32, rd: u32, opcode: u32) -> u32 {
        imm & 0xFFFFF000 | (rd << 7) | opcode
    }
    fn j(imm: u32, rd: u32) -> u32 {
        let bit20 = (imm >> 20) & 1;
        let bits_10_1 = (imm >> 1) & 0x3FF;
        let bit11 = (imm >> 11) & 1;
        let bits_19_12 = (imm >> 12) & 0xFF;
        (bit20 << 31) | (bits_10_1 << 21) | (bit11 << 20) | (bits_19_12 << 12) | (rd << 7) | 0x6F
    }
    // ---- RV64I R-type ----
    #[test]
    fn test_add() { assert_eq!(decode(r(0x00, 3, 2, 0, 1)), Inst::Add { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_sub() { assert_eq!(decode(r(0x20, 3, 2, 0, 1)), Inst::Sub { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_sll() { assert_eq!(decode(r(0x00, 3, 2, 1, 1)), Inst::Sll { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_slt() { assert_eq!(decode(r(0x00, 3, 2, 2, 1)), Inst::Slt { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_sltu() { assert_eq!(decode(r(0x00, 3, 2, 3, 1)), Inst::Sltu { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_xor() { assert_eq!(decode(r(0x00, 3, 2, 4, 1)), Inst::Xor { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_srl() { assert_eq!(decode(r(0x00, 3, 2, 5, 1)), Inst::Srl { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_sra() { assert_eq!(decode(r(0x20, 3, 2, 5, 1)), Inst::Sra { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_or()  { assert_eq!(decode(r(0x00, 3, 2, 6, 1)), Inst::Or  { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_and() { assert_eq!(decode(r(0x00, 3, 2, 7, 1)), Inst::And { rd: 1, rs1: 2, rs2: 3 }); }

    // ---- RV64M R-type ----
    #[test]
    fn test_mul()    { assert_eq!(decode(r(0x01, 3, 2, 0, 1)), Inst::Mul    { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_mulh()   { assert_eq!(decode(r(0x01, 3, 2, 1, 1)), Inst::Mulh   { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_mulhsu() { assert_eq!(decode(r(0x01, 3, 2, 2, 1)), Inst::Mulhsu { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_mulhu()  { assert_eq!(decode(r(0x01, 3, 2, 3, 1)), Inst::Mulhu  { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_div()    { assert_eq!(decode(r(0x01, 3, 2, 4, 1)), Inst::Div    { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_divu()   { assert_eq!(decode(r(0x01, 3, 2, 5, 1)), Inst::Divu   { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_rem()    { assert_eq!(decode(r(0x01, 3, 2, 6, 1)), Inst::Rem    { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_remu()   { assert_eq!(decode(r(0x01, 3, 2, 7, 1)), Inst::Remu   { rd: 1, rs1: 2, rs2: 3 }); }

    // ---- RV64I I-type (ALU immediate) ----
    #[test]
    fn test_addi() { assert_eq!(decode(i(0xFFF, 2, 0, 1, 0x13)), Inst::Addi { rd: 1, rs1: 2, imm: -1 }); }
    #[test]
    fn test_addi_zero() { assert_eq!(decode(i(0, 0, 0, 0, 0x13)), Inst::Addi { rd: 0, rs1: 0, imm: 0 }); }
    #[test]
    fn test_slti() { assert_eq!(decode(i(10, 2, 2, 1, 0x13)), Inst::Slti { rd: 1, rs1: 2, imm: 10 }); }
    #[test]
    fn test_sltiu() { assert_eq!(decode(i(10, 2, 3, 1, 0x13)), Inst::Sltiu { rd: 1, rs1: 2, imm: 10 }); }
    #[test]
    fn test_xori() { assert_eq!(decode(i(0xFF, 2, 4, 1, 0x13)), Inst::Xori { rd: 1, rs1: 2, imm: 0xFF }); }
    #[test]
    fn test_ori()  { assert_eq!(decode(i(0xFF, 2, 6, 1, 0x13)), Inst::Ori  { rd: 1, rs1: 2, imm: 0xFF }); }
    #[test]
    fn test_andi() { assert_eq!(decode(i(0xFF, 2, 7, 1, 0x13)), Inst::Andi { rd: 1, rs1: 2, imm: 0xFF }); }

    // ---- Shifts (I-type with funct3=1/5) ----
    #[test]
    fn test_slli() {
        let raw = (0 << 26) | (5 << 20) | (2 << 15) | (1 << 12) | (1 << 7) | 0x13;
        assert_eq!(decode(raw), Inst::Slli { rd: 1, rs1: 2, shamt: 5 });
    }
    #[test]
    fn test_srli() {
        let raw = (0 << 26) | (5 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x13;
        assert_eq!(decode(raw), Inst::Srli { rd: 1, rs1: 2, shamt: 5 });
    }
    #[test]
    fn test_srai() {
        let raw = (0x10 << 26) | (5 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x13;
        assert_eq!(decode(raw), Inst::Srai { rd: 1, rs1: 2, shamt: 5 });
    }

    // ---- U-type ----
    #[test]
    fn test_lui() {
        assert_eq!(decode(u(0x12345000, 1, 0x37)), Inst::Lui { rd: 1, imm: 0x12345000 });
    }
    #[test]
    fn test_auipc() {
        assert_eq!(decode(u(0x12345000, 1, 0x17)), Inst::Auipc { rd: 1, imm: 0x12345000 });
    }

    // ---- J-type ----
    #[test]
    fn test_jal() {
        assert_eq!(decode(j(0x100, 1)), Inst::Jal { rd: 1, imm: 0x100 });
    }
    #[test]
    fn test_jal_neg() {
        assert_eq!(decode(j(0x1FFFFE, 1)), Inst::Jal { rd: 1, imm: -2 });
    }

    // ---- I-type (JALR) ----
    #[test]
    fn test_jalr() {
        assert_eq!(decode(i(0x100, 2, 0, 1, 0x67)), Inst::Jalr { rd: 1, rs1: 2, imm: 0x100 });
    }

    // ---- B-type ----
    #[test]
    fn test_beq()  { assert_eq!(decode(b(8, 3, 2, 0)), Inst::Beq  { rs1: 2, rs2: 3, imm: 8 }); }
    #[test]
    fn test_bne()  { assert_eq!(decode(b(8, 3, 2, 1)), Inst::Bne  { rs1: 2, rs2: 3, imm: 8 }); }
    #[test]
    fn test_blt()  { assert_eq!(decode(b(8, 3, 2, 4)), Inst::Blt  { rs1: 2, rs2: 3, imm: 8 }); }
    #[test]
    fn test_bge()  { assert_eq!(decode(b(8, 3, 2, 5)), Inst::Bge  { rs1: 2, rs2: 3, imm: 8 }); }
    #[test]
    fn test_bltu() { assert_eq!(decode(b(8, 3, 2, 6)), Inst::Bltu { rs1: 2, rs2: 3, imm: 8 }); }
    #[test]
    fn test_bgeu() { assert_eq!(decode(b(8, 3, 2, 7)), Inst::Bgeu { rs1: 2, rs2: 3, imm: 8 }); }

    // ---- I-type (loads) ----
    #[test]
    fn test_lb()  { assert_eq!(decode(i(0x100, 2, 0, 1, 0x03)), Inst::Lb  { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_lh()  { assert_eq!(decode(i(0x100, 2, 1, 1, 0x03)), Inst::Lh  { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_lw()  { assert_eq!(decode(i(0x100, 2, 2, 1, 0x03)), Inst::Lw  { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_ld()  { assert_eq!(decode(i(0x100, 2, 3, 1, 0x03)), Inst::Ld  { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_lbu() { assert_eq!(decode(i(0x100, 2, 4, 1, 0x03)), Inst::Lbu { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_lhu() { assert_eq!(decode(i(0x100, 2, 5, 1, 0x03)), Inst::Lhu { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_lwu() { assert_eq!(decode(i(0x100, 2, 6, 1, 0x03)), Inst::Lwu { rd: 1, rs1: 2, imm: 0x100 }); }

    // ---- S-type ----
    #[test]
    fn test_sb() { assert_eq!(decode(s(0x108, 3, 2, 0)), Inst::Sb { rs1: 2, rs2: 3, imm: 0x108 }); }
    #[test]
    fn test_sh() { assert_eq!(decode(s(0x108, 3, 2, 1)), Inst::Sh { rs1: 2, rs2: 3, imm: 0x108 }); }
    #[test]
    fn test_sw() { assert_eq!(decode(s(0x108, 3, 2, 2)), Inst::Sw { rs1: 2, rs2: 3, imm: 0x108 }); }
    #[test]
    fn test_sd() { assert_eq!(decode(s(0x108, 3, 2, 3)), Inst::Sd { rs1: 2, rs2: 3, imm: 0x108 }); }

    // ---- System instructions ----
    #[test]
    fn test_ecall()  { assert_eq!(decode(0x00000073), Inst::Ecall); }
    #[test]
    fn test_ebreak() { assert_eq!(decode(0x00100073), Inst::Ebreak); }
    #[test]
    fn test_mret()   { assert_eq!(decode(0x30200073), Inst::Mret); }
    #[test]
    fn test_sret()   { assert_eq!(decode(0x10200073), Inst::Sret); }
    #[test]
    fn test_wfi()    { assert_eq!(decode(0x10500073), Inst::Fence); }
    #[test]
    fn test_fence()  { assert_eq!(decode(0x0FF0000F), Inst::Fence); }
    #[test]
    fn test_sfence_vma() { assert_eq!(decode(0x12000073), Inst::Fence); }
    #[test]
    fn test_sfence_vma_rs1() { assert_eq!(decode(0x12050073), Inst::Fence); }

    // ---- CSR instructions ----
    #[test]
    fn test_csrrw()  { assert_eq!(decode(0x30051073), Inst::Csrrw  { rd: 0, rs1: 10, csr: 0x300 }); }
    #[test]
    fn test_csrrs()  { assert_eq!(decode(0x30052073), Inst::Csrrs  { rd: 0, rs1: 10, csr: 0x300 }); }
    #[test]
    fn test_csrrc()  { assert_eq!(decode(0x30053073), Inst::Csrrc  { rd: 0, rs1: 10, csr: 0x300 }); }
    #[test]
    fn test_csrrwi() { assert_eq!(decode(0x30055073), Inst::Csrrwi { rd: 0, uimm: 10, csr: 0x300 }); }
    #[test]
    fn test_csrrsi() { assert_eq!(decode(0x30056073), Inst::Csrrsi { rd: 0, uimm: 10, csr: 0x300 }); }
    #[test]
    fn test_csrrci() { assert_eq!(decode(0x30057073), Inst::Csrrci { rd: 0, uimm: 10, csr: 0x300 }); }

    // ---- 32-bit immediate ALU (RV64) ----
    #[test]
    fn test_addiw() { assert_eq!(decode(iw(0x100, 2, 0, 1)), Inst::Addiw { rd: 1, rs1: 2, imm: 0x100 }); }
    #[test]
    fn test_slliw() {
        let raw = (5 << 20) | (2 << 15) | (1 << 12) | (1 << 7) | 0x1B;
        assert_eq!(decode(raw), Inst::Slliw { rd: 1, rs1: 2, shamt: 5 });
    }
    #[test]
    fn test_srliw() {
        let raw = (5 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x1B;
        assert_eq!(decode(raw), Inst::Srliw { rd: 1, rs1: 2, shamt: 5 });
    }
    #[test]
    fn test_sraiw() {
        let raw = (0x20 << 25) | (5 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x1B;
        assert_eq!(decode(raw), Inst::Sraiw { rd: 1, rs1: 2, shamt: 5 });
    }

    // ---- 32-bit register ALU (RV64) ----
    #[test]
    fn test_addw() { assert_eq!(decode(rw(0x00, 3, 2, 0, 1)), Inst::Addw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_subw() { assert_eq!(decode(rw(0x20, 3, 2, 0, 1)), Inst::Subw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_sllw() { assert_eq!(decode(rw(0x00, 3, 2, 1, 1)), Inst::Sllw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_srlw() { assert_eq!(decode(rw(0x00, 3, 2, 5, 1)), Inst::Srlw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_sraw() { assert_eq!(decode(rw(0x20, 3, 2, 5, 1)), Inst::Sraw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_mulw() { assert_eq!(decode(rw(0x01, 3, 2, 0, 1)), Inst::Mulw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_divw() { assert_eq!(decode(rw(0x01, 3, 2, 4, 1)), Inst::Divw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_divuw(){ assert_eq!(decode(rw(0x01, 3, 2, 5, 1)), Inst::Divuw{ rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_remw() { assert_eq!(decode(rw(0x01, 3, 2, 6, 1)), Inst::Remw { rd: 1, rs1: 2, rs2: 3 }); }
    #[test]
    fn test_remuw(){ assert_eq!(decode(rw(0x01, 3, 2, 7, 1)), Inst::Remuw{ rd: 1, rs1: 2, rs2: 3 }); }

    // ---- Atomic instructions ----
    fn amos(imm: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (imm << 27) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x2F
    }
    #[test]
    fn test_amoaddw()  { assert_eq!(decode(amos(0, 3, 2, 2, 1)), Inst::AmoaddW { rd: 1, rs1: 2, rs2: 3, aq: false, rl: false }); }
    #[test]
    fn test_amoswapw() { assert_eq!(decode(amos(1, 3, 2, 2, 1)), Inst::AmoswapW{ rd: 1, rs1: 2, rs2: 3, aq: false, rl: false }); }
    #[test]
    fn test_lrw()      { assert_eq!(decode(amos(2, 0, 2, 2, 1)), Inst::LrW { rd: 1, rs1: 2, aq: false, rl: false }); }
    #[test]
    fn test_scw()      { assert_eq!(decode(amos(3, 3, 2, 2, 1)), Inst::ScW { rd: 1, rs1: 2, rs2: 3, aq: false, rl: false }); }
    #[test]
    fn test_lrd()      { assert_eq!(decode(amos(2, 0, 2, 3, 1)), Inst::LrD { rd: 1, rs1: 2, aq: false, rl: false }); }
    #[test]
    fn test_scd()      { assert_eq!(decode(amos(3, 3, 2, 3, 1)), Inst::ScD { rd: 1, rs1: 2, rs2: 3, aq: false, rl: false }); }

    // ---- Illegal instructions ----
    #[test]
    fn test_illegal_opcode() { assert_eq!(decode(0xFFFFFFFF), Inst::Illegal(0xFFFFFFFF)); }
    #[test]
    fn test_illegal_load_funct3() {
        let raw = i(0x100, 2, 7, 1, 0x03);
        assert_eq!(decode(raw), Inst::Illegal(raw));
    }
    #[test]
    fn test_illegal_store_funct3() {
        let raw = s(0x108, 3, 2, 7);
        assert_eq!(decode(raw), Inst::Illegal(raw));
    }

    // ---- Max register numbers ----
    #[test]
    fn test_max_regs() {
        assert_eq!(decode(r(0x00, 31, 31, 0, 31)), Inst::Add { rd: 31, rs1: 31, rs2: 31 });
    }

    // ---- Immediate sign extension ----
    #[test]
    fn test_imm_sign_ext() {
        let raw = i(0x800, 0, 0, 1, 0x13);
        assert_eq!(decode(raw), Inst::Addi { rd: 1, rs1: 0, imm: -2048 });
    }
    #[test]
    fn test_imm_pos() {
        let raw = i(0x7FF, 0, 0, 1, 0x13);
        assert_eq!(decode(raw), Inst::Addi { rd: 1, rs1: 0, imm: 2047 });
    }
}
