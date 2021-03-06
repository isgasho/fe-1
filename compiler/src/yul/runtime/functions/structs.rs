use crate::yul::names;
use fe_analyzer::namespace::types::Struct;
use yultsur::*;

/// Generate a YUL function that can be used to create an instance of
/// `struct_type`
pub fn generate_new_fn(struct_type: &Struct) -> yul::Statement {
    let function_name = names::struct_new_call(&struct_type.name);

    if struct_type.is_empty() {
        // We return 0 here because it is safe to assume that we never write to an empty
        // struct. If we end up writing to an empty struct that's an actual Fe
        // bug.
        let body = statement! { return_val := 0 };
        return function_definition! {
            function [function_name]() -> return_val {
                 [body]
            }
        };
    }

    let params = struct_type
        .get_field_names()
        .iter()
        .map(|key| {
            identifier! {(key)}
        })
        .collect::<Vec<_>>();

    let body = struct_type
        .get_field_names()
        .iter()
        .enumerate()
        .map(|(index, key)| {
            if index == 0 {
                let param_identifier_exp = identifier_expression! {(key)};
                statements! {
                    (return_val := alloc(32))
                    (mstore(return_val, [param_identifier_exp]))
                }
            } else {
                let ptr_identifier = format!("{}_ptr", key);
                let ptr_identifier = identifier! {(ptr_identifier)};
                let ptr_identifier_exp = identifier_expression! {(ptr_identifier)};
                let param_identifier_exp = identifier_expression! {(key)};
                statements! {
                    (let [ptr_identifier] := alloc(32))
                    (mstore([ptr_identifier_exp], [param_identifier_exp]))
                }
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    function_definition! {
        function [function_name]([params...]) -> return_val {
            [body...]
        }
    }
}

/// Generate a YUL function that can be used to read a property of `struct_type`
pub fn generate_get_fn(struct_type: &Struct, field_name: &str) -> yul::Statement {
    let function_name = names::struct_getter_call(&struct_type.name, field_name);
    let field_index = struct_type
        .get_field_index(field_name)
        .unwrap_or_else(|| panic!("No field {} in {}", field_name, struct_type.name));
    let field_offset = field_index * 32;

    let offset = literal_expression! {(field_offset)};
    let return_expression = expression! { add(ptr, [offset]) };
    let body = statement! { (return_val := [return_expression]) };
    function_definition! {
        function [function_name](ptr) -> return_val {
             [body]
        }
    }
}

/// Builds a set of functions used to interact with structs used in a contract
pub fn struct_apis(struct_type: Struct) -> Vec<yul::Statement> {
    [
        vec![generate_new_fn(&struct_type)],
        struct_type
            .get_field_names()
            .iter()
            .map(|field| generate_get_fn(&struct_type, &field))
            .collect(),
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use crate::yul::runtime::functions::structs;
    use fe_analyzer::namespace::types::{
        Base,
        Struct,
    };

    #[test]
    fn test_empty_struct() {
        assert_eq!(
            structs::generate_new_fn(&Struct::new("Foo")).to_string(),
            "function struct_Foo_new() -> return_val { return_val := 0 }"
        )
    }

    #[test]
    fn test_struct_api_generation() {
        let mut val = Struct::new("Foo");
        val.add_field("bar", &Base::Bool);
        val.add_field("bar2", &Base::Bool);
        assert_eq!(
            structs::generate_new_fn(&val).to_string(),
            "function struct_Foo_new(bar, bar2) -> return_val { return_val := alloc(32) mstore(return_val, bar) let bar2_ptr := alloc(32) mstore(bar2_ptr, bar2) }"
        )
    }

    #[test]
    fn test_struct_getter_generation() {
        let mut val = Struct::new("Foo");
        val.add_field("bar", &Base::Bool);
        val.add_field("bar2", &Base::Bool);
        assert_eq!(
            structs::generate_get_fn(&val, &val.get_field_names().get(0).unwrap()).to_string(),
            "function struct_Foo_get_bar_ptr(ptr) -> return_val { return_val := add(ptr, 0) }"
        );
        assert_eq!(
            structs::generate_get_fn(&val, &val.get_field_names().get(1).unwrap()).to_string(),
            "function struct_Foo_get_bar2_ptr(ptr) -> return_val { return_val := add(ptr, 32) }"
        );
    }
}
