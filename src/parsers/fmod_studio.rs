use crate::models::{Error, Function};
use crate::repr::JsonConverter;
use pest::{error, Parser};

#[derive(Parser)]
#[grammar = "./grammars/fmod_studio.pest"]
struct FmodStudioParser;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub functions: Vec<Function>,
}

pub fn parse(source: &str) -> Result<Header, Error> {
    let declarations = FmodStudioParser::parse(Rule::api, source)?
        .next()
        .ok_or(Error::FileMalformed)?;

    let arrays = vec![String::from("arguments")];
    let converter = JsonConverter::new(arrays);

    let mut header = Header::default();
    for declaration in declarations.into_inner() {
        match declaration.as_rule() {
            Rule::Function => header.functions.push(converter.convert(declaration)?),
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

#[cfg(test)]
mod tests {
    use crate::fmod_studio::{parse, Header};
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{Argument, Function, Pointer};

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
