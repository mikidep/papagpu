#[repr(u32)]
pub enum Prec {
    Takes,
    Equals,
    Gives,
    Undef,
}

impl From<u32> for Prec {
    fn from(x: u32) -> Self {
        match x {
            1 => Prec::Gives,
            2 => Prec::Equals,
            3 => Prec::Takes,
            _ => Prec::Undef,
        }
    }
}

impl From<Prec> for u32 {
    fn from(x: Prec) -> Self {
        match x {
            Prec::Gives => 1,
            Prec::Equals => 2,
            Prec::Takes => 3,
            Prec::Undef => 0,
        }
    }
}

#[cfg(not(target_arch = "spirv"))]
impl std::fmt::Display for Prec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Prec::Gives => write!(f, "⋖"),
            Prec::Equals => write!(f, "≐"),
            Prec::Takes => write!(f, "⋗"),
            Prec::Undef => write!(f, "⊥"),
        }
    }
}
