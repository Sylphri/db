#[cfg(test)]
mod tests {
    use crate::*;
    
    // --- parse_table_schema() ---
    #[test]
    fn valid_table_schema() {
        let schema = parse_table_schema("./src/tests_input/valid_table_schema.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
        let schema = schema.unwrap();
        assert!(schema.name == "TestTable");
        assert!(schema.cols.len() == 3);
        assert!(schema.cols[0].0 == "id");
        assert!(schema.cols[0].1 == ColType::Int);
        assert!(schema.cols[1].0 == "name");
        assert!(schema.cols[1].1 == ColType::Str);
        assert!(schema.cols[2].0 == "age");
        assert!(schema.cols[2].1 == ColType::Int);
    }

    #[test]
    #[should_panic(expected = "ERROR: table name can't be empty: ./src/tests_input/schema_with_empty_table_name.tbls")]
    fn schema_with_empty_table_name() {
        let schema = parse_table_schema("./src/tests_input/schema_with_empty_table_name.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
    } 

    #[test]
    #[should_panic(expected = "ERROR: column with name 'id' already exists in table scheme: ./src/tests_input/double_column_declaration.tbls")]
    fn double_column_declaration() {
        let schema = parse_table_schema("./src/tests_input/double_column_declaration.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
    }

    #[test]
    #[should_panic(expected = "ERROR: unknown column type at line 1 in a file ./src/tests_input/invalid_column_type.tbls")]
    fn invalid_column_type() {
        let schema = parse_table_schema("./src/tests_input/invalid_column_type.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
    }
    
    #[test]
    #[should_panic(expected = "ERROR: table name not provided in a file ./src/tests_input/empty_table_schema.tbls")]
    fn empty_table_schema() {
        let schema = parse_table_schema("./src/tests_input/empty_table_schema.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
    }
    
    #[test]
    #[should_panic(expected = "ERROR: invalid format for column at line 1 in a file: ./src/tests_input/invalid_column_format.tbls")]
    fn invalid_column_format() {
        let schema = parse_table_schema("./src/tests_input/invalid_column_format.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
    }
    
    #[test]
    #[should_panic(expected = "ERROR: empty column name at line 1 in a file ./src/tests_input/empty_column_name.tbls")]
    fn empty_column_name() {
        let schema = parse_table_schema("./src/tests_input/empty_column_name.tbls");
        if let Err(ref err) = schema {
            assert!(false, "{}", err);
        }
    }

    // --- parse_query() ---
    #[test]
    fn valid_query() {
        let query = "id name select id 10 > filter-and";
        let expected = vec![
            Token::Word(WordType::Str(String::from("id"))),
            Token::Word(WordType::Str(String::from("name"))),
            Token::Op(OpType::Select),
            Token::Word(WordType::Str(String::from("id"))),
            Token::Word(WordType::Int(10)),
            Token::Op(OpType::More),
            Token::Op(OpType::FilterAnd),
        ];
        match parse_query(query) {
            Ok(tokens) => assert!(expected == tokens),
            Err(err)   => assert!(false, "{}", err),
        }
        
        let query = "id 5 != name \"John Watson\" == delete";
        let expected = vec![
            Token::Word(WordType::Str(String::from("id"))),
            Token::Word(WordType::Int(5)),
            Token::Op(OpType::NotEqual),
            Token::Word(WordType::Str(String::from("name"))),
            Token::Word(WordType::Str(String::from("John Watson"))),
            Token::Op(OpType::Equal),
            Token::Op(OpType::Delete),
        ];
        match parse_query(query) {
            Ok(tokens) => assert!(expected == tokens),
            Err(err)   => assert!(false, "{}", err),
        }
    }

    #[test]
    #[should_panic(expected = "ERROR: unclosed string literal in a query")]
    fn unclosed_string() {
        let query = "3 \"John Watson 20 insert";
        if let Err(err) = parse_query(query) {
            assert!(false, "{}", err);
        }
    }

    // --- logical_op_check() ---
    #[test]
    fn valid_logical_op() {
        let words = vec![ 
            WordType::Str("name".to_string()), 
            WordType::Str("John".to_string()),
        ];
        let table = Table {
            schema: TableSchema {
                name: "test".to_string(),
                cols: vec![("name".to_string(), ColType::Str)],
            },
            rows: vec![],
        };
        let expected = Condition {
            idx: 0,
            value: WordType::Str("John".to_string()),
            op: OpType::Equal,
        };
        assert!(expected == logical_op_check(OpType::Equal, &words, &table).unwrap());
    }

    #[test]
    #[should_panic(expected = "ERROR: not enough arguments for `==` operation, provided 1 but needed 2")]
    fn one_argument_for_logic_op() {
        let words = vec![WordType::Str("name".to_string())];
        let table = Table {
            schema: TableSchema {
                name: "test".to_string(),
                cols: vec![],
            },
            rows: vec![],
        };
        if let Err(err) = logical_op_check(OpType::Equal, &words, &table) {
            assert!(false, "{}", err);
        }
    }

    #[test]
    #[should_panic(expected = "ERROR: invalid argument for `>` operation, expected string but found Int(10)")]
    fn not_string_for_col_name() {
        let words = vec![WordType::Int(10), WordType::Int(5)];
        let table = Table {
            schema: TableSchema {
                name: "test".to_string(),
                cols: vec![],
            },
            rows: vec![],
        };
        if let Err(err) = logical_op_check(OpType::More, &words, &table) {
            assert!(false, "{}", err);
        }
    }
    
    #[test]
    #[should_panic(expected = "ERROR: no such column `age` in table `test`")]
    fn not_existing_column() {
        let words = vec![WordType::Str("age".to_string()), WordType::Int(5)];
        let table = Table {
            schema: TableSchema {
                name: "test".to_string(),
                cols: vec![("id".to_string(), ColType::Int)],
            },
            rows: vec![],
        };
        if let Err(err) = logical_op_check(OpType::More, &words, &table) {
            assert!(false, "{}", err);
        }
    }
    
    #[test]
    #[should_panic(expected = "ERROR: invalid argument for `>` operation expected type Int but found type Str(\"8\")")]
    fn types_mismatch_between_col_and_word() {
        let words = vec![WordType::Str("id".to_string()), WordType::Str("8".to_string())];
        let table = Table {
            schema: TableSchema {
                name: "test".to_string(),
                cols: vec![("id".to_string(), ColType::Int)],
            },
            rows: vec![],
        };
        if let Err(err) = logical_op_check(OpType::More, &words, &table) {
            assert!(false, "{}", err);
        }
    }
}
