use errors::*;
use snips_queries_ontology::{AmountOfMoneyValue, DurationValue, Grain, InstantTimeValue,
                             NumberValue, OrdinalValue, Precision, SlotValue, TemperatureValue,
                             TimeIntervalValue};
use rustling_ontology::{DimensionKind, Grain as RustlingGrain};
use rustling_ontology::dimension::Precision as RustlingPrecision;
use rustling_ontology::output::{AmountOfMoneyOutput, DurationOutput, FloatOutput, IntegerOutput,
                                OrdinalOutput, Output, TemperatureOutput, TimeIntervalOutput,
                                TimeOutput};

pub trait FromRustling<T>: Sized {
    fn from_rustling(rustling_output: T) -> Self;
}

impl FromRustling<IntegerOutput> for NumberValue {
    fn from_rustling(rustling_output: IntegerOutput) -> NumberValue {
        NumberValue {
            value: rustling_output.0 as f64,
        }
    }
}

impl FromRustling<FloatOutput> for NumberValue {
    fn from_rustling(rustling_output: FloatOutput) -> NumberValue {
        NumberValue {
            value: rustling_output.0 as f64,
        }
    }
}

impl FromRustling<OrdinalOutput> for OrdinalValue {
    fn from_rustling(rustling_output: OrdinalOutput) -> OrdinalValue {
        OrdinalValue {
            value: rustling_output.0,
        }
    }
}

impl FromRustling<TimeOutput> for InstantTimeValue {
    fn from_rustling(rustling_output: TimeOutput) -> InstantTimeValue {
        InstantTimeValue {
            value: rustling_output.moment.to_string(),
            grain: Grain::from_rustling(rustling_output.grain),
            precision: Precision::from_rustling(rustling_output.precision),
        }
    }
}

impl FromRustling<TimeIntervalOutput> for TimeIntervalValue {
    fn from_rustling(rustling_output: TimeIntervalOutput) -> TimeIntervalValue {
        match rustling_output {
            TimeIntervalOutput::After(after) => TimeIntervalValue {
                from: Some(after.moment.to_string()),
                to: None,
            },
            TimeIntervalOutput::Before(before) => TimeIntervalValue {
                from: None,
                to: Some(before.moment.to_string()),
            },
            TimeIntervalOutput::Between(from, to, _) => TimeIntervalValue {
                from: Some(from.to_string()),
                to: Some(to.to_string()),
            },
        }
    }
}

impl FromRustling<AmountOfMoneyOutput> for AmountOfMoneyValue {
    fn from_rustling(rustling_output: AmountOfMoneyOutput) -> AmountOfMoneyValue {
        AmountOfMoneyValue {
            value: rustling_output.value,
            precision: Precision::from_rustling(rustling_output.precision),
            unit: rustling_output.unit.map(|s| s.to_string()),
        }
    }
}

impl FromRustling<TemperatureOutput> for TemperatureValue {
    fn from_rustling(rustling_output: TemperatureOutput) -> TemperatureValue {
        TemperatureValue {
            value: rustling_output.value,
            unit: rustling_output.unit.map(|s| s.to_string()),
        }
    }
}

impl FromRustling<DurationOutput> for DurationValue {
    fn from_rustling(rustling_output: DurationOutput) -> DurationValue {
        let mut years: i64 = 0;
        let mut quarters: i64 = 0;
        let mut months: i64 = 0;
        let mut weeks: i64 = 0;
        let mut days: i64 = 0;
        let mut hours: i64 = 0;
        let mut minutes: i64 = 0;
        let mut seconds: i64 = 0;
        for comp in rustling_output.period.comps().iter() {
            match comp.grain {
                RustlingGrain::Year => years = comp.quantity,
                RustlingGrain::Quarter => quarters = comp.quantity,
                RustlingGrain::Month => months = comp.quantity,
                RustlingGrain::Week => weeks = comp.quantity,
                RustlingGrain::Day => days = comp.quantity,
                RustlingGrain::Hour => hours = comp.quantity,
                RustlingGrain::Minute => minutes = comp.quantity,
                RustlingGrain::Second => seconds = comp.quantity,
            }
        }

        DurationValue {
            years,
            quarters,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
            precision: Precision::from_rustling(rustling_output.precision),
        }
    }
}

impl FromRustling<RustlingGrain> for Grain {
    fn from_rustling(rustling_output: RustlingGrain) -> Grain {
        match rustling_output {
            RustlingGrain::Year => Grain::Year,
            RustlingGrain::Quarter => Grain::Quarter,
            RustlingGrain::Month => Grain::Month,
            RustlingGrain::Week => Grain::Week,
            RustlingGrain::Day => Grain::Day,
            RustlingGrain::Hour => Grain::Hour,
            RustlingGrain::Minute => Grain::Minute,
            RustlingGrain::Second => Grain::Second,
        }
    }
}

impl FromRustling<RustlingPrecision> for Precision {
    fn from_rustling(rustling_output: RustlingPrecision) -> Precision {
        match rustling_output {
            RustlingPrecision::Approximate => Precision::Approximate,
            RustlingPrecision::Exact => Precision::Exact,
        }
    }
}

impl FromRustling<Output> for SlotValue {
    fn from_rustling(rustling_output: Output) -> SlotValue {
        match rustling_output {
            Output::AmountOfMoney(v) => {
                SlotValue::AmountOfMoney(AmountOfMoneyValue::from_rustling(v))
            }
            Output::Duration(v) => SlotValue::Duration(DurationValue::from_rustling(v)),
            Output::Float(v) => SlotValue::Number(NumberValue::from_rustling(v)),
            Output::Integer(v) => SlotValue::Number(NumberValue::from_rustling(v)),
            Output::Ordinal(v) => SlotValue::Ordinal(OrdinalValue::from_rustling(v)),
            Output::Temperature(v) => SlotValue::Temperature(TemperatureValue::from_rustling(v)),
            Output::Time(v) => SlotValue::InstantTime(InstantTimeValue::from_rustling(v)),
            Output::TimeInterval(v) => SlotValue::TimeInterval(TimeIntervalValue::from_rustling(v)),
        }
    }
}

#[derive(Copy, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinEntityKind {
    AmountOfMoney,
    Duration,
    Number,
    Ordinal,
    Temperature,
    Time,
}

impl BuiltinEntityKind {
    pub fn all() -> Vec<BuiltinEntityKind> {
        vec![
            BuiltinEntityKind::AmountOfMoney,
            BuiltinEntityKind::Duration,
            BuiltinEntityKind::Number,
            BuiltinEntityKind::Ordinal,
            BuiltinEntityKind::Temperature,
            BuiltinEntityKind::Time,
        ]
    }
}

impl BuiltinEntityKind {
    pub fn identifier(&self) -> &str {
        match *self {
            BuiltinEntityKind::AmountOfMoney => "snips/amountOfMoney",
            BuiltinEntityKind::Duration => "snips/duration",
            BuiltinEntityKind::Number => "snips/number",
            BuiltinEntityKind::Ordinal => "snips/ordinal",
            BuiltinEntityKind::Temperature => "snips/temperature",
            BuiltinEntityKind::Time => "snips/datetime",
        }
    }

    pub fn from_identifier(identifier: &str) -> Result<BuiltinEntityKind> {
        Self::all()
            .into_iter()
            .find(|kind| kind.identifier() == identifier)
            .ok_or(
                format!("Unknown EntityKind identifier: {}", identifier).into(),
            )
    }

    pub fn from_rustling_output(v: &Output) -> BuiltinEntityKind {
        match *v {
            Output::AmountOfMoney(_) => BuiltinEntityKind::AmountOfMoney,
            Output::Duration(_) => BuiltinEntityKind::Duration,
            Output::Float(_) => BuiltinEntityKind::Number,
            Output::Integer(_) => BuiltinEntityKind::Number,
            Output::Ordinal(_) => BuiltinEntityKind::Ordinal,
            Output::Temperature(_) => BuiltinEntityKind::Temperature,
            Output::Time(_) => BuiltinEntityKind::Time,
            Output::TimeInterval(_) => BuiltinEntityKind::Time,
        }
    }

    pub fn dimension_kind(&self) -> DimensionKind {
        match *self {
            BuiltinEntityKind::AmountOfMoney => DimensionKind::AmountOfMoney,
            BuiltinEntityKind::Duration => DimensionKind::Duration,
            BuiltinEntityKind::Number => DimensionKind::Number,
            BuiltinEntityKind::Ordinal => DimensionKind::Ordinal,
            BuiltinEntityKind::Temperature => DimensionKind::Temperature,
            BuiltinEntityKind::Time => DimensionKind::Time,
        }
    }
}

