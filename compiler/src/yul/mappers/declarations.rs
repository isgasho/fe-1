use crate::errors::CompileError;
use crate::yul::mappers::expressions;
use crate::yul::names;
use fe_analyzer::namespace::types::{
    FeSized,
    FixedSize,
};
use fe_analyzer::Context;
use fe_parser::ast as fe;
use fe_parser::span::Spanned;
use yultsur::*;

/// Builds a Yul statement from a Fe variable declaration
pub fn var_decl(
    context: &Context,
    stmt: &Spanned<fe::FuncStmt>,
) -> Result<yul::Statement, CompileError> {
    let decl_type = context.get_declaration(stmt).expect("missing attributes");

    if let fe::FuncStmt::VarDecl { target, value, .. } = &stmt.node {
        let target = names::var_name(expressions::expr_name_str(&target)?);

        return Ok(if let Some(value) = value {
            let value = expressions::expr(context, &value)?;
            statement! { let [target] := [value] }
        } else {
            match decl_type {
                FixedSize::Base(_) => statement! { let [target] := 0 },
                typ => {
                    let size = literal_expression! { (typ.size()) };
                    statement! { let [target] := alloc([size]) }
                }
            }
        });
    }

    unreachable!()
}

#[cfg(test)]
mod tests {
    use crate::yul::mappers::declarations::var_decl;
    use fe_analyzer::namespace::types::{
        Array,
        Base,
        FixedSize,
        Type,
        U256,
    };
    use fe_analyzer::test_utils::ContextHarness;
    use fe_analyzer::{
        Context,
        ExpressionAttributes,
        Location,
    };
    use fe_parser as parser;

    fn map(context: &Context, src: &str) -> String {
        let tokens = parser::get_parse_tokens(src).expect("Couldn't parse declaration");
        let stmt = &parser::parsers::vardecl_stmt(&tokens[..])
            .expect("Couldn't build declaration AST")
            .1;

        let decl = var_decl(context, &stmt).expect("Couldn't map declaration AST");

        decl.to_string()
    }

    #[test]
    fn decl_u256() {
        let mut harness = ContextHarness::new("foo: u256 = bar");
        harness.add_declaration("foo: u256 = bar", FixedSize::Base(U256));
        harness.add_expression(
            "bar",
            ExpressionAttributes::new(Type::Base(U256), Location::Value),
        );

        assert_eq!(map(&harness.context, &harness.src), "let $foo := $bar");
    }

    #[test]
    fn decl_array() {
        let mut harness = ContextHarness::new("foo: address[10]");
        harness.add_declaration(
            "foo: address[10]",
            FixedSize::Array(Array {
                dimension: 10,
                inner: Base::Address,
            }),
        );

        assert_eq!(
            map(&harness.context, &harness.src),
            "let $foo := alloc(320)"
        );
    }
}
