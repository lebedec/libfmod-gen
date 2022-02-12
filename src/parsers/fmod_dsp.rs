use crate::models::{Callback, Constant, Enumeration, Error, Flags, OpaqueType, Structure};
use crate::repr::JsonConverter;
use pest::{error, Parser};

#[derive(Parser)]
#[grammar = "./grammars/fmod_dsp.pest"]
struct FmodDspParser;

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
    let declarations = FmodDspParser::parse(Rule::api, source)?
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
    use crate::fmod_dsp::{parse, Header};
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{Argument, Callback, Enumeration, Enumerator, Field, Structure};

    #[test]
    fn test_should_ignore_define_directive() {
        let source = "#define _FMOD_DSP_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_parse_enumeration_without_name() {
        let source = r#"
            typedef enum
            {
                FMOD_DSP_PROCESS_PERFORM,
                FMOD_DSP_PROCESS_QUERY
            } FMOD_DSP_PROCESS_OPERATION;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![Enumeration {
                    name: "FMOD_DSP_PROCESS_OPERATION".into(),
                    enumerators: vec![
                        Enumerator {
                            name: "FMOD_DSP_PROCESS_PERFORM".into(),
                            value: None
                        },
                        Enumerator {
                            name: "FMOD_DSP_PROCESS_QUERY".into(),
                            value: None
                        }
                    ]
                }],
                structures: vec![],
                callbacks: vec![],
            })
        )
    }

    #[test]
    fn test_should_parse_enumeration_with_trailing_coma() {
        let source = r#"
            typedef enum
            {
                FMOD_DSP_PARAMETER_DATA_TYPE_USER = 0,
                FMOD_DSP_PARAMETER_DATA_TYPE_ATTENUATION_RANGE = -6,
            } FMOD_DSP_PARAMETER_DATA_TYPE;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![Enumeration {
                    name: "FMOD_DSP_PARAMETER_DATA_TYPE".into(),
                    enumerators: vec![
                        Enumerator {
                            name: "FMOD_DSP_PARAMETER_DATA_TYPE_USER".into(),
                            value: Some("0".into())
                        },
                        Enumerator {
                            name: "FMOD_DSP_PARAMETER_DATA_TYPE_ATTENUATION_RANGE".into(),
                            value: Some("-6".into())
                        }
                    ]
                }],
                structures: vec![],
                callbacks: vec![],
            })
        )
    }

    #[test]
    fn test_should_parse_callback_with_varargs() {
        let source = r#"
            typedef void (F_CALL *FMOD_DSP_LOG_FUNC) (int line, ...);
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
                    return_type: FundamentalType("void".into()),
                    pointer: None,
                    name: "FMOD_DSP_LOG_FUNC".into(),
                    arguments: vec![Argument {
                        as_const: None,
                        argument_type: FundamentalType("int".into()),
                        pointer: None,
                        name: "line".into()
                    }],
                    varargs: Option::Some(", ...".into())
                }]
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_const_char_const_field() {
        let source = r#"
            typedef struct FMOD_DSP_PARAMETER_DESC_BOOL
            {
                FMOD_BOOL           defaultval;
                const char* const*  valuenames;
            } FMOD_DSP_PARAMETER_DESC_BOOL;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_DSP_PARAMETER_DESC_BOOL".into(),
                    fields: vec![
                        Field {
                            as_const: None,
                            as_array: None,
                            field_type: UserType("FMOD_BOOL".into()),
                            pointer: None,
                            name: "defaultval".into()
                        },
                        Field {
                            as_const: Some("const".into()),
                            as_array: None,
                            field_type: FundamentalType("char* const*".into()),
                            pointer: None,
                            name: "valuenames".into()
                        },
                    ],
                    union: None
                }],
                callbacks: vec![],
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_array_field_and_defined_dimension() {
        let source = r#"
            typedef struct FMOD_DSP_PARAMETER_3DATTRIBUTES_MULTI
            {
                int                numlisteners;
                float              weight[FMOD_MAX_LISTENERS];
            } FMOD_DSP_PARAMETER_3DATTRIBUTES_MULTI;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_DSP_PARAMETER_3DATTRIBUTES_MULTI".into(),
                    fields: vec![
                        Field {
                            as_const: None,
                            as_array: None,
                            field_type: FundamentalType("int".into()),
                            pointer: None,
                            name: "numlisteners".into()
                        },
                        Field {
                            as_const: None,
                            as_array: Some("[FMOD_MAX_LISTENERS]".into()),
                            field_type: FundamentalType("float".into()),
                            pointer: None,
                            name: "weight".into()
                        }
                    ],
                    union: None
                }],
                callbacks: vec![],
            })
        )
    }

    #[test]
    fn test_should_ignore_macros() {
        let source = r#"
            #define FMOD_DSP_INIT_PARAMDESC_FLOAT(_paramstruct, _name, _label, _description, _min, _max, _defaultval) \
                memset(&(_paramstruct), 0, sizeof(_paramstruct)); \
                (_paramstruct).type         = FMOD_DSP_PARAMETER_TYPE_FLOAT; \
                strncpy((_paramstruct).name,  _name,  15); \
                strncpy((_paramstruct).label, _label, 15); \
                (_paramstruct).description  = _description; \
                (_paramstruct).floatdesc.min          = _min; \
                (_paramstruct).floatdesc.max          = _max; \
                (_paramstruct).floatdesc.defaultval   = _defaultval; \
                (_paramstruct).floatdesc.mapping.type = FMOD_DSP_PARAMETER_FLOAT_MAPPING_TYPE_AUTO;
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }
}
