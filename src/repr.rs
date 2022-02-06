use pest::iterators::Pair;
use pest::RuleType;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

pub struct JsonConverter {
    pub arrays: Vec<String>,
}

impl JsonConverter {
    pub fn new(arrays: Vec<String>) -> Self {
        JsonConverter { arrays }
    }

    pub fn convert_to_value<R>(&self, pair: Pair<'_, R>) -> Value
    where
        R: RuleType,
    {
        let rule = format!("{:?}", pair.as_rule());
        let data = pair.as_str();
        let inner = pair.into_inner();
        if inner.peek().is_none() {
            Value::String(data.into())
        } else {
            if self.arrays.contains(&rule) {
                Value::Array(inner.map(|pair| self.convert_to_value(pair)).collect())
            } else {
                Value::Object(Map::from_iter(inner.map(|pair| {
                    (format!("{:?}", pair.as_rule()), self.convert_to_value(pair))
                })))
            }
        }
    }

    pub fn convert<T, R>(&self, pair: Pair<'_, R>) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
        R: RuleType,
    {
        serde_json::from_value(self.convert_to_value(pair))
    }
}
