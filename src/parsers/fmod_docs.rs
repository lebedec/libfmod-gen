use crate::models::{Error, ParameterModifier};

use regex::Regex;
use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn parse_fragment(content: &str) -> Result<HashMap<String, ParameterModifier>, Error> {
    let mut modifiers = HashMap::new();

    let function_pattern = Regex::new("<span class=\"nf\">(\\w+)</span>").unwrap();
    let optional_pattern =
        Regex::new("<dt>(\\w+) <span><a class=\"token\" href=\"(.+)\" title=\"Optional\">Opt")
            .unwrap();
    let output_pattern =
        Regex::new("<dt>(\\w+) <span><a class=\"token\" href=\"(.+)\" title=\"Output\">Out")
            .unwrap();

    let mut function = "";
    for line in content.lines() {
        if let Some(captures) = function_pattern.captures(line) {
            function = captures.get(1).unwrap().as_str();
        } else if let Some(captures) = optional_pattern.captures(line) {
            let argument = captures.get(1).unwrap().as_str();
            let key = format!("{}+{}", function, argument);
            modifiers.insert(key, ParameterModifier::Optional);
        } else if let Some(captures) = output_pattern.captures(line) {
            let argument = captures.get(1).unwrap().as_str();
            let key = format!("{}+{}", function, argument);
            modifiers.insert(key, ParameterModifier::Output);
        }
    }
    Ok(modifiers)
}

pub fn parse_parameter_modifiers(
    paths: &[PathBuf],
) -> Result<HashMap<String, ParameterModifier>, Error> {
    let mut output = HashMap::new();
    for path in paths {
        let html = fs::read_to_string(path)?;
        output.extend(parse_fragment(&html)?)
    }
    Ok(output)
}
