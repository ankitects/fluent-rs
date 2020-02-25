use std::borrow::Cow;
use std::convert::TryInto;
use std::default::Default;
use std::str::FromStr;

use intl_pluralrules::operands::PluralOperands;

use crate::bundle::FluentArgs;
use crate::types::FluentValue;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum FluentNumberStyle {
    Decimal,
    Currency,
    Percent,
}

impl std::default::Default for FluentNumberStyle {
    fn default() -> Self {
        Self::Decimal
    }
}

impl From<&str> for FluentNumberStyle {
    fn from(input: &str) -> Self {
        match input {
            "decimal" => Self::Decimal,
            "currency" => Self::Currency,
            "percent" => Self::Percent,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum FluentNumberCurrencyDisplayStyle {
    Symbol,
    Code,
    Name,
}

impl std::default::Default for FluentNumberCurrencyDisplayStyle {
    fn default() -> Self {
        Self::Symbol
    }
}

impl From<&str> for FluentNumberCurrencyDisplayStyle {
    fn from(input: &str) -> Self {
        match input {
            "symbol" => Self::Symbol,
            "code" => Self::Code,
            "name" => Self::Name,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FluentNumberOptions {
    pub style: FluentNumberStyle,
    pub currency: Option<String>,
    pub currency_display: FluentNumberCurrencyDisplayStyle,
    pub use_grouping: bool,
    pub minimum_integer_digits: Option<usize>,
    pub minimum_fraction_digits: Option<usize>,
    pub maximum_fraction_digits: Option<usize>,
    pub minimum_significant_digits: Option<usize>,
    pub maximum_significant_digits: Option<usize>,
}

impl Default for FluentNumberOptions {
    fn default() -> Self {
        Self {
            style: Default::default(),
            currency: None,
            currency_display: Default::default(),
            use_grouping: true,
            minimum_integer_digits: None,
            minimum_fraction_digits: None,
            maximum_fraction_digits: None,
            minimum_significant_digits: None,
            maximum_significant_digits: None,
        }
    }
}

impl FluentNumberOptions {
    pub fn merge(&mut self, opts: &FluentArgs) {
        for (key, value) in opts {
            match (*key, value) {
                ("style", FluentValue::String(n)) => {
                    self.style = n.as_ref().into();
                }
                ("currency", FluentValue::String(n)) => {
                    self.currency = Some(n.to_string());
                }
                ("currencyDisplay", FluentValue::String(n)) => {
                    self.currency_display = n.as_ref().into();
                }
                ("minimumIntegerDigits", FluentValue::Number(n)) => {
                    self.minimum_integer_digits = Some(n.into());
                }
                ("minimumFractionDigits", FluentValue::Number(n)) => {
                    self.minimum_fraction_digits = Some(n.into());
                }
                ("maximumFractionDigits", FluentValue::Number(n)) => {
                    self.maximum_fraction_digits = Some(n.into());
                }
                ("minimumSignificantDigits", FluentValue::Number(n)) => {
                    self.minimum_significant_digits = Some(n.into());
                }
                ("maximumSignificantDigits", FluentValue::Number(n)) => {
                    self.maximum_significant_digits = Some(n.into());
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FluentNumber {
    pub value: f64,
    pub options: FluentNumberOptions,
}

impl FluentNumber {
    pub fn new(value: f64, options: FluentNumberOptions) -> Self {
        Self { value, options }
    }

    pub fn as_string(&self) -> Cow<'static, str> {
        let mut max_frac_digits = self.options.maximum_fraction_digits.unwrap_or(15);
        // since the plural code currently parses the resulting fractional digits into a usize, we can't
        // have a precision above 9 digits on 32 bit platforms
        if std::mem::size_of::<usize>() < 8 {
            max_frac_digits = max_frac_digits.min(9);
        }
        // create the string with maximum precision
        let with_max_precision = format!(
            "{number:.precision$}",
            number = self.value,
            precision = max_frac_digits
        );
        // and then remove any excess trailing zeros
        let mut val: Cow<str> = with_max_precision.trim_end_matches('0').into();
        // adding back any required to meet minimum_fraction_digits
        if let Some(minfd) = self.options.minimum_fraction_digits {
            let pos = val.find('.').expect("expected . in formatted string");
            let frac_num = val.len() - pos - 1;
            let zeros_needed = minfd - frac_num;
            if zeros_needed > 0 {
                val = format!("{}{}", val, "0".repeat(zeros_needed)).into();
            }
        }
        // lop off any trailing '.', then return an owned value
        val.trim_end_matches('.').to_string().into()
    }
}

impl FromStr for FluentNumber {
    type Err = std::num::ParseFloatError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        f64::from_str(input).map(|n| {
            let mfd = input.find('.').map(|pos| input.len() - pos - 1);
            let opts = FluentNumberOptions {
                minimum_fraction_digits: mfd,
                ..Default::default()
            };
            FluentNumber::new(n, opts)
        })
    }
}

impl<'l> From<FluentNumber> for FluentValue<'l> {
    fn from(input: FluentNumber) -> Self {
        FluentValue::Number(input)
    }
}

macro_rules! from_num {
    ($num:ty) => {
        impl From<$num> for FluentNumber {
            fn from(n: $num) -> Self {
                FluentNumber {
                    value: n as f64,
                    options: FluentNumberOptions::default(),
                }
            }
        }
        impl From<&$num> for FluentNumber {
            fn from(n: &$num) -> Self {
                FluentNumber {
                    value: *n as f64,
                    options: FluentNumberOptions::default(),
                }
            }
        }
        impl From<FluentNumber> for $num {
            fn from(input: FluentNumber) -> Self {
                input.value as $num
            }
        }
        impl From<&FluentNumber> for $num {
            fn from(input: &FluentNumber) -> Self {
                input.value as $num
            }
        }
        impl From<$num> for FluentValue<'_> {
            fn from(n: $num) -> Self {
                FluentValue::Number(n.into())
            }
        }
        impl From<&$num> for FluentValue<'_> {
            fn from(n: &$num) -> Self {
                FluentValue::Number(n.into())
            }
        }
    };
    ($($num:ty)+) => {
        $(from_num!($num);)+
    };
}

impl From<&FluentNumber> for PluralOperands {
    fn from(input: &FluentNumber) -> Self {
        let mut operands: PluralOperands = input
            .as_string()
            .as_ref()
            .try_into()
            .expect("Failed to generate operands out of FluentNumber");
        if let Some(mfd) = input.options.minimum_fraction_digits {
            if mfd > operands.v {
                operands.f *= 10_usize.pow(mfd as u32 - operands.v as u32);
                operands.v = mfd;
            }
        }
        // XXX: Add support for other options.
        operands
    }
}

from_num!(i8 i16 i32 i64 i128 isize);
from_num!(u8 u16 u32 u64 u128 usize);
from_num!(f32 f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_from_copy_ref() {
        let x = 1i16;
        let y = &x;
        let z: FluentValue = y.into();
        assert_eq!(z, FluentValue::try_number(1));
    }
}
