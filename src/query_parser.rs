use crate::query::QueryElement;

/// This is the main entry point for parsing our simple query language into a structured format.
///
/// ```
/// use access_json::query::QueryElement;
/// use access_json::parse_query;
///
/// assert_eq!(
///   parse_query(".field.array[8]").unwrap(),
///   vec![QueryElement::field("field"), QueryElement::field("array"), QueryElement::array_item(8)]);
/// ```
pub fn parse_query(input: &str) -> Result<Vec<QueryElement>, QueryParseErr> {
    let mut output = Vec::new();
    let mut parser = Parser::from(input);
    while let Some(it) = parser.next()? {
        output.push(it);
    }
    Ok(output)
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum QueryParseErr {
    BadCharacter(usize),
    MissingField,
    MissingNumber(usize),
    BadArray(usize),
    BadField(usize),
    UnexpectedEOF(char),
    Unexpected(usize, char),
    BadIndex(usize, String),
}

impl std::fmt::Display for QueryParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for QueryParseErr {}

struct Parser {
    data: Vec<char>,
    position: usize,
}

impl From<&str> for Parser {
    fn from(input: &str) -> Parser {
        Parser {
            data: input.chars().collect(),
            position: 0,
        }
    }
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.data.get(self.position).cloned()
    }
    fn advance(&mut self) -> Option<char> {
        let found = self.peek();
        self.position += 1;
        found
    }
    fn consume(&mut self, expected: char) -> Result<(), QueryParseErr> {
        match self.advance() {
            None => Err(QueryParseErr::UnexpectedEOF(expected)),
            Some(actual) => {
                if actual == expected {
                    Ok(())
                } else {
                    Err(QueryParseErr::Unexpected(self.position - 1, expected))
                }
            }
        }
    }
    fn read_array(&mut self) -> Result<QueryElement, QueryParseErr> {
        self.consume('[')?;
        let mut digits = String::new();
        let start = self.position;

        while let Some(ch) = self.advance() {
            if ch == ']' {
                break;
            } else if ch.is_digit(10) {
                digits.push(ch);
            } else {
                return Err(QueryParseErr::BadArray(self.position - 1));
            }
        }

        if digits.is_empty() {
            Err(QueryParseErr::MissingNumber(start))
        } else {
            let num = digits
                .parse::<usize>()
                .map_err(|e| QueryParseErr::BadIndex(start, e.to_string()))?;
            Ok(QueryElement::ArrayItem(num))
        }
    }
    fn read_field(&mut self) -> Result<QueryElement, QueryParseErr> {
        self.consume('.')?;
        let mut id = String::new();
        while let Some(ch) = self.peek() {
            if ch == '.' || ch == '[' {
                break;
            } else if ch.is_whitespace() {
                Err(QueryParseErr::BadField(self.position - 1))?;
            }
            self.consume(ch)?;
            id.push(ch);
        }
        if id.is_empty() {
            Err(QueryParseErr::MissingField)
        } else {
            Ok(QueryElement::Field(id))
        }
    }
    fn next(&mut self) -> Result<Option<QueryElement>, QueryParseErr> {
        if let Some(ch) = self.peek() {
            Ok(Some(if ch == '[' {
                self.read_array()?
            } else if ch == '.' {
                self.read_field()?
            } else {
                return Err(QueryParseErr::BadCharacter(self.position));
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_query;
    use super::QueryParseErr;
    use crate::query::QueryElement as Q;

    #[test]
    fn test_dots_happy() {
        assert_eq!(
            parse_query(".a.b.c").unwrap(),
            vec![Q::field("a"), Q::field("b"), Q::field("c")]
        )
    }

    #[test]
    fn test_array_happy() {
        assert_eq!(
            parse_query("[0][7]").unwrap(),
            vec![Q::array_item(0), Q::array_item(7)]
        )
    }

    #[test]
    fn test_parse_mixed() {
        assert_eq!(
            parse_query(".a.b[7].c.e[5]").unwrap(),
            vec![
                Q::field("a"),
                Q::field("b"),
                Q::array_item(7),
                Q::field("c"),
                Q::field("e"),
                Q::array_item(5)
            ]
        )
    }

    #[test]
    fn test_missing_field() {
        assert_eq!(
            parse_query(".a.b.").unwrap_err(),
            QueryParseErr::MissingField
        );
    }

    #[test]
    fn test_bad_numbers() {
        assert_eq!(
            parse_query("[0][]").unwrap_err(),
            QueryParseErr::MissingNumber(4)
        )
    }

    #[test]
    fn test_array_not_closed() {
        assert_eq!(
            parse_query("[").unwrap_err(),
            QueryParseErr::MissingNumber(1)
        )
    }
}
