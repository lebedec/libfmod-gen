use crate::models::{Callback, Constant, Error, Flags, OpaqueType, Structure};
use crate::repr::JsonConverter;
use pest::{error, Parser};

#[derive(Parser)]
#[grammar = "./grammars/fmod_output.pest"]
struct FmodOutputParser;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub opaque_types: Vec<OpaqueType>,
    pub constants: Vec<Constant>,
    pub flags: Vec<Flags>,
    pub structures: Vec<Structure>,
    pub callbacks: Vec<Callback>,
}

pub fn parse(source: &str) -> Result<Header, Error> {
    let declarations = FmodOutputParser::parse(Rule::api, source)?
        .next()
        .ok_or(Error::FileMalformed)?;

    let arrays = vec![
        String::from("flags"),
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
    use crate::fmod_output::{parse, Header};
    use crate::models::Type::FundamentalType;
    use crate::models::{Argument, Callback};

    #[test]
    fn test_should_parse_callback_with_varargs() {
        let source = r#"
            typedef void (F_CALL *FMOD_OUTPUT_LOG_FUNC) (int line, ...);
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                opaque_types: vec![],
                constants: vec![],
                flags: vec![],
                structures: vec![],
                callbacks: vec![Callback {
                    return_type: FundamentalType("void".into()),
                    pointer: None,
                    name: "FMOD_OUTPUT_LOG_FUNC".into(),
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
    fn test_should_ignore_macros() {
        let source = r#"
            #define FMOD_OUTPUT_READFROMMIXER(_state, _buffer, _length) \
                (_state)->readfrommixer(_state, _buffer, _length)
        "#;
        assert_eq!(parse(source), Ok(Header::default()))
    }
}
