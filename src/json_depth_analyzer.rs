use std::fmt;

#[derive(Debug, Clone)]
pub struct JsonDepthAnalyzer {
    state: Vec<ParserState>,
}

impl JsonDepthAnalyzer {
    pub fn new() -> JsonDepthAnalyzer {
        JsonDepthAnalyzer { state: vec![] }
    }

    pub fn depth(&self) -> usize {
        self.state.len()
    }

    pub fn process(&mut self, c: u8) -> Result<(), ParserError> {
        match (self.state.last(), c) {
            (Some(ParserState::String), b'"') => {
                self.state.pop();
                Ok(())
            }
            (_, b'"') => {
                self.state.push(ParserState::String);
                Ok(())
            }
            (Some(ParserState::String), b'\\') => {
                *self.state.last_mut().unwrap() = ParserState::StringEscape;
                Ok(())
            }
            (Some(ParserState::StringEscape), b'u') => {
                *self.state.last_mut().unwrap() = ParserState::StringHex4;
                Ok(())
            }
            (Some(ParserState::StringHex4), c) => {
                *self.state.last_mut().unwrap() = ParserState::StringHex3;
                if c.is_ascii_hexdigit() {
                    Ok(())
                } else {
                    Err(ParserError::WrongHexCharacter { got: c })
                }
            }
            (Some(ParserState::StringHex3), c) => {
                *self.state.last_mut().unwrap() = ParserState::StringHex2;
                if c.is_ascii_hexdigit() {
                    Ok(())
                } else {
                    Err(ParserError::WrongHexCharacter { got: c })
                }
            }
            (Some(ParserState::StringHex2), c) => {
                *self.state.last_mut().unwrap() = ParserState::StringHex1;
                if c.is_ascii_hexdigit() {
                    Ok(())
                } else {
                    Err(ParserError::WrongHexCharacter { got: c })
                }
            }
            (Some(ParserState::StringHex1), c) => {
                *self.state.last_mut().unwrap() = ParserState::String;
                if c.is_ascii_hexdigit() {
                    Ok(())
                } else {
                    Err(ParserError::WrongHexCharacter { got: c })
                }
            }
            (Some(ParserState::StringEscape), c) => {
                *self.state.last_mut().unwrap() = ParserState::String;
                if "\"\\/bfnrt".bytes().any(|e| c == e) {
                    Ok(())
                } else {
                    Err(ParserError::WrongEscapeCharacter { got: c })
                }
            }
            (Some(ParserState::String), _) => Ok(()),

            (_, b'{') => {
                self.state.push(ParserState::Object);
                Ok(())
            }
            (Some(ParserState::Object), b'}') => {
                self.state.pop();
                Ok(())
            }
            (got, b'}') => Err(ParserError::WrongState {
                got: got.cloned(),
                expected: ParserState::Object,
            }),
            (_, b'[') => {
                self.state.push(ParserState::Array);
                Ok(())
            }
            (Some(ParserState::Array), b']') => {
                self.state.pop();
                Ok(())
            }
            (got, b']') => Err(ParserError::WrongState {
                got: got.cloned(),
                expected: ParserState::Array,
            }),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParserError {
    WrongState {
        got: Option<ParserState>,
        expected: ParserState,
    },
    WrongHexCharacter {
        got: u8,
    },
    WrongEscapeCharacter {
        got: u8,
    },
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::WrongState {
                got: Some(got),
                expected,
            } => write!(f, "expected state ({:?}), got ({:?})", expected, got),
            Self::WrongState {
                got: None,
                expected,
            } => write!(f, "expected state ({:?}), got nothing", expected),
            Self::WrongHexCharacter { got } => write!(f, "expected hex character, got '{}'", got),
            Self::WrongEscapeCharacter { got } => {
                write!(f, "expected escape sequence, got \"{}\"", got)
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ParserState {
    Object,
    Array,
    String,
    StringEscape,
    StringHex4,
    StringHex3,
    StringHex2,
    StringHex1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_of_single_object() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[{\"a\": \"hello\"}]";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 0);
    }

    #[test]
    fn empty_array() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[]";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 0);
    }

    #[test]
    fn empty_object() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "{}";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 0);
    }

    #[test]
    fn wrong_nesting() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[{]}";
        assert_eq!(json.bytes().all(|c| parser.process(c).is_ok()), false);
    }

    #[test]
    fn recover_wrong_nesting() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[{]";
        for c in json.bytes() {
            let _ = parser.process(c);
        }

        let json = "}]";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 0);
    }

    #[test]
    fn open_string() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[{\"}]";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 3);
    }

    #[test]
    fn open_escape() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[{\"\\";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 3);
    }

    #[test]
    fn open_unicode() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[{\"\\ueF4";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 3);
    }

    #[test]
    fn escaped() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "[\"\\n\\u1234\"]";
        assert!(json.bytes().all(|c| parser.process(c).is_ok()));
        assert_eq!(parser.depth(), 0);
    }

    #[test]
    fn invalid_escape() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "\"\\x";
        assert_eq!(json.bytes().all(|c| parser.process(c).is_ok()), false);
        assert_eq!(parser.depth(), 1);
    }

    #[test]
    fn invalid_unicode() {
        let mut parser = JsonDepthAnalyzer::new();
        let json = "\"\\u123x";
        assert_eq!(json.bytes().all(|c| parser.process(c).is_ok()), false);
        assert_eq!(parser.depth(), 1);
    }
}
