use std::fmt::{Display, Formatter, Result};

use crate::error::McuError;

/// Access width for a volatile memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// 8-bit access.
    U8,
    /// 16-bit access.
    U16,
    /// 32-bit access.
    U32,
}

impl Width {
    #[must_use]
    /// Returns the bit-width (8, 16, or 32).
    pub const fn size(self) -> u8 {
        match self {
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
        }
    }
}

impl Display for Width {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}-bit", self.size())
    }
}

/// A width-tagged register value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    /// 8-bit value.
    U8(u8),
    /// 16-bit value.
    U16(u16),
    /// 32-bit value.
    U32(u32),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::U8(v) => write!(f, "{v:#04X}"),
            Self::U16(v) => write!(f, "{v:#06X}"),
            Self::U32(v) => write!(f, "{v:#010X}"),
        }
    }
}

impl Value {
    /// Returns the bit-width of this value (8, 16, or 32).
    #[must_use]
    pub const fn size(self) -> u8 {
        match self {
            Self::U8(_) => 8,
            Self::U16(_) => 16,
            Self::U32(_) => 32,
        }
    }

    /// Returns the corresponding [`Width`] discriminant.
    #[must_use]
    pub const fn width(self) -> Width {
        match self {
            Self::U8(_) => Width::U8,
            Self::U16(_) => Width::U16,
            Self::U32(_) => Width::U32,
        }
    }
}

impl From<u8> for Value {
    fn from(val: u8) -> Self {
        Self::U8(val)
    }
}

impl From<u16> for Value {
    fn from(val: u16) -> Self {
        Self::U16(val)
    }
}

impl From<u32> for Value {
    fn from(val: u32) -> Self {
        Self::U32(val)
    }
}

impl TryFrom<Value> for u8 {
    type Error = McuError;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::U8(v) => Ok(v),
            other => Err(McuError::WidthMismatch {
                requested: Width::U8.size(),
                actual: other.size(),
            }),
        }
    }
}

impl TryFrom<Value> for u16 {
    type Error = McuError;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::U16(v) => Ok(v),
            other => Err(McuError::WidthMismatch {
                requested: Width::U16.size(),
                actual: other.size(),
            }),
        }
    }
}

impl From<Value> for u32 {
    fn from(value: Value) -> Self {
        match value {
            Value::U8(v) => Self::from(v),
            Value::U16(v) => Self::from(v),
            Value::U32(v) => v,
        }
    }
}

/// Conversion between native integer types and width-tagged [`Value`]s.
pub trait VolatileAccess: Sized {
    /// Wraps this integer in a [`Value`].
    fn to_value(self) -> Value;

    /// Extracts the integer from a [`Value`], returning `None` on width mismatch.
    fn from_value(val: Value) -> Option<Self>;

    /// Returns the [`Width`] associated with this type.
    fn width() -> Width;
}

impl VolatileAccess for u8 {
    fn to_value(self) -> Value {
        Value::U8(self)
    }
    fn from_value(val: Value) -> Option<Self> {
        match val {
            Value::U8(v) => Some(v),
            _ => None,
        }
    }
    fn width() -> Width {
        Width::U8
    }
}

impl VolatileAccess for u16 {
    fn to_value(self) -> Value {
        Value::U16(self)
    }

    fn from_value(val: Value) -> Option<Self> {
        match val {
            Value::U16(v) => Some(v),
            _ => None,
        }
    }

    fn width() -> Width {
        Width::U16
    }
}

impl VolatileAccess for u32 {
    fn to_value(self) -> Value {
        Value::U32(self)
    }

    fn from_value(val: Value) -> Option<Self> {
        match val {
            Value::U32(v) => Some(v),
            _ => None,
        }
    }

    fn width() -> Width {
        Width::U32
    }
}

/// A 32-bit memory-mapped address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Addr(u32);

impl Addr {
    #[must_use]
    /// Creates a new address from a raw `u32`.
    pub const fn new(addr: u32) -> Self {
        Self(addr)
    }
}

impl From<u32> for Addr {
    fn from(addr: u32) -> Self {
        Self(addr)
    }
}

impl From<Addr> for u32 {
    fn from(addr: Addr) -> Self {
        addr.0
    }
}

impl Display for Addr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:#010X}", self.0)
    }
}
