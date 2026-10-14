//! The condition expressions of a behavior graph's transitions (`hkbExpressionCondition`,
//! `hkbStringCondition`): a transition fires on its event only while its condition holds.
//!
//! The vanilla graphs write them as plain C-like text over the graph's variables:
//! `iSyncIdleLocomotion == 1`, `!IsPlayer`, `(staggerDirection > .25) &&
//! (staggerDirection < .75)`, `Speed >= fMinSpeed`, `(iRightHandType == 8) ||
//! (iRightHandType == 9)`. The grammar here is exactly that: numbers (`.25` too), variable
//! names, parentheses, unary `!` and `-`, `* /`, `+ -`, `< <= > >=`, `== !=`, `&&`, `||`;
//! a bare variable is true when it is not zero. An expression it cannot read (the one
//! vanilla `iSyncIdleLocomotion=18`, whose single `=` the grammar has no meaning for) is
//! kept as text and counts as true, so the transition behaves as before conditions were
//! evaluated.

/// A parsed condition. Equality compares the source text.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub source: String,
    expression: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
enum Expression {
    Number(f32),
    Variable(String),
    Not(Box<Expression>),
    Negate(Box<Expression>),
    Binary(Operator, Box<Expression>, Box<Expression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f32),
    Name(String),
    Operator(Operator),
    Not,
    Open,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        match c {
            c if c.is_whitespace() => at += 1,
            '(' => {
                tokens.push(Token::Open);
                at += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                at += 1;
            }
            '0'..='9' | '.' => {
                let start = at;
                while at < chars.len() && (chars[at].is_ascii_digit() || chars[at] == '.') {
                    at += 1;
                }
                let literal: String = chars[start..at].iter().collect();
                tokens.push(Token::Number(literal.parse().ok()?));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = at;
                while at < chars.len() && (chars[at].is_alphanumeric() || chars[at] == '_') {
                    at += 1;
                }
                tokens.push(Token::Name(chars[start..at].iter().collect()));
            }
            '!' if next == Some('=') => {
                tokens.push(Token::Operator(Operator::NotEqual));
                at += 2;
            }
            '!' => {
                tokens.push(Token::Not);
                at += 1;
            }
            '=' if next == Some('=') => {
                tokens.push(Token::Operator(Operator::Equal));
                at += 2;
            }
            '&' if next == Some('&') => {
                tokens.push(Token::Operator(Operator::And));
                at += 2;
            }
            '|' if next == Some('|') => {
                tokens.push(Token::Operator(Operator::Or));
                at += 2;
            }
            '<' | '>' => {
                let (plain, with_equal) = if c == '<' { (Operator::Less, Operator::LessEqual) } else { (Operator::Greater, Operator::GreaterEqual) };
                if next == Some('=') {
                    tokens.push(Token::Operator(with_equal));
                    at += 2;
                } else {
                    tokens.push(Token::Operator(plain));
                    at += 1;
                }
            }
            '+' => {
                tokens.push(Token::Operator(Operator::Add));
                at += 1;
            }
            '-' => {
                tokens.push(Token::Operator(Operator::Subtract));
                at += 1;
            }
            '*' => {
                tokens.push(Token::Operator(Operator::Multiply));
                at += 1;
            }
            '/' => {
                tokens.push(Token::Operator(Operator::Divide));
                at += 1;
            }
            _ => return None,
        }
    }
    Some(tokens)
}

/// Binary operator levels, loosest first.
const LEVELS: &[&[Operator]] = &[
    &[Operator::Or],
    &[Operator::And],
    &[Operator::Equal, Operator::NotEqual],
    &[Operator::Less, Operator::LessEqual, Operator::Greater, Operator::GreaterEqual],
    &[Operator::Add, Operator::Subtract],
    &[Operator::Multiply, Operator::Divide],
];

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn binary(&mut self, level: usize) -> Option<Expression> {
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        while let Some(Token::Operator(operator)) = self.peek().cloned() {
            if !LEVELS[level].contains(&operator) {
                break;
            }
            self.at += 1;
            let right = self.binary(level + 1)?;
            left = Expression::Binary(operator, Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn unary(&mut self) -> Option<Expression> {
        match self.peek()?.clone() {
            Token::Not => {
                self.at += 1;
                Some(Expression::Not(Box::new(self.unary()?)))
            }
            Token::Operator(Operator::Subtract) => {
                self.at += 1;
                Some(Expression::Negate(Box::new(self.unary()?)))
            }
            Token::Number(value) => {
                self.at += 1;
                Some(Expression::Number(value))
            }
            Token::Name(name) => {
                self.at += 1;
                Some(Expression::Variable(name))
            }
            Token::Open => {
                self.at += 1;
                let inner = self.binary(0)?;
                (self.peek() == Some(&Token::Close)).then(|| self.at += 1)?;
                Some(inner)
            }
            _ => None,
        }
    }
}

impl Expression {
    fn value(&self, variable: &dyn Fn(&str) -> Option<f32>) -> f32 {
        let flag = |condition: bool| f32::from(condition);
        match self {
            Self::Number(value) => *value,
            // A variable the graph does not have reads as zero.
            Self::Variable(name) => variable(name).unwrap_or(0.0),
            Self::Not(inner) => flag(inner.value(variable) == 0.0),
            Self::Negate(inner) => -inner.value(variable),
            Self::Binary(operator, left, right) => {
                let (left, right) = (left.value(variable), right.value(variable));
                match operator {
                    Operator::Or => flag(left != 0.0 || right != 0.0),
                    Operator::And => flag(left != 0.0 && right != 0.0),
                    Operator::Equal => flag(left == right),
                    Operator::NotEqual => flag(left != right),
                    Operator::Less => flag(left < right),
                    Operator::LessEqual => flag(left <= right),
                    Operator::Greater => flag(left > right),
                    Operator::GreaterEqual => flag(left >= right),
                    Operator::Add => left + right,
                    Operator::Subtract => left - right,
                    Operator::Multiply => left * right,
                    Operator::Divide => {
                        if right == 0.0 {
                            0.0
                        } else {
                            left / right
                        }
                    }
                }
            }
        }
    }
}

/// One line of an `hkbEvaluateExpressionModifier`: `variablename = expression` or
/// `eventname = boolean expression` (the form in the engine's description of the field).
/// The Skyrim graphs also write the event form as `eventname if (boolean expression)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// The variable assigned, or the event raised: the graph tells which by what it has.
    pub target: String,
    pub value: Condition,
}

impl Statement {
    /// `None` when the text has neither form or its right-hand side cannot be read.
    pub fn parse(source: &str) -> Option<Self> {
        let (target, value) = match split_assignment(source) {
            Some(parts) => parts,
            None => {
                let at = source.find(" if ")?;
                (&source[..at], &source[at + 4..])
            }
        };
        let target = target.trim();
        let valid = !target.is_empty() && target.chars().all(|c| c.is_alphanumeric() || c == '_');
        let value = Condition::parse(value);
        (valid && value.is_readable()).then(|| Self { target: target.to_owned(), value })
    }
}

/// Split `left = right` at the first `=` that is not part of `==`, `!=`, `<=` or `>=`.
fn split_assignment(source: &str) -> Option<(&str, &str)> {
    let bytes = source.as_bytes();
    (0..bytes.len()).find_map(|at| {
        let lone = bytes[at] == b'='
            && bytes.get(at + 1) != Some(&b'=')
            && !matches!(at.checked_sub(1).map(|before| bytes[before]), Some(b'=' | b'!' | b'<' | b'>'));
        lone.then(|| (&source[..at], &source[at + 1..]))
    })
}

impl Condition {
    /// The expression's numeric value (a comparison is 1 or 0); `None` when the text was
    /// not understood.
    pub fn value(&self, variable: &dyn Fn(&str) -> Option<f32>) -> Option<f32> {
        self.expression.as_ref().map(|expression| expression.value(variable))
    }

    pub fn parse(source: &str) -> Self {
        let expression = tokenize(source).and_then(|tokens| {
            let mut parser = Parser { tokens, at: 0 };
            let expression = parser.binary(0)?;
            (parser.at == parser.tokens.len()).then_some(expression)
        });
        Self { source: source.to_owned(), expression }
    }

    /// Whether the text was understood.
    pub fn is_readable(&self) -> bool {
        self.expression.is_some()
    }

    /// Whether the condition holds when `variable` gives the graph's variables by name.
    /// An unreadable condition holds.
    pub fn holds(&self, variable: &dyn Fn(&str) -> Option<f32>) -> bool {
        self.expression.as_ref().is_none_or(|expression| expression.value(variable) != 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holds(source: &str, values: &[(&str, f32)]) -> bool {
        let condition = Condition::parse(source);
        assert!(condition.is_readable(), "{source}");
        condition.holds(&|name| values.iter().find(|(known, _)| known.eq_ignore_ascii_case(name)).map(|(_, value)| *value))
    }

    /// The shapes the vanilla Skyrim and Fallout 4 graphs use.
    #[test]
    fn the_vanilla_conditions_read_and_evaluate() {
        assert!(holds("iSyncIdleLocomotion == 1", &[("iSyncIdleLocomotion", 1.0)]));
        assert!(!holds("iSyncIdleLocomotion == 1", &[("iSyncIdleLocomotion", 0.0)]));
        assert!(holds("!IsPlayer", &[("IsPlayer", 0.0)]));
        assert!(!holds("!IsPlayer", &[("IsPlayer", 1.0)]));
        assert!(holds("IsPlayer", &[("IsPlayer", 1.0)]), "a bare variable is true when set");
        assert!(holds("iIsInSneak == 0", &[]), "an unknown variable reads zero");
        assert!(holds("(bFailMoveStart==0) ", &[("bFailMoveStart", 0.0)]), "trailing space");
        let stagger = "(staggerDirection > .25) && (staggerDirection < .75)";
        assert!(holds(stagger, &[("staggerDirection", 0.5)]));
        assert!(!holds(stagger, &[("staggerDirection", 0.9)]));
        let ors = "(staggerDirection < .25) || (staggerDirection > .75) || (staggerMagnitude < .3)";
        assert!(holds(ors, &[("staggerDirection", 0.5), ("staggerMagnitude", 0.1)]));
        assert!(!holds(ors, &[("staggerDirection", 0.5), ("staggerMagnitude", 0.5)]));
        assert!(holds("Speed >= fMinSpeed", &[("Speed", 80.0), ("fMinSpeed", 50.0)]), "variables on both sides");
        assert!(holds("(iRightHandType == 8) || (iRightHandType == 9)", &[("iRightHandType", 9.0)]));
        assert!(holds("iRightHandEquipped != 0", &[("iRightHandEquipped", 3.0)]));
    }

    #[test]
    fn statements_assign_a_variable_or_raise_an_event() {
        let assign = Statement::parse("iCombatStance = 1 ").expect("an assignment");
        assert_eq!(assign.target, "iCombatStance");
        assert_eq!(assign.value.value(&|_| None), Some(1.0));
        let event = Statement::parse("weaponDraw = (iCombatStance == 1)").expect("an event");
        assert_eq!(event.target, "weaponDraw");
        let held = |name: &str| (name == "iCombatStance").then_some(1.0);
        assert_eq!(event.value.value(&held), Some(1.0), "the comparison is the value, `==` is not split");
        let legacy = Statement::parse("weaponSheathe if (iCombatStance == 0)").expect("the older event form");
        assert_eq!(legacy.target, "weaponSheathe");
        assert_eq!(legacy.value.value(&held), Some(0.0));
        assert!(Statement::parse("no operator").is_none());
        assert!(Statement::parse("x = (").is_none());
        assert_eq!(Statement::parse("a = b >= 2").unwrap().value.value(&|_| Some(3.0)), Some(1.0), ">= is not an assignment");
    }

    #[test]
    fn precedence_and_arithmetic_follow_c() {
        assert!(holds("1 + 2 * 3 == 7", &[]));
        assert!(holds("(1 + 2) * 3 == 9", &[]));
        assert!(holds("!0 && 1 || 0", &[]));
        assert!(holds("-1 < 0", &[]));
        assert!(holds("1 / 0 == 0", &[]), "a division by zero is zero, not a panic");
        assert!(holds("0 || 1 && 0 == 0", &[]), "&& binds tighter than ||, == tighter than &&");
    }

    /// The one vanilla expression the grammar has no reading for (a single `=`) is kept
    /// as text and does not block its transition.
    #[test]
    fn unreadable_text_is_kept_and_holds() {
        for source in ["iSyncIdleLocomotion=18", "a ==", "(a == 1", "a b", "a # b", ""] {
            let condition = Condition::parse(source);
            assert!(!condition.is_readable(), "{source:?}");
            assert_eq!(condition.source, source);
            assert!(condition.holds(&|_| None), "{source:?} holds");
        }
    }
}
