use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use reconstruct::strip_unused_labels;

use crate::{
    Res,
    ast::{
        Ast, AstNode, BinOp, BinaryExpression, BlockStatement, CommentAstNode, Constant, ConstantExpression, Expression, FunctionCallExpression,
        FunctionDeclarationAstNode, FunctionImplementation, GosubStatement, GotoStatement, IdentifierExpression, IfStatement, IndexerExpression,
        LabelStatement, LetStatement, ParameterSpecifier, ParensExpression, PredefinedCallStatement, ProcedureCallStatement, ProcedureDeclarationAstNode,
        ProcedureImplementation, Statement, UnaryExpression, UnaryOp, VariableDeclarationStatement, VariableParameterSpecifier, VariableSpecifier,
        constant::NumberFormat,
    },
    compiler::workspace::Workspace,
    executable::{
        DeserializationError, DeserializationErrorType, EntryType, Executable, OpCode, PPECommand, PPEExpr, PPEScript, PPEVisitor, StatementDefinition,
        TableEntry, VariableType,
    },
    parser::{ErrorReporter, UserTypeRegistry, lexer::Token},
    semantic::SemanticVisitor,
};

use self::evaluation_visitor::OptimizationVisitor;

pub mod evaluation_visitor;
pub mod reconstruct;
pub mod relabel_visitor;
pub mod rename_visitor;

#[cfg(test)]
pub mod test_evaluation_visitor;

pub struct DecompilerIssue {
    pub byte_offset: usize,
    pub bug: DeserializationErrorType,
}

#[derive(Default)]
pub struct Decompiler {
    executable: Executable,
    script: PPEScript,

    functions: Vec<AstNode>,

    label_lookup: HashMap<usize, usize>,
    used_labels: HashSet<usize>,

    function_lookup: HashMap<usize, usize>,
    cur_ptr: usize,
    issues: Vec<DecompilerIssue>,
    optimize_output: bool,
}

impl Decompiler {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn new(executable: Executable, optimize_output: bool) -> Result<Self, DeserializationError> {
        let script = PPEScript::from_ppe_file(&executable)?;
        Ok(Self {
            executable,
            script,
            label_lookup: HashMap::new(),
            function_lookup: HashMap::new(),
            used_labels: HashSet::new(),
            functions: Vec::new(),
            cur_ptr: 0,
            issues: Vec::new(),
            optimize_output,
        })
    }

    fn analyze_labels(&mut self) -> HashMap<usize, usize> {
        let mut labels = HashSet::new();

        for statement in &self.script.statements {
            match statement.command {
                PPECommand::Goto(label) | PPECommand::Gosub(label) | PPECommand::IfNot(_, label) => {
                    labels.insert(label);
                }
                _ => {}
            }
        }
        let mut label_list = labels.into_iter().collect::<Vec<usize>>();
        label_list.sort();

        let mut label_offsets = HashMap::new();
        for (i, label) in label_list.iter().enumerate() {
            label_offsets.insert(*label, i);
        }
        label_offsets
    }

    /// Returns the decompile of this [`Decompiler`].
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn decompile(&mut self) -> Res<Ast> {
        self.label_lookup = self.analyze_labels();

        {
            let mut visitor = VariableConstantVisitor {
                executable: &mut self.executable,
            };
            self.script.visit(&mut visitor);
        }

        self.executable.variable_table.analyze_usage(&self.script);
        self.executable.variable_table.analyze_locals();
        self.executable.variable_table.generate_names();

        let mut ast = Ast::default();

        self.generate_function_declarations(&mut ast);
        self.generate_global_variable_declarations(&mut ast);

        let mut statements = Vec::new();
        while self.cur_ptr < self.script.statements.len() {
            let statement = &self.script.statements[self.cur_ptr];
            let byte_offset = statement.span.start * 2;

            if let Some(func) = self.function_lookup.get(&byte_offset) {
                self.parse_function(*func);
                continue;
            }

            if self.label_lookup.contains_key(&(byte_offset)) {
                let label = self.get_label_name(byte_offset);
                self.used_labels.insert(byte_offset);
                statements.push(LabelStatement::create_empty_statement(label));
            }
            if let Some(bugs) = self.script.bugged_offsets.get_mut(&statement.span.start) {
                for bug in bugs.drain(..) {
                    self.issues.push(DecompilerIssue { byte_offset, bug: bug.clone() });
                    statements.push(CommentAstNode::create_empty_statement(format!(
                        " PPLC bug use detected in next statement: {bug}"
                    )));
                }
            }
            statements.push(self.decompile_statement(&statement.command));
            self.cur_ptr += 1;
        }
        while let Some(Statement::PredifinedCall(c)) = statements.last() {
            if c.get_func().opcode != OpCode::END || statements.len() <= 1 {
                break;
            }
            statements.pop();
        }

        if !self.functions.is_empty() {
            statements.push(PredefinedCallStatement::create_empty_statement(OpCode::END.get_definition(), Vec::new()));
        }

        // Generate exit label - there is a case where this is needed
        // Gets removed if not used
        statements.push(LabelStatement::create_empty_statement(unicase::Ascii::new("EXIT_LABEL".to_string())));
        for (offset, _i) in &self.label_lookup {
            if !self.used_labels.contains(offset) {
                statements.push(LabelStatement::create_empty_statement(self.get_label_name(*offset)));
            }
        }
        ast.nodes.push(AstNode::Main(BlockStatement::empty(statements)));

        ast.nodes.append(&mut self.functions);

        for (k, bugs) in &self.script.bugged_offsets {
            for bug in bugs {
                ast.nodes.push(AstNode::TopLevelStatement(CommentAstNode::create_empty_statement(format!(
                    "{k:04X}: statement: {bug}"
                ))));
            }
        }

        if !self.script.bugged_offsets.is_empty() {
            ast.nodes.push(AstNode::TopLevelStatement(CommentAstNode::create_empty_statement(format!(
                " {} error(s) detected while decompiling",
                self.script.bugged_offsets.len(),
            ))));
        }

        Ok(ast)
    }

    fn generate_global_variable_declarations(&mut self, ast: &mut Ast) {
        for var in self.executable.variable_table.get_entries() {
            if let EntryType::Variable = var.entry_type {
                let var_decl = generate_variable_declaration(var);
                ast.nodes.push(AstNode::TopLevelStatement(var_decl));
            }
        }
    }

    fn generate_function_declarations(&mut self, ast: &mut Ast) {
        for entry in self.executable.variable_table.get_entries() {
            match entry.entry_type {
                EntryType::Function | EntryType::Procedure => {
                    // offset == 0 seems to be a bug used for preventing decompilation
                    if unsafe { entry.value.data.procedure_value.start_offset } == 0 {
                        continue;
                    }
                    self.function_lookup
                        .insert(unsafe { entry.value.data.procedure_value.start_offset as usize }, entry.header.id);

                    if entry.header.variable_type == VariableType::Function {
                        let parameters = self.generate_parameter_list(entry);
                        let return_value = self
                            .executable
                            .variable_table
                            .get_var_entry(unsafe { entry.value.data.function_value.return_var as usize });
                        let func_decl =
                            FunctionDeclarationAstNode::empty(unicase::Ascii::new(entry.name.clone()), parameters, return_value.header.variable_type);
                        ast.nodes.push(AstNode::FunctionDeclaration(func_decl));
                    } else {
                        let parameters = self.generate_parameter_list(entry);
                        let proc_decl = ProcedureDeclarationAstNode::empty(unicase::Ascii::new(entry.name.clone()), parameters);
                        ast.nodes.push(AstNode::ProcedureDeclaration(proc_decl));
                    }
                }
                _ => {}
            }
        }
    }

    fn decompile_expression(&self, expression: &PPEExpr) -> Expression {
        match expression {
            PPEExpr::Invalid => todo!(),
            PPEExpr::Value(id) => unsafe {
                let Some(entry) = self.executable.variable_table.try_get_entry(*id) else {
                    return ConstantExpression::create_empty_expression(Constant::String(format!("ERROR IN EXPRESSION can't read table index : {:04X}", *id)));
                };
                if entry.entry_type == EntryType::Constant {
                    let constant = match entry.value.get_type() {
                        VariableType::BigStr | VariableType::String => Constant::String(entry.value.as_string()),
                        VariableType::Float => Constant::Double(entry.value.data.float_value as f64),
                        VariableType::Double => Constant::Double(entry.value.data.double_value),
                        VariableType::Boolean => Constant::Boolean(entry.value.as_bool()),
                        VariableType::Unsigned => Constant::Unsigned(entry.value.data.unsigned_value),
                        //VariableType::Integer |
                        _ => Constant::Integer(entry.value.as_int(), NumberFormat::Default),
                    };
                    ConstantExpression::create_empty_expression(constant)
                } else {
                    IdentifierExpression::create_empty_expression(unicase::Ascii::new(entry.name.clone()))
                }
            },
            PPEExpr::Member(_expr, _id) => {
                todo!()
            }
            PPEExpr::MemberFunctionCall(_, _, _) => {
                todo!()
            }
            PPEExpr::UnaryExpression(op, expr) => {
                let mut expr = self.decompile_expression(expr);

                if matches!(expr, Expression::Binary(_)) {
                    expr = ParensExpression::create_empty_expression(expr);
                }

                let expr = UnaryExpression::create_empty_expression(*op, expr);
                if self.optimize_output {
                    expr.visit_mut(&mut OptimizationVisitor::default())
                } else {
                    expr
                }
            }
            PPEExpr::BinaryExpression(op, left, right) => {
                let left = add_parens_if_required(*op, self.decompile_expression(left));
                let right = add_parens_if_required(*op, self.decompile_expression(right));

                let expr = BinaryExpression::create_empty_expression(*op, left, right);
                if self.optimize_output {
                    expr.visit_mut(&mut OptimizationVisitor::default())
                } else {
                    expr
                }
            }
            PPEExpr::Dim(id, dims) => {
                IndexerExpression::create_empty_expression(self.get_variable_name(*id), dims.iter().map(|e| self.decompile_expression(e)).collect())
            }
            PPEExpr::PredefinedFunctionCall(f, args) => FunctionCallExpression::create_empty_expression(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new(f.name.to_string())),
                args.iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let expr = self.decompile_expression(e);
                        if let Some(args) = &f.args {
                            if let Some(arg) = args.get(i) {
                                return convert_argument(expr, arg);
                            }
                        }
                        expr
                    })
                    .collect(),
            ),
            PPEExpr::FunctionCall(f, args) => FunctionCallExpression::create_empty_expression(
                IdentifierExpression::create_empty_expression(self.get_variable_name(*f)),
                args.iter().map(|e| self.decompile_expression(e)).collect(),
            ),
        }
    }

    fn decompile_statement(&self, statement: &PPECommand) -> Statement {
        match statement {
            PPECommand::EndFunc | PPECommand::EndProc | PPECommand::End => {
                PredefinedCallStatement::create_empty_statement(OpCode::END.get_definition(), Vec::new())
            }
            PPECommand::Return => PredefinedCallStatement::create_empty_statement(OpCode::RETURN.get_definition(), Vec::new()),
            PPECommand::Stop => PredefinedCallStatement::create_empty_statement(OpCode::STOP.get_definition(), Vec::new()),
            PPECommand::Goto(label) => GotoStatement::create_empty_statement(self.get_label_name(*label)),
            PPECommand::Gosub(label) => GosubStatement::create_empty_statement(self.get_label_name(*label)),

            PPECommand::IfNot(expr, label) => {
                let expr = self.decompile_expression(expr);
                let expr = Statement::try_boolean_conversion(&expr);
                IfStatement::create_empty_statement(expr.negate_expression(), GotoStatement::create_empty_statement(self.get_label_name(*label)))
            }
            PPECommand::ProcedureCall(p, args) => {
                ProcedureCallStatement::create_empty_statement(self.get_variable_name(*p), args.iter().map(|e| self.decompile_expression(e)).collect())
            }
            PPECommand::PredefinedCall(p, args) => PredefinedCallStatement::create_empty_statement(
                p,
                args.iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let expr = self.decompile_expression(e);
                        if let Some(args) = &p.args {
                            if let Some(arg) = args.get(i) {
                                return convert_argument(expr, arg);
                            }
                        }
                        expr
                    })
                    .collect(),
            ),
            PPECommand::Let(left, expr) => {
                let (identifier, arguments) = match self.decompile_expression(left) {
                    Expression::FunctionCall(f) => (unicase::Ascii::new(f.get_expression().to_string()), f.get_arguments().clone()),
                    Expression::Identifier(id) => (id.get_identifier().clone(), Vec::new()),
                    Expression::Indexer(f) => (unicase::Ascii::new(f.get_identifier().to_string()), f.get_arguments().clone()),

                    x => panic!("Invalid expression {x:?}"),
                };
                let id = left.get_id().unwrap();
                let mut value_expr = self.decompile_expression(expr);

                if self.executable.variable_table.get_var_entry(id).header.variable_type == VariableType::Boolean {
                    value_expr = Statement::try_boolean_conversion(&value_expr);
                }

                LetStatement::create_empty_statement(identifier, Token::Eq, arguments, value_expr)
            }
        }
    }

    fn get_label_name(&self, label: usize) -> unicase::Ascii<String> {
        if let Some(name) = self.label_lookup.get(&label) {
            unicase::Ascii::new(format!("LABEL{:03}", *name + 1))
        } else {
            unicase::Ascii::new("EXIT_LABEL".to_string())
        }
    }

    fn get_variable_name(&self, p: usize) -> unicase::Ascii<String> {
        unicase::Ascii::new(self.executable.variable_table.get_var_entry(p).name.clone())
    }

    fn parse_function(&mut self, func: usize) {
        let entry = self.executable.variable_table.get_var_entry(func);
        let mut func_body = self.generate_local_variable_declarations(entry);
        while self.cur_ptr < self.script.statements.len() {
            let statement = &self.script.statements[self.cur_ptr];
            let byte_offset = statement.span.start * 2;
            if self.label_lookup.contains_key(&(byte_offset)) {
                self.used_labels.insert(byte_offset);
                func_body.push(LabelStatement::create_empty_statement(self.get_label_name(byte_offset)));
            }

            if matches!(statement.command, PPECommand::EndFunc) || matches!(statement.command, PPECommand::EndProc) {
                if entry.header.variable_type == VariableType::Function {
                    let parameters = self.generate_parameter_list(entry);
                    let return_value = self
                        .executable
                        .variable_table
                        .get_var_entry(unsafe { entry.value.data.function_value.return_var as usize });
                    let func_impl = FunctionImplementation::empty(
                        func,
                        unicase::Ascii::new(entry.name.clone()),
                        parameters,
                        return_value.header.variable_type,
                        func_body,
                    );
                    self.functions.push(AstNode::Function(func_impl));
                } else {
                    let parameters = self.generate_parameter_list(entry);
                    let proc_impl = ProcedureImplementation::empty(func, unicase::Ascii::new(entry.name.clone()), parameters, func_body);
                    self.functions.push(AstNode::Procedure(proc_impl));
                }
                self.cur_ptr += 1;
                break;
            }

            func_body.push(self.decompile_statement(&statement.command));
            self.cur_ptr += 1;
        }

        if self.cur_ptr < self.script.statements.len() && self.script.statements[self.cur_ptr].command == PPECommand::End {
            self.cur_ptr += 1;
        }
    }

    fn generate_local_variable_declarations(&self, entry: &TableEntry) -> Vec<Statement> {
        unsafe {
            let mut decl = Vec::new();

            let (start, end) = if entry.header.variable_type == VariableType::Function {
                let start = entry.value.data.function_value.first_var_id as usize + entry.value.data.function_value.parameters as usize + 1;
                (start, start + entry.value.data.function_value.local_variables as usize - 1)
            } else {
                let start = entry.value.data.function_value.first_var_id as usize + entry.value.data.function_value.parameters as usize + 1;
                (start, start + entry.value.data.function_value.local_variables as usize)
            };

            for i in start..end {
                let local_var = self.executable.variable_table.get_var_entry(i);
                if local_var.entry_type == EntryType::LocalVariable {
                    decl.push(generate_variable_declaration(local_var));
                }
            }

            decl
        }
    }

    fn generate_parameter_list(&self, entry: &TableEntry) -> Vec<ParameterSpecifier> {
        unsafe {
            let mut parameters = Vec::new();

            let to;
            let pass_flags;
            let first_var;

            if entry.header.variable_type == VariableType::Function {
                to = entry.value.data.function_value.parameters as usize;
                first_var = entry.value.data.function_value.first_var_id as usize;
                pass_flags = 0;
            } else {
                to = entry.value.data.procedure_value.parameters as usize;
                pass_flags = entry.value.data.procedure_value.pass_flags;
                first_var = entry.value.data.procedure_value.first_var_id as usize;
            }

            for i in 0..to {
                let param = self.executable.variable_table.get_var_entry(first_var + 1 + i);
                let mut dimensions = Vec::new();
                match param.header.dim {
                    1 => {
                        dimensions.push(param.header.vector_size);
                    }
                    2 => {
                        dimensions.push(param.header.vector_size);
                        dimensions.push(param.header.matrix_size);
                    }
                    3 => {
                        dimensions.push(param.header.vector_size);
                        dimensions.push(param.header.matrix_size);
                        dimensions.push(param.header.cube_size);
                    }
                    _ => {}
                }
                let is_var = pass_flags & (1 << i) != 0;
                parameters.push(ParameterSpecifier::Variable(VariableParameterSpecifier::empty(
                    is_var,
                    param.header.variable_type,
                    Some(VariableSpecifier::empty(unicase::Ascii::new(param.name.clone()), dimensions)),
                )));
            }

            parameters
        }
    }
}

fn convert_argument(expr: Expression, arg: &crate::executable::ArgumentDefinition) -> Expression {
    if arg.arg_type == VariableType::Boolean {
        return Statement::try_boolean_conversion(&expr);
    }
    if arg.number_format == NumberFormat::Hex {
        if let Expression::Const(c) = &expr {
            if let Constant::Integer(i, _) = c.get_constant_value() {
                return ConstantExpression::create_empty_expression(Constant::Integer(*i, NumberFormat::ColorCode));
            }
        }
    }
    arg.flags.convert_expr(expr)
}

fn add_parens_if_required(op: BinOp, expr: Expression) -> Expression {
    let add_parens = if let Expression::Binary(bin_op) = &expr {
        bin_op.get_op().get_priority() < op.get_priority()
    } else {
        false
    };

    if add_parens { ParensExpression::create_empty_expression(expr) } else { expr }
}

fn generate_variable_declaration(var: &TableEntry) -> Statement {
    let dims = match var.header.dim {
        1 => {
            vec![var.header.vector_size]
        }
        2 => {
            vec![var.header.vector_size, var.header.matrix_size]
        }
        3 => {
            vec![var.header.vector_size, var.header.matrix_size, var.header.cube_size]
        }
        _ => Vec::new(),
    };
    VariableDeclarationStatement::create_empty_statement(
        var.header.variable_type,
        vec![VariableSpecifier::empty(unicase::Ascii::new(var.name.clone()), dims)],
    )
}

/// .
/// # Errors
/// # Panics
///
/// Panics if .
pub fn decompile(executable: Executable, raw: bool) -> Res<(Ast, Vec<DecompilerIssue>)> {
    match Decompiler::new(executable, !raw) {
        Ok(mut d) => {
            let mut ast = d.decompile()?;

            let reg = UserTypeRegistry::default();
            let errors: Arc<std::sync::Mutex<crate::parser::ErrorReporter>> = Arc::new(Mutex::new(ErrorReporter::default()));
            let mut visitor = SemanticVisitor::new(&Workspace::default(), errors.clone(), reg);
            ast.visit(&mut visitor);
            visitor.finish();

            if !raw {
                for node in &mut ast.nodes {
                    match node {
                        AstNode::Function(f) => {
                            reconstruct::reconstruct_block(&visitor, f.get_statements_mut());
                        }
                        AstNode::Procedure(p) => {
                            reconstruct::reconstruct_block(&visitor, p.get_statements_mut());
                        }
                        AstNode::Main(block) => {
                            reconstruct::reconstruct_block(&visitor, block.get_statements_mut());
                        }
                        _ => {}
                    }
                }
                ast = reconstruct::finish_ast(&mut ast);
                ast = strip_unused_labels(&mut ast);
                ast = relabel_visitor::relabel_ast(&mut ast);
            }

            Ok((ast, d.issues))
        }
        Err(err) => Err(Box::new(err.error_type)),
    }
}

struct VariableConstantVisitor<'a> {
    executable: &'a mut Executable,
}

impl<'a> PPEVisitor<()> for VariableConstantVisitor<'a> {
    fn visit_dim_expression(&mut self, id: usize, dim: &[PPEExpr]) -> () {
        // Changes CONST[expr] to VAR[expr]
        // There are some files out there that try to change the entry typ to constant for DIM variables
        if self.executable.variable_table.get_var_entry(id).entry_type == EntryType::Constant {
            self.executable.variable_table.get_var_entry_mut(id).entry_type = EntryType::Variable;
        }
        for d in dim {
            d.visit(self);
        }
    }

    fn visit_value(&mut self, _id: usize) -> () {}
    fn visit_member(&mut self, expr: &PPEExpr, _id: usize) -> () {
        expr.visit(self)
    }
    fn visit_unary_expression(&mut self, _op: UnaryOp, expr: &PPEExpr) -> () {
        expr.visit(self)
    }
    fn visit_binary_expression(&mut self, _op: BinOp, left: &PPEExpr, right: &PPEExpr) -> () {
        left.visit(self);
        right.visit(self);
    }

    fn visit_predefined_function_call(&mut self, _def: &crate::executable::FunctionDefinition, arguments: &[PPEExpr]) -> () {
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_function_call(&mut self, _id: usize, arguments: &[PPEExpr]) -> () {
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_member_function_call(&mut self, _expr: &PPEExpr, arguments: &[PPEExpr], _id: usize) -> () {
        for arg in arguments {
            arg.visit(self);
        }
    }

    fn visit_end(&mut self) -> () {}
    fn visit_return(&mut self) -> () {}
    fn visit_if(&mut self, cond: &PPEExpr, _label: &usize) -> () {
        cond.visit(self);
    }
    fn visit_proc_call(&mut self, _id: usize, arguments: &[PPEExpr]) -> () {
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_predefined_call(&mut self, def: &StatementDefinition, arguments: &[PPEExpr]) -> () {
        match def.sig {
            crate::executable::StatementSignature::Invalid => {}
            crate::executable::StatementSignature::ArgumentsWithVariable(_, _) => {}
            crate::executable::StatementSignature::VariableArguments(_, _, _) => {}
            crate::executable::StatementSignature::SpecialCaseDlockg => {}
            crate::executable::StatementSignature::SpecialCaseDcreate => {}
            crate::executable::StatementSignature::SpecialCaseSort => {
                // Ensure that #1 sort argument is a variable
                if let PPEExpr::Value(id) = &arguments[0] {
                    if self.executable.variable_table.get_var_entry(*id).entry_type == EntryType::Constant {
                        self.executable.variable_table.get_var_entry_mut(*id).entry_type = EntryType::Variable;
                    }
                }
            }
            crate::executable::StatementSignature::SpecialCaseVarSeg => {}
            crate::executable::StatementSignature::SpecialCasePop => {}
        }
        for arg in arguments {
            arg.visit(self);
        }
    }
    fn visit_goto(&mut self, _label: &usize) -> () {}
    fn visit_gosub(&mut self, _label: &usize) -> () {}
    fn visit_end_func(&mut self) -> () {}
    fn visit_end_proc(&mut self) -> () {}
    fn visit_stop(&mut self) -> () {}
    fn visit_let(&mut self, target: &PPEExpr, value: &PPEExpr) -> () {
        target.visit(self);
        value.visit(self)
    }
}
