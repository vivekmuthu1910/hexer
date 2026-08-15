use super::common_dt::DataType;
use bytemuck::{AnyBitPattern, cast_slice};

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
    /// Decode a Buffer into a Grid using today's layout assumptions:
    /// host Endianness (`cast_slice`), Stride = Width.
    pub fn from_buffer(buffer: &[u8], data_type: DataType, width: usize) -> Self {
        let values = match data_type {
            DataType::U8 => decode_host(buffer, Value::U8),
            DataType::I8 => decode_host(buffer, Value::I8),
            DataType::U16 => decode_host(buffer, Value::U16),
            DataType::I16 => decode_host(buffer, Value::I16),
            DataType::U32 => decode_host(buffer, Value::U32),
            DataType::I32 => decode_host(buffer, Value::I32),
            DataType::U64 => decode_host(buffer, Value::U64),
            DataType::I64 => decode_host(buffer, Value::I64),
            DataType::F32 => decode_host(buffer, Value::F32),
            DataType::F64 => decode_host(buffer, Value::F64),
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

fn decode_host<T, F>(buffer: &[u8], wrap: F) -> Vec<Value>
where
    T: AnyBitPattern + Copy,
    F: Fn(T) -> Value,
{
    cast_slice::<u8, T>(buffer)
        .iter()
        .copied()
        .map(wrap)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_buffer_yields_values_in_row_major_order() {
        let buffer = [1u8, 2, 3, 4, 5];
        let grid = Grid::from_buffer(&buffer, DataType::U8, 2);

        assert_eq!(grid.width(), 2);
        assert_eq!(grid.height(), 2); // floor(5/2) — parity with total_rows
        assert_eq!(grid.value_at(0, 0), Some(&Value::U8(1)));
        assert_eq!(grid.value_at(0, 1), Some(&Value::U8(2)));
        assert_eq!(grid.value_at(1, 0), Some(&Value::U8(3)));
        assert_eq!(grid.value_at(1, 1), Some(&Value::U8(4)));
        assert_eq!(grid.value_at(2, 0), Some(&Value::U8(5))); // still addressable
        assert_eq!(grid.value_at(0, 2), Some(&Value::U8(3))); // flat index parity
        assert_eq!(grid.value_at(2, 1), None);
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn u16_decodes_with_host_endianness() {
        // Bytes as laid out for native cast_slice on little-endian hosts.
        let buffer = [0x34, 0x12, 0x78, 0x56];
        let grid = Grid::from_buffer(&buffer, DataType::U16, 2);

        assert_eq!(grid.width(), 2);
        assert_eq!(grid.height(), 1);
        assert_eq!(grid.value_at(0, 0), Some(&Value::U16(0x1234)));
        assert_eq!(grid.value_at(0, 1), Some(&Value::U16(0x5678)));
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn f32_decodes_with_host_endianness() {
        let buffer = 1.0f32.to_le_bytes();
        let grid = Grid::from_buffer(&buffer, DataType::F32, 1);

        assert_eq!(grid.height(), 1);
        assert_eq!(grid.value_at(0, 0), Some(&Value::F32(1.0)));
    }

    #[test]
    fn stride_equals_width_for_height() {
        let buffer = [0u8; 12];
        let grid = Grid::from_buffer(&buffer, DataType::U8, 4);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.values().len(), 12);
    }
}
