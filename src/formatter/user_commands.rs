use std::collections::HashMap;

use crate::SyntaxNode;
use crate::cst::CommandInvocation;
use crate::syntax_kind::SyntaxKind;

/// Scan top-level function()/macro() definitions and return a map of
/// lowercased name -> original casing as written in the definition.
///
/// Example: `function(MyHelper ...)` produces `"myhelper" -> "MyHelper"`.
pub fn scan_user_command_definitions(root: &SyntaxNode) -> HashMap<String, String> {
    let mut defs = HashMap::new();

    for child in root.children() {
        if child.kind() != SyntaxKind::COMMAND_INVOCATION {
            continue;
        }
        let Some(cmd) = CommandInvocation::cast(child) else {
            continue;
        };
        let Some(name) = cmd.name_text() else {
            continue;
        };
        let name_lower = name.to_lowercase();
        if name_lower != "function" && name_lower != "macro" {
            continue;
        }
        // First argument is the user-defined command name
        let Some(arg_list) = cmd.argument_list() else {
            continue;
        };
        let Some(first_arg) = arg_list.arguments().next() else {
            continue;
        };
        let defined_name = first_arg.text().to_string();
        defs.insert(defined_name.to_lowercase(), defined_name);
    }

    defs
}
