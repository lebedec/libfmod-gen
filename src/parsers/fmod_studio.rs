use crate::api::Error;
use pest::iterators::Pair;
use pest::{error, Parser};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Parser)]
#[grammar = "./grammars/fmod_studio.pest"]
struct FmodStudioParser;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Pointer {
    NormalPointer(String),
    DoublePointer(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Type {
    FundamentalType(String),
    UserType(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Argument {
    pub as_const: Option<String>,
    pub argument_type: Type,
    pub pointer: Option<Pointer>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Function {
    pub return_type: Type,
    pub name: String,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub functions: Vec<Function>,
}

struct JsonConverter {
    pub arrays: Vec<String>,
}

impl JsonConverter {
    pub fn new(arrays: Vec<String>) -> Self {
        JsonConverter { arrays }
    }

    pub fn convert_to_value(&self, pair: Pair<'_, Rule>) -> Value {
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

    pub fn convert<T>(&self, pair: Pair<'_, Rule>) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(self.convert_to_value(pair))
    }
}

pub fn parse(source: &str) -> Result<Header, Error> {
    let declarations = FmodStudioParser::parse(Rule::api, source)?
        .next()
        .ok_or(Error::FileMalformed)?;

    let arrays = vec!["arguments"];
    let formatter = JsonConverter::new(arrays.into_iter().map(String::from).collect());

    let mut header = Header::default();
    for declaration in declarations.into_inner() {
        match declaration.as_rule() {
            Rule::Function => header.functions.push(formatter.convert(declaration)?),
            _ => continue,
        }
    }

    Ok(header)
}

impl From<error::Error<Rule>> for Error {
    fn from(error: error::Error<Rule>) -> Self {
        Self::Pest(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::fmod_studio::Pointer::NormalPointer;
    use crate::parsers::fmod_studio::Type::{FundamentalType, UserType};
    use crate::parsers::fmod_studio::{parse, Argument, Function, Header, Pointer};

    fn normal() -> Option<Pointer> {
        Some(Pointer::NormalPointer("*".into()))
    }

    fn double() -> Option<Pointer> {
        Some(Pointer::DoublePointer("**".into()))
    }

    #[test]
    fn test_should_ignore_comments() {
        let source = "/* FMOD Studio API - C header file. */";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_infdef_directive() {
        let source = "#ifndef FMOD_STUDIO_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_define_directive() {
        let source = "#define FMOD_STUDIO_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_include_directive() {
        let source = r#"
            #include "fmod_studio_common.h"
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_extern_linkage() {
        let source = r#"
            #ifdef __cplusplus
            extern "C"
            {
            #endif
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_parse_function_with_one_argument() {
        let source = r#"
            FMOD_RESULT F_API FMOD_Studio_System_Release(FMOD_STUDIO_SYSTEM *system);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                functions: vec![Function {
                    return_type: UserType("FMOD_RESULT".into()),
                    name: "FMOD_Studio_System_Release".into(),
                    arguments: vec![Argument {
                        as_const: None,
                        argument_type: UserType("FMOD_STUDIO_SYSTEM".into()),
                        pointer: normal(),
                        name: "system".into()
                    }]
                }]
            })
        )
    }

    #[test]
    fn test_should_parse_function_with_no_pointer_fundamental_type_argument() {
        let source = r#"
            FMOD_RESULT F_API FMOD_Studio_System_SetListenerWeight(int index);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                functions: vec![Function {
                    return_type: UserType("FMOD_RESULT".into()),
                    name: "FMOD_Studio_System_SetListenerWeight".into(),
                    arguments: vec![Argument {
                        as_const: None,
                        argument_type: FundamentalType("int".into()),
                        pointer: None,
                        name: "index".into()
                    }]
                }]
            })
        )
    }

    #[test]
    fn test_should_parse_function_with_double_pointer_argument() {
        let source = r#"
            FMOD_RESULT F_API FMOD_Studio_System_GetCoreSystem(FMOD_STUDIO_SYSTEM *system, FMOD_SYSTEM **coresystem);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                functions: vec![Function {
                    return_type: UserType("FMOD_RESULT".into()),
                    name: "FMOD_Studio_System_GetCoreSystem".into(),
                    arguments: vec![
                        Argument {
                            as_const: None,
                            argument_type: UserType("FMOD_STUDIO_SYSTEM".into()),
                            pointer: normal(),
                            name: "system".into()
                        },
                        Argument {
                            as_const: None,
                            argument_type: UserType("FMOD_SYSTEM".into()),
                            pointer: double(),
                            name: "coresystem".into()
                        }
                    ]
                }]
            })
        )
    }

    #[test]
    fn test_should_parse_function_with_const_pointer_fundamental_type_argument() {
        let source = r#"
            FMOD_RESULT F_API FMOD_Studio_ParseID(const char *idstring, FMOD_GUID *id);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                functions: vec![Function {
                    return_type: UserType("FMOD_RESULT".into()),
                    name: "FMOD_Studio_ParseID".into(),
                    arguments: vec![
                        Argument {
                            as_const: Some("const".into()),
                            argument_type: FundamentalType("char".into()),
                            pointer: normal(),
                            name: "idstring".into()
                        },
                        Argument {
                            as_const: None,
                            argument_type: UserType("FMOD_GUID".into()),
                            pointer: normal(),
                            name: "id".into()
                        }
                    ]
                }]
            })
        )
    }
}
