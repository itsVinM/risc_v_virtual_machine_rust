/// All RISC-V instruction formats decoded via bit manipulation (no string parsing).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inst {
    // RV64I
    Lui   { rd: u8, imm: i64 },
    Auipc { rd: u8, imm: i64 },
    Jal   { rd: u8, imm: i64 },
    Jalr  { rd: u8, rs1: u8, imm: i64 },
    Beq   { rs1: u8, rs2: u8, imm: i64 },
    Bne   { rs1: u8, rs2: u8, imm: i64 },
    Blt   { rs1: u8, rs2: u8, imm: i64 },
    Bge   { rs1: u8, rs2: u8, imm: i64 },
    Bltu  { rs1: u8, rs2: u8, imm: i64 },
    Bgeu  { rs1: u8, rs2: u8, imm: i64 },
    Lb    { rd: u8, rs1: u8, imm: i64 },
    Lh    { rd: u8, rs1: u8, imm: i64 },
    Lw    { rd: u8, rs1: u8, imm: i64 },
    Ld    { rd: u8, rs1: u8, imm: i64 },
    Lbu   { rd: u8, rs1: u8, imm: i64 },
    Lhu   { rd: u8, rs1: u8, imm: i64 },
    Lwu   { rd: u8, rs1: u8, imm: i64 },
    Sb    { rs1: u8, rs2: u8, imm: i64 },
    Sh    { rs1: u8, rs2: u8, imm: i64 },
    Sw    { rs1: u8, rs2: u8, imm: i64 },
    Sd    { rs1: u8, rs2: u8, imm: i64 },
    Addi  { rd: u8, rs1: u8, imm: i64 },
    Slti  { rd: u8, rs1: u8, imm: i64 },
    Sltiu { rd: u8, rs1: u8, imm: i64 },
    Xori  { rd: u8, rs1: u8, imm: i64 },
    Ori   { rd: u8, rs1: u8, imm: i64 },
    Andi  { rd: u8, rs1: u8, imm: i64 },
    Slli  { rd: u8, rs1: u8, shamt: u8 },
    Srli  { rd: u8, rs1: u8, shamt: u8 },
    Srai  { rd: u8, rs1: u8, shamt: u8 },
    Add   { rd: u8, rs1: u8, rs2: u8 },
    Sub   { rd: u8, rs1: u8, rs2: u8 },
    Sll   { rd: u8, rs1: u8, rs2: u8 },
    Slt   { rd: u8, rs1: u8, rs2: u8 },
    Sltu  { rd: u8, rs1: u8, rs2: u8 },
    Xor   { rd: u8, rs1: u8, rs2: u8 },
    Srl   { rd: u8, rs1: u8, rs2: u8 },
    Sra   { rd: u8, rs1: u8, rs2: u8 },
    Or    { rd: u8, rs1: u8, rs2: u8 },
    And   { rd: u8, rs1: u8, rs2: u8 },
    // RV64I W-suffix (32-bit ops sign-extended)
    Addiw { rd: u8, rs1: u8, imm: i64 },
    Slliw { rd: u8, rs1: u8, shamt: u8 },
    Srliw { rd: u8, rs1: u8, shamt: u8 },
    Sraiw { rd: u8, rs1: u8, shamt: u8 },
    Addw  { rd: u8, rs1: u8, rs2: u8 },
    Subw  { rd: u8, rs1: u8, rs2: u8 },
    Sllw  { rd: u8, rs1: u8, rs2: u8 },
    Srlw  { rd: u8, rs1: u8, rs2: u8 },
    Sraw  { rd: u8, rs1: u8, rs2: u8 },
    // RV64M
    Mul    { rd: u8, rs1: u8, rs2: u8 },
    Mulh   { rd: u8, rs1: u8, rs2: u8 },
    Mulhsu { rd: u8, rs1: u8, rs2: u8 },
    Mulhu  { rd: u8, rs1: u8, rs2: u8 },
    Div    { rd: u8, rs1: u8, rs2: u8 },
    Divu   { rd: u8, rs1: u8, rs2: u8 },
    Rem    { rd: u8, rs1: u8, rs2: u8 },
    Remu   { rd: u8, rs1: u8, rs2: u8 },
    Mulw   { rd: u8, rs1: u8, rs2: u8 },
    Divw   { rd: u8, rs1: u8, rs2: u8 },
    Divuw  { rd: u8, rs1: u8, rs2: u8 },
    Remw   { rd: u8, rs1: u8, rs2: u8 },
    Remuw  { rd: u8, rs1: u8, rs2: u8 },
    // System
    Ecall,
    Ebreak,
    Fence,
    Csrrw  { rd: u8, rs1: u8, csr: u16 },
    Csrrs  { rd: u8, rs1: u8, csr: u16 },
    Csrrc  { rd: u8, rs1: u8, csr: u16 },
    Csrrwi { rd: u8, uimm: u8, csr: u16 },
    Csrrsi { rd: u8, uimm: u8, csr: u16 },
    Csrrci { rd: u8, uimm: u8, csr: u16 },
    // Mret
    Mret,
    // Compressed (C extension) - placeholder, returns Illegal if not handled
    Illegal(u32),
}

// ── Bit field helpers ─────────────────────────────────────────────────────────
// Every RISC-V instruction is 32 bits. These extract named slices of those bits.
// Instruction layout: [31..25 funct7][24..20 rs2][19..15 rs1][14..12 funct3][11..7 rd][6..0 opcode]

#[inline(always)] fn bits(x: u32, lo: u32, hi: u32) -> u32 { (x >> lo) & ((1 << (hi - lo + 1)) - 1) }
#[inline(always)] fn bit(x: u32, n: u32) -> u32 { (x >> n) & 1 }

// Sign-extend: takes a value and the position of its sign bit, extends to i64
#[inline(always)] fn sign_ext(x: u32, bit_pos: u32) -> i64 {
    let shift = 63 - bit_pos;
    ((x as i64) << shift) >> shift
}

// ── Immediate decoders ────────────────────────────────────────────────────────
// RISC-V scrambles immediate bits differently per format to keep rs1/rs2 in fixed positions.
// Each function reassembles the bits into a sign-extended i64.

fn imm_i(raw: u32) -> i64 { sign_ext(bits(raw, 20, 31), 11) }
fn imm_s(raw: u32) -> i64 { sign_ext((bits(raw, 25, 31) << 5) | bits(raw, 7, 11), 11) }
fn imm_b(raw: u32) -> i64 {
    // B-format: [12|10:5|4:1|11] — branch offset, always even (bit 0 implicit)
    let imm = (bit(raw, 31) << 12) | (bit(raw, 7) << 11)
            | (bits(raw, 25, 30) << 5) | (bits(raw, 8, 11) << 1);
    sign_ext(imm, 12)
}
fn imm_u(raw: u32) -> i64 { sign_ext(raw & 0xFFFF_F000, 31) }
fn imm_j(raw: u32) -> i64 {
    // J-format: [20|10:1|11|19:12] — jump offset, always even (bit 0 implicit)
    let imm = (bit(raw, 31) << 20) | (bits(raw, 12, 19) << 12)
            | (bit(raw, 20) << 11) | (bits(raw, 21, 30) << 1);
    sign_ext(imm, 20)
}

// ── Decoder ───────────────────────────────────────────────────────────────────
// Opcode (bits 6:0) selects the instruction group.
// funct3 (bits 14:12) and funct7 (bits 31:25) disambiguate within a group.
// All opcodes and encoding values come from RISC-V Vol I, Chapter 24.

pub fn decode(raw: u32) -> Inst {
    let opcode = raw & 0x7F;
    let rd     = bits(raw,  7, 11) as u8;
    let rs1    = bits(raw, 15, 19) as u8;
    let rs2    = bits(raw, 20, 24) as u8;
    let funct3 = bits(raw, 12, 14);
    let funct7 = bits(raw, 25, 31);
    let csr    = bits(raw, 20, 31) as u16;

    match opcode {
        0x37 => Inst::Lui   { rd, imm: imm_u(raw) }, // load upper immediate
        0x17 => Inst::Auipc { rd, imm: imm_u(raw) }, // add upper immediate to pc
        0x6F => Inst::Jal   { rd, imm: imm_j(raw) }, // jump and link
        0x67 => Inst::Jalr  { rd, rs1, imm: imm_i(raw) }, // jump and link register

        0x63 => match funct3 { // branches — compare rs1,rs2 and jump by imm if true
            0x0 => Inst::Beq  { rs1, rs2, imm: imm_b(raw) },
            0x1 => Inst::Bne  { rs1, rs2, imm: imm_b(raw) },
            0x4 => Inst::Blt  { rs1, rs2, imm: imm_b(raw) },
            0x5 => Inst::Bge  { rs1, rs2, imm: imm_b(raw) },
            0x6 => Inst::Bltu { rs1, rs2, imm: imm_b(raw) },
            0x7 => Inst::Bgeu { rs1, rs2, imm: imm_b(raw) },
            _   => Inst::Illegal(raw),
        },

        0x03 => match funct3 { // loads — rd = mem[rs1 + imm], funct3 = width + signedness
            0x0 => Inst::Lb  { rd, rs1, imm: imm_i(raw) }, // load byte (signed)
            0x1 => Inst::Lh  { rd, rs1, imm: imm_i(raw) }, // load halfword (signed)
            0x2 => Inst::Lw  { rd, rs1, imm: imm_i(raw) }, // load word (signed)
            0x3 => Inst::Ld  { rd, rs1, imm: imm_i(raw) }, // load doubleword
            0x4 => Inst::Lbu { rd, rs1, imm: imm_i(raw) }, // load byte (unsigned)
            0x5 => Inst::Lhu { rd, rs1, imm: imm_i(raw) }, // load halfword (unsigned)
            0x6 => Inst::Lwu { rd, rs1, imm: imm_i(raw) }, // load word (unsigned)
            _   => Inst::Illegal(raw),
        },

        0x23 => match funct3 { // stores — mem[rs1 + imm] = rs2, funct3 = width
            0x0 => Inst::Sb { rs1, rs2, imm: imm_s(raw) }, // store byte
            0x1 => Inst::Sh { rs1, rs2, imm: imm_s(raw) }, // store halfword
            0x2 => Inst::Sw { rs1, rs2, imm: imm_s(raw) }, // store word
            0x3 => Inst::Sd { rs1, rs2, imm: imm_s(raw) }, // store doubleword
            _   => Inst::Illegal(raw),
        },

        0x13 => { // immediate ALU ops — rd = rs1 OP imm
            let shamt = bits(raw, 20, 25) as u8; // shift amount for shift instructions
            match funct3 {
                0x0 => Inst::Addi  { rd, rs1, imm: imm_i(raw) },
                0x2 => Inst::Slti  { rd, rs1, imm: imm_i(raw) },
                0x3 => Inst::Sltiu { rd, rs1, imm: imm_i(raw) },
                0x4 => Inst::Xori  { rd, rs1, imm: imm_i(raw) },
                0x6 => Inst::Ori   { rd, rs1, imm: imm_i(raw) },
                0x7 => Inst::Andi  { rd, rs1, imm: imm_i(raw) },
                0x1 => Inst::Slli  { rd, rs1, shamt },
                0x5 => if funct7 >> 1 == 0x10 { Inst::Srai { rd, rs1, shamt } } // arithmetic (sign-fill)
                       else                    { Inst::Srli { rd, rs1, shamt } }, // logical (zero-fill)
                _   => Inst::Illegal(raw),
            }
        },

        0x33 => match (funct7, funct3) { // register ALU ops — rd = rs1 OP rs2
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
            // M extension (funct7=0x01): multiply/divide
            (0x01, 0x0) => Inst::Mul    { rd, rs1, rs2 },
            (0x01, 0x1) => Inst::Mulh   { rd, rs1, rs2 }, // upper 64 bits of signed*signed
            (0x01, 0x2) => Inst::Mulhsu { rd, rs1, rs2 }, // upper 64 bits of signed*unsigned
            (0x01, 0x3) => Inst::Mulhu  { rd, rs1, rs2 }, // upper 64 bits of unsigned*unsigned
            (0x01, 0x4) => Inst::Div    { rd, rs1, rs2 },
            (0x01, 0x5) => Inst::Divu   { rd, rs1, rs2 },
            (0x01, 0x6) => Inst::Rem    { rd, rs1, rs2 },
            (0x01, 0x7) => Inst::Remu   { rd, rs1, rs2 },
            _           => Inst::Illegal(raw),
        },

        0x1B => { // 32-bit immediate ALU ops (RV64 only) — result sign-extended to 64 bits
            let shamt = bits(raw, 20, 24) as u8;
            match funct3 {
                0x0 => Inst::Addiw { rd, rs1, imm: imm_i(raw) },
                0x1 => Inst::Slliw { rd, rs1, shamt },
                0x5 => if funct7 == 0x20 { Inst::Sraiw { rd, rs1, shamt } }
                       else              { Inst::Srliw { rd, rs1, shamt } },
                _   => Inst::Illegal(raw),
            }
        },

        0x3B => match (funct7, funct3) { // 32-bit register ALU ops (RV64 only)
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

        0x73 => match funct3 { // system instructions
            // funct3=0: match full word because ecall/ebreak/mret share opcode+funct3
            0x0 => match raw {
                0x0000_0073 => Inst::Ecall,
                0x0010_0073 => Inst::Ebreak,
                0x1050_0073 => Inst::Fence,  // WFI — wait for interrupt, no-op on single core
                0x3020_0073 => Inst::Mret,   // return from machine-mode trap
                _           => Inst::Illegal(raw),
            },
            // CSR instructions: read old value into rd, then write/set/clear with rs1
            0x1 => Inst::Csrrw  { rd, rs1,       csr }, // atomic read+write
            0x2 => Inst::Csrrs  { rd, rs1,       csr }, // atomic read+set bits
            0x3 => Inst::Csrrc  { rd, rs1,       csr }, // atomic read+clear bits
            0x5 => Inst::Csrrwi { rd, uimm: rs1, csr }, // immediate versions
            0x6 => Inst::Csrrsi { rd, uimm: rs1, csr },
            0x7 => Inst::Csrrci { rd, uimm: rs1, csr },
            _   => Inst::Illegal(raw),
        },

        0x0F => Inst::Fence, // memory ordering — no-op on single core
        _    => Inst::Illegal(raw),
    }
}
