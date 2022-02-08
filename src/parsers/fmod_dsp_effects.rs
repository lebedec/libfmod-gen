use crate::models::{Constant, Enumeration, Error, Structure};
use crate::repr::JsonConverter;
use pest::{error, Parser};

#[derive(Parser)]
#[grammar = "./grammars/fmod_dsp_effects.pest"]
struct FmodDspEffectsParser;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub constants: Vec<Constant>,
    pub enumerations: Vec<Enumeration>,
    pub structures: Vec<Structure>,
}

pub fn parse(source: &str) -> Result<Header, Error> {
    let declarations = FmodDspEffectsParser::parse(Rule::api, source)?
        .next()
        .ok_or(Error::FileMalformed)?;

    let arrays = vec![String::from("enumerators"), String::from("fields")];
    let converter = JsonConverter::new(arrays);

    let mut header = Header::default();
    for declaration in declarations.into_inner() {
        match declaration.as_rule() {
            Rule::Constant => header.constants.push(converter.convert(declaration)?),
            Rule::Enumeration => header.enumerations.push(converter.convert(declaration)?),
            Rule::Structure => header.structures.push(converter.convert(declaration)?),
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
    use crate::fmod_dsp_effects::{parse, Header};
    use crate::models::{Enumeration, Enumerator};

    #[test]
    fn test_should_parse_enumeration_without_name() {
        let source = r#"
            typedef enum
            {
                FMOD_DSP_ENVELOPEFOLLOWER_ATTACK,
                FMOD_DSP_ENVELOPEFOLLOWER_RELEASE,
            } FMOD_DSP_ENVELOPEFOLLOWER;
        "#;
        assert_eq!(
            parse(source),
            Ok(Header {
                constants: vec![],
                enumerations: vec![Enumeration {
                    name: "FMOD_DSP_ENVELOPEFOLLOWER".into(),
                    enumerators: vec![
                        Enumerator {
                            name: "FMOD_DSP_ENVELOPEFOLLOWER_ATTACK".into(),
                            value: None
                        },
                        Enumerator {
                            name: "FMOD_DSP_ENVELOPEFOLLOWER_RELEASE".into(),
                            value: None
                        }
                    ]
                }],
                structures: vec![],
            })
        )
    }
}
