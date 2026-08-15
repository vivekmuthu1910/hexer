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

/// Grid of Values laid out with Width and Stride (Padding = Stride−Width).
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    width: usize,
    stride: usize,
    show_padding: bool,
    value_size: usize,
    values: Vec<Value>,
}

impl Grid {
    /// Decode a Buffer into a Grid with Stride = Width (no Padding).
    pub fn from_buffer(
        buffer: &[u8],
        data_type: DataType,
        endianness: Endianness,
        width: usize,
    ) -> Self {
        Self::from_buffer_layout(buffer, data_type, endianness, width, width, false)
    }

    /// Decode a Buffer into a Grid with explicit Width, Stride, and Padding visibility.
    /// Stride is counted in Values; when Stride < Width it is clamped up to Width.
    pub fn from_buffer_layout(
        buffer: &[u8],
        data_type: DataType,
        endianness: Endianness,
        width: usize,
        stride: usize,
        show_padding: bool,
    ) -> Self {
        let value_size = data_type.byte_size();
        let stride = stride.max(width);
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
        Self {
            width,
            stride,
            show_padding,
            value_size,
            values,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Columns in a Grid row for Cursor/paint: Width, or Stride when Padding is shown.
    pub fn grid_width(&self) -> usize {
        if self.show_padding {
            self.stride
        } else {
            self.width
        }
    }

    /// Height derived from value count and Stride (includes a partial final row).
    pub fn height(&self) -> usize {
        if self.stride == 0 || self.values.is_empty() {
            0
        } else {
            (self.values.len() + self.stride - 1) / self.stride
        }
    }

    fn index_at(&self, row: usize, col: usize) -> Option<usize> {
        if col >= self.grid_width() {
            return None;
        }
        row.checked_mul(self.stride)?.checked_add(col)
    }

    pub fn value_at(&self, row: usize, col: usize) -> Option<&Value> {
        let index = self.index_at(row, col)?;
        self.values.get(index)
    }

    /// Byte offset (Address) of the Value at (row, col) within the Buffer.
    pub fn address_at(&self, row: usize, col: usize) -> Option<usize> {
        let index = self.index_at(row, col)?;
        if index < self.values.len() {
            Some(index * self.value_size)
        } else {
            None
        }
    }

    /// Byte offset (Address) of the first Value on `row`.
    pub fn row_address(&self, row: usize) -> Option<usize> {
        self.address_at(row, 0)
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
        assert_eq!(grid.height(), 3); // partial final row for the 5th Value
        assert_eq!(grid.value_at(0, 0), Some(&Value::U8(1)));
        assert_eq!(grid.value_at(0, 1), Some(&Value::U8(2)));
        assert_eq!(grid.value_at(1, 0), Some(&Value::U8(3)));
        assert_eq!(grid.value_at(1, 1), Some(&Value::U8(4)));
        assert_eq!(grid.value_at(2, 0), Some(&Value::U8(5)));
        assert_eq!(grid.value_at(0, 2), None); // beyond Width
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

    #[test]
    fn address_at_is_byte_offset_for_u8() {
        let buffer = [0u8; 8];
        let grid = Grid::from_buffer(&buffer, DataType::U8, Endianness::Little, 4);

        assert_eq!(grid.address_at(0, 0), Some(0));
        assert_eq!(grid.address_at(0, 3), Some(3));
        assert_eq!(grid.address_at(1, 0), Some(4));
        assert_eq!(grid.address_at(1, 2), Some(6));
        assert_eq!(grid.address_at(2, 0), None);
    }

    #[test]
    fn address_at_scales_by_value_byte_size() {
        let buffer = [0u8; 16];
        let grid = Grid::from_buffer(&buffer, DataType::U16, Endianness::Little, 4);

        // 4 U16 Values per row → row 1 starts at byte 8
        assert_eq!(grid.address_at(0, 0), Some(0));
        assert_eq!(grid.address_at(0, 1), Some(2));
        assert_eq!(grid.address_at(1, 0), Some(8));
        assert_eq!(grid.address_at(1, 3), Some(14));
    }

    #[test]
    fn row_start_address_is_byte_offset_of_first_value() {
        let buffer = [0u8; 24];
        let grid = Grid::from_buffer(&buffer, DataType::U32, Endianness::Little, 2);

        assert_eq!(grid.row_address(0), Some(0));
        assert_eq!(grid.row_address(1), Some(8));
        assert_eq!(grid.row_address(2), Some(16));
        assert_eq!(grid.row_address(3), None);
    }

    #[test]
    fn stride_greater_than_width_omits_padding_by_default() {
        // Values: 0 1 2 3 4 5 6 7 8 9 10 11
        // Width=3, Stride=5 → row0: 0,1,2 (skip 3,4); row1: 5,6,7 (skip 8,9); row2: 10,11,(pad)
        let buffer: Vec<u8> = (0u8..12).collect();
        let grid = Grid::from_buffer_layout(
            &buffer,
            DataType::U8,
            Endianness::Little,
            3,
            5,
            false,
        );

        assert_eq!(grid.width(), 3);
        assert_eq!(grid.stride(), 5);
        assert_eq!(grid.height(), 3); // includes partial row at Values 10..11
        assert_eq!(grid.value_at(0, 0), Some(&Value::U8(0)));
        assert_eq!(grid.value_at(0, 2), Some(&Value::U8(2)));
        assert_eq!(grid.value_at(0, 3), None); // Padding omitted
        assert_eq!(grid.value_at(1, 0), Some(&Value::U8(5)));
        assert_eq!(grid.value_at(1, 2), Some(&Value::U8(7)));
        assert_eq!(grid.value_at(2, 0), Some(&Value::U8(10)));
        assert_eq!(grid.address_at(1, 0), Some(5));
        assert_eq!(grid.row_address(1), Some(5));
    }

    #[test]
    fn stride_greater_than_width_can_show_padding() {
        let buffer: Vec<u8> = (0u8..12).collect();
        let grid = Grid::from_buffer_layout(
            &buffer,
            DataType::U8,
            Endianness::Little,
            3,
            5,
            true,
        );

        assert_eq!(grid.height(), 3);
        assert_eq!(grid.value_at(0, 3), Some(&Value::U8(3)));
        assert_eq!(grid.value_at(0, 4), Some(&Value::U8(4)));
        assert_eq!(grid.value_at(1, 3), Some(&Value::U8(8)));
        assert_eq!(grid.value_at(2, 0), Some(&Value::U8(10)));
        assert_eq!(grid.address_at(0, 4), Some(4));
    }

    #[test]
    fn height_derives_from_value_count_and_stride() {
        let buffer = [0u8; 20];
        let grid = Grid::from_buffer_layout(
            &buffer,
            DataType::U8,
            Endianness::Little,
            4,
            6,
            false,
        );
        // 20 Values / Stride 6 → 3 complete rows + partial row at 18..19
        assert_eq!(grid.height(), 4);
        assert_eq!(grid.row_address(2), Some(12));
        assert_eq!(grid.row_address(3), Some(18));
        assert_eq!(grid.row_address(4), None);
    }

    #[test]
    fn stride_defaults_to_width_when_equal() {
        let buffer = [0u8; 8];
        let grid = Grid::from_buffer_layout(
            &buffer,
            DataType::U8,
            Endianness::Little,
            4,
            4,
            false,
        );
        assert_eq!(grid.stride(), 4);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.value_at(1, 0), Some(&Value::U8(0)));
        assert_eq!(grid.address_at(1, 0), Some(4));
    }
}
