use std::fmt;
use std::error;

#[derive(Debug)]
pub enum XmlErrorKind {
    /// Character disallowed in current context
    BadChar(char),
    /// recursion depth max exceeded
    MaxRecurDepth(u32),
    /// text ends before parsing complete
    TextEnd,
    /// available text does not match any variant of the parsing rule
    NoValidVariant,
    /// illegal substring encountered
    IllegalSubstr,
    /// use of name xml which is reserved
    ReservedNameXml,
    /// mismatch between opening and closing tags
    MismatchedTags(String, String),
    /// did not see opening <![CDATA[ tag while attempting to parse CDSect
    BadCDATAStart,
    /// No available data when trying to parse for character data
    /// Need to make this an error because the rest of the parser doesn't expect
    /// zero-length elements
    NoData,
    /// did not see opening <?xml when attempting to parse XmlDecl
    BadXDeclStart,
    /// did not see a keyword when one was expected
    KeywordMatchFail,
}

#[derive(Debug)]
pub struct XmlError {
    /// the kind of error encountered
    category: XmlErrorKind,
    /// index in document where error is encountered
    doc_idx: usize,
    /// if there is a different issue causing this one, it gets reported here
    underlying: Option<Box<XmlError>>,
    /// report context name if doing so is potentially useful
    context: String,
}

impl fmt::Display for XmlErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            XmlErrorKind::BadChar(c) => {
                write!(f, "Encountered `{}`, which is invalid in context", c)
            }
            XmlErrorKind::MaxRecurDepth(d) => {
                write!(f, "Recursion depth {} exceeds max depth parameter", d)
            }
            XmlErrorKind::TextEnd => write!(f, "encountered end of text unexpectedly"),
            XmlErrorKind::NoValidVariant => write!(f, "no valid pattern variant for this input"),
            XmlErrorKind::IllegalSubstr => write!(f, "Encountered a substring which is disallowed"),
            XmlErrorKind::ReservedNameXml => write!(
                f,
                "Encountered reserved name xml in context that disallows it"
            ),
            XmlErrorKind::MismatchedTags(start, end) => {
                write!(f, "tags `{}` and `{}` do not match", start, end)
            }
            XmlErrorKind::BadCDATAStart => write!(
                f,
                "unable to recognize starting tag of CDATA section. open tag should be <![CDATA["
            ),
            XmlErrorKind::NoData => write!(
                f,
                "No data when data expected, error because parser must report empty data specially to work"
            ),
            XmlErrorKind::BadXDeclStart => write!(
                f,
                "did not see <?xml when attempting to parse XML declaration"
            ),
            XmlErrorKind::KeywordMatchFail => write!(
                f,
                "failed when trying to match keyword, check spelling and capitalization"
            ),
        }
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.underlying {
            Some(cause) => write!(f, "XMLError at index {}: {}. Caused by {}. Additional context: {}", 
                self.doc_idx, self.category, cause, self.context),
            None => write!(f, "XMLError at index {}: {}. Additional context: {}",
                self.doc_idx, self.category, self.context),
        }
    }
}


impl From<XmlErrorKind> for XmlError {
    fn from(value: XmlErrorKind) -> Self {
        XmlError {
            category : value,
            doc_idx : 0,
            underlying : None,
            context : String::new(),
        }
    }
}

impl<T> From<XmlError> for Result<T, XmlError> {
    fn from(value: XmlError) -> Self {
        Err(value)
    }
}

