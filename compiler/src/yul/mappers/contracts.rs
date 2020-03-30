use crate::errors::CompileError;
use crate::yul::mappers::{constructor, functions, types};
use crate::yul::namespace::scopes::{ContractScope, ModuleScope, Scope, Shared, ContractDef};
use crate::yul::namespace::types::{Type, FixedSize};
use crate::yul::runtime::abi as runtime_abi;
use crate::yul::runtime::functions as runtime_functions;

use std::rc::Rc;
use vyper_parser::ast as vyp;
use vyper_parser::span::Spanned;
use yultsur::*;
use crate::yul::namespace::events::Event;

/// Builds a Yul object from a Vyper contract.
pub fn contract_def(
    module_scope: Shared<ModuleScope>,
    name: String,
    body: &Vec<Spanned<vyp::ContractStmt>>,
) -> Result<yul::Statement, CompileError> {
    let contract_scope = ContractScope::new(Rc::clone(&module_scope));

    let mut statements = body
        .iter()
        .map(|stmt| contract_stmt(Rc::clone(&contract_scope), &stmt.node))
        .collect::<Result<Vec<Option<yul::Statement>>, CompileError>>()?
        .into_iter()
        .filter_map(|stmt| stmt)
        .collect::<Vec<yul::Statement>>();

    statements.append(&mut runtime_functions::all());
    statements.push(runtime_abi::switch(
        &contract_scope.borrow().interface,
        &contract_scope.borrow().defs,
    )?);

    Ok(yul::Statement::Object(yul::Object {
        // TODO: use actual name
        name: identifier! { Contract },
        code: constructor::code(),
        objects: vec![yul::Object {
            name: identifier! { runtime },
            code: yul::Code {
                block: yul::Block { statements },
            },
            objects: vec![],
        }],
    }))
}

fn contract_stmt(
    scope: Shared<ContractScope>,
    stmt: &vyp::ContractStmt,
) -> Result<Option<yul::Statement>, CompileError> {
    match stmt {
        vyp::ContractStmt::ContractField { qual, name, typ } => {
            contract_field(scope, qual, name.node.to_string(), &typ.node)?;
            Ok(None)
        }
        vyp::ContractStmt::FuncDef {
            qual,
            name,
            args,
            return_type,
            body,
        } => {
            let function =
                functions::func_def(scope, qual, name.node.to_string(), args, return_type, body)?;
            Ok(Some(function))
        },
        vyp::ContractStmt::EventDef { name, fields } => {
            event_def(scope, name.node.to_string(), fields)?;
            Ok(None)
        }
    }
}

fn contract_field(
    scope: Shared<ContractScope>,
    qual: &Option<Spanned<vyp::ContractFieldQual>>,
    name: String,
    typ: &vyp::TypeDesc,
) -> Result<(), CompileError> {
    match types::type_desc(Scope::Contract(Rc::clone(&scope)), typ)? {
        Type::Map(map) => scope.borrow_mut().add_map(name, map),
        _ => {
            return Err(CompileError::static_str(
                "Contract field type not supported",
            ))
        }
    };

    Ok(())
}

fn event_def(
    scope: Shared<ContractScope>,
    name: String,
    fields: &Vec<Spanned<vyp::EventField>>,
) -> Result<(), CompileError> {
    let fields = fields
        .iter()
        .map(|f| event_field(Rc::clone(&scope), &f.node))
        .collect::<Result<Vec<FixedSize>, CompileError>>()?;

    scope.borrow_mut().add_event(name, Event { fields });

    Ok(())
}

fn event_field(
    scope: Shared<ContractScope>,
    field: &vyp::EventField,
) -> Result<FixedSize, CompileError> {
    types::type_desc_fixed_size(Scope::Contract(scope), &field.typ.node)
}
