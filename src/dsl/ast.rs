use lazy_static::*;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest_consume::{match_nodes, Error, Parser};

type ParseResult<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

use super::super::structures;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("syntax.pest");

// Define the precedence of binary operations. We use `lazy_static` so that
// this is only ever constructed once.
lazy_static::lazy_static! {
    static ref PRECCLIMBER: PrecClimber<Rule> = PrecClimber::new(
        vec![
            // loosest binding
            Operator::new(Rule::ast_or, Assoc::Left),
            Operator::new(Rule::ast_and, Assoc::Left),
            // tighest binding
        ]
    );
}

#[derive(Parser)]
#[grammar = "dsl/syntax.pest"]
pub struct AstParser;

#[pest_consume::parser]
impl AstParser {
    fn LEQ(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn GEQ(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn EQ(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn NEQ(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn LT(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn GT(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn input_literal(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn num(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected non-negative number"))
    }

    fn hex_num(input: Node) -> ParseResult<u64> {
        let string = input.as_str();
        // drop the hex literal prefix
        let string = string.chars().skip(2).collect::<String>();
        Ok(u64::from_str_radix(&string, 16).expect("Expected non-negative number"))
    }

    fn comparison_operator(input: Node) -> ParseResult<structures::ComparisonOperator> {
        Ok(match_nodes!(input.into_children();
            [LEQ(_)] => structures::ComparisonOperator::LessThanOrEqual,
            [GEQ(_)] => structures::ComparisonOperator::GreaterThanOrEqual,
            [EQ(_)] => structures::ComparisonOperator::Equal,
            [NEQ(_)] => structures::ComparisonOperator::NotEqual,
            [LT(_)] => structures::ComparisonOperator::LessThan,
            [GT(_)] => structures::ComparisonOperator::GreaterThan
        ))
    }

    fn ast_p_v_comp(input: Node) -> ParseResult<structures::Condition> {
        Ok(match_nodes!(input.into_children();
                [comparison_operator(c), num(n)] => structures::Condition::ComparisonPortVal(n, c)
        ))
    }

    fn ast_v_p_comp(input: Node) -> ParseResult<structures::Condition> {
        Ok(match_nodes!(input.into_children();
            [num(n), comparison_operator(c)] => structures::Condition::ComparisonValPort(n, c)
        ))
    }

    fn ast_comparison(input: Node) -> ParseResult<structures::Condition> {
        Ok(match_nodes!(input.into_children();
            [ast_p_v_comp(c)] => c,
            [ast_v_p_comp(c)] => c
        ))
    }

    #[prec_climb(ast_bool, PRECCLIMBER)]
    fn ast_bool_expression(
        left: structures::Condition,
        op: Node,
        right: structures::Condition,
    ) -> ParseResult<structures::Condition> {
        Ok(match op.as_rule() {
            Rule::ast_and => structures::Condition::And(Box::new(left), Box::new(right)),
            Rule::ast_or => structures::Condition::Or(Box::new(left), Box::new(right)),
            _ => unreachable!(),
        })
    }

    fn ast_bool(input: Node) -> ParseResult<structures::Condition> {
        Ok(match_nodes!(input.into_children();
            [ast_not(_), ast_bool_expression(b)] => structures::Condition::Not(Box::new(b)),
            [ast_bool_expression(b)] => b,
            [ast_comparison(b)] => b
        ))
    }

    fn ast_not(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn ast_and(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn ast_or(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn z3_noop(_input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(structures::TerminalRoutingProgram::Noop)
    }
    fn ast_noop(_input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(structures::TerminalRoutingProgram::Noop)
    }

    fn z3_rshift(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [hex_num(n)] => structures::TerminalRoutingProgram::RShift(n as usize)
        ))
    }
    fn ast_rshift(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => structures::TerminalRoutingProgram::RShift(n as usize)
        ))
    }

    fn z3_add(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [hex_num(n)] => structures::TerminalRoutingProgram::Add(n)
        ))
    }
    fn ast_add(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => structures::TerminalRoutingProgram::Add(n)
        ))
    }
    fn z3_subpv(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [hex_num(n)] => structures::TerminalRoutingProgram::SubPortVal(n)
        ))
    }
    fn ast_subpv(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => structures::TerminalRoutingProgram::SubPortVal(n)
        ))
    }

    fn z3_subvp(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [hex_num(n)] => structures::TerminalRoutingProgram::SubValPort(n)
        ))
    }
    fn ast_subvp(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => structures::TerminalRoutingProgram::SubValPort(n)
        ))
    }
    fn z3_constant(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [hex_num(n)] => structures::TerminalRoutingProgram::Constant(n)
        ))
    }
    fn ast_constant(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [num(n)] => structures::TerminalRoutingProgram::Constant(n)
        ))
    }

    fn z3_address_translation(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [z3_constant(z)] => z,
            [z3_subvp(z)] => z,
            [z3_subpv(z)] => z,
            [z3_add(z)] => z,
            [z3_rshift(z)] => z,
            [z3_noop(z)] => z
        ))
    }
    fn ast_translation_terminal(input: Node) -> ParseResult<structures::TerminalRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [ast_constant(a)] => a,
            [ast_subvp(a)] => a,
            [ast_subpv(a)] => a,
            [ast_add(a)] => a,
            [ast_rshift(a)] => a,
            [ast_noop(a)] => a
        ))
    }
    fn range_z3(input: Node) -> ParseResult<structures::MemoryLayout> {
        Ok(match_nodes!(input.into_children();
                [num(n1), num(n2), num(n3)] => structures::MemoryLayout::new(n1 as usize, n2 as usize, Some(n3 as usize)),
        ))
    }
    fn range_ast(input: Node) -> ParseResult<structures::MemoryLayout> {
        Ok(match_nodes!(input.into_children();
                [num(n1), num(n2), num(n3)] => structures::MemoryLayout::new(n1 as usize, n2 as usize, Some(n3 as usize)),
                [num(n1), num(n2)] => structures::MemoryLayout::new(n1 as usize, n2 as usize, None),
        ))
    }
    fn partition(input: Node) -> ParseResult<structures::MemoryLayout> {
        Ok(match_nodes!(input.into_children();
            [range_z3(z)] => z,
            [range_ast(a)] => a
        ))
    }

    fn ast_translation_sequence(input: Node) -> ParseResult<structures::SequenceRoutingProg> {
        Ok(match_nodes!(input.into_children();
            [ast_translation_terminal(n)..] => structures::SequenceRoutingProg::Sequence(n.collect())
        ))
    }
    fn ast_translation_mid_level(input: Node) -> ParseResult<structures::SequenceRoutingProg> {
        Ok(match_nodes!(input.into_children();
                [ast_translation_sequence(s)] => s,
                [ast_translation_terminal(n)] => structures::SequenceRoutingProg::Prog(n)
        ))
    }

    fn ast_translation_switch_case(
        input: Node,
    ) -> ParseResult<(structures::Condition, structures::SequenceRoutingProg)> {
        Ok(match_nodes!(input.into_children();
            [ast_bool(b), ast_translation_mid_level(n)] => (b,n)
        ))
    }
    fn ast_translation_switch_default(input: Node) -> ParseResult<structures::SequenceRoutingProg> {
        Ok(match_nodes!(input.into_children();
            [ast_translation_mid_level(n)] => n
        ))
    }

    fn ast_translation_switch(input: Node) -> ParseResult<structures::TopLevelRoutingProgram> {
        Ok(match_nodes!(input.into_children();
            [ast_translation_switch_case(sw).., ast_translation_switch_default(sd)] => structures::TopLevelRoutingProgram::Switch(sw.collect(), Box::new(sd)),
        ))
    }
    fn ast_translation_top_level(input: Node) -> ParseResult<structures::TopLevelRoutingProgram> {
        Ok(match_nodes!(input.into_children();
                [ast_translation_switch(sw)] => sw,
                [ast_translation_mid_level(n)] => structures::TopLevelRoutingProgram::Prog(n)
        ))
    }
}

impl AstParser {
    pub fn parse_partition<S: AsRef<str>>(
        input: S,
    ) -> ParseResult<structures::TopLevelMemoryLayout> {
        let inputs = AstParser::parse(Rule::partition, input.as_ref())?;
        let input = inputs.single()?;
        Ok(AstParser::partition(input)?.into())
    }

    pub fn parse_z3_address_translation<S: AsRef<str>>(
        input: S,
    ) -> ParseResult<structures::TopLevelRoutingProgram> {
        let inputs = AstParser::parse(Rule::z3_address_translation, input.as_ref())?;
        let input = inputs.single()?;
        Ok(AstParser::z3_address_translation(input)?.into())
    }
}
