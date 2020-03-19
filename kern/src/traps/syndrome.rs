use aarch64::ESR_EL1;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Fault {
    AddressSize,
    Translation,
    AccessFlag,
    Permission,
    Alignment,
    TlbConflict,
    Other(u8),
}

impl From<u32> for Fault {
    fn from(val: u32) -> Fault {
        use self::Fault::*;
        unsafe {
            match ESR_EL1.get_value(ESR_EL1::IFSC) as u8 {
                0 | 1 | 2 | 3 => AddressSize,
                4 | 5 | 6 | 7 => Translation,

                0b01001 =>  AccessFlag,
                0b01010 => AccessFlag,
                0b01011 => AccessFlag,

                0b01101 => Permission,
                0b01110 => Permission,
                0b01111 => Permission,

                0b0110000 => TlbConflict,
                val => Other(val)
            }
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Syndrome {
    Unknown,
    WfiWfe,
    SimdFp,
    IllegalExecutionState,
    Svc(u16),
    Hvc(u16),
    Smc(u16),
    MsrMrsSystem,
    InstructionAbort { kind: Fault, level: u8 },
    PCAlignmentFault,
    DataAbort { kind: Fault, level: u8 },
    SpAlignmentFault,
    TrappedFpu,
    SError,
    Breakpoint,
    Step,
    Watchpoint,
    Brk(u16),
    Other(u32),
}

//TODO: refactor this into Fault::from()
fn parse_abort(esr: u32) -> Syndrome {
    use self::Syndrome::InstructionAbort;
    use self::Fault::*;
    let fault_kind = self::Fault::from(esr);
    let level = unsafe {match ESR_EL1.get_value(ESR_EL1::IFSC) as u8 {
        0 => 0 ,
        1 => 1 ,
        2 => 2 ,
        3 => 3 ,

        4 => 0 ,
        5 => 1 ,
        6 => 2 ,
        7 => 3 ,

        0b01001 => 1 ,
        0b01010 => 2 ,
        0b01011 => 3 ,

        0b01101 => 1 ,
        0b01110 => 2 ,
        0b01111 => 3 ,

        0b010000 => 1 , //CHECK: i just assumed that this will be the same level?
        0b011000 => 1 ,

        0b010100 => 0 ,
        0b010101 => 1 ,
        0b010110 => 2 ,
        0b010111 => 3 ,

        0b011100 => 0 ,
        0b011101 => 1 ,
        0b011110 => 2 ,
        0b011111 => 3 ,

        0b0110000 => 1,  //CHECK: again, assumed same level?
        _ => 1
    }};
    Syndrome::InstructionAbort{ kind: fault_kind, level }
}

/// Converts a raw syndrome value (ESR) into a `Syndrome` (ref: D1.10.4).
impl From<u32> for Syndrome {
    fn from(esr: u32) -> Syndrome {
        use self::Syndrome::*;
        let ec = unsafe {ESR_EL1.get_value(ESR_EL1::EC) as u8};
        match ec {
            0 => Unknown,
            1 => WfiWfe,
            0b0111 => SimdFp,
            0b01110 => IllegalExecutionState,
            0b10001 | 0b010101 => { //CHECK: note that for this one and the following, we are only handling the 64-bit variants
                let imm = unsafe {ESR_EL1.get_value(ESR_EL1::ISS_HSVC_IMM) as u16};
                Svc(imm)
            },
            0b10010 | 0b010110 => {
                let imm = unsafe {ESR_EL1.get_value(ESR_EL1::ISS_HSVC_IMM) as u16};
                Hvc(imm)
            },
            0b10011 | 0b10111 => {
                let imm = unsafe {ESR_EL1.get_value(ESR_EL1::ISS_HSVC_IMM) as u16};
                Smc(imm)
            },
            0b011000 => MsrMrsSystem,
            0b0100000 | 0b0100001 => parse_abort(esr),
            0b010010 => PCAlignmentFault,
            0b0100100 | 0b0100101 => parse_abort(esr),
            0b0100110 => SpAlignmentFault,
            0b0101000 | 0b101100 => TrappedFpu,
            0b0101111 => SError,
            0b0110000 | 0b0110001 => Breakpoint,
            0b0110010 | 0b0110011 => Step,
            0b0110100 | 0b0110101 => Watchpoint,
            0b0111000 | 0b0111100 => {
                let comment = unsafe {ESR_EL1.get_value(ESR_EL1::ISS_BRK_CMMT) as u16};
                Brk(comment)
            },
            other => Other(other as u32)
        }

        //unimplemented!("From<u32> for Syndrome")
    }
}
