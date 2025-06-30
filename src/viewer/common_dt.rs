use std::fmt;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum DataType {
    #[default]
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum DisplayType {
    #[default]
    Decimal,
    HexaDecimal,
}

impl DataType {
    pub const ALL: [DataType; 10] = [
        DataType::U8,
        DataType::I8,
        DataType::U16,
        DataType::I16,
        DataType::U32,
        DataType::I32,
        DataType::U64,
        DataType::I64,
        DataType::F32,
        DataType::F64,
    ];
}
#[derive(Debug, Default, PartialEq, Eq)]
pub enum Endianness {
    #[default]
    Little,
    Big,
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::U8 => write!(f, "U8"),
            DataType::I8 => write!(f, "I8"),
            DataType::U16 => write!(f, "U16"),
            DataType::I16 => write!(f, "I16"),
            DataType::U32 => write!(f, "U32"),
            DataType::I32 => write!(f, "I32"),
            DataType::U64 => write!(f, "U64"),
            DataType::I64 => write!(f, "I64"),
            DataType::F32 => write!(f, "F32"),
            DataType::F64 => write!(f, "F64"),
        }
    }
}
