use crate::{context::Context, values::Value};

use crate::bytecode::opcode;

use oxc_ast::ast::{self, Program};

pub struct Compiler<'ctx> {
  code: Vec<usize>,
  name: String,
  constants: Vec<Value>,
  ctx: &'ctx mut Context,
}

#[allow(dead_code)]
pub struct CompilerReturn {
  name: String,
  pub code: Vec<usize>,
  pub constants: Vec<Value>,
}

#[allow(dead_code)]
impl<'ctx> Compiler<'ctx> {
  fn new(name: String, ctx: &'ctx mut Context) -> Self {
    Self { name, code: Vec::new(), constants: Vec::new(), ctx }
  }
  pub fn compile(program: &Program, ctx: &'ctx mut Context) -> CompilerReturn {
    let mut compiler = Compiler::new("main".to_string(), ctx);
    compiler.generate(program);
    CompilerReturn { name: compiler.name, code: compiler.code, constants: compiler.constants }
  }

  pub fn generate(&mut self, program: &Program) -> () {
    for statement in program.body.iter() {
      self.generate_statement(statement);
    }
    // end of program
    self.code.push(opcode::OPCODE_HALF);
  }

  pub fn generate_statement(&mut self, statement: &ast::Statement) {
    match statement {
      ast::Statement::ExpressionStatement(stmt) => {
        self.generate_expression(&stmt.expression);
      }
      ast::Statement::Declaration(decl) => {
        self.generate_declaration(decl);
      }
      ast::Statement::IfStatement(stmt) => {
        self.generate_if_statement(stmt);
      }
      ast::Statement::EmptyStatement(_) => {
        self.generate_empty_statement();
      }
      ast::Statement::BlockStatement(stmt) => {
        self.generate_block_statement(stmt);
      }
      _ => {
        print!("{:?}", statement);
        panic!("Unknown statement")
      }
    }
  }
  pub fn generate_block_statement(&mut self, statement: &ast::BlockStatement) {
    for stmt in statement.body.iter() {
      self.generate_statement(stmt);
    }
  }
  pub fn generate_declaration(&mut self, declaration: &ast::Declaration) {
    match declaration {
      ast::Declaration::VariableDeclaration(decl) => {
        self.generate_variable_declaration(decl);
      }
      _ => {
        panic!("Unknown declaration")
      }
    }
  }

  pub fn generate_if_statement(&mut self, statement: &ast::IfStatement) {
    // 1. check the condition
    self.generate_expression(&statement.test);
    // 2. jump if false
    self.emit(opcode::OPCODE_JUMP_IF_FALSE);
    // 3. jump address to the consequent
    let jump_if_false_address = self.code.len();
    // 4. emit 0, we will fill this later
    self.emit(0);
    // 5. generate the consequent
    self.generate_statement(&statement.consequent);
    // 6. jump to the end of the if statement
    self.emit(opcode::OPCODE_JUMP);
    // 7. jump address to the end of the if statement
    let jump_address = self.code.len();
    // 8. emit 0, we will fill this later
    self.emit(0);
    // 9. fill the jump if false address
    self.code[jump_if_false_address] = self.code.len();
    // 10. generate the alternate if it exists
    if let Some(alternate) = &statement.alternate {
      // 11. generate the alternate
      self.generate_statement(alternate);
    }
    // 12. fill the jump address
    self.code[jump_address] = self.code.len();
  }

  pub fn generate_variable_declaration(&mut self, declaration: &ast::VariableDeclaration) {
    match declaration.kind {
      ast::VariableDeclarationKind::Let => self.generate_let_variable_declaration(declaration),
      _ => {
        panic!("Unknown variable declaration kind")
      }
    }
  }
  pub fn generate_let_variable_declaration(&mut self, declaration: &ast::VariableDeclaration) {
    for declarator in declaration.declarations.iter() {
      self._binding_pattern(&declarator.id, &declarator.init);
    }
  }
  // !todo: we need to return the index of the variable for make more efficient to get the variable?
  pub fn _binding_pattern(&mut self, pattern: &ast::BindingPattern, init: &Option<ast::Expression>) {
    match &pattern.kind {
      ast::BindingPatternKind::BindingIdentifier(ident) => {
        let idx = self.ctx.define_variable(ident.name.as_str().to_owned(), None);
        self.declarator_init(init, idx);
      }
      ast::BindingPatternKind::ArrayPattern(elem) => {
        for element in &elem.elements {
          if let Some(element) = element {
            self._binding_pattern(&element, init);
          }
        }
      }
      ast::BindingPatternKind::ObjectPattern(objects) => {
        for property in &objects.properties {
          match &property.key {
            ast::PropertyKey::Identifier(ident) => {
              let idx = self.ctx.define_variable(ident.name.as_str().to_owned(), None);
              self.declarator_init(init, idx);
            }
            // ast::PropertyKey::PrivateIdentifier(ident) => {
            //   self.ctx.define_variable(ident.name.as_str().to_owned(), None);
            // }
            ast::PropertyKey::Expression(_) => {
              panic!("Expression key not supported")
            }
            _ => {
              panic!("Unknown property key")
            }
          }
        }
      }
      ast::BindingPatternKind::AssignmentPattern(_) => {
        panic!("Assignment pattern not supported")
      }
    }
  }

  pub fn declarator_init(&mut self, init: &Option<ast::Expression>, idx: usize) {
    if let Some(init) = init {
      self.generate_expression(&init);
      self.emit(opcode::OPCODE_SET_CONTEXT);
      self.emit(idx);
    }
  }
  pub fn generate_empty_statement(&mut self) {
    // We want to generate a half opcode here? huh... I don't know what to do here yet.
    self.emit(opcode::OPCODE_HALF);
  }
  pub fn generate_expression(&mut self, expression: &ast::Expression) {
    match &expression {
      ast::Expression::NumericLiteral(value) => {
        self.generate_numeric_literal(value);
      }
      ast::Expression::BooleanLiteral(value) => {
        self.generate_boolean_literal(value);
      }
      ast::Expression::StringLiteral(literal) => {
        self.generate_string_literal(literal);
      }
      ast::Expression::BinaryExpression(binary) => {
        self.generate_binary_expression(binary);
      }
      ast::Expression::Identifier(identifier) => {
        self.generate_identifier(identifier);
      }
      _ => {
        panic!("Unknown expression")
      }
    }
  }

  pub fn generate_identifier(&mut self, identifier: &ast::IdentifierReference) {
    if let Some(index) = self.ctx.get_variable_index(&identifier.name) {
      self.emit(opcode::OPCODE_LOAD_CONTEXT);
      self.emit(index);
      return;
    }
    if !self.ctx.is_global_variable(&identifier.name) {
      panic!("[Compiler] {} is not implemented yet", identifier.name);
    }
    panic!("[Compiler] Reference Error: {} is not defined", identifier.name);
  }

  pub fn generate_numeric_literal(&mut self, literal: &ast::NumericLiteral) {
    let index = self.numerics_constants_index(literal.value);
    self.emit(opcode::OPCODE_CONST);
    self.emit(index as usize);
  }

  pub fn generate_boolean_literal(&mut self, literal: &ast::BooleanLiteral) {
    self.constants.push(Value::Boolean(literal.value));
    let index = self.constants.len() - 1;
    self.emit(opcode::OPCODE_CONST);
    self.emit(index as usize);
  }

  pub fn generate_string_literal(&mut self, literal: &ast::StringLiteral) {
    let index = self.string_constants_index(literal.value.as_str());
    self.emit(opcode::OPCODE_CONST);
    self.emit(index as usize);
  }

  pub fn emit(&mut self, byte: usize) {
    self.code.push(byte);
  }

  // numeric constants index
  pub fn numerics_constants_index(&mut self, value: f64) -> usize {
    let value = Value::Number(value);
    for (index, current_value) in self.constants.iter().enumerate() {
      // 1. check if the value is a number
      if !current_value.is_number() {
        continue;
      }
      // 2. check if the value is exists in the constants
      if current_value.get_number() == value.get_number() {
        return index;
      }
    }
    // 3. if the value is not exists in the constants, push it
    self.constants.push(value);
    return self.constants.len() - 1;
  }

  // string constants index
  pub fn string_constants_index(&mut self, value: &str) -> usize {
    let value = Value::String(value.to_string());

    // maybe it's not the best way to do this, dont reuse the same constants
    for (index, current_value) in self.constants.iter().enumerate() {
      // 1. check if the value is a string
      if !current_value.is_string() {
        continue;
      }
      // 2. check if the value is exists in the constants
      if current_value.get_string() == value.get_string() {
        return index;
      }
    }
    // 3. if the value is not exists in the constants, push it
    self.constants.push(value);
    return self.constants.len() - 1;
  }

  pub fn generate_binary_expression(&mut self, binary: &ast::BinaryExpression) {
    self.generate_expression(&binary.left);
    self.generate_expression(&binary.right);
    match binary.operator.as_str() {
      "+" => {
        self.emit(opcode::OPCODE_ADD);
      }
      "-" => {
        self.emit(opcode::OPCODE_SUB);
      }
      "*" => {
        self.emit(opcode::OPCODE_MUL);
      }
      "/" => {
        self.emit(opcode::OPCODE_DIV);
      }
      "===" => {
        self.emit(opcode::OPCODE_EQ);
      }
      _ => {
        // **, %, <<, >>, >>>, &, |, ^, ==, !=, ===, !==, <, <=, >, >=, in, instanceof
        panic!("Unknown binary operator")
      }
    }
  }

  // debug disassemble
}
