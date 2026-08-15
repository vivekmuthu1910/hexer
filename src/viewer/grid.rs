use super::common_dt::{DataType, Endianness};

/// One typed unit decoded from the Buffer.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
}

/// Grid of Values laid out with Width (Stride = Width for current parity).
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    width: usize,
    values: Vec<Value>,
}

impl Grid {
    /// Decode a Buffer into a Grid. Stride = Width.
    /// Endianness selects byte order for multi-byte Values (default Little).
    pub fn from_buffer(
        buffer: &[u8],
        data_type: DataType,
        endianness: Endianness,
        width: usize,
    ) -> Self {
        let values = match data_type {
            DataType::U8 => buffer.iter().map(|&b| Value::U8(b)).collect(),
            DataType::I8 => buffer.iter().map(|&b| Value::I8(b as i8)).collect(),
            DataType::U16 => decode_endian(
                buffer,
                endianness,
                u16::from_le_bytes,
                u16::from_be_bytes,
                Value::U16,
            ),
            DataType::I16 => decode_endian(
                buffer,
                endianness,
                i16::from_le_bytes,
                i16::from_be_bytes,
                Value::I16,
            ),
            DataType::U32 => decode_endian(
                buffer,
                endianness,
                u32::from_le_bytes,
                u32::from_be_bytes,
                Value::U32,
            ),
            DataType::I32 => decode_endian(
                buffer,
                endianness,
                i32::from_le_bytes,
                i32::from_be_bytes,
                Value::I32,
            ),
            DataType::U64 => decode_endian(
                buffer,
                endianness,
                u64::from_le_bytes,
                u64::from_be_bytes,
                Value::U64,
            ),
            DataType::I64 => decode_endian(
                buffer,
                endianness,
                i64::from_le_bytes,
                i64::from_be_bytes,
                Value::I64,
            ),
            DataType::F32 => decode_endian(
                buffer,
                endianness,
                f32::from_le_bytes,
                f32::from_be_bytes,
                Value::F32,
            ),
            DataType::F64 => decode_endian(
                buffer,
                endianness,
                f64::from_le_bytes,
                f64::from_be_bytes,
                Value::F64,
            ),
        };
        Self { width, values }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    /// Height derived as floor(value_count / Width), matching current total_rows.
    pub fn height(&self) -> usize {
        if self.width == 0 {
            0
        } else {
            self.values.len() / self.width
        }
    }

    pub fn value_at(&self, row: usize, col: usize) -> Option<&Value> {
        // Flat index row * Width + col — matches prior cast_slice indexing
        // (horizontal scroll may use col >= Width).
        self.values.get(row * self.width + col)
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

fn decode_endian<const N: usize, T, FLe, FBe, FWrap>(
    buffer: &[u8],
    endianness: Endianness,
    from_le: FLe,
    from_be: FBe,
    wrap: FWrap,
) -> Vec<Value>
where
    FLe: Fn([u8; N]) -> T,
    FBe: Fn([u8; N]) -> T,
    FWrap: Fn(T) -> Value,
{
    buffer
        .chunks_exact(N)
        .map(|chunk| {
            let bytes: [u8; N] = chunk.try_into().expect("chunks_exact size");
            let value = match endianness {
                Endianness::Little => from_le(bytes),
                Endianness::Big => from_be(bytes),
            };
            wrap(value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_buffer_yields_values_in_row_major_order() {
        let buffer = [1u8, 2, 3, 4, 5];
        let grid = Grid::from_buffer(&buffer, DataType::U8, Endianness::Little, 2);

        assert_eq!(grid.width(), 2);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.value_at(0, 0), Some(&Value::U8(1)));
        assert_eq!(grid.value_at(0, 1), Some(&Value::U8(2)));
        assert_eq!(grid.value_at(1, 0), Some(&Value::U8(3)));
        assert_eq!(grid.value_at(1, 1), Some(&Value::U8(4)));
        assert_eq!(grid.value_at(2, 0), Some(&Value::U8(5)));
        assert_eq!(grid.value_at(0, 2), Some(&Value::U8(3)));
        assert_eq!(grid.value_at(2, 1), None);
    }

    #[test]
    fn u16_little_endian_decodes_multi_byte_values() {
        let buffer = [0x34, 0x12, 0x78, 0x56];
        let grid = Grid::from_buffer(&buffer, DataType::U16, Endianness::Little, 2);

        assert_eq!(grid.value_at(0, 0), Some(&Value::U16(0x1234)));
        assert_eq!(grid.value_at(0, 1), Some(&Value::U16(0x5678)));
    }

    #[test]
    fn u16_big_endian_decodes_multi_byte_values() {
        let buffer = [0x34, 0x12, 0x78, 0x56];
        let grid = Grid::from_buffer(&buffer, DataType::U16, Endianness::Big, 2);

        assert_eq!(grid.value_at(0, 0), Some(&Value::U16(0x3412)));
        assert_eq!(grid.value_at(0, 1), Some(&Value::U16(0x7856)));
    }

    #[test]
    fn u8_is_unaffected_by_endianness() {
        let buffer = [0xABu8, 0xCD];
        let little = Grid::from_buffer(&buffer, DataType::U8, Endianness::Little, 2);
        let big = Grid::from_buffer(&buffer, DataType::U8, Endianness::Big, 2);

        assert_eq!(little.values(), big.values());
        assert_eq!(little.value_at(0, 0), Some(&Value::U8(0xAB)));
        assert_eq!(big.value_at(0, 1), Some(&Value::U8(0xCD)));
    }

    #[test]
    fn i8_is_unaffected_by_endianness() {
        let buffer = [0xFFu8, 0x01];
        let little = Grid::from_buffer(&buffer, DataType::I8, Endianness::Little, 2);
        let big = Grid::from_buffer(&buffer, DataType::I8, Endianness::Big, 2);

        assert_eq!(little.values(), big.values());
        assert_eq!(little.value_at(0, 0), Some(&Value::I8(-1)));
        assert_eq!(big.value_at(0, 1), Some(&Value::I8(1)));
    }

    #[test]
    fn f32_little_and_big_endian_differ() {
        let le_bytes = 1.0f32.to_le_bytes();
        let be_bytes = 1.0f32.to_be_bytes();

        let from_le = Grid::from_buffer(&le_bytes, DataType::F32, Endianness::Little, 1);
        let from_be = Grid::from_buffer(&be_bytes, DataType::F32, Endianness::Big, 1);
        let wrong_order = Grid::from_buffer(&le_bytes, DataType::F32, Endianness::Big, 1);

        assert_eq!(from_le.value_at(0, 0), Some(&Value::F32(1.0)));
        assert_eq!(from_be.value_at(0, 0), Some(&Value::F32(1.0)));
        assert_ne!(wrong_order.value_at(0, 0), Some(&Value::F32(1.0)));
    }

    #[test]
    fn stride_equals_width_for_height() {
        let buffer = [0u8; 12];
        let grid = Grid::from_buffer(&buffer, DataType::U8, Endianness::Little, 4);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.values().len(), 12);
    }

    #[test]
    fn default_endianness_is_little() {
        assert_eq!(Endianness::default(), Endianness::Little);
    }
}
