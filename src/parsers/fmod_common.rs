use crate::models::{
    Callback, Constant, Enumeration, Error, Flags, OpaqueType, Structure, TypeAlias,
};
use crate::repr::JsonConverter;
use pest::{error, Parser};

#[derive(Parser)]
#[grammar = "./grammars/fmod_common.pest"]
struct FmodCommonParser;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub opaque_types: Vec<OpaqueType>,
    pub constants: Vec<Constant>,
    pub flags: Vec<Flags>,
    pub enumerations: Vec<Enumeration>,
    pub structures: Vec<Structure>,
    pub callbacks: Vec<Callback>,
    pub type_aliases: Vec<TypeAlias>,
}

pub fn parse(source: &str) -> Result<Header, Error> {
    let declarations = FmodCommonParser::parse(Rule::api, source)?
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
            Rule::TypeAlias => header.type_aliases.push(converter.convert(declaration)?),
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
    use crate::fmod_common::{parse, Header};
    use crate::models::Type::FundamentalType;
    use crate::models::{
        Argument, Callback, Constant, Enumeration, Enumerator, Field, Flag, Flags, OpaqueType,
        Structure, TypeAlias,
    };

    #[test]
    fn test_should_ignore_ifndef_directive() {
        let source = "#ifndef _FMOD_COMMON_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_define_directive() {
        let source = "#define _FMOD_COMMON_H";
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_fcall_import_helper() {
        let source = r#"
            #if defined(_WIN32) || defined(__CYGWIN__)
                #define F_CALL __stdcall
            #else
                #define F_CALL
            #endif
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_fexport_import_helper() {
        let source = r#"
            #if defined(_WIN32) || defined(__CYGWIN__) || defined(__ORBIS__) || defined(F_USE_DECLSPEC)
                #define F_EXPORT __declspec(dllexport)
            #elif defined(__APPLE__) || defined(__ANDROID__) || defined(__linux__) || defined(F_USE_ATTRIBUTE)
                #define F_EXPORT __attribute__((visibility("default")))
            #else
                #define F_EXPORT
            #endif
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_fapi_import_helper() {
        let source = r#"
            #ifdef DLL_EXPORTS
                #define F_API F_EXPORT F_CALL
            #else
                #define F_API F_CALL
            #endif
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_ignore_fcallback_import_helper() {
        let source = r#"
            #define F_CALLBACK F_CALL
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_parse_fundamental_type_alias() {
        let source = r#"
            typedef unsigned long long FMOD_PORT_INDEX;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![TypeAlias {
                    base_type: FundamentalType("unsigned long long".into()),
                    name: "FMOD_PORT_INDEX".into()
                }]
            })
        )
    }

    #[test]
    fn test_should_parse_opaque_type() {
        let source = r#"
            typedef struct FMOD_SYSTEM FMOD_SYSTEM;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![OpaqueType {
                    name: "FMOD_SYSTEM".into()
                }],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_hex_constant() {
        let source = r#"
            #define FMOD_VERSION    0x00020203
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![Constant {
                    name: "FMOD_VERSION".into(),
                    value: "0x00020203".into()
                }],
                flags: vec![],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_flags() {
        let source = r#"
            typedef unsigned int FMOD_DEBUG_FLAGS;
            #define FMOD_DEBUG_LEVEL_NONE                       0x00000000
            #define FMOD_DEBUG_LEVEL_ERROR                      0x00000001
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![Flags {
                    flags_type: FundamentalType("unsigned int".into()),
                    name: "FMOD_DEBUG_FLAGS".to_string(),
                    flags: vec![
                        Flag {
                            name: "FMOD_DEBUG_LEVEL_NONE".into(),
                            value: "0x00000000".into()
                        },
                        Flag {
                            name: "FMOD_DEBUG_LEVEL_ERROR".into(),
                            value: "0x00000001".into()
                        },
                    ]
                }],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_flags_with_binary_or() {
        let source = r#"
            typedef unsigned int FMOD_CHANNELMASK;
            #define FMOD_CHANNELMASK_FRONT_LEFT                 0x00000001
            #define FMOD_CHANNELMASK_FRONT_RIGHT                0x00000002
            #define FMOD_CHANNELMASK_MONO                       (FMOD_CHANNELMASK_FRONT_LEFT)
            #define FMOD_CHANNELMASK_STEREO                     (FMOD_CHANNELMASK_FRONT_LEFT | FMOD_CHANNELMASK_FRONT_RIGHT)
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![Flags {
                    flags_type: FundamentalType("unsigned int".into()),
                    name: "FMOD_CHANNELMASK".to_string(),
                    flags: vec![
                        Flag {
                            name: "FMOD_CHANNELMASK_FRONT_LEFT".into(),
                            value: "0x00000001".into()
                        },
                        Flag {
                            name: "FMOD_CHANNELMASK_FRONT_RIGHT".into(),
                            value: "0x00000002".into()
                        },
                        Flag {
                            name: "FMOD_CHANNELMASK_MONO".into(),
                            value: "(FMOD_CHANNELMASK_FRONT_LEFT)".into()
                        },
                        Flag {
                            name: "FMOD_CHANNELMASK_STEREO".into(),
                            value: "(FMOD_CHANNELMASK_FRONT_LEFT | FMOD_CHANNELMASK_FRONT_RIGHT)"
                                .into()
                        },
                    ]
                }],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_flags_with_calculation() {
        let source = r#"
            typedef unsigned int FMOD_THREAD_STACK_SIZE;
            #define FMOD_THREAD_STACK_SIZE_MIXER                (80  * 1024)
            #define FMOD_THREAD_STACK_SIZE_FEEDER               (16  * 1024)
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![Flags {
                    flags_type: FundamentalType("unsigned int".into()),
                    name: "FMOD_THREAD_STACK_SIZE".to_string(),
                    flags: vec![
                        Flag {
                            name: "FMOD_THREAD_STACK_SIZE_MIXER".into(),
                            value: "(80  * 1024)".into()
                        },
                        Flag {
                            name: "FMOD_THREAD_STACK_SIZE_FEEDER".into(),
                            value: "(16  * 1024)".into()
                        },
                    ]
                }],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_multiple_flags_with_atomic_value() {
        let source = r#"
            typedef long long FMOD_THREAD_AFFINITY;
            #define FMOD_THREAD_AFFINITY_GROUP_DEFAULT          0x4000000000000000
            #define FMOD_THREAD_AFFINITY_MIXER                  FMOD_THREAD_AFFINITY_GROUP_A
            
            typedef unsigned int FMOD_CHANNELMASK;
            #define FMOD_CHANNELMASK_FRONT_LEFT                 0x00000001
            #define FMOD_CHANNELMASK_FRONT_RIGHT                0x00000002
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![
                    Flags {
                        flags_type: FundamentalType("long long".into()),
                        name: "FMOD_THREAD_AFFINITY".to_string(),
                        flags: vec![
                            Flag {
                                name: "FMOD_THREAD_AFFINITY_GROUP_DEFAULT".into(),
                                value: "0x4000000000000000".into()
                            },
                            Flag {
                                name: "FMOD_THREAD_AFFINITY_MIXER".into(),
                                value: "FMOD_THREAD_AFFINITY_GROUP_A".into()
                            }
                        ]
                    },
                    Flags {
                        flags_type: FundamentalType("unsigned int".into()),
                        name: "FMOD_CHANNELMASK".to_string(),
                        flags: vec![
                            Flag {
                                name: "FMOD_CHANNELMASK_FRONT_LEFT".into(),
                                value: "0x00000001".into()
                            },
                            Flag {
                                name: "FMOD_CHANNELMASK_FRONT_RIGHT".into(),
                                value: "0x00000002".into()
                            }
                        ]
                    }
                ],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_flags_with_aliases() {
        let source = r#"
            typedef long long FMOD_THREAD_AFFINITY;
            /* Platform agnostic thread groupings */
            #define FMOD_THREAD_AFFINITY_GROUP_DEFAULT          0x4000000000000000
            #define FMOD_THREAD_AFFINITY_GROUP_A                0x4000000000000001
            /* Thread defaults */
            #define FMOD_THREAD_AFFINITY_MIXER                  FMOD_THREAD_AFFINITY_GROUP_A
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![Flags {
                    flags_type: FundamentalType("long long".into()),
                    name: "FMOD_THREAD_AFFINITY".to_string(),
                    flags: vec![
                        Flag {
                            name: "FMOD_THREAD_AFFINITY_GROUP_DEFAULT".into(),
                            value: "0x4000000000000000".into()
                        },
                        Flag {
                            name: "FMOD_THREAD_AFFINITY_GROUP_A".into(),
                            value: "0x4000000000000001".into()
                        },
                        Flag {
                            name: "FMOD_THREAD_AFFINITY_MIXER".into(),
                            value: "FMOD_THREAD_AFFINITY_GROUP_A".into()
                        }
                    ]
                }],
                enumerations: vec![],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_ignore_preset() {
        let source = r#"
            #define FMOD_PRESET_OFF {  1000,    7,  11, 5000, 100, 100, 100, 250, 0,    20,  96, -80.0f }
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }

    #[test]
    fn test_should_parse_enumeration_with_negative_value() {
        let source = r#"
            typedef enum FMOD_SPEAKER
            {
                FMOD_SPEAKER_NONE = -1,
                FMOD_SPEAKER_FRONT_LEFT = 0,
                FMOD_SPEAKER_FRONT_RIGHT,
                FMOD_SPEAKER_FORCEINT = 65536
            } FMOD_SPEAKER;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![Enumeration {
                    name: "FMOD_SPEAKER".into(),
                    enumerators: vec![
                        Enumerator {
                            name: "FMOD_SPEAKER_NONE".into(),
                            value: Some("-1".into())
                        },
                        Enumerator {
                            name: "FMOD_SPEAKER_FRONT_LEFT".into(),
                            value: Some("0".into())
                        },
                        Enumerator {
                            name: "FMOD_SPEAKER_FRONT_RIGHT".into(),
                            value: None
                        },
                        Enumerator {
                            name: "FMOD_SPEAKER_FORCEINT".into(),
                            value: Some("65536".into())
                        }
                    ]
                }],
                structures: vec![],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_callback_with_void_pointer_return() {
        let source = r#"
            typedef void* (F_CALL *FMOD_MEMORY_ALLOC_CALLBACK) (unsigned int size);
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
                    return_type: FundamentalType("void*".into()),
                    name: "FMOD_MEMORY_ALLOC_CALLBACK".into(),
                    arguments: vec![Argument {
                        as_const: None,
                        argument_type: FundamentalType("unsigned int".into()),
                        pointer: None,
                        name: "size".into()
                    }],
                    varargs: None
                }],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_multiple_fields() {
        let source = r#"
            typedef struct FMOD_VECTOR
            {
                float x;
                float y;
                float z;
            } FMOD_VECTOR;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_VECTOR".into(),
                    fields: vec![
                        Field {
                            as_const: None,
                            as_array: None,
                            field_type: FundamentalType("float".into()),
                            pointer: None,
                            name: "x".into()
                        },
                        Field {
                            as_const: None,
                            as_array: None,
                            field_type: FundamentalType("float".into()),
                            pointer: None,
                            name: "y".into()
                        },
                        Field {
                            as_const: None,
                            as_array: None,
                            field_type: FundamentalType("float".into()),
                            pointer: None,
                            name: "z".into()
                        },
                    ],
                    union: None
                }],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }

    #[test]
    fn test_should_parse_structure_with_array_field() {
        let source = r#"
            typedef struct FMOD_GUID
            {
                unsigned char  Data4[8];
            } FMOD_GUID;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                enumerations: vec![],
                structures: vec![Structure {
                    name: "FMOD_GUID".into(),
                    fields: vec![Field {
                        as_const: None,
                        as_array: Some("[8]".into()),
                        field_type: FundamentalType("unsigned char".into()),
                        pointer: None,
                        name: "Data4".into()
                    },],
                    union: None
                }],
                callbacks: vec![],
                type_aliases: vec![]
            })
        )
    }
}
