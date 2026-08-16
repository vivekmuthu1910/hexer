use std::fmt;

use clap::ValueEnum;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, ValueEnum)]
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

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, ValueEnum)]
pub enum DisplayType {
    #[default]
    #[value(alias = "dec")]
    Decimal,
    #[value(name = "hex")]
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

    /// Byte size of one Value under this Data Type.
    pub fn byte_size(self) -> usize {
        match self {
            DataType::U8 | DataType::I8 => 1,
            DataType::U16 | DataType::I16 => 2,
            DataType::U32 | DataType::I32 | DataType::F32 => 4,
            DataType::U64 | DataType::I64 | DataType::F64 => 8,
        }
    }
}
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, ValueEnum)]
pub enum Endianness {
    #[default]
    #[value(alias = "le")]
    Little,
    #[value(alias = "be")]
    Big,
}

/// View Mode toggles Grid chrome and defaults — not a separate rendering engine.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, ValueEnum)]
pub enum ViewMode {
    #[default]
    Binary,
    Image,
}

impl ViewMode {
    pub fn toggle(self) -> Self {
        match self {
            ViewMode::Binary => ViewMode::Image,
            ViewMode::Image => ViewMode::Binary,
        }
    }

    /// Image requires an explicit Width; Binary may auto-fit.
    pub fn requires_explicit_width(self) -> bool {
        matches!(self, ViewMode::Image)
    }

    pub fn uses_address_gutter(self) -> bool {
        matches!(self, ViewMode::Binary)
    }
}

impl DisplayType {
    /// UI label for this Display Type (`Hex`, not HexaDecimal).
    pub fn label(self) -> &'static str {
        match self {
            DisplayType::Decimal => "Decimal",
            DisplayType::HexaDecimal => "Hex",
        }
    }
}

/// In-app help line stating Stride is counted in Values.
pub fn stride_help_text() -> &'static str {
    "Stride is counted in Values (not bytes)."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_mode_defaults_to_binary() {
        assert_eq!(ViewMode::default(), ViewMode::Binary);
    }

    #[test]
    fn view_mode_toggles_between_binary_and_image() {
        assert_eq!(ViewMode::Binary.toggle(), ViewMode::Image);
        assert_eq!(ViewMode::Image.toggle(), ViewMode::Binary);
    }

    #[test]
    fn image_requires_explicit_width_binary_does_not() {
        assert!(ViewMode::Image.requires_explicit_width());
        assert!(!ViewMode::Binary.requires_explicit_width());
    }

    #[test]
    fn binary_uses_address_gutter_image_does_not() {
        assert!(ViewMode::Binary.uses_address_gutter());
        assert!(!ViewMode::Image.uses_address_gutter());
    }

    #[test]
    fn display_type_label_is_hex_not_hexadecimal() {
        assert_eq!(DisplayType::HexaDecimal.label(), "Hex");
        assert_eq!(DisplayType::Decimal.label(), "Decimal");
        assert!(!DisplayType::HexaDecimal.label().contains("Hexa"));
    }

    #[test]
    fn stride_help_states_values_unit() {
        let help = stride_help_text();
        assert!(help.contains("Stride"));
        assert!(help.contains("Values"));
        assert!(!help.to_lowercase().contains("byte stride"));
    }
}
