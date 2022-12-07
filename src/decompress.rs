use crate::{DecodingError, Register};

type DecompressionResult = Result<u32, DecodingError>;

enum CrInstr {
    Sub,
}

enum CiInstr {
    Addi,
    Lw,
}

fn build_rtype(instruction_type: CrInstr, rd: u16, rs1: u16, rs2: u16) -> u32 {
    let mold = |funct7: u32, rs2: u16, rs1: u16, funct3: u32, rd: u16, opcode: u32| -> u32 {
        let rd: u32 = rd.into();
        let rs1: u32 = rs1.into();
        let rs2: u32 = rs2.into();

        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    };

    match instruction_type {
        CrInstr::Sub => mold(0b0100000, rs2, rs1, 0b000, rd, 0b0110011),
    }
}

fn build_itype(instruction_type: CiInstr, rd: u16, rs1: u16, imm: u16) -> u32 {
    let mold = |imm: u16, rs1: u16, funct3: u32, rd: u16, opcode: u32| -> u32 {
        let rd: u32 = rd.into();
        let rs1: u32 = rs1.into();
        let imm: u32 = imm.into();

        (imm << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    };

    match instruction_type {
        CiInstr::Addi => mold(imm, rs1, 0b000, rd, 0b0010011),
        CiInstr::Lw => mold(imm, Register::Sp as u16, 0b010, rd, 0b0000011),
    }
}
pub fn decompress_q0(i: u16) -> DecompressionResult {
    match (i >> 13) & 0b111 {
        0b000 => Err(DecodingError::Illegal),
        0b001 /* C.FLD */ => Err(DecodingError::Unimplemented),
        0b010 /* C.LW */ => Err(DecodingError::Unimplemented),
        0b011 /* C.LD */ => Err(DecodingError::Unimplemented),
        0b100 => Err(DecodingError::Reserved),
        0b101 /* C.FSD */ => Err(DecodingError::Unimplemented),
        0b110 /* C.SW */ => Err(DecodingError::Unimplemented),
        0b111 /* C.SD */ => Err(DecodingError::Unimplemented),
        _ => unreachable!(),
    }
}

pub fn decompress_q1(i: u16) -> DecompressionResult {
    match (i >> 13) & 0b111 {
        0b000 /* C.ADDI */ => Err(DecodingError::Unimplemented),
        0b001 /* C.ADDIW */ => Err(DecodingError::Unimplemented),
        0b010 /* C.LI */ => {
            let rd = (i >> 7) & 0b11111;
            let imm = get_imm(i, InstrFormat::Ci);

            assert!(rd != 0, "rd == 0 is reserved!");

            Ok(build_itype(CiInstr::Addi, rd, Register::Zero as u16, imm))
        }
        0b011 /* C.LUI/C.ADDI16SP */ => Err(DecodingError::Unimplemented),
        0b100 /* MISC-ALU */ => match (i >> 10) & 0b11 {
            0b00 => Err(DecodingError::Unimplemented),
            0b01 => Err(DecodingError::Unimplemented),
            0b10 => Err(DecodingError::Unimplemented),
            0b11 => {
                let rs1_rd = 8 + ((i >> 7) & 0b111);
                let rs2 = 8 + ((i >> 2) & 0b111);

                match ((i >> 12) & 0b1, (i >> 5) & 0b11) {
                    (0, 0b00) => Ok(build_rtype(CrInstr::Sub, rs1_rd, rs1_rd, rs2)),
                    (1, 0b10) => Err(DecodingError::Reserved),
                    (1, 0b11) => Err(DecodingError::Reserved),
                    _ => unreachable!(),
                }
            }
            _ => Err(DecodingError::Unimplemented),
        },
        0b101 /* C.J */ => Err(DecodingError::Unimplemented),
        0b110 /* C.BEQZ */ => Err(DecodingError::Unimplemented),
        0b111 /* C.BNEZ */ => Err(DecodingError::Unimplemented),
        _ => unreachable!(),
    }
}

pub fn decompress_q2(i: u16) -> DecompressionResult {
    match (i >> 13) & 0b111 {
        0b000 /* C.SLLI{,64} */ => Err(DecodingError::Unimplemented),
        0b001 /* C.FLDSP */ => Err(DecodingError::Unimplemented),
        0b010 /* C.LWSP */ => {
            let rd = (i >> 7) & 0b11111;
            let imm = get_imm(i, InstrFormat::Ci).inv_permute(&[5, 4, 3, 2, 7, 6]);

            assert!(rd != 0, "rd == 0 is reserved!");

            Ok(build_itype(CiInstr::Lw, rd, 0, imm))
        }
        0b011 /* C.LDSP */ => Err(DecodingError::Unimplemented),
        0b100 /* C.{RJ,MV,EBREAK,JALR,ADD} */ => Err(DecodingError::Unimplemented),
        0b101 /* C.FSDSP */ => Err(DecodingError::Unimplemented),
        0b110 /* C.SWSP */ => Err(DecodingError::Unimplemented),
        0b111 /* C.SDSP */ => Err(DecodingError::Unimplemented),
        _ => unreachable!(),
    }
}

enum InstrFormat {
    Ci,
}

#[inline(always)]
fn get_imm(i: u16, fmt: InstrFormat) -> u16 {
    match fmt {
        InstrFormat::Ci => ((i >> 7) & 0b10_0000) | ((i >> 2) & 0b1_1111),
    }
}

trait Permutable {
    /// When going from an number to the permuted representation in an instruction.
    fn permute(self, perm: &[usize]) -> Self;

    /// When going from a permuted number in an instruction to the binary representation.
    fn inv_permute(self, perm: &[usize]) -> Self;
}

impl Permutable for u16 {
    fn inv_permute(self, perm: &[usize]) -> Self {
        debug_assert!(
            perm.len() <= 16, 
            "Permutation of u16 cannot exceed 16 entries."
        );
        debug_assert!(
            perm.iter().all(|x| x < &16), 
            "Permutation indices for u16 cannot exceed 15."
        );

        perm.iter()
            .rev()
            .enumerate()
            .map(|(bit, offset)| ((self >> bit) & 0b1) << offset)
            .sum()
    }

    fn permute(self, perm: &[usize]) -> Self {
        debug_assert!(
            perm.len() <= 16,
            "Permutation of u16 cannot exceed 16 entries."
        );
        debug_assert!(
            perm.iter().all(|x| x < &16),
            "Permutation indices for u16 cannot exceed 15."
        );

        perm.iter()
            .rev()
            .enumerate()
            .map(|(bit, offset)| ((self >> offset) & 0b1) << bit)
            .sum()
    }
}

impl Permutable for u32 {
    fn inv_permute(self, perm: &[usize]) -> Self {
        debug_assert!(
            perm.len() <= 32,
            "Permutation of u32 cannot exceed 32 entries."
        );
        debug_assert!(
            perm.iter().all(|x| x < &32),
            "Permutation indices for u32 cannot exceed 31."
        );

        perm.iter()
            .rev()
            .enumerate()
            .map(|(bit, offset)| ((self >> bit) & 0b1) << offset)
            .sum()
    }

    fn permute(self, perm: &[usize]) -> Self {
        debug_assert!(
            perm.len() <= 32,
            "Permutation of u32 cannot exceed 32 entries."
        );
        debug_assert!(
            perm.iter().all(|x| x < &32),
            "Permutation indices for u32 cannot exceed 31."
        );

        perm.iter()
            .rev()
            .enumerate()
            .map(|(bit, offset)| ((self >> offset) & 0b1) << bit)
            .sum()
    }
}
