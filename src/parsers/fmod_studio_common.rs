use crate::models::{Callback, Constant, Enumeration, Error, Flags, OpaqueType, Structure};
use crate::repr::JsonConverter;
use pest::{error, Parser};

#[derive(Parser)]
#[grammar = "./grammars/fmod_studio_common.pest"]
struct FmodStudioCommonParser;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub opaque_types: Vec<OpaqueType>,
    pub constants: Vec<Constant>,
    pub flags: Vec<Flags>,
    pub enumerations: Vec<Enumeration>,
    pub structures: Vec<Structure>,
    pub callbacks: Vec<Callback>,
}

pub fn parse(source: &str) -> Result<Header, Error> {
    let declarations = FmodStudioCommonParser::parse(Rule::api, source)?
        .next()
        .ok_or(Error::FileMalformed)?;

    let arrays = vec![
        String::from("flags"),
        String::from("enumerators"),
        String::from("fields"),
        String::from("arguments"),
    ];
    let converter = JsonConverter::new(arrays);

    let mut header = Header::default();
    for declaration in declarations.into_inner() {
        match declaration.as_rule() {
            Rule::OpaqueType => header.opaque_types.push(converter.convert(declaration)?),
            Rule::Constant => header.constants.push(converter.convert(declaration)?),
            Rule::Flags => header.flags.push(converter.convert(declaration)?),
            Rule::Enumeration => header.enumerations.push(converter.convert(declaration)?),
            Rule::Structure => header.structures.push(converter.convert(declaration)?),
            Rule::Callback => header.callbacks.push(converter.convert(declaration)?),
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
    use crate::fmod_studio_common::{parse, Header};
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{
        Argument, Callback, Constant, Enumeration, Enumerator, Field, Flag, Flags, OpaqueType,
        Pointer, Structure, Union,
    };

    fn normal() -> Option<Pointer> {
        Some(Pointer::NormalPointer("*".into()))
    }

    #[test]
    fn test_should_ignore_ifndef_directive() {
        let source = "#ifndef FMOD_STUDIO_COMMON_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_define_directive() {
        let source = "#define FMOD_STUDIO_COMMON_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_include_directive() {
        let source = r#"
            #include "fmod.h"
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_parse_opaque_type() {
        let source = r#"
            typedef struct FMOD_STUDIO_EVENTDESCRIPTION FMOD_STUDIO_EVENTDESCRIPTION;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![OpaqueType {
                    name: "FMOD_STUDIO_EVENTDESCRIPTION".into()
                }],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_int_constant() {
        let source = r#"
            #define FMOD_STUDIO_LOAD_MEMORY_ALIGNMENT 32
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![Constant {
                    name: "FMOD_STUDIO_LOAD_MEMORY_ALIGNMENT".into(),
                    value: "32".into()
                }],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_hex_constant() {
        let source = r#"
            #define FMOD_STUDIO_INIT_NORMAL 0x00000000
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![Constant {
                    name: "FMOD_STUDIO_INIT_NORMAL".into(),
                    value: "0x00000000".into()
                }],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_flags() {
        let source = r#"
            typedef unsigned int FMOD_STUDIO_INITFLAGS;
            #define FMOD_STUDIO_INIT_NORMAL                             0x00000000
            #define FMOD_STUDIO_INIT_LIVEUPDATE                         0x00000001
            #define FMOD_STUDIO_INIT_ALLOW_MISSING_PLUGINS              0x00000002
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![Flags {
                    flags_type: FundamentalType("unsigned int".into()),
                    name: "FMOD_STUDIO_INITFLAGS".to_string(),
                    flags: vec![
                        Flag {
                            name: "FMOD_STUDIO_INIT_NORMAL".into(),
                            value: "0x00000000".into()
                        },
                        Flag {
                            name: "FMOD_STUDIO_INIT_LIVEUPDATE".into(),
                            value: "0x00000001".into()
                        },
                        Flag {
                            name: "FMOD_STUDIO_INIT_ALLOW_MISSING_PLUGINS".into(),
                            value: "0x00000002".into()
                        },
                    ]
                }],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_enumeration() {
        let source = r#"
            typedef enum FMOD_STUDIO_LOADING_STATE
            {
                FMOD_STUDIO_LOADING_STATE_UNLOADED,
                FMOD_STUDIO_LOADING_STATE_LOADED,
            
                FMOD_STUDIO_LOADING_STATE_FORCEINT = 65536
            } FMOD_STUDIO_LOADING_STATE;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![Enumeration {
                    name: "FMOD_STUDIO_LOADING_STATE".into(),
                    enumerators: vec![
                        Enumerator {
                            name: "FMOD_STUDIO_LOADING_STATE_UNLOADED".into(),
                            value: None
                        },
                        Enumerator {
                            name: "FMOD_STUDIO_LOADING_STATE_LOADED".into(),
                            value: None
                        },
                        Enumerator {
                            name: "FMOD_STUDIO_LOADING_STATE_FORCEINT".into(),
                            value: Some("65536".into())
                        }
                    ]
                }],
                structures: vec![],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_one_field() {
        let source = r#"
            typedef struct FMOD_STUDIO_BANK_INFO
            {
                int                      size;
            } FMOD_STUDIO_BANK_INFO;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_STUDIO_BANK_INFO".into(),
                    fields: vec![Field {
                        as_const: None,
                        as_array: None,
                        field_type: FundamentalType("int".into()),
                        pointer: None,
                        name: "size".into()
                    }],
                    union: None
                }],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_multiple_fields() {
        let source = r#"
            typedef struct FMOD_STUDIO_PARAMETER_DESCRIPTION
            {
                const char                 *name;
                FMOD_STUDIO_PARAMETER_ID    id;
            } FMOD_STUDIO_PARAMETER_DESCRIPTION;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_STUDIO_PARAMETER_DESCRIPTION".into(),
                    fields: vec![
                        Field {
                            as_const: Some("const".into()),
                            as_array: None,
                            field_type: FundamentalType("char".into()),
                            pointer: normal(),
                            name: "name".into()
                        },
                        Field {
                            as_const: None,
                            as_array: None,
                            field_type: UserType("FMOD_STUDIO_PARAMETER_ID".into()),
                            pointer: None,
                            name: "id".into()
                        }
                    ],
                    union: None
                }],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_union() {
        let source = r#"
            typedef struct FMOD_STUDIO_USER_PROPERTY
            {
                FMOD_STUDIO_USER_PROPERTY_TYPE  type;
            
                union
                {
                    FMOD_BOOL   boolvalue;
                    float       floatvalue;
                };
            } FMOD_STUDIO_USER_PROPERTY;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_STUDIO_USER_PROPERTY".into(),
                    fields: vec![Field {
                        as_const: None,
                        as_array: None,
                        field_type: UserType("FMOD_STUDIO_USER_PROPERTY_TYPE".into()),
                        pointer: None,
                        name: "type".into()
                    }],
                    union: Some(Union {
                        fields: vec![
                            Field {
                                as_const: None,
                                as_array: None,
                                field_type: UserType("FMOD_BOOL".into()),
                                pointer: None,
                                name: "boolvalue".into()
                            },
                            Field {
                                as_const: None,
                                as_array: None,
                                field_type: FundamentalType("float".into()),
                                pointer: None,
                                name: "floatvalue".into()
                            }
                        ]
                    })
                }],
                callbacks: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_callback_with_one_argument() {
        let source = r#"
            typedef FMOD_RESULT (F_CALLBACK *FMOD_STUDIO_SYSTEM_CALLBACK) (FMOD_STUDIO_SYSTEM *system);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![Callback {
                    return_type: UserType("FMOD_RESULT".into()),
                    pointer: None,
                    name: "FMOD_STUDIO_SYSTEM_CALLBACK".into(),
                    arguments: vec![Argument {
                        as_const: None,
                        argument_type: UserType("FMOD_STUDIO_SYSTEM".into()),
                        pointer: normal(),
                        name: "system".into()
                    }],
                    varargs: None
                }]
            })
        )
    }

    #[test]
    fn test_should_parse_callback_with_multiple_arguments() {
        let source = r#"
            typedef FMOD_RESULT (F_CALLBACK *FMOD_STUDIO_EVENT_CALLBACK) (FMOD_STUDIO_EVENT_CALLBACK_TYPE type, void *parameters);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![Callback {
                    return_type: UserType("FMOD_RESULT".into()),
                    pointer: None,
                    name: "FMOD_STUDIO_EVENT_CALLBACK".into(),
                    arguments: vec![
                        Argument {
                            as_const: None,
                            argument_type: UserType("FMOD_STUDIO_EVENT_CALLBACK_TYPE".into()),
                            pointer: None,
                            name: "type".into()
                        },
                        Argument {
                            as_const: None,
                            argument_type: FundamentalType("void".into()),
                            pointer: normal(),
                            name: "parameters".into()
                        }
                    ],
                    varargs: None
                }]
            })
        )
    }
}
